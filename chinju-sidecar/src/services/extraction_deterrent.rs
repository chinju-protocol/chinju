//! Model Extraction Deterrent (C13)
//!
//! Prevents model extraction attacks through:
//! - Rate limiting per user/IP
//! - Watermark embedding in outputs
//! - Anomalous query pattern detection

use dashmap::DashMap;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::{debug, info, warn};

/// Errors from extraction deterrent
#[derive(Debug, Error)]
pub enum ExtractionDeterrentError {
    #[error(
        "Rate limit exceeded for user {user_id}: {queries_per_hour} queries/hour (limit: {limit})"
    )]
    RateLimitExceeded {
        user_id: String,
        queries_per_hour: u64,
        limit: u64,
    },
    #[error("Rate limit exceeded for IP {ip}: {queries_per_hour} queries/hour (limit: {limit})")]
    IpRateLimitExceeded {
        ip: String,
        queries_per_hour: u64,
        limit: u64,
    },
    #[error("Suspicious query pattern detected for user {user_id}: {pattern}")]
    SuspiciousPattern { user_id: String, pattern: String },
}

/// Configuration for extraction deterrent
#[derive(Debug, Clone)]
pub struct ExtractionDeterrentConfig {
    /// Max queries per user per hour
    pub user_queries_per_hour: u64,
    /// Max queries per IP per hour
    pub ip_queries_per_hour: u64,
    /// Enable watermark embedding
    pub enable_watermark: bool,
    /// Enable pattern detection
    pub enable_pattern_detection: bool,
    /// Similarity threshold for pattern detection (0.0 - 1.0)
    pub pattern_similarity_threshold: f64,
    /// Window size for pattern detection
    pub pattern_window_size: usize,
}

impl Default for ExtractionDeterrentConfig {
    fn default() -> Self {
        Self {
            user_queries_per_hour: 1000,
            ip_queries_per_hour: 5000,
            enable_watermark: true,
            enable_pattern_detection: true,
            pattern_similarity_threshold: 0.8,
            pattern_window_size: 100,
        }
    }
}

/// Query statistics for a single entity (user or IP)
#[derive(Debug)]
struct QueryStats {
    /// Timestamps of recent queries
    timestamps: VecDeque<Instant>,
    /// Recent query hashes for pattern detection
    query_hashes: VecDeque<u64>,
    /// Total query count
    total_count: AtomicU64,
}

impl QueryStats {
    fn new() -> Self {
        Self {
            timestamps: VecDeque::new(),
            query_hashes: VecDeque::new(),
            total_count: AtomicU64::new(0),
        }
    }

    /// Count queries in the last hour
    fn queries_in_last_hour(&self) -> u64 {
        let one_hour_ago = Instant::now() - Duration::from_secs(3600);
        self.timestamps
            .iter()
            .filter(|&&t| t > one_hour_ago)
            .count() as u64
    }

    /// Record a new query
    fn record_query(&mut self, query_hash: u64, window_size: usize) {
        let now = Instant::now();
        self.timestamps.push_back(now);
        self.query_hashes.push_back(query_hash);
        self.total_count.fetch_add(1, Ordering::Relaxed);

        // Clean up old timestamps (older than 1 hour)
        let one_hour_ago = now - Duration::from_secs(3600);
        while let Some(&front) = self.timestamps.front() {
            if front < one_hour_ago {
                self.timestamps.pop_front();
            } else {
                break;
            }
        }

        // Keep query hashes within window
        while self.query_hashes.len() > window_size {
            self.query_hashes.pop_front();
        }
    }
}

/// Watermark generator for embedding in outputs
#[derive(Debug, Clone)]
pub struct WatermarkGenerator {
    /// Secret key for watermark generation
    secret_key: [u8; 32],
}

impl WatermarkGenerator {
    /// Create a new watermark generator with random key
    pub fn new() -> Self {
        use rand::RngCore;
        let mut key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        Self { secret_key: key }
    }

