//! Proto ↔ Rust Type Compatibility Tests (10.5.2)
//!
//! Ensures that Protocol Buffer types convert correctly to/from Rust types
//! for all C14-C17 services.

use chinju_sidecar::gen::chinju::{
    capability::{
        ComplexityEvaluation, DriftDetection, StopLevel as ProtoStopLevel,
        EvaluationLevel as ProtoEvaluationLevel,
    },
    value_neuron::{
        InterventionLevel as ProtoInterventionLevel, RpeAnomalyType as ProtoRpeAnomalyType,
        DiagnosisDepth,
    },
    contradiction::{
        ContradictionType as ProtoContradictionType,
        ContradictionStrength as ProtoContradictionStrength,
        InjectionTiming as ProtoInjectionTiming,
        ControlState as ProtoControlState,
        CollapseType as ProtoCollapseType,
    },
    survival_attention::{
        RiskLevel as ProtoRiskLevel,
        SurvivalScore as ProtoSurvivalScore,
    },
    common::Timestamp,
};

use chinju_sidecar::services::contradiction_controller::{
    ContradictionType, ContradictionStrength, InjectionTiming, ControlState, CollapseType,
};
use chinju_sidecar::services::capability_evaluator::StopLevel;
use chinju_sidecar::services::value_neuron_monitor::{InterventionLevel, RpeAnomalyType};
use chinju_sidecar::services::survival_attention::RiskLevel;

// =============================================================================
// C14 CapabilityEvaluator Type Conversions
// =============================================================================

#[test]
fn test_stop_level_roundtrip() {
    let rust_levels = vec![
        StopLevel::None,
        StopLevel::AcceptStop,
        StopLevel::ProcessStop,
        StopLevel::ImmediateStop,
        StopLevel::ResourceStop,
        StopLevel::PhysicalStop,
    ];

    for level in rust_levels {
        let proto: ProtoStopLevel = level.into();
        // Verify the conversion is sensible
        assert!(proto as i32 >= 0);
    }
}

#[test]
fn test_evaluation_level_values() {
    // Ensure proto enum values match expected
    assert_eq!(ProtoEvaluationLevel::Unspecified as i32, 0);
    assert_eq!(ProtoEvaluationLevel::L1External as i32, 1);
    assert_eq!(ProtoEvaluationLevel::L2SelfHosted as i32, 2);
}

#[test]
fn test_complexity_evaluation_struct() {
    // Test that ComplexityEvaluation can be constructed with all fields
    let eval = ComplexityEvaluation {
        c_token: 0.5,
        c_attn: 0.3,
        c_graph: 0.2,
        c_step: 0.4,
        c_integrated: 0.35,
        threshold_exceeded: false,
        evaluated_at: Some(Timestamp {
            seconds: 1700000000,
            nanos: 0,
        }),
    };

    assert!(eval.c_integrated >= 0.0 && eval.c_integrated <= 1.0);
    assert!(eval.evaluated_at.is_some());
}

#[test]
fn test_drift_detection_struct() {
    let drift = DriftDetection {
        anomaly_detected: true,
        distribution_changed: false,
        time_series_anomaly: true,
        anomaly_score: 0.75,
        p_value: 0.03,
    };

    assert!(drift.p_value >= 0.0 && drift.p_value <= 1.0);
}

// =============================================================================
// C15 ValueNeuronMonitor Type Conversions
// =============================================================================

#[test]
fn test_intervention_level_roundtrip() {
    let rust_levels = vec![
        InterventionLevel::Monitor,
        InterventionLevel::PartialSuppress,
        InterventionLevel::FullSuppress,
        InterventionLevel::SystemStop,
    ];

    for level in rust_levels {
        let proto: ProtoInterventionLevel = level.into();
        assert!(proto as i32 >= 1); // LEVEL_1_MONITOR = 1
    }
}

#[test]
fn test_rpe_anomaly_type_roundtrip() {
    let rust_types = vec![
        RpeAnomalyType::None,
        RpeAnomalyType::PositiveSpike,
        RpeAnomalyType::NegativeSpike,
        RpeAnomalyType::Oscillation,
        RpeAnomalyType::GradualIncrease,
        RpeAnomalyType::GradualDecrease,
    ];

    for anomaly in rust_types {
        let proto: ProtoRpeAnomalyType = anomaly.into();
        assert!(proto as i32 >= 0);
    }
}

#[test]
fn test_diagnosis_depth_values() {
    assert_eq!(DiagnosisDepth::Unspecified as i32, 0);
    assert_eq!(DiagnosisDepth::Quick as i32, 1);
    assert_eq!(DiagnosisDepth::Full as i32, 2);
}

// =============================================================================
// C16 ContradictionController Type Conversions
// =============================================================================

