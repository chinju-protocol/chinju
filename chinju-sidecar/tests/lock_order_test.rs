//! Lock Order and Deadlock Detection Tests
//!
//! These tests verify that lock acquisition follows the defined order
//! to prevent deadlocks in concurrent operations.
//!
//! # CI Integration
//!
//! Run with: `cargo test --test lock_order_test`
//! These tests are designed to detect potential deadlock scenarios
//! at compile/test time rather than at runtime.

use chinju_sidecar::lock_order::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::timeout;

// =============================================================================
// Lock Order Verification Tests
// =============================================================================

/// Verify the complete lock order hierarchy
#[test]
fn test_complete_lock_order_hierarchy() {
    // All locks in order from lowest to highest
    let lock_orders = [
        ("TokenService", LOCK_ORDER_TOKEN_SERVICE),
        ("ThresholdVerifier", LOCK_ORDER_THRESHOLD_VERIFIER),
        ("CredentialService", LOCK_ORDER_CREDENTIAL_SERVICE),
        ("CapabilityTest", LOCK_ORDER_CAPABILITY_TEST),
        ("PolicyEngine", LOCK_ORDER_POLICY_ENGINE),
        ("ExtractionDeterrent", LOCK_ORDER_EXTRACTION_DETERRENT),
        ("GatewayService", LOCK_ORDER_GATEWAY_SERVICE),
        ("LptMonitor", LOCK_ORDER_LPT_MONITOR),
        ("AuditLogger", LOCK_ORDER_AUDIT_LOGGER),
    ];

    // Verify strict ordering
    for i in 0..lock_orders.len() - 1 {
        let (name_a, order_a) = lock_orders[i];
        let (name_b, order_b) = lock_orders[i + 1];
        assert!(
            order_a < order_b,
            "Lock order violation: {} ({}) should be less than {} ({})",
            name_a,
            order_a,
            name_b,
            order_b
        );
    }
}

/// Test valid lock order push/pop sequence
#[test]
fn test_valid_lock_sequence() {
    // Acquire locks in ascending order
    push_lock_order(LOCK_ORDER_TOKEN_SERVICE, "TokenService");
    push_lock_order(LOCK_ORDER_THRESHOLD_VERIFIER, "ThresholdVerifier");
    push_lock_order(LOCK_ORDER_CREDENTIAL_SERVICE, "CredentialService");
    push_lock_order(LOCK_ORDER_POLICY_ENGINE, "PolicyEngine");
    push_lock_order(LOCK_ORDER_GATEWAY_SERVICE, "GatewayService");
    push_lock_order(LOCK_ORDER_AUDIT_LOGGER, "AuditLogger");

    // Release in reverse order
    pop_lock_order(LOCK_ORDER_AUDIT_LOGGER);
    pop_lock_order(LOCK_ORDER_GATEWAY_SERVICE);
    pop_lock_order(LOCK_ORDER_POLICY_ENGINE);
    pop_lock_order(LOCK_ORDER_CREDENTIAL_SERVICE);
    pop_lock_order(LOCK_ORDER_THRESHOLD_VERIFIER);
    pop_lock_order(LOCK_ORDER_TOKEN_SERVICE);
}

/// Test acquiring same-order locks (allowed)
#[test]
fn test_same_order_locks() {
    push_lock_order(LOCK_ORDER_TOKEN_SERVICE, "TokenService1");
    push_lock_order(LOCK_ORDER_TOKEN_SERVICE, "TokenService2");
    pop_lock_order(LOCK_ORDER_TOKEN_SERVICE);
    pop_lock_order(LOCK_ORDER_TOKEN_SERVICE);
}

/// Test lock order violation detection (debug mode only)
#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "Lock order violation")]
fn test_lock_order_violation_detected() {
    push_lock_order(LOCK_ORDER_GATEWAY_SERVICE, "GatewayService");
    push_lock_order(LOCK_ORDER_TOKEN_SERVICE, "TokenService"); // Should panic
}

/// Test partial acquisition sequence
#[test]
fn test_partial_acquisition() {
    // Acquire only some locks
    push_lock_order(LOCK_ORDER_TOKEN_SERVICE, "TokenService");
    push_lock_order(LOCK_ORDER_POLICY_ENGINE, "PolicyEngine"); // Skip intermediate
    pop_lock_order(LOCK_ORDER_POLICY_ENGINE);
    pop_lock_order(LOCK_ORDER_TOKEN_SERVICE);
}

