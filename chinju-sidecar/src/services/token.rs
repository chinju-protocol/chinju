//! Token Service Implementation (10.1.1)
//!
//! Manages Survival Token balance for the AI system.
//! Implements the core C5 patent concept: AI cannot operate
//! without external token supply.
//!
//! Two components:
//! - `TokenService`: Basic internal token management (used by GatewayService)
//! - `TokenServiceImpl`: Full gRPC service implementing TokenService trait

use crate::gen::chinju::api::token::token_service_server::TokenService as TokenServiceTrait;
use crate::gen::chinju::api::token::*;
use crate::gen::chinju::common::{ErrorDetail, Identifier, Timestamp};
use crate::gen::chinju::token::{
    AiOperatingState, BalanceState, ConsumptionReason, DecayParameters, GrantReason,
    OperatingLimits, SurvivalToken, TokenBalance, TokenConsumption, TokenGrant, TokenMetadata,
};
use crate::ids::RequestId;
use crate::services::signature::ThresholdVerifier;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{Request, Response, Status};
use tracing::{debug, info, warn};
use uuid::Uuid;

// =============================================================================
// Basic Token Service (internal use)
// =============================================================================

/// Token Service for managing AI survival tokens (internal)
///
/// This is the basic implementation used by GatewayService.
/// For the full gRPC service, use `TokenServiceImpl`.
pub struct TokenService {
    /// Current token balance
    balance: u64,
    /// Total tokens consumed
    total_consumed: u64,
    /// Initial balance
    #[allow(dead_code)]
    initial_balance: u64,
}

impl TokenService {
    /// Create a new token service with initial balance
    pub fn new(initial_balance: u64) -> Self {
        info!(initial_balance, "Initializing Token Service");
        Self {
            balance: initial_balance,
            total_consumed: 0,
            initial_balance,
        }
    }

    /// Get current balance
    pub fn get_balance(&self) -> u64 {
        self.balance
    }

    /// Get total consumed
    pub fn total_consumed(&self) -> u64 {
        self.total_consumed
    }

    /// Consume tokens for an operation
    /// Returns true if successful, false if insufficient balance
    pub fn consume(&mut self, amount: u64) -> bool {
        if self.balance >= amount {
            self.balance -= amount;
            self.total_consumed += amount;
            info!(
                amount,
                remaining = self.balance,
                total_consumed = self.total_consumed,
                "Tokens consumed"
            );
            true
        } else {
            warn!(
                requested = amount,
                available = self.balance,
                "Insufficient tokens"
            );
            false
        }
    }

    /// Grant tokens (from authorized source)
    pub fn grant(&mut self, amount: u64) {
        self.balance += amount;
        info!(amount, new_balance = self.balance, "Tokens granted");
    }

    /// Apply decay (tokens naturally decrease over time)
    pub fn apply_decay(&mut self, rate: f64) {
        let decay_amount = (self.balance as f64 * rate) as u64;
        if decay_amount > 0 && self.balance > decay_amount {
            self.balance -= decay_amount;
            info!(decay_amount, remaining = self.balance, "Decay applied");
        }
    }

    /// Check if balance is healthy
    pub fn is_healthy(&self, warning_threshold: u64) -> bool {
        self.balance >= warning_threshold
    }

    /// Check if balance is critical
    pub fn is_critical(&self, minimum: u64) -> bool {
        self.balance <= minimum
    }

    /// Reset for testing
    #[cfg(test)]
    pub fn reset(&mut self) {
        self.balance = self.initial_balance;
        self.total_consumed = 0;
    }
}

impl Default for TokenService {
    fn default() -> Self {
        Self::new(10000) // Default 10k tokens
    }
}

// =============================================================================
// gRPC Token Service Implementation (10.1.1)
// =============================================================================

/// Configuration for TokenServiceImpl
#[derive(Debug, Clone)]
pub struct TokenServiceConfig {
    /// AI system identifier
    pub ai_id: String,
    /// Initial token balance
    pub initial_balance: u64,
    /// Decay rate per second (0.0001 = 0.01% per second)
    pub decay_rate: f64,
    /// Minimum balance before shutdown
    pub minimum_balance: u64,
    /// Warning threshold
    pub warning_threshold: u64,
    /// Large grant threshold (requires threshold signature)
    pub large_grant_threshold: u64,
}