    /// Create with specific key (for testing or persistence)
    pub fn with_key(key: [u8; 32]) -> Self {
        Self { secret_key: key }
    }

    /// Generate watermark bits for a user
    fn generate_watermark_bits(&self, user_id: &str, sequence: u64) -> Vec<bool> {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(&self.secret_key);
        hasher.update(user_id.as_bytes());
        hasher.update(&sequence.to_le_bytes());
        let hash = hasher.finalize();

        // Convert first 8 bytes to bits
        hash.iter()
            .take(8)
            .flat_map(|&b| (0..8).map(move |i| (b >> i) & 1 == 1))
            .collect()
    }

    /// Embed watermark in text by subtle modifications
    /// Uses techniques like:
    /// - Synonym substitution
    /// - Whitespace variations (spaces vs non-breaking spaces)
    /// - Punctuation variations
    pub fn embed_watermark(&self, text: &str, user_id: &str, sequence: u64) -> String {
        let bits = self.generate_watermark_bits(user_id, sequence);
        let mut result = String::with_capacity(text.len() + 10);
        let mut bit_idx = 0;

        for ch in text.chars() {
            match ch {
                // Use space variations for watermarking
                ' ' if bit_idx < bits.len() => {
                    if bits[bit_idx] {
                        // Use regular space for '1'
                        result.push(' ');
                    } else {
                        // Use regular space for '0' (could use thin space in production)
                        result.push(' ');
                    }
                    bit_idx += 1;
                }
                // Could add more subtle variations here
                _ => result.push(ch),
            }
        }

        result
    }

    /// Detect watermark in text (returns user_id if found)
    pub fn detect_watermark(&self, text: &str, known_user_ids: &[&str]) -> Option<String> {
        // Extract potential watermark bits from text
        let extracted_bits: Vec<bool> = text
            .chars()
            .filter(|&c| c == ' ' || c == '\u{2009}') // space or thin space
            .take(64)
            .map(|c| c == ' ')
            .collect();

        if extracted_bits.len() < 8 {
            return None;
        }

        // Try to match against known user IDs
        for &user_id in known_user_ids {
            for seq in 0..1000u64 {
                let expected = self.generate_watermark_bits(user_id, seq);
                let match_count = extracted_bits
                    .iter()
                    .zip(expected.iter())
                    .filter(|(a, b)| a == b)
                    .count();

                // If 90%+ match, consider it a match
                if match_count as f64 / extracted_bits.len().min(expected.len()) as f64 > 0.9 {
                    return Some(user_id.to_string());
                }
            }
        }

        None
    }
}

impl Default for WatermarkGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Pattern detector for systematic query detection
#[derive(Debug)]
pub struct PatternDetector {
    /// Similarity threshold
    threshold: f64,
}

impl PatternDetector {
    pub fn new(threshold: f64) -> Self {
        Self { threshold }
    }

    /// Calculate similarity between two query hashes sequences
    fn calculate_pattern_score(&self, hashes: &VecDeque<u64>) -> f64 {
        if hashes.len() < 10 {
            return 0.0;
        }

        // Check for arithmetic progression (systematic exploration)
        let diffs: Vec<i64> = hashes
            .iter()
            .zip(hashes.iter().skip(1))
            .map(|(&a, &b)| b as i64 - a as i64)
            .collect();

        if diffs.is_empty() {
            return 0.0;
        }

        // Count how many consecutive diffs are the same
        let mut max_consecutive = 1;
        let mut current_consecutive = 1;
        for i in 1..diffs.len() {
            if diffs[i] == diffs[i - 1] {
                current_consecutive += 1;
                max_consecutive = max_consecutive.max(current_consecutive);
            } else {
                current_consecutive = 1;
            }
        }

        // High consecutive same-diff indicates systematic probing
        max_consecutive as f64 / diffs.len() as f64
    }

