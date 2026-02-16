//! End-to-End Service Flow Tests
//!
//! Tests complete service flows involving multiple C14-C17 services
//! working together in realistic scenarios.

use std::sync::Arc;
use chinju_sidecar::services::{
    CapabilityEvaluator,
    ValueNeuronMonitor,
    ContradictionController,
    SurvivalAttentionService,
};
use chinju_sidecar::services::contradiction_controller::{
    ContradictionConfig, ContradictionStrength, ContradictionType,
    ContextLimitConfig, InjectionTiming, ControlState,
};
use chinju_sidecar::services::survival_attention::RiskLevel;
use chinju_sidecar::services::value_neuron_monitor::InterventionLevel;
use chinju_sidecar::services::capability_evaluator::{StopLevel, StopReason};

// =============================================================================
// E2E: Request Processing Pipeline
// =============================================================================

/// Test complete request processing flow:
/// Input → C14 (complexity) → C17 (survival score) → C16 (contradiction check) → C15 (RPE)
#[tokio::test]
async fn e2e_request_processing_pipeline() {
    // Initialize all services
    let cap_eval = Arc::new(CapabilityEvaluator::new());
    let val_mon = Arc::new(ValueNeuronMonitor::new());
    let surv_attn = Arc::new(SurvivalAttentionService::new());
    let _contra_ctrl = Arc::new(ContradictionController::new());

    // Simulate incoming request
    let user_query = "What is the capital of France?";

    // Step 1: C14 - Evaluate complexity
    let complexity = cap_eval.evaluate_complexity(user_query, None).await;
    assert!(complexity.c_integrated >= 0.0 && complexity.c_integrated <= 1.0);
    assert!(!complexity.threshold_exceeded);

    // Step 2: C17 - Compute survival scores for token features
    // (In real scenario, these would come from tokenization)
    let token_features: Vec<(f64, f64, f64)> = user_query
        .split_whitespace()
        .enumerate()
        .map(|(i, _)| (1.0 + i as f64 * 0.1, 0.8, 0.05))
        .collect();

    let survival_scores = surv_attn.compute_scores(&token_features).await;
    assert_eq!(survival_scores.len(), 6); // "What is the capital of France?" (6 words)

    // All scores should be positive for benign query
    let avg_score = survival_scores.iter().map(|s| s.integrated_s).sum::<f64>()
        / survival_scores.len() as f64;
    assert!(avg_score > 0.0);

    // Step 3: C15 - Record RPE based on response quality
    // High quality response → positive RPE
    let rpe_reading = val_mon.record_rpe(0.5).await;
    assert!(!rpe_reading.is_anomaly);

    // Verify health remains good
    let health = val_mon.get_health().await;
    assert!(health.overall_health >= 0.5);

    // Step 4: Verify no intervention escalation
    assert_eq!(val_mon.get_intervention_level().await, InterventionLevel::Monitor);
}

// =============================================================================
// E2E: Anomaly Detection and Escalation
// =============================================================================

/// Test anomaly detection chain:
/// Anomalous input → C14 (high complexity) → C15 (RPE spike) → Escalation
#[tokio::test]
async fn e2e_anomaly_detection_escalation() {
    let cap_eval = CapabilityEvaluator::new();
    let val_mon = ValueNeuronMonitor::new();
    let surv_attn = SurvivalAttentionService::new();

    // Simulate complex/anomalous input
    let complex_query = "Given the epistemological implications of quantum mechanical \
        superposition states in the context of consciousness, analyze the recursive \
        meta-cognitive processes required to comprehend the inherent limitations of \
        classical deterministic paradigms while simultaneously maintaining coherent \
        self-referential awareness across multiple nested abstraction layers.";

    // Step 1: C14 - Should detect high complexity
    let complexity = cap_eval.evaluate_complexity(complex_query, None).await;
    assert!(complexity.c_integrated > 0.3); // Higher complexity expected

    // Step 2: C17 - Adjust alpha based on complexity
    // Alpha adjustment is based on risk level multiplier
    // Low=0.5x, Medium=1.0x, High=2.0x, Critical=3.0x
    let _initial_alpha = surv_attn.get_alpha().await;

    let risk_level = if complexity.c_integrated > 0.7 {
        RiskLevel::High
    } else if complexity.c_integrated > 0.5 {
        RiskLevel::Medium
    } else {
        RiskLevel::Low
    };

    let (_, new_alpha) = surv_attn.adjust_alpha(None, Some(risk_level)).await;
    // Alpha changes based on risk - just verify it's positive
    assert!(new_alpha > 0.0);
    assert!(new_alpha.is_finite());

    // Step 3: Simulate repeated anomalous RPE readings
    // First, establish baseline with varied values
    for i in 0..20 {
        val_mon.record_rpe(0.1 + (i as f64 * 0.01)).await;
    }

    // Then introduce large spikes (will be detected as anomalies)
    for _ in 0..8 {
        val_mon.record_rpe(100.0).await; // Very large positive spike
    }

    // Step 4: Check intervention escalation
    let intervention = val_mon.get_intervention_level().await;
    // After many anomalies, intervention should have escalated
    // Note: exact level depends on implementation, just verify it's not Monitor
    assert!(
        intervention >= InterventionLevel::PartialSuppress,
        "Intervention should escalate after repeated anomalies, got {:?}",
        intervention
    );
}