impl Default for TokenServiceConfig {
    fn default() -> Self {
        Self {
            ai_id: "chinju-sidecar-001".to_string(),
            initial_balance: 10000,
            decay_rate: 0.0001,
            minimum_balance: 100,
            warning_threshold: 1000,
            large_grant_threshold: 5000,
        }
    }
}

/// Consumption record for history
#[derive(Clone)]
struct ConsumptionRecord {
    consumption: TokenConsumption,
    reason: ConsumptionReason,
}

/// Token Service gRPC Implementation
///
/// Implements the full TokenService proto API with:
/// - Balance management
/// - Consumption tracking with history
/// - Grant authorization (threshold signature for large grants)
/// - Depletion forecasting
/// - Balance watching (streaming)
pub struct TokenServiceImpl {
    /// Configuration
    config: TokenServiceConfig,
    /// Current balance
    balance: Arc<RwLock<u64>>,
    /// Total consumed
    total_consumed: Arc<RwLock<u64>>,
    /// Consumption history
    history: Arc<RwLock<Vec<ConsumptionRecord>>>,
    /// Grant records
    grants: Arc<RwLock<Vec<TokenGrant>>>,
    /// Idempotency cache (request_id -> consumption_id)
    idempotency_cache: Arc<RwLock<HashMap<RequestId, String>>>,
    /// Threshold verifier for large grants/emergency replenish
    threshold_verifier: Arc<ThresholdVerifier>,
    /// Last decay timestamp
    last_decay_at: Arc<RwLock<i64>>,
    /// Balance watchers (owner_id -> sender)
    watchers:
        Arc<RwLock<HashMap<String, Vec<tokio::sync::mpsc::Sender<Result<BalanceUpdate, Status>>>>>>,
}

impl TokenServiceImpl {
    /// Create a new token service with default config
    pub fn new() -> Self {
        Self::with_config(TokenServiceConfig::default())
    }

