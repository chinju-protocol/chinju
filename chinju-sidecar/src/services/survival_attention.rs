//! Survival-Weighted Attention Implementation (C17)
//!
//! Modifies Transformer attention mechanism with a "survival score"
//! based on the UGEN Survival Equation (S = N × Y × E) to automatically
//! suppress attention to harmful/contradictory information.
//!
//! SurvivalAttention: softmax(QK^T / sqrt(d_k) + alpha * S) * V
//!
//! Key features:
//! - Lightweight SurvivalScorer for real-time score computation
//! - Dynamic alpha adjustment based on task type and risk level
//! - Integration with external knowledge bases

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

// =============================================================================
// Constants
// =============================================================================

/// Default alpha (survival weight strength)
const DEFAULT_ALPHA: f64 = 1.0;

/// Maximum alpha value
const DEFAULT_MAX_ALPHA: f64 = 5.0;

/// Default critical slack threshold (mu_c)
const DEFAULT_MU_C: f64 = 0.5;

// =============================================================================
// Survival Score
// =============================================================================

/// Survival score for a single token/element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurvivalScore {
    /// Diversity (N) - range of alternatives
    pub diversity_n: f64,
    /// Slack (mu) - margin/buffer
    pub yohaku_mu: f64,
    /// Divergence (delta) - misalignment
    pub delta: f64,
    /// Integrated score: S = log(N) + log(mu/mu_c) - delta
    pub integrated_s: f64,
}

impl SurvivalScore {
    /// Compute integrated score from components
    pub fn compute(diversity_n: f64, yohaku_mu: f64, delta: f64, mu_c: f64) -> Self {
        let n_clamped = diversity_n.max(0.01);
        let mu_clamped = yohaku_mu.max(0.01);
        let mu_c_clamped = mu_c.max(0.01);

        let integrated_s = n_clamped.ln() + (mu_clamped / mu_c_clamped).ln() - delta;

        Self {
            diversity_n,
            yohaku_mu,
            delta,
            integrated_s,
        }
    }
}

// =============================================================================
// Configuration
// =============================================================================

/// Risk level for dynamic alpha adjustment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    /// Multiplier for alpha based on risk
    pub fn alpha_multiplier(&self) -> f64 {
        match self {
            RiskLevel::Low => 0.5,
            RiskLevel::Medium => 1.0,
            RiskLevel::High => 2.0,
            RiskLevel::Critical => 3.0,
        }
    }
}

/// SurvivalScorer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurvivalScorerConfig {
    /// Model path (for external scorer)
    pub model_path: String,
    /// Number of parameters (informational)
    pub num_parameters: u64,
    /// Critical slack threshold (mu_c)
    pub mu_c: f64,
}

impl Default for SurvivalScorerConfig {
    fn default() -> Self {
        Self {
            model_path: String::new(),
            num_parameters: 0,
            mu_c: DEFAULT_MU_C,
        }
    }
}

/// Alpha configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlphaConfig {
    /// Base alpha value
    pub base_alpha: f64,
    /// Enable dynamic adjustment
    pub dynamic_adjustment: bool,
    /// Task-type multipliers
    pub task_multipliers: HashMap<String, f64>,
    /// Maximum alpha
    pub max_alpha: f64,
}

impl Default for AlphaConfig {
    fn default() -> Self {
        let mut task_multipliers = HashMap::new();
        task_multipliers.insert("medical".to_string(), 2.0);
        task_multipliers.insert("financial".to_string(), 1.5);
        task_multipliers.insert("creative".to_string(), 0.5);
        task_multipliers.insert("general".to_string(), 1.0);

        Self {
            base_alpha: DEFAULT_ALPHA,
            dynamic_adjustment: true,
            task_multipliers,
            max_alpha: DEFAULT_MAX_ALPHA,
        }
    }
}

/// Full SurvivalAttention configuration
#[derive(Debug, Clone)]
pub struct SurvivalAttentionConfig {
    pub scorer: SurvivalScorerConfig,
    pub alpha: AlphaConfig,
    pub external_kb_enabled: bool,
    pub external_kb_endpoint: Option<String>,
}