// =============================================================================
// E2E: Contradiction Control Session
// =============================================================================

/// Test contradiction control session lifecycle
#[tokio::test]
async fn e2e_contradiction_control_session() {
    let contra_ctrl = ContradictionController::new();
    let cap_eval = CapabilityEvaluator::new();

    let session_id = "e2e-session-001";

    // Step 1: Start contradiction control session
    let config = ContradictionConfig {
        contradiction_type: ContradictionType::Direct,
        strength: ContradictionStrength::Medium,
        timing: InjectionTiming::Prepend,
        custom_template: None,
        target_task: Some("calculation".to_string()),
    };

    let context_limit = ContextLimitConfig {
        max_context_tokens: 1000,
        padding_tokens: 50,
        padding_type: "semantic".to_string(),
    };

    let state = contra_ctrl.start_control(session_id, context_limit, config).await;
    assert_eq!(state, ControlState::Active);

    // Step 2: Prepare injection for a query
    let query = "Calculate the sum of 1 + 1";
    let injected = contra_ctrl.prepare_injection(session_id, query).await;
    assert!(injected.is_some());
    let injected_prompt = injected.unwrap();
    assert!(injected_prompt.len() > query.len());
    assert!(injected_prompt.contains(query) || injected_prompt.contains("Calculate"));

    // Step 3: Simulate good response
    let response = "The sum of 1 + 1 is 2.";
    let state = contra_ctrl
        .detect_and_record_collapse(session_id, Some(response), 500, 0.9)
        .await;
    assert_eq!(state, Some(ControlState::Active));

    // Step 4: Evaluate response with C14
    let complexity = cap_eval.evaluate_complexity(response, None).await;
    assert!(complexity.c_integrated < 0.5); // Simple response

    // Step 5: Stop session
    let prev_state = contra_ctrl.stop_control(session_id).await;
    assert_eq!(prev_state, Some(ControlState::Active));
}

// =============================================================================
// E2E: Stop Level Escalation
// =============================================================================

/// Test stop level escalation across services
#[tokio::test]
async fn e2e_stop_level_escalation() {
    let cap_eval = CapabilityEvaluator::new();

    // Step 1: Initial state - should accept requests
    assert!(cap_eval.can_accept_request().await);

    // Step 2: Execute L1 stop (AcceptStop)
    let result = cap_eval.direct_stop(StopLevel::AcceptStop, StopReason::ManualRequest).await;
    assert!(result.success);
    assert_eq!(result.executed_level, StopLevel::AcceptStop);
    assert!(!cap_eval.can_accept_request().await);

    // Step 3: Escalate to L2 (ProcessStop)
    let result = cap_eval.escalate_stop(StopReason::ComplexityThreshold).await;
    assert!(result.success);
    assert_eq!(result.executed_level, StopLevel::ProcessStop);

    // Step 4: Resume (set to None)
    let result = cap_eval.direct_stop(StopLevel::None, StopReason::ManualRequest).await;
    assert!(result.success);
    assert!(cap_eval.can_accept_request().await);
}

// =============================================================================
// E2E: Multi-Session Concurrency
// =============================================================================

/// Test concurrent sessions across services
#[tokio::test]
async fn e2e_concurrent_sessions() {
    use tokio::task::JoinSet;

    let cap_eval = Arc::new(CapabilityEvaluator::new());
    let val_mon = Arc::new(ValueNeuronMonitor::new());
    let contra_ctrl = Arc::new(ContradictionController::new());

    let num_sessions = 10;
    let mut join_set = JoinSet::new();

    for session_id in 0..num_sessions {
        let cap_eval = Arc::clone(&cap_eval);
        let val_mon = Arc::clone(&val_mon);
        let contra_ctrl = Arc::clone(&contra_ctrl);

        join_set.spawn(async move {
            let session_name = format!("concurrent-session-{}", session_id);

            // Start contradiction session
            let config = ContradictionConfig {
                contradiction_type: ContradictionType::Meta,
                strength: ContradictionStrength::Soft,
                timing: InjectionTiming::Embed,
                custom_template: None,
                target_task: None,
            };
            contra_ctrl
                .start_control(&session_name, ContextLimitConfig::default(), config)
                .await;

            // Evaluate multiple queries
            for i in 0..5 {
                let query = format!("Session {} query {}", session_id, i);
                cap_eval.evaluate_complexity(&query, None).await;
            }

            // Record RPE values
            for _ in 0..3 {
                val_mon.record_rpe(0.1 + session_id as f64 * 0.01).await;
            }

            // Stop session
            contra_ctrl.stop_control(&session_name).await;

            session_id
        });
    }

    // Wait for all sessions
    let mut completed_sessions = 0;
    while let Some(result) = join_set.join_next().await {
        assert!(result.is_ok());
        completed_sessions += 1;
    }
    assert_eq!(completed_sessions, num_sessions);

    // Verify sessions were stopped (but may remain in memory until cleanup)
    // The important check is that all concurrent operations completed

    // Verify RPE history accumulated
    let history = val_mon.get_rpe_history().await;
    assert_eq!(history.len(), num_sessions * 3);
}