    /// Create with custom config
    pub fn with_config(config: TokenServiceConfig) -> Self {
        info!(
            ai_id = %config.ai_id,
            initial_balance = config.initial_balance,
            "Initializing Token Service gRPC implementation"
        );
        Self {
            balance: Arc::new(RwLock::new(config.initial_balance)),
            total_consumed: Arc::new(RwLock::new(0)),
            history: Arc::new(RwLock::new(Vec::new())),
            grants: Arc::new(RwLock::new(Vec::new())),
            idempotency_cache: Arc::new(RwLock::new(HashMap::new())),
            threshold_verifier: Arc::new(ThresholdVerifier::default_config()),
            last_decay_at: Arc::new(RwLock::new(chrono::Utc::now().timestamp())),
            watchers: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Get current timestamp
    fn now() -> Option<Timestamp> {
        let now = chrono::Utc::now();
        Some(Timestamp {
            seconds: now.timestamp(),
            nanos: now.timestamp_subsec_nanos() as i32,
        })
    }

    /// Get balance state from current balance
    fn get_balance_state(&self, balance: u64) -> BalanceState {
        if balance <= self.config.minimum_balance {
            BalanceState::Exhausted
        } else if balance <= self.config.minimum_balance * 2 {
            BalanceState::Critical
        } else if balance <= self.config.warning_threshold {
            BalanceState::Low
        } else {
            BalanceState::Healthy
        }
    }

    /// Build a TokenBalance proto message
    async fn build_token_balance(&self) -> TokenBalance {
        let balance = *self.balance.read().await;
        let total_consumed = *self.total_consumed.read().await;
        let last_decay = *self.last_decay_at.read().await;

        TokenBalance {
            ai_id: Some(Identifier {
                namespace: "ai".to_string(),
                id: self.config.ai_id.clone(),
                version: 1,
            }),
            current_balance: balance,
            reserved: 0, // TODO: Implement reservation
            total_consumed,
            decay: Some(DecayParameters {
                rate_per_second: self.config.decay_rate,
                minimum_balance: self.config.minimum_balance,
                warning_threshold: self.config.warning_threshold,
                last_decay_at: Some(Timestamp {
                    seconds: last_decay,
                    nanos: 0,
                }),
            }),
            updated_at: Self::now(),
            state: self.get_balance_state(balance).into(),
        }
    }

    /// Notify watchers of balance change
    async fn notify_watchers(&self, owner_id: &str, change_type: BalanceChangeType, amount: u64) {
        let balance = self.build_token_balance().await;
        let state = self.get_balance_state(balance.current_balance);

        let update = BalanceUpdate {
            balance: Some(balance),
            state: state.into(),
            change_type: change_type.into(),
            change_amount: amount,
            timestamp: Self::now(),
            previous_state: BalanceState::Unspecified.into(), // TODO: Track previous state
        };

        // 10.2.1: Copy watcher list to avoid holding lock during async send
        let senders: Vec<_> = {
            let watchers = self.watchers.read().await;
            watchers
                .get(owner_id)
                .map(|s| s.clone())
                .unwrap_or_default()
        };

        for sender in senders {
            let _ = sender.send(Ok(update.clone())).await;
        }
    }
}

impl Default for TokenServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[tonic::async_trait]
impl TokenServiceTrait for TokenServiceImpl {
    async fn get_balance(
        &self,
        request: Request<GetBalanceRequest>,
    ) -> Result<Response<GetBalanceResponse>, Status> {
        let _req = request.into_inner();

        let balance = self.build_token_balance().await;
        let state = self.get_balance_state(balance.current_balance);

        Ok(Response::new(GetBalanceResponse {
            balance: Some(balance),
            state: state.into(),
            current_limits: Some(OperatingLimits {
                max_requests_per_second: 10.0,
                max_concurrent: 5,
                max_tokens_per_request: 10000,
                streaming_allowed: true,
                allowed_models: vec!["gpt-4".to_string(), "gpt-3.5-turbo".to_string()],
            }),
        }))
    }

    async fn consume_token(
        &self,
        request: Request<ConsumeTokenRequest>,
    ) -> Result<Response<ConsumeTokenResponse>, Status> {
        let req = request.into_inner();
        let amount = req.amount;
        let request_id = RequestId::new(req.request_id.clone())
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        let reason =
            ConsumptionReason::try_from(req.reason).unwrap_or(ConsumptionReason::Unspecified);

        // Check idempotency
        {
            let cache = self.idempotency_cache.read().await;
            if let Some(consumption_id) = cache.get(&request_id) {
                debug!(request_id = %request_id, "Idempotent request - returning cached result");
                // Return the existing consumption record
                let history = self.history.read().await;
                if let Some(record) = history.iter().find(|r| {
                    r.consumption.consumption_id.as_ref().map(|id| &id.id) == Some(consumption_id)
                }) {
                    return Ok(Response::new(ConsumeTokenResponse {
                        success: true,
                        new_balance: Some(self.build_token_balance().await),
                        error: None,
                        consumption_record: Some(record.consumption.clone()),
                    }));
                }
            }
        }

        // Try to consume
        let (success, new_balance) = {
            let mut balance = self.balance.write().await;
            if *balance >= amount {
                *balance -= amount;
                let mut total = self.total_consumed.write().await;
                *total += amount;
                (true, *balance)
            } else {
                (false, *balance)
            }
        };

        if !success {
            return Ok(Response::new(ConsumeTokenResponse {
                success: false,
                new_balance: Some(self.build_token_balance().await),
                error: Some(ErrorDetail {
                    code: "INSUFFICIENT_BALANCE".to_string(),
                    message: format!(
                        "Insufficient balance: {} required, {} available",
                        amount, new_balance
                    ),
                    metadata: [("field".to_string(), "amount".to_string())]
                        .into_iter()
                        .collect(),
                    suggestions: vec![],
                    documentation_url: String::new(),
                    severity: crate::gen::chinju::common::Severity::Error.into(),
                }),
                consumption_record: None,
            }));
        }

        // Create consumption record
        let consumption_id = format!("cons_{}", Uuid::new_v4());
        let consumption = TokenConsumption {
            consumption_id: Some(Identifier {
                namespace: "consumption".to_string(),
                id: consumption_id.clone(),
                version: 1,
            }),
            ai_id: Some(Identifier {
                namespace: "ai".to_string(),
                id: self.config.ai_id.clone(),
                version: 1,
            }),
            amount,
            request_id: request_id.to_string(),
            consumed_at: Self::now(),
            consumption_type: reason.into(),
        };

        // Store in history and idempotency cache
        {
            let mut history = self.history.write().await;
            history.push(ConsumptionRecord {
                consumption: consumption.clone(),
                reason,
            });
        }
        {
            let mut cache = self.idempotency_cache.write().await;
            cache.insert(request_id.clone(), consumption_id);
        }

        // Notify watchers
        let owner_id = req
            .owner_id
            .as_ref()
            .map(|id| id.id.as_str())
            .unwrap_or("default");
        self.notify_watchers(owner_id, BalanceChangeType::Consumption, amount)
            .await;

        info!(
            amount,
            new_balance,
            reason = ?reason,
            "Token consumed via gRPC"
        );

        Ok(Response::new(ConsumeTokenResponse {
            success: true,
            new_balance: Some(self.build_token_balance().await),
            error: None,
            consumption_record: Some(consumption),
        }))
    }

    async fn grant_token(
        &self,
        request: Request<GrantTokenRequest>,
    ) -> Result<Response<GrantTokenResponse>, Status> {
        let req = request.into_inner();
        let amount = req.amount;
        let reason = GrantReason::try_from(req.reason).unwrap_or(GrantReason::Unspecified);

        // Check if large grant requires threshold signature
        if amount > self.config.large_grant_threshold {
            if let Some(auth) = &req.authorization {
                let message = format!("GRANT_TOKEN:{}:{}", amount, req.justification);
                match self
                    .threshold_verifier
                    .verify_proto(message.as_bytes(), auth)
                    .await
                {
                    Ok(true) => {
                        info!(amount, "Large grant authorized via threshold signature");
                    }
                    Ok(false) => {
                        warn!(
                            amount,
                            "Threshold signature verification failed for large grant"
                        );
                        return Ok(Response::new(GrantTokenResponse {
                            success: false,
                            new_balance: None,
                            grant_record: None,
                            error: Some(ErrorDetail {
                                code: "AUTHORIZATION_FAILED".to_string(),
                                message: "Threshold signature verification failed".to_string(),
                                metadata: [("field".to_string(), "authorization".to_string())]
                                    .into_iter()
                                    .collect(),
                                suggestions: vec![],
                                documentation_url: String::new(),
                                severity: crate::gen::chinju::common::Severity::Error.into(),
                            }),
                        }));
                    }
                    Err(e) => {
                        // Allow in dev mode if verifier not initialized
                        if !self.threshold_verifier.is_initialized().await {
                            warn!(
                                amount,
                                error = %e,
                                "Threshold verifier not initialized - allowing grant in dev mode"
                            );
                        } else {
                            return Err(Status::permission_denied(format!(
                                "Threshold signature error: {}",
                                e
                            )));
                        }
                    }
                }
            } else {
                return Ok(Response::new(GrantTokenResponse {
                    success: false,
                    new_balance: None,
                    grant_record: None,
                    error: Some(ErrorDetail {
                        code: "AUTHORIZATION_REQUIRED".to_string(),
                        message: format!(
                            "Threshold signature required for grants > {}",
                            self.config.large_grant_threshold
                        ),
                        metadata: [("field".to_string(), "authorization".to_string())]
                            .into_iter()
                            .collect(),
                        suggestions: vec![],
                        documentation_url: String::new(),
                        severity: crate::gen::chinju::common::Severity::Error.into(),
                    }),
                }));
            }
        }

        // Grant tokens
        {
            let mut balance = self.balance.write().await;
            *balance += amount;
        }

        // Create grant record
        let grant = TokenGrant {
            recipient: Some(Identifier {
                namespace: "ai".to_string(),
                id: self.config.ai_id.clone(),
                version: 1,
            }),
            amount,
            validity: None,
            reason: req.justification.clone(),
            authorization: req.granter_signature,
            issued_at: Self::now(),
        };

        // Store grant
        {
            let mut grants = self.grants.write().await;
            grants.push(grant.clone());
        }

        // Notify watchers
        let owner_id = req
            .owner_id
            .as_ref()
            .map(|id| id.id.as_str())
            .unwrap_or("default");
        self.notify_watchers(owner_id, BalanceChangeType::Grant, amount)
            .await;

        info!(amount, reason = ?reason, "Token granted via gRPC");

        Ok(Response::new(GrantTokenResponse {
            success: true,
            new_balance: Some(self.build_token_balance().await),
            grant_record: Some(grant),
            error: None,
        }))
    }

    async fn get_consumption_history(
        &self,
        request: Request<GetConsumptionHistoryRequest>,
    ) -> Result<Response<GetConsumptionHistoryResponse>, Status> {
        let req = request.into_inner();
        let limit = if req.limit > 0 {
            req.limit as usize
        } else {
            100
        };

        let history = self.history.read().await;

        // Filter by time range and reason
        let filtered: Vec<_> = history
            .iter()
            .filter(|r| {
                // Filter by time range
                if let (Some(from), Some(consumed_at)) = (&req.from, &r.consumption.consumed_at) {
                    if consumed_at.seconds < from.seconds {
                        return false;
                    }
                }
                if let (Some(to), Some(consumed_at)) = (&req.to, &r.consumption.consumed_at) {
                    if consumed_at.seconds > to.seconds {
                        return false;
                    }
                }
                // Filter by reason
                if !req.reason_filter.is_empty() {
                    let reason_i32: i32 = r.reason.into();
                    if !req.reason_filter.contains(&reason_i32) {
                        return false;
                    }
                }
                true
            })
            .take(limit)
            .cloned()
            .collect();

        let consumptions: Vec<_> = filtered.iter().map(|r| r.consumption.clone()).collect();
        let total: u64 = consumptions.iter().map(|c| c.amount).sum();

        Ok(Response::new(GetConsumptionHistoryResponse {
            consumptions,
            next_page_token: String::new(), // TODO: Implement pagination
            total_consumed: total,
            average_rate: if !filtered.is_empty() {
                total as f64 / filtered.len() as f64
            } else {
                0.0
            },
        }))
    }

    type WatchBalanceStream = tokio_stream::wrappers::ReceiverStream<Result<BalanceUpdate, Status>>;

    async fn watch_balance(
        &self,
        request: Request<WatchBalanceRequest>,
    ) -> Result<Response<Self::WatchBalanceStream>, Status> {
        let req = request.into_inner();
        let owner_id = req
            .owner_id
            .as_ref()
            .map(|id| id.id.clone())
            .unwrap_or_else(|| "default".to_string());

        let (tx, rx) = tokio::sync::mpsc::channel(10);

        // Register watcher
        {
            let mut watchers = self.watchers.write().await;
            watchers.entry(owner_id.clone()).or_default().push(tx);
        }

        info!(owner_id = %owner_id, "Balance watcher registered");

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(
            rx,
        )))
    }