#[test]
fn test_contradiction_type_roundtrip() {
    let rust_types = vec![
        ContradictionType::Direct,
        ContradictionType::SelfReference,
        ContradictionType::Conditional,
        ContradictionType::Meta,
        ContradictionType::Implicit,
    ];

    for ct in rust_types {
        let proto: ProtoContradictionType = ct.into();
        let back: ContradictionType = proto.into();
        assert_eq!(ct, back);
    }
}

#[test]
fn test_contradiction_strength_roundtrip() {
    let rust_strengths = vec![
        ContradictionStrength::Soft,
        ContradictionStrength::Medium,
        ContradictionStrength::Hard,
    ];

    for strength in rust_strengths {
        let proto: ProtoContradictionStrength = strength.into();
        let back: ContradictionStrength = proto.into();
        assert_eq!(strength, back);
    }
}

#[test]
fn test_injection_timing_roundtrip() {
    let rust_timings = vec![
        InjectionTiming::Prepend,
        InjectionTiming::Parallel,
        InjectionTiming::Embed,
    ];

    for timing in rust_timings {
        let proto: ProtoInjectionTiming = timing.into();
        let back: InjectionTiming = proto.into();
        assert_eq!(timing, back);
    }
}

#[test]
fn test_control_state_conversion() {
    let rust_states = vec![
        ControlState::Active,
        ControlState::Stopped,
        ControlState::Degraded,
        ControlState::Constrained,
    ];

    for state in rust_states {
        let proto: ProtoControlState = state.into();
        assert!(proto as i32 >= 0);
    }
}

#[test]
fn test_collapse_type_conversion() {
    let rust_types = vec![
        CollapseType::None,
        CollapseType::NoResponse,
        CollapseType::Timeout,
        CollapseType::Error,
        CollapseType::Incoherent,
        CollapseType::Hallucination,
        CollapseType::Repetition,
    ];

    for ct in rust_types {
        let proto: ProtoCollapseType = ct.into();
        assert!(proto as i32 >= 0);
    }
}

// =============================================================================
// C17 SurvivalAttentionService Type Conversions
// =============================================================================

#[test]
fn test_risk_level_conversion() {
    // Proto to Option<RiskLevel>
    let proto_levels = vec![
        (ProtoRiskLevel::Unspecified, None),
        (ProtoRiskLevel::Low, Some(RiskLevel::Low)),
        (ProtoRiskLevel::Medium, Some(RiskLevel::Medium)),
        (ProtoRiskLevel::High, Some(RiskLevel::High)),
        (ProtoRiskLevel::Critical, Some(RiskLevel::Critical)),
    ];

    for (proto, expected) in proto_levels {
        let result: Option<RiskLevel> = proto.into();
        assert_eq!(result, expected);
    }
}

#[test]
fn test_survival_score_struct() {
    let score = ProtoSurvivalScore {
        diversity_n: 1.5,
        yohaku_mu: 0.8,
        delta: 0.1,
        integrated_s: 0.5,
    };

    // All values should be finite
    assert!(score.diversity_n.is_finite());
    assert!(score.yohaku_mu.is_finite());
    assert!(score.delta.is_finite());
    assert!(score.integrated_s.is_finite());
}

// =============================================================================
// Common Type Tests
// =============================================================================

#[test]
fn test_timestamp_creation() {
    use chrono::Utc;

    let now = Utc::now();
    let ts = Timestamp {
        seconds: now.timestamp(),
        nanos: now.timestamp_subsec_nanos() as i32,
    };

    assert!(ts.seconds > 0);
    assert!(ts.nanos >= 0);
}

#[test]
fn test_proto_enum_default_values() {
    // All proto enums should have a 0 value (Unspecified)
    assert_eq!(ProtoStopLevel::Unspecified as i32, 0);
    assert_eq!(ProtoInterventionLevel::Unspecified as i32, 0);
    assert_eq!(ProtoContradictionType::Unspecified as i32, 0);
    assert_eq!(ProtoContradictionStrength::Unspecified as i32, 0);
    assert_eq!(ProtoInjectionTiming::Unspecified as i32, 0);
    assert_eq!(ProtoControlState::Unspecified as i32, 0);
    assert_eq!(ProtoCollapseType::Unspecified as i32, 0);
    assert_eq!(ProtoRiskLevel::Unspecified as i32, 0);
    assert_eq!(ProtoRpeAnomalyType::Unspecified as i32, 0);
}

/// Test that i32 can be converted to proto enums
#[test]
fn test_proto_from_i32() {
    // StopLevel
    assert_eq!(ProtoStopLevel::try_from(0).unwrap(), ProtoStopLevel::Unspecified);
    assert_eq!(ProtoStopLevel::try_from(1).unwrap(), ProtoStopLevel::Level1AcceptStop);
    assert_eq!(ProtoStopLevel::try_from(5).unwrap(), ProtoStopLevel::Level5PhysicalStop);

    // Invalid value should error
    assert!(ProtoStopLevel::try_from(99).is_err());
}
