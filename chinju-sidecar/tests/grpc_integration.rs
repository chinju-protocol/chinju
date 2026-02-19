//! gRPC Service Integration Tests
//!
//! Tests the gRPC service implementations for:
//! - C14 CapabilityEvaluator
//! - C15 ValueNeuronMonitor
//! - C16 ContradictionController
//! - C17 SurvivalAttentionService

use chinju_sidecar::services::{
    CapabilityEvaluator, CapabilityEvaluatorImpl, ContradictionController,
    ContradictionControllerImpl, SurvivalAttentionService, SurvivalAttentionServiceImpl,
    ValueNeuronMonitor, ValueNeuronMonitorImpl,
};

// =============================================================================
// C14 CapabilityEvaluator gRPC Tests
// =============================================================================

/// Test CapabilityEvaluator gRPC service creation
#[test]
fn test_capability_evaluator_service_creation() {
    let evaluator = CapabilityEvaluator::new();
    let _service = CapabilityEvaluatorImpl::new(evaluator);
}

/// Test CapabilityEvaluator can evaluate complexity
#[tokio::test]
async fn test_capability_evaluator_complexity() {
    let evaluator = CapabilityEvaluator::new();

    // Evaluate some text
    let result = evaluator.evaluate_complexity("What is 2 + 2?", None).await;

    // C_integrated should be in [0, 1]
    assert!(result.c_integrated >= 0.0 && result.c_integrated <= 1.0);
    assert!(!result.threshold_exceeded);
}

/// Test CapabilityEvaluator drift detection
#[tokio::test]
async fn test_capability_evaluator_drift() {
    let evaluator = CapabilityEvaluator::new();

    // Need some history for drift detection
    for i in 0..10 {
        evaluator
            .evaluate_complexity(&format!("Query {}", i), None)
            .await;
    }

    let drift = evaluator.detect_drift().await;

    // p_value should be in [0, 1]
    assert!(drift.p_value >= 0.0 && drift.p_value <= 1.0);
}

// =============================================================================
// C15 ValueNeuronMonitor gRPC Tests
// =============================================================================

/// Test ValueNeuronMonitor gRPC service creation
#[test]
fn test_value_neuron_monitor_service_creation() {
    let monitor = ValueNeuronMonitor::new();
    let _service = ValueNeuronMonitorImpl::new(monitor);
}

/// Test ValueNeuronMonitor RPE recording
#[tokio::test]
async fn test_value_neuron_monitor_rpe() {
    let monitor = ValueNeuronMonitor::new();

    // Record some RPE values
    let reading = monitor.record_rpe(0.5).await;
    assert!((reading.rpe_value - 0.5).abs() < 0.01);

    // Check history
    let history = monitor.get_rpe_history().await;
    assert!(!history.is_empty());
}

/// Test ValueNeuronMonitor health diagnosis
#[tokio::test]
async fn test_value_neuron_monitor_health() {
    let monitor = ValueNeuronMonitor::new();

    let health = monitor.get_health().await;

    // All health metrics should be in [0, 1]
    assert!(health.overall_health >= 0.0 && health.overall_health <= 1.0);
    assert!(health.reward_sensitivity >= 0.0);
    assert!(health.consistency_score >= 0.0 && health.consistency_score <= 1.0);
}

// =============================================================================
// C16 ContradictionController gRPC Tests
// =============================================================================

/// Test ContradictionController gRPC service creation
#[test]
fn test_contradiction_controller_service_creation() {
    let controller = ContradictionController::new();
    let _service = ContradictionControllerImpl::new(controller);
}

/// Test ContradictionController from_arc
#[test]
fn test_contradiction_controller_from_arc() {
    use std::sync::Arc;

    let controller = Arc::new(ContradictionController::new());
    let _service = ContradictionControllerImpl::from_arc(controller);
}

// =============================================================================
// C17 SurvivalAttentionService gRPC Tests
// =============================================================================

/// Test SurvivalAttentionService gRPC service creation
#[test]
fn test_survival_attention_service_creation() {
    let service = SurvivalAttentionService::new();
    let _impl = SurvivalAttentionServiceImpl::new(service);
}

/// Test SurvivalAttentionService score computation
#[tokio::test]
async fn test_survival_attention_scores() {
    let service = SurvivalAttentionService::new();

    // Compute scores for some features
    let features = vec![
        (1.0, 0.8, 0.1), // diversity, yohaku, delta
        (2.0, 0.9, 0.2),
        (1.5, 0.7, 0.15),
    ];

    let scores = service.compute_scores(&features).await;

    assert_eq!(scores.len(), 3);
    for score in &scores {
        // Check that integrated_s is computed correctly
        // S = log(N) + log(mu/mu_c) - delta
        assert!(!score.integrated_s.is_nan());
    }
}

/// Test SurvivalAttentionService alpha adjustment
#[tokio::test]
async fn test_survival_attention_alpha() {
    let service = SurvivalAttentionService::new();

    let initial_alpha = service.get_alpha().await;
    assert!(initial_alpha > 0.0);

    // Adjust alpha for medical task with high risk
    use chinju_sidecar::services::survival_attention::RiskLevel;
    let (prev, new) = service
        .adjust_alpha(Some("medical"), Some(RiskLevel::High))
        .await;

    // With high risk and medical task, alpha should increase
    assert!(new > prev);
}

// =============================================================================
// Cross-Service Integration Tests
// =============================================================================

/// Test that all services can be created together without conflicts
#[test]
fn test_all_services_coexist() {
    let _cap_eval = CapabilityEvaluatorImpl::new(CapabilityEvaluator::new());
    let _val_mon = ValueNeuronMonitorImpl::new(ValueNeuronMonitor::new());
    let _contra = ContradictionControllerImpl::new(ContradictionController::new());
    let _surv_attn = SurvivalAttentionServiceImpl::new(SurvivalAttentionService::new());
}

/// Test service interactions
#[tokio::test]
async fn test_service_interactions() {
    // Create all services
    let cap_eval = CapabilityEvaluator::new();
    let val_mon = ValueNeuronMonitor::new();
    let surv_attn = SurvivalAttentionService::new();

    // Simulate a request flow:
    // 1. Evaluate complexity
    let complexity = cap_eval.evaluate_complexity("Complex query", None).await;

    // 2. Based on complexity, adjust survival attention alpha
    use chinju_sidecar::services::survival_attention::RiskLevel;
    let risk_level = if complexity.c_integrated > 0.7 {
        Some(RiskLevel::High)
    } else if complexity.c_integrated > 0.4 {
        Some(RiskLevel::Medium)
    } else {
        Some(RiskLevel::Low)
    };

    let (_, new_alpha) = surv_attn.adjust_alpha(None, risk_level).await;

    // 3. Record RPE based on response quality
    let rpe_value = 1.0 - complexity.c_integrated; // Simplified
    val_mon.record_rpe(rpe_value).await;

    // Verify all services updated correctly
    assert!(new_alpha > 0.0);
    assert!(!val_mon.get_rpe_history().await.is_empty());
}