    async fn get_depletion_forecast(
        &self,
        request: Request<GetDepletionForecastRequest>,
    ) -> Result<Response<GetDepletionForecastResponse>, Status> {
        let req = request.into_inner();

        let balance = *self.balance.read().await;
        let history = self.history.read().await;

        // Calculate consumption rate from recent history
        let recent_consumptions: Vec<_> = history.iter().rev().take(100).collect();

        let current_rate = if !recent_consumptions.is_empty() {
            let total: u64 = recent_consumptions
                .iter()
                .map(|r| r.consumption.amount)
                .sum();
            let count = recent_consumptions.len() as f64;
            total as f64 / count
        } else {
            1.0 // Default rate if no history
        };

        let rate = if req.override_consumption_rate > 0.0 {
            req.override_consumption_rate
        } else {
            current_rate
        };

        // Calculate time to depletion
        let seconds_to_depletion = if rate > 0.0 {
            (balance as f64 / rate) as i64
        } else {
            i64::MAX
        };

        let now = chrono::Utc::now();
        let depletion_time = now + chrono::Duration::seconds(seconds_to_depletion);

        // Calculate time to each state
        let mut time_to_state = HashMap::new();
        let thresholds = [
            ("healthy", self.config.warning_threshold),
            ("low", self.config.minimum_balance * 2),
            ("critical", self.config.minimum_balance),
            ("exhausted", 0),
        ];

        for (state_name, threshold) in thresholds {
            if balance > threshold {
                let seconds = ((balance - threshold) as f64 / rate) as i64;
                time_to_state.insert(state_name.to_string(), seconds);
            } else {
                time_to_state.insert(state_name.to_string(), 0);
            }
        }

        Ok(Response::new(GetDepletionForecastResponse {
            current_consumption_rate: current_rate,
            estimated_depletion: Some(Timestamp {
                seconds: depletion_time.timestamp(),
                nanos: 0,
            }),
            recommended_replenishment: (balance as f64 * 0.5) as u64, // Recommend 50% top-up
            time_to_state,
        }))
    }

