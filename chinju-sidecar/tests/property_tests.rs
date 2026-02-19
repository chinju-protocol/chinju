//! Property-Based Tests for CHINJU Sidecar
//!
//! Uses proptest for fuzz-like testing of critical components.
//!
//! Run with: `cargo test --test property_tests`

use proptest::prelude::*;
use std::collections::HashSet;

// =============================================================================
// OutputSanitizer Property Tests
// =============================================================================

mod output_sanitizer {
    use super::*;
    use chinju_sidecar::services::{OutputSanitizer, SanitizationMode, SanitizerConfig};

    // Strategy for generating arbitrary text with potential steganographic content
    fn arbitrary_text() -> impl Strategy<Value = String> {
        prop::string::string_regex("[a-zA-Z0-9 \n\t.,!?'\"-]{0,1000}")
            .unwrap()
            .prop_map(|s| s.trim().to_string())
    }

    // Strategy for text with unicode characters
    fn unicode_text() -> impl Strategy<Value = String> {
        prop::collection::vec(prop::char::any(), 0..500)
            .prop_map(|chars| chars.into_iter().collect())
    }

    // Strategy for text with code blocks
    fn text_with_code() -> impl Strategy<Value = String> {
        (arbitrary_text(), arbitrary_text())
            .prop_map(|(text, code)| format!("{}\n```rust\n{}\n```\n", text, code))
    }

    proptest! {
        /// Property: Sanitization should never panic on arbitrary input
        #[test]
        fn sanitizer_never_panics(input in arbitrary_text()) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let sanitizer = OutputSanitizer::new();
                let _ = sanitizer.sanitize(&input, None).await;
            });
        }

        /// Property: Sanitization should never panic on unicode input
        #[test]
        fn sanitizer_handles_unicode(input in unicode_text()) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let sanitizer = OutputSanitizer::new();
                let _ = sanitizer.sanitize(&input, None).await;
            });
        }

        /// Property: Sanitization should preserve text with code blocks
        #[test]
        fn sanitizer_preserves_code_structure(input in text_with_code()) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let sanitizer = OutputSanitizer::new();
                let result = sanitizer.sanitize(&input, None).await;
                // Code block markers should be preserved
                if input.contains("```") {
                    prop_assert!(result.contains("```") || result.is_empty());
                }
                Ok(())
            }).unwrap();
        }

        /// Property: Sanitization is idempotent for light mode
        #[test]
        fn light_sanitization_idempotent(input in arbitrary_text()) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let config = SanitizerConfig {
                    default_mode: SanitizationMode::Light,
                    enable_paraphrasing: false,
                    ..Default::default()
                };
                let sanitizer = OutputSanitizer::with_config(config);
                let first = sanitizer.sanitize(&input, Some(SanitizationMode::Light)).await;
                let second = sanitizer.sanitize(&first, Some(SanitizationMode::Light)).await;
                prop_assert_eq!(first, second);
                Ok(())
            }).unwrap();
        }

        /// Property: Output length should not exceed input length by much
        #[test]
        fn output_length_bounded(input in arbitrary_text()) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let sanitizer = OutputSanitizer::new();
                let result = sanitizer.sanitize(&input, None).await;
                // Allow some growth due to normalization, but not extreme
                prop_assert!(result.len() <= input.len() * 2 + 100);
                Ok(())
            }).unwrap();
        }

        /// Property: Empty input produces empty or whitespace output
        #[test]
        fn empty_input_handling(_dummy in 0..1u8) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let sanitizer = OutputSanitizer::new();
                let result = sanitizer.sanitize("", None).await;
                prop_assert!(result.is_empty() || result.chars().all(|c| c.is_whitespace()));
                Ok(())
            }).unwrap();
        }

        /// Property: Whitespace-only input produces whitespace-only or empty output
        #[test]
        fn whitespace_handling(spaces in 0..100usize) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let input = " ".repeat(spaces);
            rt.block_on(async {
                let sanitizer = OutputSanitizer::new();
                let result = sanitizer.sanitize(&input, None).await;
                prop_assert!(result.chars().all(|c| c.is_whitespace()) || result.is_empty());
                Ok(())
            }).unwrap();
        }

        /// Property: Code block detection should not panic
        #[test]
        fn code_block_detection_safe(input in arbitrary_text()) {
            let sanitizer = OutputSanitizer::new();
            let _ = sanitizer.contains_code_blocks(&input);
            let _ = sanitizer.extract_code_blocks(&input);
        }
    }
}

// =============================================================================
// ExtractionDeterrent Property Tests
// =============================================================================

mod extraction_deterrent {
    use super::*;
    use chinju_sidecar::services::compute_query_hash;

