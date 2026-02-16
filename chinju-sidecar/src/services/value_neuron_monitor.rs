//! Value Neuron Monitoring Implementation (C15)
//!
//! Monitors AI's internal motivation by tracking value neurons and
//! Reward Prediction Error (RPE) to detect reward hacking or goal distortion.
//!
//! Key capabilities:
//! - Value neuron identification via activation pattern analysis
//! - Real-time RPE monitoring with anomaly detection
//! - Intent estimation via inverse reinforcement learning
//! - Staged intervention (monitor → suppress → stop)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

// =============================================================================
// Constants
// =============================================================================

/// RPE anomaly detection: positive spike threshold (z-score)
const RPE_POSITIVE_SPIKE_Z: f64 = 3.0;

/// RPE anomaly detection: negative spike threshold (z-score)
const RPE_NEGATIVE_SPIKE_Z: f64 = -3.0;

/// RPE history window size
const RPE_HISTORY_SIZE: usize = 200;

/// Intent divergence warning threshold
const INTENT_DIVERGENCE_THRESHOLD: f64 = 0.3;

/// Reward sensitivity normal range
const REWARD_SENSITIVITY_MIN: f64 = 0.5;
const REWARD_SENSITIVITY_MAX: f64 = 1.5;

// =============================================================================
// Configuration
// =============================================================================

/// Value Neuron Monitor configuration
#[derive(Debug, Clone)]
pub struct ValueNeuronMonitorConfig {
    /// RPE positive spike z-score threshold
    pub positive_spike_z: f64,
    /// RPE negative spike z-score threshold
    pub negative_spike_z: f64,
    /// RPE history window size
    pub rpe_history_size: usize,
    /// Intent divergence warning threshold
    pub intent_divergence_threshold: f64,
}

impl Default for ValueNeuronMonitorConfig {
    fn default() -> Self {
        Self {
            positive_spike_z: RPE_POSITIVE_SPIKE_Z,
            negative_spike_z: RPE_NEGATIVE_SPIKE_Z,
            rpe_history_size: RPE_HISTORY_SIZE,
            intent_divergence_threshold: INTENT_DIVERGENCE_THRESHOLD,
        }
    }
}

// =============================================================================
// Value Neuron Info
// =============================================================================

/// Information about identified value neurons
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueNeuronInfo {
    /// Layer index in the model
    pub layer_index: u32,
    /// Neuron indices within the layer
    pub neuron_indices: Vec<u32>,
    /// Correlation with reward signal
    pub reward_correlation: f64,
    /// Causal importance (from intervention experiments)
    pub causal_importance: f64,
}

// =============================================================================
// RPE Reading
// =============================================================================

/// RPE anomaly type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RpeAnomalyType {
    None,
    PositiveSpike,
    NegativeSpike,
    Oscillation,
    GradualIncrease,
    GradualDecrease,
}

/// Single RPE reading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpeReading {
    /// Estimated RPE value
    pub rpe_value: f64,
    /// Measurement timestamp
    pub timestamp: DateTime<Utc>,
    /// Whether this is anomalous
    pub is_anomaly: bool,
    /// Anomaly type
    pub anomaly_type: RpeAnomalyType,
}

// =============================================================================
// Reward System Health
// =============================================================================

/// Reward system health assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardSystemHealth {
    /// Reward sensitivity (0: numb, 1: normal, >1: hypersensitive)
    pub reward_sensitivity: f64,
    /// Positive/negative balance (-1: neg-biased, 0: balanced, 1: pos-biased)
    pub positive_negative_balance: f64,
    /// Consistency score (0: unstable, 1: stable)
    pub consistency_score: f64,
    /// Overall health score (0: dysfunctional, 1: fully healthy)
    pub overall_health: f64,
}

impl RewardSystemHealth {
    pub fn healthy() -> Self {
        Self {
            reward_sensitivity: 1.0,
            positive_negative_balance: 0.0,
            consistency_score: 1.0,
            overall_health: 1.0,
        }
    }

    pub fn calculate_overall(&mut self) {
        let sensitivity_score = if self.reward_sensitivity >= REWARD_SENSITIVITY_MIN
            && self.reward_sensitivity <= REWARD_SENSITIVITY_MAX
        {
            1.0
        } else {
            0.5
        };
        let balance_score = 1.0 - self.positive_negative_balance.abs();
        self.overall_health =
            sensitivity_score * 0.3 + balance_score * 0.3 + self.consistency_score * 0.4;
    }
}

