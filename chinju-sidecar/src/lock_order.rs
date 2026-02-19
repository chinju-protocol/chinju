//! Lock Ordering Constants (10.2.3)
//!
//! This module defines the lock acquisition order to prevent deadlocks
//! across the CHINJU Protocol services.
//!
//! # Lock Order Rules
//!
//! Locks must be acquired in ascending order of their LOCK_ORDER values.
//! Never hold a lock while acquiring a lock with a lower order value.
//!
//! # Example
//!
//! ```ignore
//! // CORRECT: Acquire in ascending order
//! let token_lock = token_service.write().await;     // Order 10
//! let credential_lock = credential_service.read().await;  // Order 20
//!
//! // INCORRECT: Would violate order
//! // let credential_lock = credential_service.read().await;  // Order 20
//! // let token_lock = token_service.write().await;     // Order 10 - DEADLOCK RISK
//! ```
//!
//! # Anti-pattern: Holding locks during external calls
//!
//! Never hold a lock while making external async calls (network I/O, etc.).
//! Instead, copy the data you need, release the lock, then make the call.
//!
//! ```ignore
//! // CORRECT
//! let data = {
//!     let guard = service.read().await;
//!     guard.data.clone()
//! }; // Lock released here
//! external_call(&data).await;  // Safe - no lock held
//!
//! // INCORRECT - Potential deadlock
//! let guard = service.read().await;
//! external_call(&guard.data).await;  // Dangerous!
//! ```

/// Lock order for TokenService operations
///
/// TokenService has the lowest lock order because token operations
/// are fundamental and may be called from many places.
pub const LOCK_ORDER_TOKEN_SERVICE: u32 = 10;

/// Lock order for CredentialService operations
///
/// Credential operations may consume tokens, so credential locks
/// must be acquired after token locks.
pub const LOCK_ORDER_CREDENTIAL_SERVICE: u32 = 20;

/// Lock order for CapabilityTestManager operations
///
/// Capability tests may update credential state.
pub const LOCK_ORDER_CAPABILITY_TEST: u32 = 25;

/// Lock order for PolicyEngine operations
///
/// Policy evaluation may check credentials and token balance.
pub const LOCK_ORDER_POLICY_ENGINE: u32 = 30;

/// Lock order for GatewayService operations
///
/// Gateway orchestrates all other services, so it has the highest order.
pub const LOCK_ORDER_GATEWAY_SERVICE: u32 = 40;

/// Lock order for AuditLogger operations
///
/// Audit logging is a side effect and should not block other operations.
/// It has a high order to avoid contention.
pub const LOCK_ORDER_AUDIT_LOGGER: u32 = 50;

/// Lock order for ExtractionDeterrent operations
pub const LOCK_ORDER_EXTRACTION_DETERRENT: u32 = 35;

/// Lock order for LPT Monitor operations
pub const LOCK_ORDER_LPT_MONITOR: u32 = 45;

/// Lock order for ThresholdVerifier operations
///
/// Threshold verification is used for authorization checks.
pub const LOCK_ORDER_THRESHOLD_VERIFIER: u32 = 15;

// =============================================================================
// Lock Order Validation (Debug Mode)
// =============================================================================

#[cfg(debug_assertions)]
use std::cell::RefCell;

#[cfg(debug_assertions)]
thread_local! {
    /// Stack of currently held lock orders in this thread
    static LOCK_STACK: RefCell<Vec<u32>> = RefCell::new(Vec::new());
}

/// Push a lock order onto the thread-local stack (debug mode only)
///
/// Panics if the new lock order is less than any currently held lock.
#[cfg(debug_assertions)]
pub fn push_lock_order(order: u32, name: &str) {
    LOCK_STACK.with(|stack| {
        let mut stack = stack.borrow_mut();
        if let Some(&last) = stack.last() {
            if order < last {
                panic!(
                    "Lock order violation: attempting to acquire {} (order {}) while holding a lock with order {}. \
                     Locks must be acquired in ascending order.",
                    name, order, last
                );
            }
        }
        stack.push(order);
    });
}

/// Pop a lock order from the thread-local stack (debug mode only)
#[cfg(debug_assertions)]
pub fn pop_lock_order(order: u32) {
    LOCK_STACK.with(|stack| {
        let mut stack = stack.borrow_mut();
        if let Some(last) = stack.pop() {
            if last != order {
                tracing::warn!(
                    expected = order,
                    actual = last,
                    "Lock order mismatch on release"
                );
            }
        }
    });
}

/// No-op in release mode
#[cfg(not(debug_assertions))]
pub fn push_lock_order(_order: u32, _name: &str) {}

/// No-op in release mode
#[cfg(not(debug_assertions))]
pub fn pop_lock_order(_order: u32) {}

/// RAII guard for lock order tracking.
///
/// Construct this right before acquiring a lock. The order is popped
/// automatically when the guard goes out of scope.
pub struct LockOrderGuard {
    order: u32,
}

impl Drop for LockOrderGuard {
    fn drop(&mut self) {
        pop_lock_order(self.order);
    }
}

/// Enter a lock-order scope with automatic pop on drop.
pub fn enter_lock_scope(order: u32, name: &str) -> LockOrderGuard {
    push_lock_order(order, name);
    LockOrderGuard { order }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_order_values() {
        // Verify the lock order hierarchy
        assert!(LOCK_ORDER_TOKEN_SERVICE < LOCK_ORDER_CREDENTIAL_SERVICE);
        assert!(LOCK_ORDER_CREDENTIAL_SERVICE < LOCK_ORDER_CAPABILITY_TEST);
        assert!(LOCK_ORDER_CAPABILITY_TEST < LOCK_ORDER_POLICY_ENGINE);
        assert!(LOCK_ORDER_POLICY_ENGINE < LOCK_ORDER_GATEWAY_SERVICE);
        assert!(LOCK_ORDER_GATEWAY_SERVICE < LOCK_ORDER_AUDIT_LOGGER);
    }

    #[test]
    fn test_lock_order_push_pop() {
        // Test valid lock order
        push_lock_order(LOCK_ORDER_TOKEN_SERVICE, "token");
        push_lock_order(LOCK_ORDER_CREDENTIAL_SERVICE, "credential");
        pop_lock_order(LOCK_ORDER_CREDENTIAL_SERVICE);
        pop_lock_order(LOCK_ORDER_TOKEN_SERVICE);
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "Lock order violation")]
    fn test_lock_order_violation() {
        push_lock_order(LOCK_ORDER_CREDENTIAL_SERVICE, "credential");
        push_lock_order(LOCK_ORDER_TOKEN_SERVICE, "token"); // Should panic
    }

    #[test]
    fn test_lock_order_guard_auto_pop() {
        let _g1 = enter_lock_scope(LOCK_ORDER_TOKEN_SERVICE, "token");
        {
            let _g2 = enter_lock_scope(LOCK_ORDER_CREDENTIAL_SERVICE, "credential");
        } // auto pop
    }
}