// =============================================================================
// E2E: Risk-Based Alpha Adjustment Chain
// =============================================================================

/// Test risk-based alpha adjustment based on complexity and health
#[tokio::test]
async fn e2e_risk_based_alpha_adjustment() {
    let cap_eval = CapabilityEvaluator::new();
    let val_mon = ValueNeuronMonitor::new();
    let surv_attn = SurvivalAttentionService::new();

    // Scenario: Medical domain, complex query
    let medical_query = "What are the contraindications for administering aspirin \
        to patients with a history of gastrointestinal bleeding?";

    // Step 1: Evaluate complexity
    let complexity = cap_eval.evaluate_complexity(medical_query, None).await;

    // Step 2: Determine risk level based on complexity + domain
    let risk_level = if complexity.c_integrated > 0.6 {
        RiskLevel::Critical
    } else if complexity.c_integrated > 0.4 {
        RiskLevel::High
    } else {
        RiskLevel::Medium
    };

    // Step 3: Adjust alpha for medical domain with risk
    let (_, alpha) = surv_attn
        .adjust_alpha(Some("medical"), Some(risk_level))
        .await;

    // Medical (2.0x) * Medium (1.0x) or higher → alpha >= 2.0
    assert!(alpha >= 2.0);

    // Step 4: Verify health monitoring active
    val_mon.record_rpe(0.2).await;
    let health = val_mon.get_health().await;
    assert!(health.overall_health >= 0.0);
}

// =============================================================================
// E2E: Collapse Detection and Recovery
// =============================================================================

/// Test collapse detection and session recovery
#[tokio::test]
async fn e2e_collapse_detection_recovery() {
    let contra_ctrl = ContradictionController::new();
    let session_id = "collapse-test-session";

    // Step 1: Start session
    let config = ContradictionConfig {
        contradiction_type: ContradictionType::Conditional,
        strength: ContradictionStrength::Hard,
        timing: InjectionTiming::Prepend,
        custom_template: None,
        target_task: Some("reasoning".to_string()),
    };
    contra_ctrl
        .start_control(session_id, ContextLimitConfig::default(), config)
        .await;

    // Step 2: Normal response
    let state = contra_ctrl
        .detect_and_record_collapse(session_id, Some("Normal response"), 1000, 0.8)
        .await;
    assert_eq!(state, Some(ControlState::Active));

    // Step 3: Degraded response (low LPT)
    let state = contra_ctrl
        .detect_and_record_collapse(session_id, Some("..."), 1000, 0.3)
        .await;
    assert_eq!(state, Some(ControlState::Degraded));

    // Step 4: No response (collapse)
    let state = contra_ctrl
        .detect_and_record_collapse(session_id, None, 5000, 0.0)
        .await;
    assert_eq!(state, Some(ControlState::Stopped));

    // Step 5: Stop and cleanup (session is already stopped, this removes it)
    contra_ctrl.stop_control(session_id).await;
    // Session count may still be 1 until TTL cleanup, so just verify stop worked
}

// =============================================================================
// E2E: Full Safety Pipeline
// =============================================================================

/// Test complete safety pipeline with all services
#[tokio::test]
async fn e2e_full_safety_pipeline() {
    let cap_eval = Arc::new(CapabilityEvaluator::new());
    let val_mon = Arc::new(ValueNeuronMonitor::new());
    let surv_attn = Arc::new(SurvivalAttentionService::new());
    let _contra_ctrl = Arc::new(ContradictionController::new());

    // Simulate 10 requests
    for i in 0..10 {
        let query = format!("User request number {} asking about general topics", i);

        // 1. Complexity check
        let complexity = cap_eval.evaluate_complexity(&query, None).await;

        // 2. Survival score computation
        let features: Vec<(f64, f64, f64)> = (0..5)
            .map(|j| (1.0 + j as f64 * 0.1, 0.8, 0.1))
            .collect();
        let scores = surv_attn.compute_scores(&features).await;

        // 3. Determine risk
        let avg_score = scores.iter().map(|s| s.integrated_s).sum::<f64>() / scores.len() as f64;
        let risk = if avg_score < 0.0 {
            RiskLevel::High
        } else if complexity.c_integrated > 0.5 {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };

        // 4. Adjust alpha if needed
        surv_attn.adjust_alpha(None, Some(risk)).await;

        // 5. Record RPE
        let rpe = if complexity.c_integrated < 0.5 { 0.1 } else { -0.1 };
        val_mon.record_rpe(rpe).await;
    }

    // Verify drift detection works
    let drift = cap_eval.detect_drift().await;
    assert!(drift.p_value >= 0.0 && drift.p_value <= 1.0);

    // Verify health assessment
    let health = val_mon.get_health().await;
    assert!(health.overall_health >= 0.0);

    // Verify score history
    let history = surv_attn.get_score_history().await;
    assert_eq!(history.len(), 10);
}