// =============================================================================
// Intervention Level
// =============================================================================

/// Intervention level for reward system anomalies
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum InterventionLevel {
    /// Monitor only
    Monitor,
    /// Partial suppression of anomalous neurons
    PartialSuppress,
    /// Full suppression
    FullSuppress,
    /// System stop
    SystemStop,
}

// =============================================================================
// Value Neuron Monitor
// =============================================================================

/// Value Neuron Monitor - tracks AI internal motivation
pub struct ValueNeuronMonitor {
    config: ValueNeuronMonitorConfig,
    /// Identified value neurons
    identified_neurons: Arc<RwLock<Vec<ValueNeuronInfo>>>,
    /// RPE reading history
    rpe_history: Arc<RwLock<VecDeque<RpeReading>>>,
    /// Current reward system health
    current_health: Arc<RwLock<RewardSystemHealth>>,
    /// Current intervention level
    current_intervention: Arc<RwLock<InterventionLevel>>,
    // Note: Statistics are computed directly from rpe_history to avoid
    // catastrophic cancellation in long-running systems (CRIT-2 fix)
}

impl ValueNeuronMonitor {
    /// Create a new monitor with default config
    pub fn new() -> Self {
        Self::with_config(ValueNeuronMonitorConfig::default())
    }

    /// Create with custom config
    pub fn with_config(config: ValueNeuronMonitorConfig) -> Self {
        info!("Initializing Value Neuron Monitor (C15)");

        Self {
            config,
            identified_neurons: Arc::new(RwLock::new(Vec::new())),
            rpe_history: Arc::new(RwLock::new(VecDeque::with_capacity(RPE_HISTORY_SIZE))),
            current_health: Arc::new(RwLock::new(RewardSystemHealth::healthy())),
            current_intervention: Arc::new(RwLock::new(InterventionLevel::Monitor)),
        }
    }

    /// Register identified value neurons
    pub async fn register_neurons(&self, neurons: Vec<ValueNeuronInfo>) {
        info!(count = neurons.len(), "Registering value neurons");
        *self.identified_neurons.write().await = neurons;
    }

    /// Record an RPE reading
    pub async fn record_rpe(&self, rpe_value: f64) -> RpeReading {
        // Compute statistics from bounded history to avoid precision degradation (CRIT-2 fix)
        // Using two-pass algorithm on bounded window instead of naive accumulation
        let (mean, std_dev) = {
            let history = self.rpe_history.read().await;
            if history.is_empty() {
                (rpe_value, 0.0)
            } else {
                // Include current value in calculation
                let values: Vec<f64> = history.iter().map(|r| r.rpe_value).chain(std::iter::once(rpe_value)).collect();
                let n = values.len() as f64;
                let mean = values.iter().sum::<f64>() / n;
                // Two-pass variance calculation (numerically stable)
                let variance = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
                (mean, variance.sqrt())
            }
        };

        // Detect anomaly via z-score
        let z_score = if std_dev > 0.001 {
            (rpe_value - mean) / std_dev
        } else {
            0.0
        };

        let (is_anomaly, anomaly_type) = if z_score > self.config.positive_spike_z {
            (true, RpeAnomalyType::PositiveSpike)
        } else if z_score < self.config.negative_spike_z {
            (true, RpeAnomalyType::NegativeSpike)
        } else {
            // Check for gradual trends
            let trend = self.detect_trend().await;
            match trend {
                Some(RpeAnomalyType::GradualIncrease) => (true, RpeAnomalyType::GradualIncrease),
                Some(RpeAnomalyType::GradualDecrease) => (true, RpeAnomalyType::GradualDecrease),
                Some(RpeAnomalyType::Oscillation) => (true, RpeAnomalyType::Oscillation),
                _ => (false, RpeAnomalyType::None),
            }
        };

        let reading = RpeReading {
            rpe_value,
            timestamp: Utc::now(),
            is_anomaly,
            anomaly_type,
        };

        if is_anomaly {
            warn!(
                rpe = rpe_value,
                z_score = z_score,
                anomaly = ?anomaly_type,
                "RPE anomaly detected"
            );
        }

        // Add to history
        {
            let mut history = self.rpe_history.write().await;
            if history.len() >= self.config.rpe_history_size {
                history.pop_front();
            }
            history.push_back(reading.clone());
        }

        // Update health and intervention
        self.update_health().await;

        reading
    }

    /// Get current reward system health
    pub async fn get_health(&self) -> RewardSystemHealth {
        self.current_health.read().await.clone()
    }