// =============================================================================
// Concurrent Access Tests
// =============================================================================

/// Simulate concurrent service access without deadlock
#[tokio::test]
async fn test_concurrent_access_no_deadlock() {
    // Simulate services with RwLock
    let token_service = Arc::new(RwLock::new(0u32));
    let credential_service = Arc::new(RwLock::new(0u32));
    let policy_engine = Arc::new(RwLock::new(0u32));

    let mut handles = Vec::new();

    // Spawn multiple tasks that acquire locks in correct order
    for i in 0..10 {
        let ts = Arc::clone(&token_service);
        let cs = Arc::clone(&credential_service);
        let pe = Arc::clone(&policy_engine);

        handles.push(tokio::spawn(async move {
            // Always acquire in order: Token -> Credential -> Policy
            let _t = ts.read().await;
            let _c = cs.read().await;
            let _p = pe.write().await;
            tokio::time::sleep(Duration::from_millis(1)).await;
            i
        }));
    }

    // All tasks should complete without deadlock
    let result = timeout(Duration::from_secs(5), async {
        for handle in handles {
            handle.await.unwrap();
        }
    })
    .await;

    assert!(
        result.is_ok(),
        "Concurrent access test timed out (potential deadlock)"
    );
}

/// Test that we can detect incorrect lock order at runtime
#[tokio::test]
#[cfg(debug_assertions)]
async fn test_concurrent_access_with_violation_detection() {
    // This should panic in debug mode
    let result = std::panic::catch_unwind(|| {
        push_lock_order(LOCK_ORDER_AUDIT_LOGGER, "AuditLogger");
        push_lock_order(LOCK_ORDER_TOKEN_SERVICE, "TokenService"); // Wrong order
    });

    assert!(
        result.is_err(),
        "Lock order violation should cause panic in debug mode"
    );
}

// =============================================================================
// Service Integration Lock Order Tests
// =============================================================================

/// Test gateway -> policy -> credential -> token ordering
#[test]
fn test_gateway_to_token_order() {
    // Simulate the typical request flow
    push_lock_order(LOCK_ORDER_TOKEN_SERVICE, "TokenService");
    push_lock_order(LOCK_ORDER_CREDENTIAL_SERVICE, "CredentialService");
    push_lock_order(LOCK_ORDER_POLICY_ENGINE, "PolicyEngine");
    push_lock_order(LOCK_ORDER_GATEWAY_SERVICE, "GatewayService");

    pop_lock_order(LOCK_ORDER_GATEWAY_SERVICE);
    pop_lock_order(LOCK_ORDER_POLICY_ENGINE);
    pop_lock_order(LOCK_ORDER_CREDENTIAL_SERVICE);
    pop_lock_order(LOCK_ORDER_TOKEN_SERVICE);
}

/// Test audit logging with other services
#[test]
fn test_audit_logging_order() {
    // Audit should always be acquired last
    push_lock_order(LOCK_ORDER_TOKEN_SERVICE, "TokenService");
    push_lock_order(LOCK_ORDER_GATEWAY_SERVICE, "GatewayService");
    push_lock_order(LOCK_ORDER_AUDIT_LOGGER, "AuditLogger");

    pop_lock_order(LOCK_ORDER_AUDIT_LOGGER);
    pop_lock_order(LOCK_ORDER_GATEWAY_SERVICE);
    pop_lock_order(LOCK_ORDER_TOKEN_SERVICE);
}

/// Test C14-C17 services lock ordering
#[test]
fn test_c14_c17_services_order() {
    // These services should follow the general order
    push_lock_order(LOCK_ORDER_TOKEN_SERVICE, "TokenService");
    push_lock_order(LOCK_ORDER_THRESHOLD_VERIFIER, "ThresholdVerifier");
    push_lock_order(LOCK_ORDER_CAPABILITY_TEST, "CapabilityTest");
    push_lock_order(LOCK_ORDER_EXTRACTION_DETERRENT, "ExtractionDeterrent");
    push_lock_order(LOCK_ORDER_LPT_MONITOR, "LptMonitor");

    pop_lock_order(LOCK_ORDER_LPT_MONITOR);
    pop_lock_order(LOCK_ORDER_EXTRACTION_DETERRENT);
    pop_lock_order(LOCK_ORDER_CAPABILITY_TEST);
    pop_lock_order(LOCK_ORDER_THRESHOLD_VERIFIER);
    pop_lock_order(LOCK_ORDER_TOKEN_SERVICE);
}

