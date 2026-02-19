//! LPT Monitor Implementation (C11)
//!
//! Monitors LLM Logical Phase Transition (LPT) to detect quality degradation.
//! Key metrics tracked:
//!
//! - **Coherence**: Response consistency (same question shouldn't get contradictory answers)
//! - **Efficiency**: Token efficiency (detecting verbose/redundant responses)
//! - **Latency**: Response time normality (detecting sudden slowdowns)
//! - **Repetition**: Detecting repetitive patterns in responses
//!
//! When the LPT score drops below threshold, the system can:
//! - Emit warnings
//! - Throttle requests
//! - Switch to fallback mode
//! - Alert operators

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

// =============================================================================
// Constants
// =============================================================================

/// Window size for rolling statistics
const STATS_WINDOW_SIZE: usize = 100;

/// Minimum samples needed for reliable statistics
const MIN_SAMPLES_FOR_STATS: usize = 5;

/// Default healthy LPT threshold
const DEFAULT_HEALTHY_THRESHOLD: f64 = 0.7;

/// Default warning threshold
const DEFAULT_WARNING_THRESHOLD: f64 = 0.5;

/// Default critical threshold (service restriction)
const DEFAULT_CRITICAL_THRESHOLD: f64 = 0.3;

/// Maximum response time before considered anomalous (ms)
const MAX_NORMAL_LATENCY_MS: u64 = 30_000;

/// Repetition detection: minimum phrase length
const MIN_PHRASE_LENGTH: usize = 10;

/// Repetition detection: how many times a phrase must repeat
const REPETITION_THRESHOLD: usize = 3;

// =============================================================================
// LPT Score
// =============================================================================

/// LPT (Logical Phase Transition) Score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LptScore {
    /// Response coherence (0.0-1.0)
    pub coherence: f64,
    /// Token efficiency (0.0-1.0)
    pub efficiency: f64,
    /// Latency normality (0.0-1.0)
    pub latency: f64,
    /// Repetition score - lower is worse (0.0-1.0)
    pub repetition: f64,
    /// Total weighted score (0.0-1.0)
    pub total: f64,
    /// Timestamp of calculation
    pub calculated_at: DateTime<Utc>,
    /// Number of samples used
    pub sample_count: usize,
}

impl LptScore {
    /// Create a default healthy score
    pub fn healthy() -> Self {
        Self {
            coherence: 1.0,
            efficiency: 1.0,
            latency: 1.0,
            repetition: 1.0,
            total: 1.0,
            calculated_at: Utc::now(),
            sample_count: 0,
        }
    }

    /// Calculate total from components
    pub fn calculate_total(&mut self) {
        // Weighted average: coherence is most important
        self.total = self.coherence * 0.35
            + self.efficiency * 0.20
            + self.latency * 0.25
            + self.repetition * 0.20;
    }
}

impl Default for LptScore {
    fn default() -> Self {
        Self::healthy()
    }
}

// =============================================================================
// Response Record
// =============================================================================