    async fn get_token_info(
        &self,
        request: Request<GetTokenInfoRequest>,
    ) -> Result<Response<GetTokenInfoResponse>, Status> {
        let _req = request.into_inner();

        let balance = *self.balance.read().await;
        let state = self.get_balance_state(balance);

        // Determine AI operating state based on token balance
        let ai_state = match state {
            BalanceState::Healthy => AiOperatingState::Active,
            BalanceState::Low => AiOperatingState::Throttled,
            BalanceState::Critical => AiOperatingState::Suspended,
            BalanceState::Exhausted => AiOperatingState::Shutdown,
            _ => AiOperatingState::Active,
        };

        Ok(Response::new(GetTokenInfoResponse {
            token: Some(SurvivalToken {
                token_id: Some(Identifier {
                    namespace: "token".to_string(),
                    id: self.config.ai_id.clone(),
                    version: 1,
                }),
                amount: balance,
                expires_at: None,
                attestation: None,
                issuer_signature: None,
                metadata: Some(TokenMetadata {
                    source: "chinju-sidecar".to_string(),
                    purpose: "ai_operation".to_string(),
                    notes: String::new(),
                }),
            }),
            ai_state: ai_state.into(),
        }))
    }

    async fn emergency_replenish(
        &self,
        request: Request<EmergencyReplenishRequest>,
    ) -> Result<Response<EmergencyReplenishResponse>, Status> {
        let req = request.into_inner();
        let amount = req.amount;

        warn!(amount, reason = %req.reason, "EMERGENCY REPLENISH requested");

        // Emergency replenish always requires threshold signature
        let auth = req.authorization.ok_or_else(|| {
            Status::permission_denied("Threshold signature required for emergency replenish")
        })?;

        let message = format!("EMERGENCY_REPLENISH:{}:{}", amount, req.reason);
        match self
            .threshold_verifier
            .verify_proto(message.as_bytes(), &auth)
            .await
        {
            Ok(true) => {
                info!(amount, "Emergency replenish authorized");
            }
            Ok(false) => {
                return Ok(Response::new(EmergencyReplenishResponse {
                    success: false,
                    new_balance: None,
                    error: Some(ErrorDetail {
                        code: "AUTHORIZATION_FAILED".to_string(),
                        message: "Threshold signature verification failed".to_string(),
                        metadata: [("field".to_string(), "authorization".to_string())]
                            .into_iter()
                            .collect(),
                        suggestions: vec![],
                        documentation_url: String::new(),
                        severity: crate::gen::chinju::common::Severity::Error.into(),
                    }),
                }));
            }
            Err(e) => {
                if !self.threshold_verifier.is_initialized().await {
                    warn!(
                        error = %e,
                        "Threshold verifier not initialized - allowing emergency replenish in dev mode"
                    );
                } else {
                    return Err(Status::permission_denied(format!(
                        "Threshold signature error: {}",
                        e
                    )));
                }
            }
        }

        // Grant emergency tokens
        {
            let mut balance = self.balance.write().await;
            *balance += amount;
        }

        let new_balance = *self.balance.read().await;
        info!(amount, new_balance, "Emergency tokens granted");

        Ok(Response::new(EmergencyReplenishResponse {
            success: true,
            new_balance: Some(self.build_token_balance().await),
            error: None,
        }))
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_consumption() {
        let mut svc = TokenService::new(1000);

        assert!(svc.consume(100));
        assert_eq!(svc.get_balance(), 900);
        assert_eq!(svc.total_consumed(), 100);

        // Try to consume more than available
        assert!(!svc.consume(1000));
        assert_eq!(svc.get_balance(), 900);
    }

    #[test]
    fn test_token_grant() {
        let mut svc = TokenService::new(100);

        svc.grant(500);
        assert_eq!(svc.get_balance(), 600);
    }

    #[test]
    fn test_token_decay() {
        let mut svc = TokenService::new(1000);

        svc.apply_decay(0.1); // 10% decay
        assert_eq!(svc.get_balance(), 900);
    }

    #[tokio::test]
    async fn test_token_service_impl_balance() {
        let svc = TokenServiceImpl::new();
        let req = Request::new(GetBalanceRequest { owner_id: None });

        let response = svc.get_balance(req).await.unwrap();
        let balance = response.into_inner().balance.unwrap();

        assert_eq!(balance.current_balance, 10000);
        assert_eq!(balance.state(), BalanceState::Healthy);
    }

    #[tokio::test]
    async fn test_token_service_impl_consume() {
        let svc = TokenServiceImpl::new();

        let req = Request::new(ConsumeTokenRequest {
            owner_id: None,
            amount: 100,
            reason: ConsumptionReason::ApiRequest.into(),
            request_id: "test-req-1".to_string(),
            metadata: HashMap::new(),
        });

        let response = svc.consume_token(req).await.unwrap();
        let inner = response.into_inner();

        assert!(inner.success);
        assert_eq!(inner.new_balance.unwrap().current_balance, 9900);
    }

    #[tokio::test]
    async fn test_token_service_impl_idempotency() {
        let svc = TokenServiceImpl::new();

        // First request
        let req1 = Request::new(ConsumeTokenRequest {
            owner_id: None,
            amount: 100,
            reason: ConsumptionReason::ApiRequest.into(),
            request_id: "idempotent-req".to_string(),
            metadata: HashMap::new(),
        });
        let response1 = svc.consume_token(req1).await.unwrap();
        assert!(response1.into_inner().success);

        // Same request again (idempotent)
        let req2 = Request::new(ConsumeTokenRequest {
            owner_id: None,
            amount: 100,
            reason: ConsumptionReason::ApiRequest.into(),
            request_id: "idempotent-req".to_string(),
            metadata: HashMap::new(),
        });
        let response2 = svc.consume_token(req2).await.unwrap();
        assert!(response2.into_inner().success);

        // Balance should only be reduced once
        let balance_req = Request::new(GetBalanceRequest { owner_id: None });
        let balance = svc.get_balance(balance_req).await.unwrap();
        assert_eq!(balance.into_inner().balance.unwrap().current_balance, 9900);
    }

    #[tokio::test]
    async fn test_token_service_impl_insufficient_balance() {
        let config = TokenServiceConfig {
            initial_balance: 100,
            ..Default::default()
        };
        let svc = TokenServiceImpl::with_config(config);

        let req = Request::new(ConsumeTokenRequest {
            owner_id: None,
            amount: 200,
            reason: ConsumptionReason::ApiRequest.into(),
            request_id: "test-insufficient".to_string(),
            metadata: HashMap::new(),
        });

        let response = svc.consume_token(req).await.unwrap();
        let inner = response.into_inner();

        assert!(!inner.success);
        assert!(inner.error.is_some());
        assert_eq!(inner.error.unwrap().code, "INSUFFICIENT_BALANCE");
    }

    #[tokio::test]
    async fn test_token_service_impl_rejects_invalid_request_id() {
        let svc = TokenServiceImpl::new();

        let req = Request::new(ConsumeTokenRequest {
            owner_id: None,
            amount: 100,
            reason: ConsumptionReason::ApiRequest.into(),
            request_id: "invalid request id".to_string(),
            metadata: HashMap::new(),
        });

        let err = svc.consume_token(req).await.unwrap_err();
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_token_service_impl_failed_request_not_cached_for_idempotency() {
        let config = TokenServiceConfig {
            initial_balance: 100,
            ..Default::default()
        };
        let svc = TokenServiceImpl::with_config(config);

        let fail_req = Request::new(ConsumeTokenRequest {
            owner_id: None,
            amount: 200,
            reason: ConsumptionReason::ApiRequest.into(),
            request_id: "retry-after-failure".to_string(),
            metadata: HashMap::new(),
        });
        let fail_resp = svc.consume_token(fail_req).await.unwrap().into_inner();
        assert!(!fail_resp.success);

        let retry_req = Request::new(ConsumeTokenRequest {
            owner_id: None,
            amount: 50,
            reason: ConsumptionReason::ApiRequest.into(),
            request_id: "retry-after-failure".to_string(),
            metadata: HashMap::new(),
        });
        let retry_resp = svc.consume_token(retry_req).await.unwrap().into_inner();
        assert!(retry_resp.success);
        assert_eq!(retry_resp.new_balance.unwrap().current_balance, 50);
    }
}