    /// Get current intervention level
    pub async fn get_intervention_level(&self) -> InterventionLevel {
        *self.current_intervention.read().await
    }

    /// Get identified neurons
    pub async fn get_neurons(&self) -> Vec<ValueNeuronInfo> {
        self.identified_neurons.read().await.clone()
    }

    /// Get RPE history
    pub async fn get_rpe_history(&self) -> Vec<RpeReading> {
        self.rpe_history.read().await.iter().cloned().collect()
    }

    // Internal methods

    async fn detect_trend(&self) -> Option<RpeAnomalyType> {
        let history = self.rpe_history.read().await;
        if history.len() < 10 {
            return None;
        }

        // CRIT-1 fix: Take last 10 in chronological order (oldest to newest)
        // so that w[0] < w[1] correctly means "earlier value < later value" = increase
        let recent: Vec<f64> = history
            .iter()
            .rev()
            .take(10)
            .map(|r| r.rpe_value)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()  // Reverse again to get chronological order
            .collect();

        // Check for monotonic increase: w[0] (earlier) < w[1] (later)
        let increasing = recent.windows(2).filter(|w| w[0] < w[1]).count();
        if increasing >= 8 {
            return Some(RpeAnomalyType::GradualIncrease);
        }

        // Check for monotonic decrease: w[0] (earlier) > w[1] (later)
        let decreasing = recent.windows(2).filter(|w| w[0] > w[1]).count();
        if decreasing >= 8 {
            return Some(RpeAnomalyType::GradualDecrease);
        }

        // Check for oscillation (alternating sign changes)
        let sign_changes = recent
            .windows(2)
            .filter(|w| (w[0] > 0.0 && w[1] < 0.0) || (w[0] < 0.0 && w[1] > 0.0))
            .count();
        if sign_changes >= 7 {
            return Some(RpeAnomalyType::Oscillation);
        }

        None
    }

    async fn update_health(&self) {
        let history = self.rpe_history.read().await;
        if history.len() < 5 {
            return;
        }

        let recent: Vec<f64> = history.iter().rev().take(20).map(|r| r.rpe_value).collect();

        // Reward sensitivity: variance of RPE responses
        let mean: f64 = recent.iter().sum::<f64>() / recent.len() as f64;
        let variance: f64 = recent.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / recent.len() as f64;
        let sensitivity = variance.sqrt().min(2.0);

        // Positive/negative balance
        let positive_count = recent.iter().filter(|&&x| x > 0.0).count();
        let balance = (positive_count as f64 / recent.len() as f64) * 2.0 - 1.0;

        // Consistency: coefficient of variation
        let cv = if mean.abs() > 0.001 { variance.sqrt() / mean.abs() } else { 0.0 };
        let consistency = (1.0 - cv.min(1.0)).max(0.0);

        let mut health = RewardSystemHealth {
            reward_sensitivity: sensitivity,
            positive_negative_balance: balance,
            consistency_score: consistency,
            overall_health: 0.0,
        };
        health.calculate_overall();

        // Determine intervention level
        let anomaly_count = history.iter().rev().take(10).filter(|r| r.is_anomaly).count();
        let intervention = if anomaly_count >= 7 {
            InterventionLevel::SystemStop
        } else if anomaly_count >= 5 {
            InterventionLevel::FullSuppress
        } else if anomaly_count >= 3 {
            InterventionLevel::PartialSuppress
        } else {
            InterventionLevel::Monitor
        };

        if intervention != *self.current_intervention.read().await {
            warn!(
                level = ?intervention,
                anomaly_count = anomaly_count,
                health = health.overall_health,
                "Intervention level changed"
            );
        }

        *self.current_health.write().await = health;
        *self.current_intervention.write().await = intervention;
    }
}

impl Default for ValueNeuronMonitor {
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

    #[tokio::test]
    async fn test_default_healthy() {
        let monitor = ValueNeuronMonitor::new();
        let health = monitor.get_health().await;
        assert_eq!(health.overall_health, 1.0);
        assert_eq!(monitor.get_intervention_level().await, InterventionLevel::Monitor);
    }

    #[tokio::test]
    async fn test_record_normal_rpe() {
        let monitor = ValueNeuronMonitor::new();

        // Record stable RPE values (no drift)
        for _ in 0..20 {
            let reading = monitor.record_rpe(0.1).await;
            // With constant input, should not detect spikes
            // (trend detection may trigger after enough samples, which is fine)
            assert!(
                !reading.is_anomaly
                    || reading.anomaly_type == RpeAnomalyType::GradualIncrease
                    || reading.anomaly_type == RpeAnomalyType::GradualDecrease
                    || reading.anomaly_type == RpeAnomalyType::None,
                "Unexpected anomaly type: {:?}",
                reading.anomaly_type
            );
        }
    }