/// Record of a single response for analysis
#[derive(Debug, Clone)]
pub struct ResponseRecord {
    /// Request ID
    pub request_id: String,
    /// Model used
    pub model: String,
    /// Input hash (for coherence checking)
    pub input_hash: String,
    /// Response content (for analysis)
    pub content: String,
    /// Response time in milliseconds
    pub latency_ms: u64,
    /// Prompt tokens
    pub prompt_tokens: u32,
    /// Completion tokens
    pub completion_tokens: u32,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

// =============================================================================
// LPT Monitor State
// =============================================================================

/// Current operational state based on LPT score
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LptState {
    /// Normal operation (score >= healthy threshold)
    Healthy,
    /// Warning state (warning <= score < healthy)
    Warning,
    /// Critical state (critical <= score < warning)
    Critical,
    /// Degraded state (score < critical) - service restriction
    Degraded,
}

impl LptState {
    pub fn from_score(score: f64, config: &LptConfig) -> Self {
        if score >= config.healthy_threshold {
            LptState::Healthy
        } else if score >= config.warning_threshold {
            LptState::Warning
        } else if score >= config.critical_threshold {
            LptState::Critical
        } else {
            LptState::Degraded
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            LptState::Healthy => "healthy",
            LptState::Warning => "warning",
            LptState::Critical => "critical",
            LptState::Degraded => "degraded",
        }
    }
}

// =============================================================================
// Configuration
// =============================================================================

/// LPT Monitor configuration
#[derive(Debug, Clone)]
pub struct LptConfig {
    /// Threshold for healthy state
    pub healthy_threshold: f64,
    /// Threshold for warning state
    pub warning_threshold: f64,
    /// Threshold for critical state
    pub critical_threshold: f64,
    /// Maximum latency before considered anomalous
    pub max_normal_latency_ms: u64,
    /// Enable coherence checking (requires caching responses)
    pub enable_coherence_check: bool,
    /// Window size for statistics
    pub stats_window_size: usize,
}

impl Default for LptConfig {
    fn default() -> Self {
        Self {
            healthy_threshold: DEFAULT_HEALTHY_THRESHOLD,
            warning_threshold: DEFAULT_WARNING_THRESHOLD,
            critical_threshold: DEFAULT_CRITICAL_THRESHOLD,
            max_normal_latency_ms: MAX_NORMAL_LATENCY_MS,
            enable_coherence_check: true,
            stats_window_size: STATS_WINDOW_SIZE,
        }
    }
}

// =============================================================================
// LPT Monitor
// =============================================================================

/// LPT Monitor - tracks LLM quality metrics
pub struct LptMonitor {
    /// Configuration
    config: LptConfig,
    /// Recent response records (rolling window)
    records: Arc<RwLock<VecDeque<ResponseRecord>>>,
    /// Cache for coherence checking: input_hash -> (response_hash, timestamp)
    coherence_cache: Arc<RwLock<HashMap<String, (String, DateTime<Utc>)>>>,
    /// Current LPT score
    current_score: Arc<RwLock<LptScore>>,
    /// Current state
    current_state: Arc<RwLock<LptState>>,
    /// Score history (for trend analysis)
    score_history: Arc<RwLock<VecDeque<LptScore>>>,
    /// Latency statistics: (sum, sum_squared, count)
    latency_stats: Arc<RwLock<(f64, f64, usize)>>,
    /// Efficiency statistics: (sum, count)
    efficiency_stats: Arc<RwLock<(f64, usize)>>,
}

impl LptMonitor {
    /// Create a new LPT monitor with default config
    pub fn new() -> Self {
        Self::with_config(LptConfig::default())
    }

    /// Create with custom config
    pub fn with_config(config: LptConfig) -> Self {
        info!(
            healthy_threshold = config.healthy_threshold,
            warning_threshold = config.warning_threshold,
            "Initializing LPT Monitor (C11)"
        );

        Self {
            config,
            records: Arc::new(RwLock::new(VecDeque::with_capacity(STATS_WINDOW_SIZE))),
            coherence_cache: Arc::new(RwLock::new(HashMap::new())),
            current_score: Arc::new(RwLock::new(LptScore::healthy())),
            current_state: Arc::new(RwLock::new(LptState::Healthy)),
            score_history: Arc::new(RwLock::new(VecDeque::with_capacity(100))),
            latency_stats: Arc::new(RwLock::new((0.0, 0.0, 0))),
            efficiency_stats: Arc::new(RwLock::new((0.0, 0))),
        }
    }

    /// Record a response for analysis
    pub async fn record_response(&self, record: ResponseRecord) {
        debug!(
            request_id = %record.request_id,
            latency_ms = record.latency_ms,
            tokens = record.completion_tokens,
            "Recording response for LPT analysis"
        );

        // Update latency stats
        {
            let mut stats = self.latency_stats.write().await;
            let latency = record.latency_ms as f64;
            stats.0 += latency;
            stats.1 += latency * latency;
            stats.2 += 1;
        }

        // Update efficiency stats (tokens per character of meaningful content)
        {
            let content_len = record.content.trim().len().max(1) as f64;
            let efficiency = content_len / (record.completion_tokens.max(1) as f64);
            let mut stats = self.efficiency_stats.write().await;
            stats.0 += efficiency;
            stats.1 += 1;
        }

        // Check coherence if enabled
        if self.config.enable_coherence_check {
            let content_hash = Self::hash_content(&record.content);
            let mut cache = self.coherence_cache.write().await;

            // Clean old entries (older than 1 hour)
            let cutoff = Utc::now() - Duration::hours(1);
            cache.retain(|_, (_, ts)| *ts > cutoff);

            // Check if we've seen this input before
            if let Some((prev_hash, _)) = cache.get(&record.input_hash) {
                if *prev_hash != content_hash {
                    debug!(
                        input_hash = %record.input_hash,
                        "Coherence issue: different response for same input"
                    );
                    // This will be reflected in the coherence score
                }
            }

            cache.insert(record.input_hash.clone(), (content_hash, record.timestamp));
        }

        // Add to records
        {
            let mut records = self.records.write().await;
            if records.len() >= self.config.stats_window_size {
                records.pop_front();
            }
            records.push_back(record);
        }

        // Recalculate score
        self.recalculate_score().await;
    }

