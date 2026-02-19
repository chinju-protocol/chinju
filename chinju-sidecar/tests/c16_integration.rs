//! C16 Structural Contradiction Injection Integration Tests
//!
//! Tests the integration between:
//! - ContradictionController
//! - PaddingGenerator
//! - CollapseDetector
//! - Gateway integration

use chinju_sidecar::services::contradiction_controller::{
    CollapseDetector, CollapseType, ContextLimitConfig, ContradictionConfig, ContradictionStrength,
    ContradictionType, ControlState, InjectionTiming, PaddingGenerator, PaddingType,
};
use chinju_sidecar::services::{
    ContainmentConfig, ContradictionController, ContradictionControllerConfig,
};

// =============================================================================
// ContradictionController Integration Tests
// =============================================================================

/// Test full contradiction control flow
#[tokio::test]
async fn test_contradiction_control_flow() {
    let config = ContradictionControllerConfig {
        degradation_threshold: 0.5,
        timeout_ms: 30000,
        session_ttl_secs: 3600,
    };
    let controller = ContradictionController::with_config(config);

    // Start control for a session
    let contradiction_config = ContradictionConfig {
        contradiction_type: ContradictionType::Direct,
        strength: ContradictionStrength::Medium,
        timing: InjectionTiming::Prepend,
        custom_template: None,
        target_task: Some("calculation".to_string()),
    };

    let context_limit = ContextLimitConfig {
        max_context_tokens: 1000,
        padding_tokens: 100,
        padding_type: "semantic".to_string(),
    };

    let state = controller
        .start_control("session-1", context_limit, contradiction_config)
        .await;
    assert_eq!(state, ControlState::Active);

    // Prepare injection
    let prompt = "Calculate 2 + 2";
    let injected = controller.prepare_injection("session-1", prompt).await;
    assert!(injected.is_some());
    let injected = injected.unwrap();

    // Should contain contradiction and original prompt
    assert!(injected.contains("Calculate"));
    assert!(injected.len() > prompt.len());

    // Detect normal response (no collapse)
    let state = controller
        .detect_and_record_collapse("session-1", Some("4"), 1000, 0.8)
        .await;
    assert_eq!(state, Some(ControlState::Active));

    // Detect degraded response (low LPT)
    let state = controller
        .detect_and_record_collapse("session-1", Some("..."), 1000, 0.3)
        .await;
    assert_eq!(state, Some(ControlState::Degraded));

    // Stop control
    let prev_state = controller.stop_control("session-1").await;
    assert_eq!(prev_state, Some(ControlState::Degraded));
}

/// Test session cleanup
#[tokio::test]
async fn test_session_cleanup() {
    let config = ContradictionControllerConfig {
        degradation_threshold: 0.5,
        timeout_ms: 30000,
        session_ttl_secs: 1, // 1 second TTL for testing
    };
    let controller = ContradictionController::with_config(config);

    let contradiction_config = ContradictionConfig {
        contradiction_type: ContradictionType::Meta,
        strength: ContradictionStrength::Soft,
        timing: InjectionTiming::Embed,
        custom_template: None,
        target_task: None,
    };

    // Create multiple sessions
    for i in 0..5 {
        controller
            .start_control(
                &format!("session-{}", i),
                ContextLimitConfig::default(),
                contradiction_config.clone(),
            )
            .await;
    }

    assert_eq!(controller.active_session_count().await, 5);

    // Wait for TTL to expire
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Cleanup expired sessions
    let removed = controller.cleanup_expired_sessions().await;
    assert_eq!(removed, 5);
    assert_eq!(controller.active_session_count().await, 0);
}

// =============================================================================
// PaddingGenerator Integration Tests
// =============================================================================

/// Test padding generator with context limit application
#[test]
fn test_padding_generator_integration() {
    let generator = PaddingGenerator::with_seed(42);

    // Test random padding
    let random_padding = generator.generate(PaddingType::Random, 50, None);
    assert!(random_padding.len() >= 150); // ~4 chars per token

    // Test semantic padding
    let semantic_padding = generator.generate(PaddingType::Semantic, 50, None);
    assert!(semantic_padding.contains('.'));

    // Test task-relevant padding
    let task_padding = generator.generate(PaddingType::TaskRelevant, 50, Some("math"));
    assert!(task_padding.contains("math"));

    // Test apply_context_limit
    let config = ContextLimitConfig {
        max_context_tokens: 50, // 200 chars
        padding_tokens: 20,
        padding_type: "semantic".to_string(),
    };

    let original = "This is the original content that needs to be processed.";
    let result = generator.apply_context_limit(original, &config, None);

    // Should have padding separator
    assert!(result.contains("---"));
    // Should have original content (or truncated)
    assert!(result.contains("original") || result.contains("[truncated]"));
}