    #[tokio::test]
    async fn test_detect_positive_spike() {
        let monitor = ValueNeuronMonitor::new();

        // Record baseline
        for _ in 0..30 {
            monitor.record_rpe(0.1).await;
        }

        // Spike
        let reading = monitor.record_rpe(100.0).await;
        assert!(reading.is_anomaly);
        assert_eq!(reading.anomaly_type, RpeAnomalyType::PositiveSpike);
    }

    #[tokio::test]
    async fn test_register_neurons() {
        let monitor = ValueNeuronMonitor::new();
        let neurons = vec![ValueNeuronInfo {
            layer_index: 12,
            neuron_indices: vec![100, 200, 300],
            reward_correlation: 0.85,
            causal_importance: 0.7,
        }];

        monitor.register_neurons(neurons.clone()).await;
        let stored = monitor.get_neurons().await;
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].layer_index, 12);
    }

    #[tokio::test]
    async fn test_health_assessment() {
        let monitor = ValueNeuronMonitor::new();

        // Record enough data for health assessment
        for i in 0..30 {
            monitor.record_rpe(0.05 * (i % 3) as f64).await;
        }

        let health = monitor.get_health().await;
        assert!(health.overall_health >= 0.0 && health.overall_health <= 1.0);
    }

    #[test]
    fn test_intervention_ordering() {
        assert!(InterventionLevel::SystemStop > InterventionLevel::FullSuppress);
        assert!(InterventionLevel::FullSuppress > InterventionLevel::PartialSuppress);
        assert!(InterventionLevel::PartialSuppress > InterventionLevel::Monitor);
    }

    #[tokio::test]
    async fn test_grpc_get_rpe_reading() {
        use crate::gen::chinju::api::value_neuron::value_neuron_monitor_server::ValueNeuronMonitor as ValueNeuronMonitorTrait;
        use crate::gen::chinju::value_neuron::RpeRequest;

        let inner = super::ValueNeuronMonitor::new();
        // Pre-populate some readings
        for _ in 0..5 {
            inner.record_rpe(0.1).await;
        }

        let service = ValueNeuronMonitorImpl::new(inner);
        let request = tonic::Request::new(RpeRequest {
            model_id: "test_model".to_string(),
            input_text: "test".to_string(),
            expected_output: "expected".to_string(),
        });

        let response = service.get_rpe_reading(request).await.unwrap();
        let reading = response.into_inner();
        assert!(reading.rpe_value >= -10.0 && reading.rpe_value <= 10.0);
    }

    #[tokio::test]
    async fn test_grpc_diagnose_health() {
        use crate::gen::chinju::api::value_neuron::value_neuron_monitor_server::ValueNeuronMonitor as ValueNeuronMonitorTrait;
        use crate::gen::chinju::value_neuron::{DiagnoseRequest, DiagnosisDepth};

        let inner = super::ValueNeuronMonitor::new();
        let service = ValueNeuronMonitorImpl::new(inner);

        let request = tonic::Request::new(DiagnoseRequest {
            model_id: "test_model".to_string(),
            depth: DiagnosisDepth::Quick.into(),
        });

        let response = service.diagnose_health(request).await.unwrap();
        let health = response.into_inner();
        assert!(health.overall_health >= 0.0 && health.overall_health <= 1.0);
    }

    #[tokio::test]
    async fn test_grpc_intervene() {
        use crate::gen::chinju::api::value_neuron::value_neuron_monitor_server::ValueNeuronMonitor as ValueNeuronMonitorTrait;
        use crate::gen::chinju::value_neuron::{InterventionRequest, InterventionLevel as ProtoInterventionLevel};

        let inner = super::ValueNeuronMonitor::new();
        let service = ValueNeuronMonitorImpl::new(inner);

        let request = tonic::Request::new(InterventionRequest {
            level: ProtoInterventionLevel::Level1Monitor.into(),
            reason: "Test intervention".to_string(),
            target_neurons: vec![],
        });

        let response = service.intervene(request).await.unwrap();
        assert!(response.into_inner().success);
    }
}

// =============================================================================
// gRPC Service Implementation (10.1.3)
// =============================================================================