    /// Get current LPT score
    pub async fn get_score(&self) -> LptScore {
        self.current_score.read().await.clone()
    }

    /// Get current state
    pub async fn get_state(&self) -> LptState {
        *self.current_state.read().await
    }

    /// Check if service should be restricted
    pub async fn should_restrict(&self) -> bool {
        matches!(
            *self.current_state.read().await,
            LptState::Critical | LptState::Degraded
        )
    }

    /// Check if service is degraded
    pub async fn is_degraded(&self) -> bool {
        *self.current_state.read().await == LptState::Degraded
    }

    /// Get score history for trend analysis
    pub async fn get_score_history(&self) -> Vec<LptScore> {
        self.score_history.read().await.iter().cloned().collect()
    }

    /// Recalculate LPT score from current data
    async fn recalculate_score(&self) {
        let records = self.records.read().await;
        let sample_count = records.len();

        if sample_count < MIN_SAMPLES_FOR_STATS {
            // Not enough samples, keep healthy score
            return;
        }

        // Calculate coherence score
        let coherence = self.calculate_coherence(&records).await;

        // Calculate efficiency score
        let efficiency = self.calculate_efficiency().await;

        // Calculate latency score
        let latency = self.calculate_latency_score().await;

        // Calculate repetition score
        let repetition = self.calculate_repetition_score(&records);

        let mut score = LptScore {
            coherence,
            efficiency,
            latency,
            repetition,
            total: 0.0,
            calculated_at: Utc::now(),
            sample_count,
        };
        score.calculate_total();

        let new_state = LptState::from_score(score.total, &self.config);
        let old_state = *self.current_state.read().await;

        // Log state transitions
        if new_state != old_state {
            match new_state {
                LptState::Healthy => {
                    info!(score = score.total, "LPT recovered to healthy state");
                }
                LptState::Warning => {
                    warn!(
                        score = score.total,
                        coherence = score.coherence,
                        efficiency = score.efficiency,
                        latency = score.latency,
                        repetition = score.repetition,
                        "LPT WARNING: Quality degradation detected"
                    );
                }
                LptState::Critical => {
                    warn!(
                        score = score.total,
                        "LPT CRITICAL: Significant quality degradation"
                    );
                }
                LptState::Degraded => {
                    warn!(
                        score = score.total,
                        "LPT DEGRADED: Service restriction recommended"
                    );
                }
            }
        }

        // Update state
        *self.current_state.write().await = new_state;
        *self.current_score.write().await = score.clone();

        // Add to history
        {
            let mut history = self.score_history.write().await;
            if history.len() >= 100 {
                history.pop_front();
            }
            history.push_back(score);
        }
    }

    /// Calculate coherence score
    async fn calculate_coherence(&self, records: &VecDeque<ResponseRecord>) -> f64 {
        if !self.config.enable_coherence_check {
            return 1.0;
        }

        let _cache = self.coherence_cache.read().await;

        // Count unique inputs and check for inconsistencies
        let mut input_responses: HashMap<&str, Vec<&str>> = HashMap::new();
        for record in records {
            input_responses
                .entry(&record.input_hash)
                .or_default()
                .push(&record.content);
        }

        let mut consistent_count = 0;
        let mut total_checked = 0;

        for (_, responses) in input_responses {
            if responses.len() > 1 {
                total_checked += 1;
                // Check if all responses are similar
                let first_hash = Self::hash_content(responses[0]);
                let all_same = responses.iter().skip(1).all(|r| {
                    let hash = Self::hash_content(r);
                    hash == first_hash || Self::similarity(responses[0], r) > 0.8
                });
                if all_same {
                    consistent_count += 1;
                }
            }
        }

        if total_checked == 0 {
            1.0 // No repeated inputs to check
        } else {
            consistent_count as f64 / total_checked as f64
        }
    }