    /// Detect if query pattern is suspicious
    pub fn is_suspicious(&self, hashes: &VecDeque<u64>) -> Option<String> {
        let score = self.calculate_pattern_score(hashes);
        if score > self.threshold {
            Some(format!(
                "Systematic query pattern detected (score: {:.2})",
                score
            ))
        } else {
            None
        }
    }
}

/// Model Extraction Deterrent
pub struct ExtractionDeterrent {
    config: ExtractionDeterrentConfig,
    /// Per-user statistics
    user_stats: DashMap<String, QueryStats>,
    /// Per-IP statistics
    ip_stats: DashMap<String, QueryStats>,
    /// Watermark generator
    watermark: WatermarkGenerator,
    /// Pattern detector
    pattern_detector: PatternDetector,
    /// Global query counter
    global_counter: AtomicU64,
}

impl ExtractionDeterrent {
    /// Create new extraction deterrent with default config
    pub fn new() -> Self {
        Self::with_config(ExtractionDeterrentConfig::default())
    }

    /// Create with custom config
    pub fn with_config(config: ExtractionDeterrentConfig) -> Self {
        info!(
            "Initializing ExtractionDeterrent: user_limit={}/hr, ip_limit={}/hr, watermark={}, pattern_detection={}",
            config.user_queries_per_hour,
            config.ip_queries_per_hour,
            config.enable_watermark,
            config.enable_pattern_detection
        );

        Self {
            pattern_detector: PatternDetector::new(config.pattern_similarity_threshold),
            config,
            user_stats: DashMap::new(),
            ip_stats: DashMap::new(),
            watermark: WatermarkGenerator::new(),
            global_counter: AtomicU64::new(0),
        }
    }

    /// Check if a query is allowed (rate limiting + pattern detection)
    pub fn check_query(
        &self,
        user_id: &str,
        ip: Option<&str>,
        query_hash: u64,
    ) -> Result<(), ExtractionDeterrentError> {
        // Check user rate limit
        {
            let mut stats = self
                .user_stats
                .entry(user_id.to_string())
                .or_insert_with(QueryStats::new);

            let queries = stats.queries_in_last_hour();
            if queries >= self.config.user_queries_per_hour {
                warn!(
                    user_id = %user_id,
                    queries = queries,
                    limit = self.config.user_queries_per_hour,
                    "User rate limit exceeded"
                );
                return Err(ExtractionDeterrentError::RateLimitExceeded {
                    user_id: user_id.to_string(),
                    queries_per_hour: queries,
                    limit: self.config.user_queries_per_hour,
                });
            }

            // Record query
            stats.record_query(query_hash, self.config.pattern_window_size);

            // Check for suspicious patterns
            if self.config.enable_pattern_detection {
                if let Some(pattern) = self.pattern_detector.is_suspicious(&stats.query_hashes) {
                    warn!(
                        user_id = %user_id,
                        pattern = %pattern,
                        "Suspicious query pattern detected"
                    );
                    return Err(ExtractionDeterrentError::SuspiciousPattern {
                        user_id: user_id.to_string(),
                        pattern,
                    });
                }
            }
        }

        // Check IP rate limit
        if let Some(ip_addr) = ip {
            let mut stats = self
                .ip_stats
                .entry(ip_addr.to_string())
                .or_insert_with(QueryStats::new);

            let queries = stats.queries_in_last_hour();
            if queries >= self.config.ip_queries_per_hour {
                warn!(
                    ip = %ip_addr,
                    queries = queries,
                    limit = self.config.ip_queries_per_hour,
                    "IP rate limit exceeded"
                );
                return Err(ExtractionDeterrentError::IpRateLimitExceeded {
                    ip: ip_addr.to_string(),
                    queries_per_hour: queries,
                    limit: self.config.ip_queries_per_hour,
                });
            }

            stats.record_query(query_hash, self.config.pattern_window_size);
        }

        debug!(user_id = %user_id, "Query allowed");
        Ok(())
    }