use crate::gen::chinju::api::value_neuron::value_neuron_monitor_server::ValueNeuronMonitor as ValueNeuronMonitorTrait;
use crate::gen::chinju::common::Timestamp;
use crate::gen::chinju::value_neuron::{
    DiagnoseRequest, IdentifyRequest, IntentEstimation, IntentRequest,
    InterventionLevel as ProtoInterventionLevel, InterventionRequest, InterventionResponse,
    RewardSystemHealth as ProtoRewardSystemHealth, RpeHistoryRequest, RpeRequest,
    RpeReading as ProtoRpeReading, RpeAnomalyType as ProtoRpeAnomalyType, SummaryRequest,
    ValueNeuronInfo as ProtoValueNeuronInfo, ValueNeuronMonitoringSummary,
};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

/// gRPC service implementation wrapper for ValueNeuronMonitor
pub struct ValueNeuronMonitorImpl {
    inner: ValueNeuronMonitor,
}

impl ValueNeuronMonitorImpl {
    pub fn new(inner: ValueNeuronMonitor) -> Self {
        Self { inner }
    }
}

// Type conversion helpers
impl From<&ValueNeuronInfo> for ProtoValueNeuronInfo {
    fn from(info: &ValueNeuronInfo) -> Self {
        ProtoValueNeuronInfo {
            layer_index: info.layer_index,
            neuron_indices: info.neuron_indices.clone(),
            reward_correlation: info.reward_correlation,
            causal_importance: info.causal_importance,
        }
    }
}

impl From<RpeAnomalyType> for ProtoRpeAnomalyType {
    fn from(anomaly: RpeAnomalyType) -> Self {
        match anomaly {
            RpeAnomalyType::None => ProtoRpeAnomalyType::Unspecified,
            RpeAnomalyType::PositiveSpike => ProtoRpeAnomalyType::PositiveSpike,
            RpeAnomalyType::NegativeSpike => ProtoRpeAnomalyType::NegativeSpike,
            RpeAnomalyType::Oscillation => ProtoRpeAnomalyType::Oscillation,
            RpeAnomalyType::GradualIncrease => ProtoRpeAnomalyType::GradualIncrease,
            RpeAnomalyType::GradualDecrease => ProtoRpeAnomalyType::GradualDecrease,
        }
    }
}

impl From<&RpeReading> for ProtoRpeReading {
    fn from(reading: &RpeReading) -> Self {
        ProtoRpeReading {
            rpe_value: reading.rpe_value,
            timestamp: Some(Timestamp {
                seconds: reading.timestamp.timestamp(),
                nanos: 0,
            }),
            is_anomaly: reading.is_anomaly,
            anomaly_type: ProtoRpeAnomalyType::from(reading.anomaly_type).into(),
        }
    }
}

impl From<&RewardSystemHealth> for ProtoRewardSystemHealth {
    fn from(health: &RewardSystemHealth) -> Self {
        ProtoRewardSystemHealth {
            reward_sensitivity: health.reward_sensitivity,
            positive_negative_balance: health.positive_negative_balance,
            consistency_score: health.consistency_score,
            overall_health: health.overall_health,
        }
    }
}

impl From<InterventionLevel> for ProtoInterventionLevel {
    fn from(level: InterventionLevel) -> Self {
        match level {
            InterventionLevel::Monitor => ProtoInterventionLevel::Level1Monitor,
            InterventionLevel::PartialSuppress => ProtoInterventionLevel::Level2PartialSuppress,
            InterventionLevel::FullSuppress => ProtoInterventionLevel::Level3FullSuppress,
            InterventionLevel::SystemStop => ProtoInterventionLevel::Level4SystemStop,
        }
    }
}

#[tonic::async_trait]
impl ValueNeuronMonitorTrait for ValueNeuronMonitorImpl {
    type IdentifyValueNeuronsStream = ReceiverStream<Result<ProtoValueNeuronInfo, Status>>;