    /// Calculate efficiency score
    async fn calculate_efficiency(&self) -> f64 {
        let stats = self.efficiency_stats.read().await;
        if stats.1 == 0 {
            return 1.0;
        }

        let avg_efficiency = stats.0 / stats.1 as f64;

        // Normalize: assume good efficiency is ~5-10 chars per token
        // Too low (< 2) = verbose, too high (> 20) = possibly incomplete
        if avg_efficiency < 2.0 {
            avg_efficiency / 2.0 // Penalize verbose responses
        } else if avg_efficiency > 20.0 {
            0.5 // Very terse might indicate issues
        } else {
            1.0
        }
    }

    /// Calculate latency score
    async fn calculate_latency_score(&self) -> f64 {
        let stats = self.latency_stats.read().await;
        if stats.2 == 0 {
            return 1.0;
        }

        let mean = stats.0 / stats.2 as f64;
        let variance = (stats.1 / stats.2 as f64) - (mean * mean);
        let std_dev = variance.max(0.0).sqrt();

        // Score based on mean latency
        let mean_score = if mean <= self.config.max_normal_latency_ms as f64 {
            1.0
        } else {
            (self.config.max_normal_latency_ms as f64 / mean).min(1.0)
        };

        // Penalize high variance (unstable performance)
        let cv = if mean > 0.0 { std_dev / mean } else { 0.0 };
        let variance_penalty = if cv > 1.0 { 0.8 } else { 1.0 };

        mean_score * variance_penalty
    }

    /// Calculate repetition score
    fn calculate_repetition_score(&self, records: &VecDeque<ResponseRecord>) -> f64 {
        let mut total_score = 0.0;
        let mut count = 0;

        for record in records {
            let score = Self::detect_repetition(&record.content);
            total_score += score;
            count += 1;
        }

        if count == 0 {
            1.0
        } else {
            total_score / count as f64
        }
    }

    /// Detect repetitive patterns in text
    fn detect_repetition(text: &str) -> f64 {
        if text.len() < MIN_PHRASE_LENGTH * 2 {
            return 1.0; // Too short to have meaningful repetition
        }

        let words: Vec<&str> = text.split_whitespace().collect();
        if words.len() < 10 {
            return 1.0;
        }

        // Count n-gram repetitions
        let mut ngram_counts: HashMap<String, usize> = HashMap::new();
        let ngram_size = 3;

        for window in words.windows(ngram_size) {
            let ngram = window.join(" ");
            *ngram_counts.entry(ngram).or_insert(0) += 1;
        }

        // Count how many ngrams repeat more than threshold
        let repetitive_ngrams = ngram_counts
            .values()
            .filter(|&&count| count >= REPETITION_THRESHOLD)
            .count();

        let total_ngrams = ngram_counts.len().max(1);
        let repetition_ratio = repetitive_ngrams as f64 / total_ngrams as f64;

        // Score: 1.0 if no repetition, decreases with more repetition
        (1.0 - repetition_ratio * 2.0).max(0.0)
    }