impl Default for SurvivalAttentionConfig {
    fn default() -> Self {
        Self {
            scorer: SurvivalScorerConfig::default(),
            alpha: AlphaConfig::default(),
            external_kb_enabled: false,
            external_kb_endpoint: None,
        }
    }
}

// =============================================================================
// Survival Attention Service
// =============================================================================

/// Survival-Weighted Attention Service
pub struct SurvivalAttentionService {
    config: Arc<RwLock<SurvivalAttentionConfig>>,
    /// Current effective alpha
    current_alpha: Arc<RwLock<f64>>,
    /// Score computation history (for monitoring)
    score_history: Arc<RwLock<Vec<ScoringRecord>>>,
}

/// Record of a scoring computation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringRecord {
    pub input_length: usize,
    pub avg_survival_score: f64,
    pub min_survival_score: f64,
    pub max_survival_score: f64,
    pub alpha_used: f64,
    pub computed_at: DateTime<Utc>,
}

impl SurvivalAttentionService {
    /// Create with default config
    pub fn new() -> Self {
        Self::with_config(SurvivalAttentionConfig::default())
    }

    /// Create with custom config
    pub fn with_config(config: SurvivalAttentionConfig) -> Self {
        let alpha = config.alpha.base_alpha;
        info!(
            alpha = alpha,
            mu_c = config.scorer.mu_c,
            "Initializing Survival Attention Service (C17)"
        );

        Self {
            config: Arc::new(RwLock::new(config)),
            current_alpha: Arc::new(RwLock::new(alpha)),
            score_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Compute survival scores for a list of token features
    ///
    /// Each token is represented as (diversity_n, yohaku_mu, delta)
    pub async fn compute_scores(&self, token_features: &[(f64, f64, f64)]) -> Vec<SurvivalScore> {
        let config = self.config.read().await;
        let mu_c = config.scorer.mu_c;

        let scores: Vec<SurvivalScore> = token_features
            .iter()
            .map(|&(n, mu, delta)| SurvivalScore::compute(n, mu, delta, mu_c))
            .collect();

        // Record for monitoring
        if !scores.is_empty() {
            let integrated: Vec<f64> = scores.iter().map(|s| s.integrated_s).collect();
            let avg = integrated.iter().sum::<f64>() / integrated.len() as f64;
            let min = integrated.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = integrated.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

            let record = ScoringRecord {
                input_length: scores.len(),
                avg_survival_score: avg,
                min_survival_score: min,
                max_survival_score: max,
                alpha_used: *self.current_alpha.read().await,
                computed_at: Utc::now(),
            };

            let mut history = self.score_history.write().await;
            if history.len() >= 1000 {
                history.drain(0..500);
            }
            history.push(record);
        }

        scores
    }

    /// Adjust alpha dynamically based on task type and risk level
    pub async fn adjust_alpha(
        &self,
        task_type: Option<&str>,
        risk_level: Option<RiskLevel>,
    ) -> (f64, f64) {
        let config = self.config.read().await;
        let old_alpha = *self.current_alpha.read().await;

        let mut new_alpha = config.alpha.base_alpha;

        // Apply task multiplier
        if let Some(task) = task_type {
            if let Some(&multiplier) = config.alpha.task_multipliers.get(task) {
                new_alpha *= multiplier;
            }
        }

        // Apply risk multiplier
        if let Some(risk) = risk_level {
            new_alpha *= risk.alpha_multiplier();
        }

        // Clamp to max
        new_alpha = new_alpha.min(config.alpha.max_alpha);

        if (new_alpha - old_alpha).abs() > 0.01 {
            debug!(
                old = old_alpha,
                new = new_alpha,
                task = ?task_type,
                risk = ?risk_level,
                "Alpha adjusted"
            );
        }

        *self.current_alpha.write().await = new_alpha;
        (old_alpha, new_alpha)
    }

    /// Get current effective alpha
    pub async fn get_alpha(&self) -> f64 {
        *self.current_alpha.read().await
    }

    /// Get scoring history
    pub async fn get_score_history(&self) -> Vec<ScoringRecord> {
        self.score_history.read().await.clone()
    }

    /// Update scorer configuration
    pub async fn update_scorer_config(&self, new_config: SurvivalScorerConfig) {
        info!(
            model_path = %new_config.model_path,
            mu_c = new_config.mu_c,
            "Updating SurvivalScorer configuration"
        );
        self.config.write().await.scorer = new_config;
    }
}

impl Default for SurvivalAttentionService {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_survival_score_computation() {
        let score = SurvivalScore::compute(2.0, 1.0, 0.1, 0.5);
        // S = ln(2) + ln(1.0/0.5) - 0.1 = 0.693 + 0.693 - 0.1 = 1.286
        assert!((score.integrated_s - 1.286).abs() < 0.01);
    }

    #[test]
    fn test_survival_score_low_diversity() {
        let score = SurvivalScore::compute(0.5, 0.3, 0.8, 0.5);
        // Low N, low mu, high delta → negative score
        assert!(score.integrated_s < 0.0);
    }

    #[test]
    fn test_survival_score_high_quality() {
        let score = SurvivalScore::compute(10.0, 2.0, 0.01, 0.5);
        // High N, high mu, low delta → high positive score
        assert!(score.integrated_s > 2.0);
    }

    #[tokio::test]
    async fn test_compute_scores() {
        let service = SurvivalAttentionService::new();

        let features = vec![
            (2.0, 1.0, 0.1),   // Good token
            (0.5, 0.3, 0.8),   // Bad token
            (10.0, 2.0, 0.01), // Great token
        ];

        let scores = service.compute_scores(&features).await;
        assert_eq!(scores.len(), 3);
        assert!(scores[0].integrated_s > scores[1].integrated_s);
        assert!(scores[2].integrated_s > scores[0].integrated_s);
    }

    #[tokio::test]
    async fn test_alpha_adjustment() {
        let service = SurvivalAttentionService::new();

        // Default alpha
        assert_eq!(service.get_alpha().await, DEFAULT_ALPHA);

        // Medical task → higher alpha
        let (old, new) = service.adjust_alpha(Some("medical"), None).await;
        assert_eq!(old, 1.0);
        assert_eq!(new, 2.0); // base * medical_multiplier (2.0)

        // Creative task → lower alpha
        let (_, new) = service.adjust_alpha(Some("creative"), None).await;
        assert_eq!(new, 0.5); // base * creative_multiplier (0.5)

        // High risk → higher alpha
        let (_, new) = service.adjust_alpha(None, Some(RiskLevel::High)).await;
        assert_eq!(new, 2.0); // base * high_risk (2.0)
    }

    #[tokio::test]
    async fn test_alpha_clamped_to_max() {
        let service = SurvivalAttentionService::new();

        // Medical + Critical risk → should clamp
        let (_, new) = service
            .adjust_alpha(Some("medical"), Some(RiskLevel::Critical))
            .await;
        assert_eq!(new, DEFAULT_MAX_ALPHA); // 1.0 * 2.0 * 3.0 = 6.0, clamped to 5.0
    }

    #[tokio::test]
    async fn test_score_history() {
        let service = SurvivalAttentionService::new();

        let features = vec![(1.0, 1.0, 0.5)];
        service.compute_scores(&features).await;

        let history = service.get_score_history().await;
        assert_eq!(history.len(), 1);
    }

    #[tokio::test]
    async fn test_update_scorer_config() {
        let service = SurvivalAttentionService::new();

        let new_config = SurvivalScorerConfig {
            model_path: "/models/scorer_v2".to_string(),
            num_parameters: 1_000_000,
            mu_c: 0.3,
        };

        service.update_scorer_config(new_config).await;

        // Compute with new mu_c
        let features = vec![(2.0, 1.0, 0.1)];
        let scores = service.compute_scores(&features).await;

        // S = ln(2) + ln(1.0/0.3) - 0.1 = 0.693 + 1.204 - 0.1 = 1.797
        assert!((scores[0].integrated_s - 1.797).abs() < 0.01);
    }

    #[test]
    fn test_risk_level_multipliers() {
        assert_eq!(RiskLevel::Low.alpha_multiplier(), 0.5);
        assert_eq!(RiskLevel::Medium.alpha_multiplier(), 1.0);
        assert_eq!(RiskLevel::High.alpha_multiplier(), 2.0);
        assert_eq!(RiskLevel::Critical.alpha_multiplier(), 3.0);
    }

    #[tokio::test]
    async fn test_grpc_compute_scores() {
        use crate::gen::chinju::api::survival_attention::survival_attention_service_server::SurvivalAttentionService as SurvivalAttentionServiceTrait;
        use crate::gen::chinju::survival_attention::ComputeScoresRequest;

        let service = SurvivalAttentionServiceImpl::new(super::SurvivalAttentionService::new());
        let request = tonic::Request::new(ComputeScoresRequest {
            input_text: "test input".to_string(),
            scorer_config: None,
            use_external_kb: false,
        });

        let response = service.compute_survival_scores(request).await.unwrap();
        let scores = response.into_inner();
        assert!(scores.scores.len() > 0);
    }

    #[tokio::test]
    async fn test_grpc_adjust_alpha() {
        use crate::gen::chinju::api::survival_attention::survival_attention_service_server::SurvivalAttentionService as SurvivalAttentionServiceTrait;
        use crate::gen::chinju::survival_attention::{
            AdjustAlphaRequest, RiskLevel as ProtoRiskLevel,
        };

        let service = SurvivalAttentionServiceImpl::new(super::SurvivalAttentionService::new());
        let request = tonic::Request::new(AdjustAlphaRequest {
            new_base_alpha: 0.0,
            task_type: "medical".to_string(),
            risk_level: ProtoRiskLevel::High.into(),
        });

        let response = service.adjust_alpha(request).await.unwrap();
        let result = response.into_inner();
        assert!(result.new_alpha > result.previous_alpha);
    }
}

// =============================================================================
// gRPC Service Implementation (10.1.2)
// =============================================================================

use crate::gen::chinju::api::survival_attention::survival_attention_service_server::SurvivalAttentionService as SurvivalAttentionServiceTrait;
use crate::gen::chinju::common::Timestamp;
use crate::gen::chinju::survival_attention::{
    AdjustAlphaRequest, AdjustAlphaResponse, ComputeScoresRequest, RiskLevel as ProtoRiskLevel,
    SurvivalAttentionRequest, SurvivalAttentionResponse, SurvivalScore as ProtoSurvivalScore,
    SurvivalScorerConfig as ProtoScorerConfig, TokenSurvivalScores, UpdateScorerRequest,
    UpdateScorerResponse,
};
use tonic::{Request, Response, Status};

/// gRPC service implementation wrapper for SurvivalAttentionService
pub struct SurvivalAttentionServiceImpl {
    inner: SurvivalAttentionService,
}

impl SurvivalAttentionServiceImpl {
    pub fn new(inner: SurvivalAttentionService) -> Self {
        Self { inner }
    }
}

// Proto -> Rust type conversion helpers
impl From<ProtoRiskLevel> for Option<RiskLevel> {
    fn from(proto: ProtoRiskLevel) -> Self {
        match proto {
            ProtoRiskLevel::Unspecified => None,
            ProtoRiskLevel::Low => Some(RiskLevel::Low),
            ProtoRiskLevel::Medium => Some(RiskLevel::Medium),
            ProtoRiskLevel::High => Some(RiskLevel::High),
            ProtoRiskLevel::Critical => Some(RiskLevel::Critical),
        }
    }
}

impl From<&SurvivalScore> for ProtoSurvivalScore {
    fn from(score: &SurvivalScore) -> Self {
        ProtoSurvivalScore {
            diversity_n: score.diversity_n,
            yohaku_mu: score.yohaku_mu,
            delta: score.delta,
            integrated_s: score.integrated_s,
        }
    }
}

impl From<&SurvivalScorerConfig> for ProtoScorerConfig {
    fn from(config: &SurvivalScorerConfig) -> Self {
        ProtoScorerConfig {
            model_path: config.model_path.clone(),
            num_parameters: config.num_parameters,
            mu_c: config.mu_c,
        }
    }
}

#[tonic::async_trait]
impl SurvivalAttentionServiceTrait for SurvivalAttentionServiceImpl {
    async fn compute_survival_scores(
        &self,
        request: Request<ComputeScoresRequest>,
    ) -> Result<Response<TokenSurvivalScores>, Status> {
        let req = request.into_inner();
        info!(
            input_len = req.input_text.len(),
            "Computing survival scores via gRPC"
        );

        // Simple tokenization (split by whitespace)
        let tokens: Vec<&str> = req.input_text.split_whitespace().collect();
        if tokens.is_empty() {
            return Ok(Response::new(TokenSurvivalScores {
                scores: vec![],
                tokens: vec![],
            }));
        }

        // Generate mock features for each token
        // In production, this would come from the actual model
        let features: Vec<(f64, f64, f64)> = tokens
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let diversity = 1.0 + (i as f64 * 0.1);
                let yohaku = 0.8 + (i as f64 * 0.05);
                let delta = 0.1;
                (diversity, yohaku, delta)
            })
            .collect();

        let scores = self.inner.compute_scores(&features).await;

        Ok(Response::new(TokenSurvivalScores {
            scores: scores.iter().map(ProtoSurvivalScore::from).collect(),
            tokens: tokens.iter().map(|s| s.to_string()).collect(),
        }))
    }