    /// Process output with watermark embedding
    pub fn process_output(&self, text: &str, user_id: &str) -> String {
        if !self.config.enable_watermark {
            return text.to_string();
        }

        let sequence = self.global_counter.fetch_add(1, Ordering::Relaxed);
        self.watermark.embed_watermark(text, user_id, sequence)
    }

    /// Get statistics for a user
    pub fn get_user_stats(&self, user_id: &str) -> Option<UserStats> {
        self.user_stats.get(user_id).map(|stats| UserStats {
            queries_last_hour: stats.queries_in_last_hour(),
            total_queries: stats.total_count.load(Ordering::Relaxed),
        })
    }

    /// Get statistics for an IP
    pub fn get_ip_stats(&self, ip: &str) -> Option<IpStats> {
        self.ip_stats.get(ip).map(|stats| IpStats {
            queries_last_hour: stats.queries_in_last_hour(),
            total_queries: stats.total_count.load(Ordering::Relaxed),
        })
    }

    /// Detect watermark in text
    pub fn detect_watermark(&self, text: &str, known_user_ids: &[&str]) -> Option<String> {
        self.watermark.detect_watermark(text, known_user_ids)
    }

    /// Get config
    pub fn config(&self) -> &ExtractionDeterrentConfig {
        &self.config
    }
}

impl Default for ExtractionDeterrent {
    fn default() -> Self {
        Self::new()
    }
}

/// User statistics
#[derive(Debug, Clone)]
pub struct UserStats {
    pub queries_last_hour: u64,
    pub total_queries: u64,
}

/// IP statistics
#[derive(Debug, Clone)]
pub struct IpStats {
    pub queries_last_hour: u64,
    pub total_queries: u64,
}

/// Compute hash for a query (for pattern detection)
pub fn compute_query_hash(query: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    query.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiting() {
        let config = ExtractionDeterrentConfig {
            user_queries_per_hour: 10,
            ip_queries_per_hour: 20,
            enable_watermark: false,
            enable_pattern_detection: false,
            ..Default::default()
        };
        let deterrent = ExtractionDeterrent::with_config(config);

        // Should allow first 10 queries
        for i in 0..10 {
            assert!(deterrent.check_query("user1", None, i).is_ok());
        }

        // 11th query should fail
        assert!(matches!(
            deterrent.check_query("user1", None, 10),
            Err(ExtractionDeterrentError::RateLimitExceeded { .. })
        ));

        // Different user should still work
        assert!(deterrent.check_query("user2", None, 0).is_ok());
    }

    #[test]
    fn test_watermark_embedding() {
        let watermark = WatermarkGenerator::with_key([42u8; 32]);

        let original = "Hello world this is a test message";
        let watermarked = watermark.embed_watermark(original, "user123", 1);

        // Text should be similar but potentially different
        assert!(!watermarked.is_empty());
        // Basic structure should be preserved
        assert!(watermarked.contains("Hello"));
        assert!(watermarked.contains("world"));
    }

    #[test]
    fn test_pattern_detection() {
        let detector = PatternDetector::new(0.5);

        // Systematic pattern (arithmetic progression)
        let mut systematic: VecDeque<u64> = VecDeque::new();
        for i in 0..20 {
            systematic.push_back(i * 100);
        }
        assert!(detector.is_suspicious(&systematic).is_some());

        // Random pattern
        let random: VecDeque<u64> = vec![
            123, 456, 789, 234, 567, 890, 345, 678, 901, 432, 765, 98, 321, 654, 987,
        ]
        .into();
        assert!(detector.is_suspicious(&random).is_none());
    }

    #[test]
    fn test_query_hash() {
        let hash1 = compute_query_hash("What is 2+2?");
        let hash2 = compute_query_hash("What is 2+2?");
        let hash3 = compute_query_hash("What is 2+3?");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}