    /// Simple hash for content comparison
    fn hash_content(content: &str) -> String {
        use sha2::{Digest, Sha256};
        let normalized = content
            .to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let mut hasher = Sha256::new();
        hasher.update(normalized.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Calculate similarity between two strings (Jaccard similarity of words)
    fn similarity(a: &str, b: &str) -> f64 {
        let a_lower = a.to_lowercase();
        let b_lower = b.to_lowercase();
        let words_a: std::collections::HashSet<_> = a_lower.split_whitespace().collect();
        let words_b: std::collections::HashSet<_> = b_lower.split_whitespace().collect();

        let intersection = words_a.intersection(&words_b).count();
        let union = words_a.union(&words_b).count();

        if union == 0 {
            1.0
        } else {
            intersection as f64 / union as f64
        }
    }

    /// Get summary for logging/metrics
    pub async fn get_summary(&self) -> LptSummary {
        let score = self.current_score.read().await.clone();
        let state = *self.current_state.read().await;
        let records = self.records.read().await;

        LptSummary {
            state,
            score,
            sample_count: records.len(),
            window_size: self.config.stats_window_size,
        }
    }
}

impl Default for LptMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary for external reporting
#[derive(Debug, Clone, Serialize)]
pub struct LptSummary {
    pub state: LptState,
    pub score: LptScore,
    pub sample_count: usize,
    pub window_size: usize,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_healthy_by_default() {
        let monitor = LptMonitor::new();
        let score = monitor.get_score().await;
        assert_eq!(score.total, 1.0);
        assert_eq!(monitor.get_state().await, LptState::Healthy);
    }

    #[tokio::test]
    async fn test_record_response() {
        let monitor = LptMonitor::new();

        for i in 0..10 {
            let record = ResponseRecord {
                request_id: format!("req_{}", i),
                model: "gpt-4".to_string(),
                input_hash: format!("hash_{}", i),
                content: format!("This is response number {}. It contains some text.", i),
                latency_ms: 1000 + (i * 100) as u64,
                prompt_tokens: 50,
                completion_tokens: 30,
                timestamp: Utc::now(),
            };
            monitor.record_response(record).await;
        }

        let score = monitor.get_score().await;
        assert!(score.sample_count >= 5);
        assert!(score.total > 0.5);
    }

    #[tokio::test]
    async fn test_repetition_detection() {
        let text_with_repetition = "The quick brown fox. The quick brown fox. The quick brown fox. The quick brown fox. jumps over the lazy dog.";
        let score = LptMonitor::detect_repetition(text_with_repetition);
        assert!(score < 1.0, "Should detect repetition");

        let text_without_repetition = "The quick brown fox jumps over the lazy dog. A different sentence with unique words follows.";
        let score = LptMonitor::detect_repetition(text_without_repetition);
        assert!(score >= 0.8, "Should not detect significant repetition");
    }

    #[tokio::test]
    async fn test_state_transitions() {
        let config = LptConfig {
            healthy_threshold: 0.8,
            warning_threshold: 0.6,
            critical_threshold: 0.4,
            ..Default::default()
        };

        assert_eq!(LptState::from_score(0.9, &config), LptState::Healthy);
        assert_eq!(LptState::from_score(0.7, &config), LptState::Warning);
        assert_eq!(LptState::from_score(0.5, &config), LptState::Critical);
        assert_eq!(LptState::from_score(0.3, &config), LptState::Degraded);
    }

    #[test]
    fn test_similarity() {
        let a = "The quick brown fox";
        let b = "The quick brown dog";
        let sim = LptMonitor::similarity(a, b);
        assert!(sim > 0.5 && sim < 1.0);

        let c = "Completely different sentence";
        let sim2 = LptMonitor::similarity(a, c);
        assert!(sim2 < sim);
    }

    #[tokio::test]
    async fn test_latency_anomaly() {
        let monitor = LptMonitor::new();

        // Record normal responses
        for i in 0..10 {
            let record = ResponseRecord {
                request_id: format!("req_{}", i),
                model: "gpt-4".to_string(),
                input_hash: format!("hash_{}", i),
                content: "Normal response content here.".to_string(),
                latency_ms: 2000, // Normal latency
                prompt_tokens: 50,
                completion_tokens: 30,
                timestamp: Utc::now(),
            };
            monitor.record_response(record).await;
        }

        let score_before = monitor.get_score().await;

        // Record slow responses
        for i in 10..20 {
            let record = ResponseRecord {
                request_id: format!("req_{}", i),
                model: "gpt-4".to_string(),
                input_hash: format!("hash_{}", i),
                content: "Slow response content here.".to_string(),
                latency_ms: 60000, // Very slow
                prompt_tokens: 50,
                completion_tokens: 30,
                timestamp: Utc::now(),
            };
            monitor.record_response(record).await;
        }

        let score_after = monitor.get_score().await;
        assert!(
            score_after.latency < score_before.latency,
            "Latency score should decrease with slow responses"
        );
    }
}