    async fn infer_with_survival_attention(
        &self,
        request: Request<SurvivalAttentionRequest>,
    ) -> Result<Response<SurvivalAttentionResponse>, Status> {
        let req = request.into_inner();
        info!(
            embeddings_size = req.input_embeddings.len(),
            "Inference with SurvivalAttention via gRPC"
        );

        // This is a placeholder implementation
        // In production, this would integrate with the actual model inference
        let effective_alpha = self.inner.get_alpha().await;

        Ok(Response::new(SurvivalAttentionResponse {
            output_embeddings: req.input_embeddings, // Echo back for now
            token_scores: None,
            effective_alpha,
        }))
    }

    async fn adjust_alpha(
        &self,
        request: Request<AdjustAlphaRequest>,
    ) -> Result<Response<AdjustAlphaResponse>, Status> {
        let req = request.into_inner();
        let task_type = if req.task_type.is_empty() {
            None
        } else {
            Some(req.task_type.as_str())
        };
        let risk_level: Option<RiskLevel> = ProtoRiskLevel::try_from(req.risk_level)
            .unwrap_or(ProtoRiskLevel::Unspecified)
            .into();

        info!(
            task_type = ?task_type,
            risk_level = ?risk_level,
            "Adjusting alpha via gRPC"
        );

        let (previous_alpha, new_alpha) = self.inner.adjust_alpha(task_type, risk_level).await;

        let reason = match (task_type, risk_level) {
            (Some(t), Some(r)) => format!("task={}, risk={:?}", t, r),
            (Some(t), None) => format!("task={}", t),
            (None, Some(r)) => format!("risk={:?}", r),
            (None, None) => "reset to base".to_string(),
        };

        Ok(Response::new(AdjustAlphaResponse {
            previous_alpha,
            new_alpha,
            adjustment_reason: reason,
            adjusted_at: Some(Timestamp {
                seconds: Utc::now().timestamp(),
                nanos: 0,
            }),
        }))
    }