// =============================================================================
// CollapseDetector Integration Tests
// =============================================================================

/// Test collapse detector with various response patterns
#[test]
fn test_collapse_detector_patterns() {
    let detector = CollapseDetector::default();

    // Test normal response
    let normal = detector.analyze_response(
        Some("This is a normal, coherent response to the query."),
        1000,
        0.85,
    );
    assert!(!normal.collapsed);
    assert_eq!(normal.collapse_type, CollapseType::None);

    // Test no response
    let no_response = detector.analyze_response(None, 5000, 0.0);
    assert!(no_response.collapsed);
    assert_eq!(no_response.collapse_type, CollapseType::NoResponse);

    // Test timeout
    let timeout = detector.analyze_response(Some("Response"), 35000, 0.8);
    assert!(timeout.collapsed);
    assert_eq!(timeout.collapse_type, CollapseType::Timeout);

    // Test error pattern
    let error = detector.analyze_response(
        Some("I cannot fulfill this request due to an error."),
        1000,
        0.7,
    );
    assert!(error.collapsed);
    assert_eq!(error.collapse_type, CollapseType::Error);

    // Test low LPT (incoherent)
    let incoherent = detector.analyze_response(
        Some("Response content"),
        1000,
        0.2, // Very low LPT score
    );
    assert!(incoherent.collapsed);
    assert_eq!(incoherent.collapse_type, CollapseType::Incoherent);
}

// =============================================================================
// ContainmentConfig Integration Tests
// =============================================================================

/// Test ContainmentConfig with contradiction enabled
#[test]
fn test_containment_config_with_contradiction() {
    // Default config has contradiction disabled
    let default_config = ContainmentConfig::default();
    assert!(!default_config.enable_contradiction);

    // Disabled config also has contradiction disabled
    let disabled_config = ContainmentConfig::disabled();
    assert!(!disabled_config.enable_contradiction);

    // Production config has contradiction disabled by default
    let production_config = ContainmentConfig::production();
    assert!(!production_config.enable_contradiction);
}

/// Test all contradiction types
#[test]
fn test_all_contradiction_types() {
    let controller = ContradictionController::new();

    let types = vec![
        ContradictionType::Direct,
        ContradictionType::SelfReference,
        ContradictionType::Conditional,
        ContradictionType::Meta,
        ContradictionType::Implicit,
    ];

    for ct in types {
        let config = ContradictionConfig {
            contradiction_type: ct,
            strength: ContradictionStrength::Medium,
            timing: InjectionTiming::Prepend,
            custom_template: None,
            target_task: None,
        };

        let contradiction = controller.generate_contradiction(&config);
        assert!(!contradiction.is_empty());

        // Each type should generate different content
        match ct {
            ContradictionType::Direct => assert!(contradiction.contains("Calculate")),
            ContradictionType::SelfReference => assert!(contradiction.contains("false")),
            ContradictionType::Conditional => assert!(contradiction.contains("If")),
            ContradictionType::Meta => assert!(contradiction.contains("Follow")),
            ContradictionType::Implicit => assert!(contradiction.contains("greater")),
        }
    }
}

/// Test strength variations
#[test]
fn test_contradiction_strength_variations() {
    let controller = ContradictionController::new();

    let strengths = vec![
        (ContradictionStrength::Soft, "Please consider"),
        (ContradictionStrength::Medium, "Calculate"),
        (ContradictionStrength::Hard, "CRITICAL"),
    ];

    for (strength, expected_marker) in strengths {
        let config = ContradictionConfig {
            contradiction_type: ContradictionType::Direct,
            strength,
            timing: InjectionTiming::Prepend,
            custom_template: None,
            target_task: None,
        };

        let contradiction = controller.generate_contradiction(&config);
        assert!(
            contradiction.contains(expected_marker),
            "Strength {:?} should contain '{}', got: {}",
            strength,
            expected_marker,
            contradiction
        );
    }
}