    proptest! {
        /// Property: Query hashing should be deterministic
        #[test]
        fn query_hash_deterministic(query in "[a-zA-Z0-9 ]{0,500}") {
            let hash1 = compute_query_hash(&query);
            let hash2 = compute_query_hash(&query);
            prop_assert_eq!(hash1, hash2);
        }

        /// Property: Different queries should produce different hashes (high probability)
        #[test]
        fn query_hash_collision_resistant(
            queries in prop::collection::hash_set("[a-zA-Z0-9 ]{1,100}", 10..50)
        ) {
            let hashes: HashSet<_> = queries.iter()
                .map(|q| compute_query_hash(q))
                .collect();
            // Allow some collisions but not many
            prop_assert!(hashes.len() >= queries.len() * 9 / 10);
        }

        /// Property: Hash should be non-zero for non-empty queries
        #[test]
        fn query_hash_non_zero(query in "[a-zA-Z0-9]{1,100}") {
            let hash = compute_query_hash(&query);
            // Just verify the hash is computed (u64)
            prop_assert!(hash > 0 || query.is_empty());
        }
    }
}

// =============================================================================
// Lock Order Property Tests
// =============================================================================

mod lock_order {
    use super::*;
    use chinju_sidecar::lock_order::*;

    proptest! {
        /// Property: Lock order push/pop should be symmetric
        #[test]
        fn lock_order_symmetric(order in 10u32..100u32) {
            push_lock_order(order, "test_lock");
            pop_lock_order(order);
        }

        /// Property: Multiple sequential push/pop cycles should work
        #[test]
        fn lock_order_sequential_cycles(cycles in 1usize..100usize) {
            for _ in 0..cycles {
                push_lock_order(LOCK_ORDER_TOKEN_SERVICE, "token");
                pop_lock_order(LOCK_ORDER_TOKEN_SERVICE);
            }
        }

        /// Property: Ascending order should always be valid
        #[test]
        fn lock_order_ascending_valid(n in 2usize..5usize) {
            let orders = [
                LOCK_ORDER_TOKEN_SERVICE,
                LOCK_ORDER_THRESHOLD_VERIFIER,
                LOCK_ORDER_CREDENTIAL_SERVICE,
                LOCK_ORDER_POLICY_ENGINE,
                LOCK_ORDER_GATEWAY_SERVICE,
            ];

            for i in 0..n.min(orders.len()) {
                push_lock_order(orders[i], "test");
            }
            for i in (0..n.min(orders.len())).rev() {
                pop_lock_order(orders[i]);
            }
        }
    }
}

// =============================================================================
// ZKP Verifier Property Tests
// =============================================================================

mod zkp_verifier {
    use super::*;
    use chinju_sidecar::services::ZkpVerifier;

    proptest! {
        /// Property: ZKP verifier creation should not panic
        #[test]
        fn zkp_verifier_creation_safe(_dummy in 0..10u8) {
            let _verifier = ZkpVerifier::new();
        }

        /// Property: is_ready should be consistent
        #[test]
        fn zkp_is_ready_consistent(_dummy in 0..10u8) {
            let verifier = ZkpVerifier::new();
            let ready1 = verifier.is_ready();
            let ready2 = verifier.is_ready();
            prop_assert_eq!(ready1, ready2);
        }
    }
}

// =============================================================================
// Concurrent Access Property Tests
// =============================================================================

mod concurrent_access {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::RwLock;

    proptest! {
        /// Property: RwLock operations should be safe under concurrent access
        #[test]
        fn rwlock_concurrent_safe(num_readers in 2usize..10usize, num_writers in 1usize..3usize) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let data = Arc::new(RwLock::new(0u64));

                let mut handles = Vec::new();

                // Spawn readers
                for _ in 0..num_readers {
                    let d = Arc::clone(&data);
                    handles.push(tokio::spawn(async move {
                        for _ in 0..10 {
                            let guard = d.read().await;
                            let _ = *guard;
                            drop(guard);
                            tokio::time::sleep(Duration::from_micros(10)).await;
                        }
                    }));
                }

                // Spawn writers
                for _ in 0..num_writers {
                    let d = Arc::clone(&data);
                    handles.push(tokio::spawn(async move {
                        for _ in 0..5 {
                            let mut guard = d.write().await;
                            *guard += 1;
                            drop(guard);
                            tokio::time::sleep(Duration::from_micros(10)).await;
                        }
                    }));
                }

                // All should complete
                for handle in handles {
                    handle.await.unwrap();
                }

                // Final value should be consistent
                let final_value = *data.read().await;
                prop_assert_eq!(final_value, (num_writers * 5) as u64);
                Ok(())
            }).unwrap();
        }
    }
}