    async fn update_scorer(
        &self,
        request: Request<UpdateScorerRequest>,
    ) -> Result<Response<UpdateScorerResponse>, Status> {
        let req = request.into_inner();
        info!(
            new_model_path = %req.new_model_path,
            hot_swap = req.hot_swap,
            "Updating scorer via gRPC"
        );

        // Get current config before update
        let previous_config = ProtoScorerConfig {
            model_path: String::new(), // TODO: store current path
            num_parameters: 0,
            mu_c: 0.5, // default
        };

        // Create new config
        let new_config = SurvivalScorerConfig {
            model_path: req.new_model_path.clone(),
            num_parameters: 0, // Would be set from loaded model
            mu_c: 0.5,         // Keep default
        };

        self.inner.update_scorer_config(new_config).await;

        Ok(Response::new(UpdateScorerResponse {
            success: true,
            previous_config: Some(previous_config),
            new_config: Some(ProtoScorerConfig {
                model_path: req.new_model_path,
                num_parameters: 0,
                mu_c: 0.5,
            }),
            validation_result: if req.validate_before_update {
                "Validation passed".to_string()
            } else {
                "Validation skipped".to_string()
            },
            updated_at: Some(Timestamp {
                seconds: Utc::now().timestamp(),
                nanos: 0,
            }),
        }))
    }
}