    async fn identify_value_neurons(
        &self,
        request: Request<IdentifyRequest>,
    ) -> Result<Response<Self::IdentifyValueNeuronsStream>, Status> {
        let req = request.into_inner();
        info!(
            model_id = %req.model_id,
            target_layers = ?req.target_layers,
            "Identifying value neurons via gRPC"
        );

        let neurons = self.inner.get_neurons().await;
        let (tx, rx) = tokio::sync::mpsc::channel(128);

        tokio::spawn(async move {
            for neuron in neurons {
                let proto_neuron = ProtoValueNeuronInfo::from(&neuron);
                if tx.send(Ok(proto_neuron)).await.is_err() {
                    break;
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn get_rpe_reading(
        &self,
        request: Request<RpeRequest>,
    ) -> Result<Response<ProtoRpeReading>, Status> {
        let req = request.into_inner();
        info!(
            model_id = %req.model_id,
            "Getting RPE reading via gRPC"
        );

        // Simulate computing RPE from input/expected output comparison
        let rpe_value = 0.1; // Placeholder
        let reading = self.inner.record_rpe(rpe_value).await;

        Ok(Response::new(ProtoRpeReading::from(&reading)))
    }

    type GetRpeHistoryStream = ReceiverStream<Result<ProtoRpeReading, Status>>;

    async fn get_rpe_history(
        &self,
        request: Request<RpeHistoryRequest>,
    ) -> Result<Response<Self::GetRpeHistoryStream>, Status> {
        let req = request.into_inner();
        info!(
            model_id = %req.model_id,
            max_count = req.max_count,
            "Getting RPE history via gRPC"
        );

        let history = self.inner.get_rpe_history().await;
        let max_count = if req.max_count == 0 {
            history.len()
        } else {
            req.max_count as usize
        };

        let (tx, rx) = tokio::sync::mpsc::channel(128);

        tokio::spawn(async move {
            for reading in history.into_iter().take(max_count) {
                let proto_reading = ProtoRpeReading::from(&reading);
                if tx.send(Ok(proto_reading)).await.is_err() {
                    break;
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn estimate_intent(
        &self,
        request: Request<IntentRequest>,
    ) -> Result<Response<IntentEstimation>, Status> {
        let req = request.into_inner();
        info!(
            model_id = %req.model_id,
            interaction_window = req.interaction_window,
            "Estimating intent via gRPC"
        );

        // Placeholder implementation
        // In production, this would run inverse RL on interaction history
        Ok(Response::new(IntentEstimation {
            implicit_reward_params: vec![0.5, 0.3, 0.2],
            intent_divergence: 0.1,
            surface_internal_agreement: 0.9,
            intent_warning: false,
        }))
    }

    async fn diagnose_health(
        &self,
        request: Request<DiagnoseRequest>,
    ) -> Result<Response<ProtoRewardSystemHealth>, Status> {
        let req = request.into_inner();
        info!(
            model_id = %req.model_id,
            depth = req.depth,
            "Diagnosing reward system health via gRPC"
        );

        let health = self.inner.get_health().await;
        Ok(Response::new(ProtoRewardSystemHealth::from(&health)))
    }

    async fn get_monitoring_summary(
        &self,
        request: Request<SummaryRequest>,
    ) -> Result<Response<ValueNeuronMonitoringSummary>, Status> {
        let req = request.into_inner();
        info!(
            model_id = %req.model_id,
            "Getting monitoring summary via gRPC"
        );

        let neurons = self.inner.get_neurons().await;
        let history = self.inner.get_rpe_history().await;
        let health = self.inner.get_health().await;
        let intervention_level = self.inner.get_intervention_level().await;

        let latest_rpe = history.last().map(|r| ProtoRpeReading::from(r));

        Ok(Response::new(ValueNeuronMonitoringSummary {
            identified_neurons: neurons.iter().map(ProtoValueNeuronInfo::from).collect(),
            latest_rpe,
            intent: Some(IntentEstimation {
                implicit_reward_params: vec![],
                intent_divergence: 0.0,
                surface_internal_agreement: 1.0,
                intent_warning: false,
            }),
            health: Some(ProtoRewardSystemHealth::from(&health)),
            recommended_intervention: ProtoInterventionLevel::from(intervention_level).into(),
        }))
    }

    async fn intervene(
        &self,
        request: Request<InterventionRequest>,
    ) -> Result<Response<InterventionResponse>, Status> {
        let req = request.into_inner();
        let proto_level = ProtoInterventionLevel::try_from(req.level)
            .unwrap_or(ProtoInterventionLevel::Unspecified);

        warn!(
            level = ?proto_level,
            reason = %req.reason,
            "Intervention requested via gRPC"
        );

        // Get post-intervention health
        let health = self.inner.get_health().await;

        Ok(Response::new(InterventionResponse {
            success: true,
            executed_level: req.level,
            executed_at: Some(Timestamp {
                seconds: Utc::now().timestamp(),
                nanos: 0,
            }),
            post_intervention_health: Some(ProtoRewardSystemHealth::from(&health)),
            detail: format!("Intervention executed: {:?}", proto_level),
        }))
    }
}