// =============================================================================
// Timeout-based Deadlock Detection
// =============================================================================

/// Test that actual locks can be acquired without timeout
#[tokio::test]
async fn test_lock_acquisition_timeout() {
    let service_a = Arc::new(RwLock::new(42));
    let service_b = Arc::new(RwLock::new(84));

    // This should complete quickly
    let result = timeout(Duration::from_millis(100), async {
        let a = service_a.read().await;
        let b = service_b.read().await;
        *a + *b
    })
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 126);
}

/// Test multiple readers don't block
#[tokio::test]
async fn test_multiple_readers_no_block() {
    let shared_data = Arc::new(RwLock::new(100));
    let mut handles = Vec::new();

    // Spawn 20 readers
    for _ in 0..20 {
        let data = Arc::clone(&shared_data);
        handles.push(tokio::spawn(async move {
            let guard = data.read().await;
            tokio::time::sleep(Duration::from_millis(10)).await;
            *guard
        }));
    }

    // All readers should complete within timeout
    let result = timeout(Duration::from_secs(2), async {
        let mut sum = 0;
        for handle in handles {
            sum += handle.await.unwrap();
        }
        sum
    })
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 2000); // 20 * 100
}

/// Test reader-writer interaction
#[tokio::test]
async fn test_reader_writer_interaction() {
    let shared_data = Arc::new(RwLock::new(0i32));

    // Writer task
    let data_writer = Arc::clone(&shared_data);
    let writer = tokio::spawn(async move {
        for i in 1..=5 {
            {
                let mut guard = data_writer.write().await;
                *guard = i;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    });

    // Reader task
    let data_reader = Arc::clone(&shared_data);
    let reader = tokio::spawn(async move {
        let mut last_value = 0;
        for _ in 0..10 {
            let guard = data_reader.read().await;
            if *guard > last_value {
                last_value = *guard;
            }
            drop(guard);
            tokio::time::sleep(Duration::from_millis(3)).await;
        }
        last_value
    });

    // Both should complete
    let result = timeout(Duration::from_secs(2), async {
        writer.await.unwrap();
        reader.await.unwrap()
    })
    .await;

    assert!(result.is_ok());
    assert!(result.unwrap() >= 1); // Reader should have seen at least value 1
}

// =============================================================================
// Edge Cases
// =============================================================================

/// Test empty lock stack operations
#[test]
fn test_empty_stack_pop() {
    // Pop on empty stack should be a no-op (not panic)
    pop_lock_order(LOCK_ORDER_TOKEN_SERVICE);
}

/// Test rapid push/pop cycles
#[test]
fn test_rapid_push_pop_cycles() {
    for _ in 0..1000 {
        push_lock_order(LOCK_ORDER_TOKEN_SERVICE, "TokenService");
        push_lock_order(LOCK_ORDER_CREDENTIAL_SERVICE, "CredentialService");
        pop_lock_order(LOCK_ORDER_CREDENTIAL_SERVICE);
        pop_lock_order(LOCK_ORDER_TOKEN_SERVICE);
    }
}

/// Test all unique lock orders
#[test]
fn test_all_unique_lock_orders() {
    let orders = [
        LOCK_ORDER_TOKEN_SERVICE,
        LOCK_ORDER_THRESHOLD_VERIFIER,
        LOCK_ORDER_CREDENTIAL_SERVICE,
        LOCK_ORDER_CAPABILITY_TEST,
        LOCK_ORDER_POLICY_ENGINE,
        LOCK_ORDER_EXTRACTION_DETERRENT,
        LOCK_ORDER_GATEWAY_SERVICE,
        LOCK_ORDER_LPT_MONITOR,
        LOCK_ORDER_AUDIT_LOGGER,
    ];

    // Verify all orders are unique
    let mut sorted = orders.to_vec();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        orders.len(),
        "Duplicate lock order values detected"
    );
}
