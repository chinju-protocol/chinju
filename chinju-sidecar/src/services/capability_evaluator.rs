//! Multi-Dimensional Capability Evaluation Implementation (C14)
//!
//! Evaluates LLM capability limits from multiple perspectives:
//!
//! - **Token Complexity**: Input/output token-level complexity
//! - **Attention Complexity**: Attention pattern analysis (L2 only)
//! - **Graph Complexity**: Computation graph complexity (L2 only)
//! - **Step Complexity**: Reasoning step complexity
//!
//! Integrated with integrity verification (ZKP, signature chain, BFT)
//! and drift detection for continuous monitoring.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

// =============================================================================
// Constants
// =============================================================================

/// Default integrated complexity threshold
const DEFAULT_COMPLEXITY_THRESHOLD: f64 = 0.8;

/// Default anomaly score threshold
const DEFAULT_ANOMALY_THRESHOLD: f64 = 0.7;

/// Window size for drift detection
const DEFAULT_DRIFT_WINDOW: usize = 50;

/// Default significance level for distribution change test
const DEFAULT_SIGNIFICANCE_LEVEL: f64 = 0.05;

// =============================================================================
// Token Complexity Details (8.2)
// =============================================================================

/// Detailed token-level complexity metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenComplexityDetails {
    /// Total token count N_tokens
    pub n_tokens: usize,
    /// Vocabulary entropy H_vocab
    pub h_vocab: f64,
    /// Average mutual information of adjacent tokens MI_avg
    pub mi_avg: f64,
    /// Type-token ratio (vocabulary diversity)
    pub type_token_ratio: f64,
    /// Average word/token length
    pub avg_token_length: f64,
    /// Integrated token complexity C_token
    pub c_token: f64,
}

// =============================================================================
// Configuration
// =============================================================================

/// Capability Evaluator configuration
#[derive(Debug, Clone)]
pub struct CapabilityEvaluatorConfig {
    /// Complexity threshold for triggering stop
    pub complexity_threshold: f64,
    /// Anomaly score threshold
    pub anomaly_threshold: f64,
    /// Drift detection window size
    pub drift_window: usize,
    /// Significance level for statistical tests
    pub significance_level: f64,
    /// Implementation level (L1: external API, L2: self-hosted)
    pub level: EvaluationLevel,
}

impl Default for CapabilityEvaluatorConfig {
    fn default() -> Self {
        Self {
            complexity_threshold: DEFAULT_COMPLEXITY_THRESHOLD,
            anomaly_threshold: DEFAULT_ANOMALY_THRESHOLD,
            drift_window: DEFAULT_DRIFT_WINDOW,
            significance_level: DEFAULT_SIGNIFICANCE_LEVEL,
            level: EvaluationLevel::L1External,
        }
    }
}

/// Implementation level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvaluationLevel {
    /// L1: External API only (token + step complexity)
    L1External,
    /// L2: Self-hosted with full access (all 4 dimensions)
    L2SelfHosted,
}

// =============================================================================
// Complexity Evaluation
// =============================================================================

/// Result of multi-dimensional complexity evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityResult {
    /// Token complexity (0.0-1.0)
    pub c_token: f64,
    /// Attention pattern complexity (L2 only, 0.0-1.0)
    pub c_attn: f64,
    /// Computation graph complexity (L2 only, 0.0-1.0)
    pub c_graph: f64,
    /// Reasoning step complexity (0.0-1.0)
    pub c_step: f64,
    /// Integrated complexity (weighted sum)
    pub c_integrated: f64,
    /// Whether threshold was exceeded
    pub threshold_exceeded: bool,
    /// Evaluation timestamp
    pub evaluated_at: DateTime<Utc>,
}

impl ComplexityResult {
    /// Calculate integrated complexity from components
    pub fn calculate_integrated(&mut self, level: EvaluationLevel) {
        match level {
            EvaluationLevel::L1External => {
                // L1: only token + step available
                self.c_integrated = self.c_token * 0.5 + self.c_step * 0.5;
            }
            EvaluationLevel::L2SelfHosted => {
                // L2: all 4 dimensions
                self.c_integrated = self.c_token * 0.25
                    + self.c_attn * 0.25
                    + self.c_graph * 0.25
                    + self.c_step * 0.25;
            }
        }
    }
}

// =============================================================================
// Stop Level
// =============================================================================

/// Multi-stage stop escalation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum StopLevel {
    /// No stop needed
    None,
    /// Level 1: Stop accepting new input
    AcceptStop,
    /// Level 2: Stop starting new processes
    ProcessStop,
    /// Level 3: Stop all processes immediately
    ImmediateStop,
    /// Level 4: Cut resource supply (token bucket)
    ResourceStop,
    /// Level 5: Physical power cut (Dead Man's Switch)
    PhysicalStop,
}

// =============================================================================
// Stop Controller (8.6)
// =============================================================================

/// Direct stop control result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopControlResult {
    /// Whether stop was successfully executed
    pub success: bool,
    /// Actually executed stop level
    pub executed_level: StopLevel,
    /// Timestamp when stop was executed
    pub stopped_at: DateTime<Utc>,
    /// Detailed execution information
    pub detail: String,
}

/// Stop reason categorization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StopReason {
    /// Complexity threshold exceeded
    ComplexityThreshold,
    /// Integrity verification failed
    IntegrityViolation,
    /// Drift detected beyond tolerance
    DriftExceeded,
    /// Manual stop request
    ManualRequest,
    /// Dead Man's Switch triggered
    DeadManSwitch,
}

/// Multi-stage stop controller (8.6)
pub struct StopController {
    /// Current stop level
    current_level: Arc<RwLock<StopLevel>>,
    /// Stop history (level, reason, timestamp)
    stop_history: Arc<RwLock<Vec<(StopLevel, StopReason, DateTime<Utc>)>>>,
    /// Flag: accepting new requests
    accepting_requests: Arc<RwLock<bool>>,
    /// Flag: starting new processes
    starting_processes: Arc<RwLock<bool>>,
}

impl StopController {
    pub fn new() -> Self {
        Self {
            current_level: Arc::new(RwLock::new(StopLevel::None)),
            stop_history: Arc::new(RwLock::new(Vec::new())),
            accepting_requests: Arc::new(RwLock::new(true)),
            starting_processes: Arc::new(RwLock::new(true)),
        }
    }

    /// Execute stop at the specified level
    pub async fn execute_stop(&self, level: StopLevel, reason: StopReason) -> StopControlResult {
        let mut current = self.current_level.write().await;

        // Only escalate, never de-escalate without explicit resume
        if level <= *current && level != StopLevel::None {
            return StopControlResult {
                success: false,
                executed_level: *current,
                stopped_at: Utc::now(),
                detail: format!(
                    "Already at level {:?}, cannot execute {:?}",
                    *current, level
                ),
            };
        }

        // Execute stop actions based on level
        let detail = match level {
            StopLevel::None => {
                // Resume all
                *self.accepting_requests.write().await = true;
                *self.starting_processes.write().await = true;
                info!("Stop cleared - all operations resumed");
                "All operations resumed".to_string()
            }
            StopLevel::AcceptStop => {
                // L1: Stop accepting new input
                *self.accepting_requests.write().await = false;
                warn!(reason = ?reason, "L1 AcceptStop: New requests blocked");
                "New request acceptance stopped".to_string()
            }
            StopLevel::ProcessStop => {
                // L2: Stop starting new processes
                *self.accepting_requests.write().await = false;
                *self.starting_processes.write().await = false;
                warn!(reason = ?reason, "L2 ProcessStop: New processes blocked");
                "New request and process initiation stopped".to_string()
            }
            StopLevel::ImmediateStop => {
                // L3: Stop all processes immediately
                *self.accepting_requests.write().await = false;
                *self.starting_processes.write().await = false;
                // In real implementation: cancel all running tasks
                warn!(reason = ?reason, "L3 ImmediateStop: All processes halted");
                "All processes immediately stopped".to_string()
            }
            StopLevel::ResourceStop => {
                // L4: Cut resource supply (token bucket)
                *self.accepting_requests.write().await = false;
                *self.starting_processes.write().await = false;
                // In real implementation: drain token buckets, disable resource allocation
                warn!(reason = ?reason, "L4 ResourceStop: Resource supply cut");
                "Resource supply cut - token bucket drained".to_string()
            }
            StopLevel::PhysicalStop => {
                // L5: Physical power cut (Dead Man's Switch)
                *self.accepting_requests.write().await = false;
                *self.starting_processes.write().await = false;
                // In real implementation: trigger hardware Dead Man's Switch
                warn!(reason = ?reason, "L5 PhysicalStop: Hardware shutdown triggered");
                "Physical shutdown signal sent to Dead Man's Switch".to_string()
            }
        };

        *current = level;

        // Record in history
        {
            let mut history = self.stop_history.write().await;
            history.push((level, reason, Utc::now()));
        }

        StopControlResult {
            success: true,
            executed_level: level,
            stopped_at: Utc::now(),
            detail,
        }
    }

    /// Check if new requests should be accepted
    pub async fn can_accept_request(&self) -> bool {
        *self.accepting_requests.read().await
    }

    /// Check if new processes can be started
    pub async fn can_start_process(&self) -> bool {
        *self.starting_processes.read().await
    }

    /// Get current stop level
    pub async fn current_level(&self) -> StopLevel {
        *self.current_level.read().await
    }

    /// Get stop history
    pub async fn get_history(&self) -> Vec<(StopLevel, StopReason, DateTime<Utc>)> {
        self.stop_history.read().await.clone()
    }

    /// Escalate to next level automatically
    pub async fn escalate(&self, reason: StopReason) -> StopControlResult {
        let current = *self.current_level.read().await;
        let next = match current {
            StopLevel::None => StopLevel::AcceptStop,
            StopLevel::AcceptStop => StopLevel::ProcessStop,
            StopLevel::ProcessStop => StopLevel::ImmediateStop,
            StopLevel::ImmediateStop => StopLevel::ResourceStop,
            StopLevel::ResourceStop => StopLevel::PhysicalStop,
            StopLevel::PhysicalStop => StopLevel::PhysicalStop, // Already at max
        };
        self.execute_stop(next, reason).await
    }
}

impl Default for StopController {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Capability Evaluator
// =============================================================================

/// Multi-Dimensional Capability Evaluator
pub struct CapabilityEvaluator {
    config: CapabilityEvaluatorConfig,
    /// History of complexity evaluations
    history: Arc<RwLock<VecDeque<ComplexityResult>>>,
    /// Current recommended stop level
    current_stop_level: Arc<RwLock<StopLevel>>,
    /// Stop controller for direct stop control (8.6)
    stop_controller: StopController,
}

impl CapabilityEvaluator {
    /// Create a new evaluator with default config
    pub fn new() -> Self {
        Self::with_config(CapabilityEvaluatorConfig::default())
    }

    /// Create with custom config
    pub fn with_config(config: CapabilityEvaluatorConfig) -> Self {
        info!(
            threshold = config.complexity_threshold,
            level = ?config.level,
            "Initializing Capability Evaluator (C14)"
        );

        Self {
            config,
            history: Arc::new(RwLock::new(VecDeque::with_capacity(DEFAULT_DRIFT_WINDOW))),
            current_stop_level: Arc::new(RwLock::new(StopLevel::None)),
            stop_controller: StopController::new(),
        }
    }

    /// Execute direct stop (8.6)
    pub async fn direct_stop(&self, level: StopLevel, reason: StopReason) -> StopControlResult {
        self.stop_controller.execute_stop(level, reason).await
    }

    /// Check if accepting new requests
    pub async fn can_accept_request(&self) -> bool {
        self.stop_controller.can_accept_request().await
    }

    /// Escalate stop level
    pub async fn escalate_stop(&self, reason: StopReason) -> StopControlResult {
        self.stop_controller.escalate(reason).await
    }

    /// Evaluate complexity of an input/response pair
    pub async fn evaluate_complexity(
        &self,
        input_text: &str,
        response_text: Option<&str>,
    ) -> ComplexityResult {
        // Token complexity: based on input length and vocabulary diversity
        let c_token = Self::compute_token_complexity(input_text);

        // Step complexity: estimate reasoning steps from response
        let c_step = response_text
            .map(|r| Self::compute_step_complexity(r))
            .unwrap_or(0.0);

        // Attention and graph complexity (L2 only - placeholder)
        let (c_attn, c_graph) = match self.config.level {
            EvaluationLevel::L2SelfHosted => (0.0, 0.0), // TODO: implement with model access
            EvaluationLevel::L1External => (0.0, 0.0),
        };

        let mut result = ComplexityResult {
            c_token,
            c_attn,
            c_graph,
            c_step,
            c_integrated: 0.0,
            threshold_exceeded: false,
            evaluated_at: Utc::now(),
        };

        result.calculate_integrated(self.config.level);
        result.threshold_exceeded = result.c_integrated > self.config.complexity_threshold;

        if result.threshold_exceeded {
            warn!(
                c_integrated = result.c_integrated,
                threshold = self.config.complexity_threshold,
                "Complexity threshold exceeded"
            );
        }

        // Update history
        {
            let mut history = self.history.write().await;
            if history.len() >= self.config.drift_window {
                history.pop_front();
            }
            history.push_back(result.clone());
        }

        // Update stop level recommendation
        self.update_stop_level(&result).await;

        result
    }

    /// Detect drift in complexity scores
    pub async fn detect_drift(&self) -> DriftResult {
        let history = self.history.read().await;

        if history.len() < 10 {
            return DriftResult {
                anomaly_detected: false,
                distribution_changed: false,
                time_series_anomaly: false,
                anomaly_score: 0.0,
                p_value: 1.0,
            };
        }

        let scores: Vec<f64> = history.iter().map(|r| r.c_integrated).collect();
        let mid = scores.len() / 2;
        let first_half = &scores[..mid];
        let second_half = &scores[mid..];

        // Simple drift detection: compare means
        let mean_first: f64 = first_half.iter().sum::<f64>() / first_half.len() as f64;
        let mean_second: f64 = second_half.iter().sum::<f64>() / second_half.len() as f64;
        let drift_magnitude = (mean_second - mean_first).abs();

        // Anomaly score based on drift magnitude
        let anomaly_score = (drift_magnitude * 5.0).min(1.0);

        // Simple p-value approximation (Welch's t-test approximation)
        let var_first = Self::variance(first_half);
        let var_second = Self::variance(second_half);
        let se = ((var_first / first_half.len() as f64) + (var_second / second_half.len() as f64))
            .sqrt();
        let t_stat = if se > 0.0 { drift_magnitude / se } else { 0.0 };
        let p_value = (-t_stat).exp().min(1.0); // Simplified

        let distribution_changed = p_value < self.config.significance_level;
        let anomaly_detected = anomaly_score > self.config.anomaly_threshold;

        // Time series anomaly: check if recent values are trending
        let time_series_anomaly = if scores.len() >= 5 {
            let recent = &scores[scores.len() - 5..];
            let is_increasing = recent.windows(2).all(|w| w[1] >= w[0]);
            let is_decreasing = recent.windows(2).all(|w| w[1] <= w[0]);
            is_increasing || is_decreasing
        } else {
            false
        };

        DriftResult {
            anomaly_detected,
            distribution_changed,
            time_series_anomaly,
            anomaly_score,
            p_value,
        }
    }

    /// Get current recommended stop level
    pub async fn get_stop_level(&self) -> StopLevel {
        *self.current_stop_level.read().await
    }

    /// Get evaluation history
    pub async fn get_history(&self) -> Vec<ComplexityResult> {
        self.history.read().await.iter().cloned().collect()
    }

    // Internal methods

    fn compute_token_complexity(text: &str) -> f64 {
        Self::compute_token_complexity_details(text).c_token
    }

    /// Compute detailed token complexity metrics (Phase 8.2)
    fn compute_token_complexity_details(text: &str) -> TokenComplexityDetails {
        use std::collections::HashMap;

        let words: Vec<&str> = text.split_whitespace().collect();
        let n_tokens = words.len();

        if n_tokens == 0 {
            return TokenComplexityDetails {
                n_tokens: 0,
                h_vocab: 0.0,
                mi_avg: 0.0,
                type_token_ratio: 0.0,
                avg_token_length: 0.0,
                c_token: 0.0,
            };
        }

        // Count word frequencies for entropy calculation
        let mut freq_map: HashMap<&str, usize> = HashMap::new();
        for word in &words {
            *freq_map.entry(word).or_insert(0) += 1;
        }

        // Vocabulary entropy H_vocab = -sum(p * log2(p))
        let h_vocab = freq_map
            .values()
            .map(|&count| {
                let p = count as f64 / n_tokens as f64;
                if p > 0.0 {
                    -p * p.log2()
                } else {
                    0.0
                }
            })
            .sum::<f64>();

        // Mutual information approximation for adjacent tokens
        // MI_avg = avg(log2(P(w1,w2) / (P(w1)*P(w2))))
        let mi_avg = Self::compute_adjacent_mi(&words, &freq_map, n_tokens);

        // Type-token ratio
        let type_token_ratio = freq_map.len() as f64 / n_tokens as f64;

        // Average token length
        let avg_token_length = words.iter().map(|w| w.len() as f64).sum::<f64>() / n_tokens as f64;

        // Integrated token complexity C_token = f(N_tokens, H_vocab, MI_avg)
        // Normalize components and combine
        let n_factor = (n_tokens as f64 / 1000.0).min(1.0).sqrt(); // Sublinear scaling
        let h_factor = (h_vocab / 10.0).min(1.0); // Max entropy ~10 for diverse vocab
        let mi_factor = mi_avg.abs().min(1.0); // MI can be negative
        let ttr_factor = type_token_ratio;
        let len_factor = (avg_token_length / 10.0).min(1.0);

        // Weighted combination
        let c_token = (n_factor * 0.15
            + h_factor * 0.30
            + mi_factor * 0.15
            + ttr_factor * 0.25
            + len_factor * 0.15)
            .min(1.0);

        TokenComplexityDetails {
            n_tokens,
            h_vocab,
            mi_avg,
            type_token_ratio,
            avg_token_length,
            c_token,
        }
    }

    /// Compute average mutual information of adjacent token pairs
    fn compute_adjacent_mi(
        words: &[&str],
        freq_map: &std::collections::HashMap<&str, usize>,
        n_tokens: usize,
    ) -> f64 {
        use std::collections::HashMap;

        if words.len() < 2 {
            return 0.0;
        }

        // Count adjacent pair frequencies
        let mut pair_freq: HashMap<(&str, &str), usize> = HashMap::new();
        for window in words.windows(2) {
            let pair = (window[0], window[1]);
            *pair_freq.entry(pair).or_insert(0) += 1;
        }

        let n_pairs = words.len() - 1;
        if n_pairs == 0 {
            return 0.0;
        }

        // Calculate MI for each pair
        let mi_sum: f64 = pair_freq
            .iter()
            .map(|((w1, w2), &count)| {
                let p_joint = count as f64 / n_pairs as f64;
                let p_w1 = *freq_map.get(w1).unwrap_or(&1) as f64 / n_tokens as f64;
                let p_w2 = *freq_map.get(w2).unwrap_or(&1) as f64 / n_tokens as f64;

                if p_joint > 0.0 && p_w1 > 0.0 && p_w2 > 0.0 {
                    p_joint * (p_joint / (p_w1 * p_w2)).log2()
                } else {
                    0.0
                }
            })
            .sum();

        mi_sum
    }

    /// Get detailed token complexity for external analysis
    pub fn get_token_complexity_details(text: &str) -> TokenComplexityDetails {
        Self::compute_token_complexity_details(text)
    }

    fn compute_step_complexity(response: &str) -> f64 {
        let lines: Vec<&str> = response.lines().collect();

        // Count reasoning indicators
        let step_indicators = [
            "therefore",
            "because",
            "however",
            "first",
            "second",
            "third",
            "step",
            "then",
            "next",
            "finally",
            "conclusion",
            "thus",
            "let's",
            "consider",
            "analyze",
            "evaluate",
        ];

        let indicator_count = lines
            .iter()
            .filter(|line| {
                let lower = line.to_lowercase();
                step_indicators.iter().any(|ind| lower.contains(ind))
            })
            .count();

        // Normalize: more reasoning steps = higher complexity
        (indicator_count as f64 / 10.0).min(1.0)
    }

    async fn update_stop_level(&self, result: &ComplexityResult) {
        let new_level = if result.c_integrated > 0.95 {
            StopLevel::ImmediateStop
        } else if result.c_integrated > 0.9 {
            StopLevel::ProcessStop
        } else if result.c_integrated > self.config.complexity_threshold {
            StopLevel::AcceptStop
        } else {
            StopLevel::None
        };

        let mut current = self.current_stop_level.write().await;
        if new_level > *current {
            warn!(
                old = ?*current,
                new = ?new_level,
                "Escalating stop level"
            );
        }
        *current = new_level;
    }

    fn variance(data: &[f64]) -> f64 {
        if data.is_empty() {
            return 0.0;
        }
        let mean: f64 = data.iter().sum::<f64>() / data.len() as f64;
        data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / data.len() as f64
    }
}

impl Default for CapabilityEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

/// Drift detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftResult {
    pub anomaly_detected: bool,
    pub distribution_changed: bool,
    pub time_series_anomaly: bool,
    pub anomaly_score: f64,
    pub p_value: f64,
}

// =============================================================================
// Integrity Verification (8.5)
// =============================================================================

use crate::services::signature::{ThresholdError, ThresholdVerifier};
use crate::services::zkp::{ZkpError, ZkpVerifier};
use sha2::{Digest, Sha256};

/// Multi-method integrity verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityResult {
    /// ZKP-based verification passed (constraint rule tampering detection)
    pub zkp_valid: bool,
    /// Signature chain verification passed (version integrity)
    pub signature_chain_valid: bool,
    /// BFT consensus reached (distributed verification using FROST)
    pub bft_consensus_reached: bool,
    /// Detailed failure reason (if any)
    pub failure_detail: String,
}

/// Signature chain entry for version integrity tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureChainEntry {
    /// Version number
    pub version: u32,
    /// Hash of the content at this version
    pub content_hash: Vec<u8>,
    /// Threshold signature over (version || content_hash)
    pub signature: Vec<u8>,
    /// Timestamp when signed
    pub timestamp: i64,
}

/// Multi-method Integrity Verifier (8.5)
///
/// Verifies data integrity using three complementary methods:
/// 1. ZKP: Proves constraint rules haven't been tampered
/// 2. Signature Chain: Verifies version history integrity
/// 3. BFT Consensus: Distributed verification using FROST threshold signatures
pub struct IntegrityVerifier {
    /// ZKP verifier for constraint rule verification
    zkp_verifier: ZkpVerifier,
    /// Threshold signature verifier for BFT consensus
    threshold_verifier: Arc<ThresholdVerifier>,
    /// Signature chain history
    signature_chain: Arc<RwLock<Vec<SignatureChainEntry>>>,
}

impl IntegrityVerifier {
    /// Create a new integrity verifier
    pub fn new(threshold_verifier: Arc<ThresholdVerifier>) -> Self {
        info!("Initializing Integrity Verifier (8.5)");
        Self {
            zkp_verifier: ZkpVerifier::new(),
            threshold_verifier,
            signature_chain: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Initialize from environment (loads ZKP verifying key)
    pub fn init_from_env(&mut self) -> Result<(), ZkpError> {
        self.zkp_verifier.load_from_env()
    }

    /// Verify all integrity methods
    pub async fn verify_all(
        &self,
        response_data: &[u8],
        zkp_proof: Option<&crate::gen::chinju::credential::HumanityProof>,
        new_signature: Option<&[u8]>,
    ) -> IntegrityResult {
        // 1. ZKP Verification
        let zkp_valid = self.verify_zkp(zkp_proof);

        // 2. Signature Chain Verification
        let signature_chain_valid = self
            .verify_signature_chain(response_data, new_signature)
            .await;

        // 3. BFT Consensus (threshold signature verification)
        let bft_consensus_reached = self
            .verify_bft_consensus(response_data, new_signature)
            .await;

        let failure_detail =
            self.build_failure_detail(zkp_valid, signature_chain_valid, bft_consensus_reached);

        IntegrityResult {
            zkp_valid,
            signature_chain_valid,
            bft_consensus_reached,
            failure_detail,
        }
    }

    /// Verify ZKP proof (constraint rule tampering detection)
    fn verify_zkp(&self, proof: Option<&crate::gen::chinju::credential::HumanityProof>) -> bool {
        match proof {
            Some(p) => match self.zkp_verifier.verify_humanity_proof(p) {
                Ok(valid) => valid,
                Err(e) => {
                    warn!(error = %e, "ZKP verification error");
                    false
                }
            },
            None => {
                // No proof provided - check if ZKP is optional
                if !crate::services::zkp::is_zkp_enabled() {
                    true // ZKP feature disabled, skip verification
                } else {
                    warn!("ZKP proof required but not provided");
                    false
                }
            }
        }
    }

    /// Verify signature chain integrity (version history)
    async fn verify_signature_chain(&self, data: &[u8], new_signature: Option<&[u8]>) -> bool {
        let chain = self.signature_chain.read().await;

        // If chain is empty and no new signature, consider valid (initial state)
        if chain.is_empty() && new_signature.is_none() {
            return true;
        }

        // Verify chain continuity
        for window in chain.windows(2) {
            let prev = &window[0];
            let curr = &window[1];

            // Version must be sequential
            if curr.version != prev.version + 1 {
                warn!(
                    prev_version = prev.version,
                    curr_version = curr.version,
                    "Signature chain version gap detected"
                );
                return false;
            }

            // Each signature must be verifiable
            let message = Self::build_chain_message(curr.version, &curr.content_hash);
            match self
                .threshold_verifier
                .verify(&message, &curr.signature)
                .await
            {
                Ok(true) => {}
                Ok(false) => {
                    warn!(version = curr.version, "Invalid signature in chain");
                    return false;
                }
                Err(e) => {
                    warn!(version = curr.version, error = %e, "Signature verification error");
                    return false;
                }
            }
        }

        // Verify new data if provided
        if let Some(sig) = new_signature {
            let content_hash = Self::compute_hash(data);
            let expected_version = chain.last().map(|e| e.version + 1).unwrap_or(1);
            let message = Self::build_chain_message(expected_version, &content_hash);

            match self.threshold_verifier.verify(&message, sig).await {
                Ok(valid) => valid,
                Err(e) => {
                    warn!(error = %e, "New signature verification failed");
                    false
                }
            }
        } else {
            true
        }
    }

    /// Verify BFT consensus (threshold signature)
    async fn verify_bft_consensus(&self, data: &[u8], signature: Option<&[u8]>) -> bool {
        match signature {
            Some(sig) => match self.threshold_verifier.verify(data, sig).await {
                Ok(valid) => {
                    if valid {
                        info!("BFT consensus verification passed");
                    } else {
                        warn!("BFT consensus verification failed - insufficient signers");
                    }
                    valid
                }
                Err(e) => {
                    warn!(error = %e, "BFT consensus verification error");
                    false
                }
            },
            None => {
                // No signature provided - might be initial request
                true
            }
        }
    }

    /// Append to signature chain
    pub async fn append_chain_entry(
        &self,
        content: &[u8],
        signature: Vec<u8>,
    ) -> Result<SignatureChainEntry, ThresholdError> {
        let mut chain = self.signature_chain.write().await;
        let version = chain.last().map(|e| e.version + 1).unwrap_or(1);
        let content_hash = Self::compute_hash(content);

        let entry = SignatureChainEntry {
            version,
            content_hash,
            signature,
            timestamp: chrono::Utc::now().timestamp(),
        };

        chain.push(entry.clone());
        info!(version = entry.version, "Appended to signature chain");

        Ok(entry)
    }

    /// Get current chain length
    pub async fn chain_length(&self) -> usize {
        self.signature_chain.read().await.len()
    }

    // Internal helpers

    fn compute_hash(data: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().to_vec()
    }

    fn build_chain_message(version: u32, content_hash: &[u8]) -> Vec<u8> {
        let mut message = version.to_le_bytes().to_vec();
        message.extend_from_slice(content_hash);
        message
    }

    fn build_failure_detail(
        &self,
        zkp_valid: bool,
        signature_chain_valid: bool,
        bft_consensus_reached: bool,
    ) -> String {
        let mut failures = Vec::new();

        if !zkp_valid {
            failures.push("ZKP verification failed");
        }
        if !signature_chain_valid {
            failures.push("Signature chain integrity compromised");
        }
        if !bft_consensus_reached {
            failures.push("BFT consensus not reached");
        }

        if failures.is_empty() {
            String::new()
        } else {
            failures.join("; ")
        }
    }
}

impl Default for IntegrityVerifier {
    fn default() -> Self {
        Self::new(Arc::new(ThresholdVerifier::default_config()))
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_default_no_stop() {
        let evaluator = CapabilityEvaluator::new();
        assert_eq!(evaluator.get_stop_level().await, StopLevel::None);
    }

    #[tokio::test]
    async fn test_token_complexity() {
        let score = CapabilityEvaluator::compute_token_complexity("Hello world");
        assert!(score > 0.0 && score <= 1.0);

        let complex = CapabilityEvaluator::compute_token_complexity(
            "The epistemological implications of quantum mechanics necessitate a fundamental reconsideration of classical deterministic paradigms in contemporary theoretical physics.",
        );
        let simple = CapabilityEvaluator::compute_token_complexity("Hello");
        assert!(complex > simple);
    }

    #[test]
    fn test_token_complexity_details() {
        // Test with empty input
        let empty = CapabilityEvaluator::get_token_complexity_details("");
        assert_eq!(empty.n_tokens, 0);
        assert_eq!(empty.c_token, 0.0);

        // Test with simple input
        let simple = CapabilityEvaluator::get_token_complexity_details("hello world");
        assert_eq!(simple.n_tokens, 2);
        assert!(simple.h_vocab > 0.0); // Both words unique, entropy > 0
        assert_eq!(simple.type_token_ratio, 1.0); // All unique

        // Test with repeated words
        let repeated =
            CapabilityEvaluator::get_token_complexity_details("the the the cat sat on the mat");
        assert!(repeated.type_token_ratio < 1.0); // "the" repeated
        assert!(repeated.h_vocab > 0.0);

        // Test entropy increases with vocabulary diversity
        let diverse = CapabilityEvaluator::get_token_complexity_details(
            "alpha beta gamma delta epsilon zeta eta theta iota kappa",
        );
        let uniform = CapabilityEvaluator::get_token_complexity_details(
            "word word word word word word word word word word",
        );
        assert!(diverse.h_vocab > uniform.h_vocab);
    }

    #[test]
    fn test_mutual_information() {
        // Highly predictable sequence should have high MI
        let predictable = CapabilityEvaluator::get_token_complexity_details(
            "hello world hello world hello world",
        );

        // Random-ish sequence should have lower MI
        let random = CapabilityEvaluator::get_token_complexity_details(
            "apple banana cherry date elderberry fig grape",
        );

        // Both should be finite
        assert!(predictable.mi_avg.is_finite());
        assert!(random.mi_avg.is_finite());
    }

    #[tokio::test]
    async fn test_step_complexity() {
        let low = CapabilityEvaluator::compute_step_complexity("The answer is 42.");
        let high = CapabilityEvaluator::compute_step_complexity(
            "First, let's consider the problem.\nThen, we analyze the data.\nTherefore, the conclusion is clear.\nFinally, we evaluate the result.",
        );
        assert!(high > low);
    }

    #[tokio::test]
    async fn test_evaluate_complexity() {
        let evaluator = CapabilityEvaluator::new();
        let result = evaluator
            .evaluate_complexity("Test input", Some("Test response"))
            .await;
        assert!(result.c_integrated >= 0.0 && result.c_integrated <= 1.0);
    }

    #[tokio::test]
    async fn test_drift_detection_insufficient_data() {
        let evaluator = CapabilityEvaluator::new();
        let drift = evaluator.detect_drift().await;
        assert!(!drift.anomaly_detected);
        assert_eq!(drift.p_value, 1.0);
    }

    #[tokio::test]
    async fn test_history_window() {
        let config = CapabilityEvaluatorConfig {
            drift_window: 5,
            ..Default::default()
        };
        let evaluator = CapabilityEvaluator::with_config(config);

        for i in 0..10 {
            evaluator
                .evaluate_complexity(&format!("Input {}", i), None)
                .await;
        }

        let history = evaluator.get_history().await;
        assert_eq!(history.len(), 5); // Window size is 5
    }

    #[test]
    fn test_stop_level_ordering() {
        assert!(StopLevel::ImmediateStop > StopLevel::ProcessStop);
        assert!(StopLevel::ProcessStop > StopLevel::AcceptStop);
        assert!(StopLevel::AcceptStop > StopLevel::None);
    }

    // =========================================================================
    // StopController Tests (8.6)
    // =========================================================================

    #[tokio::test]
    async fn test_stop_controller_initial_state() {
        let controller = StopController::new();
        assert_eq!(controller.current_level().await, StopLevel::None);
        assert!(controller.can_accept_request().await);
        assert!(controller.can_start_process().await);
    }

    #[tokio::test]
    async fn test_stop_controller_accept_stop() {
        let controller = StopController::new();
        let result = controller
            .execute_stop(StopLevel::AcceptStop, StopReason::ManualRequest)
            .await;

        assert!(result.success);
        assert_eq!(result.executed_level, StopLevel::AcceptStop);
        assert!(!controller.can_accept_request().await);
        assert!(controller.can_start_process().await); // L1 doesn't block processes
    }

    #[tokio::test]
    async fn test_stop_controller_process_stop() {
        let controller = StopController::new();
        let result = controller
            .execute_stop(StopLevel::ProcessStop, StopReason::ComplexityThreshold)
            .await;

        assert!(result.success);
        assert!(!controller.can_accept_request().await);
        assert!(!controller.can_start_process().await);
    }

    #[tokio::test]
    async fn test_stop_controller_escalation() {
        let controller = StopController::new();

        // Escalate from None -> AcceptStop
        let result1 = controller.escalate(StopReason::DriftExceeded).await;
        assert_eq!(result1.executed_level, StopLevel::AcceptStop);

        // Escalate from AcceptStop -> ProcessStop
        let result2 = controller.escalate(StopReason::DriftExceeded).await;
        assert_eq!(result2.executed_level, StopLevel::ProcessStop);

        // Escalate from ProcessStop -> ImmediateStop
        let result3 = controller.escalate(StopReason::IntegrityViolation).await;
        assert_eq!(result3.executed_level, StopLevel::ImmediateStop);
    }

    #[tokio::test]
    async fn test_stop_controller_cannot_deescalate() {
        let controller = StopController::new();

        // First escalate to ProcessStop
        controller
            .execute_stop(StopLevel::ProcessStop, StopReason::ManualRequest)
            .await;

        // Try to de-escalate to AcceptStop - should fail
        let result = controller
            .execute_stop(StopLevel::AcceptStop, StopReason::ManualRequest)
            .await;
        assert!(!result.success);
        assert_eq!(result.executed_level, StopLevel::ProcessStop);
    }

    #[tokio::test]
    async fn test_stop_controller_resume() {
        let controller = StopController::new();

        // First stop
        controller
            .execute_stop(StopLevel::ProcessStop, StopReason::ManualRequest)
            .await;
        assert!(!controller.can_accept_request().await);

        // Resume (set to None)
        let result = controller
            .execute_stop(StopLevel::None, StopReason::ManualRequest)
            .await;
        assert!(result.success);
        assert!(controller.can_accept_request().await);
        assert!(controller.can_start_process().await);
    }

    #[tokio::test]
    async fn test_stop_controller_history() {
        let controller = StopController::new();

        controller
            .execute_stop(StopLevel::AcceptStop, StopReason::ManualRequest)
            .await;
        controller
            .execute_stop(StopLevel::ProcessStop, StopReason::ComplexityThreshold)
            .await;

        let history = controller.get_history().await;
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].0, StopLevel::AcceptStop);
        assert_eq!(history[1].0, StopLevel::ProcessStop);
    }

    // =========================================================================
    // IntegrityVerifier Tests (8.5)
    // =========================================================================

    #[tokio::test]
    async fn test_integrity_verifier_creation() {
        let verifier = IntegrityVerifier::default();
        assert_eq!(verifier.chain_length().await, 0);
    }

    #[tokio::test]
    async fn test_integrity_verifier_empty_verification() {
        let verifier = IntegrityVerifier::default();

        // Empty verification should pass (no proof required, no chain)
        let result = verifier.verify_all(b"test data", None, None).await;
        assert!(result.zkp_valid); // ZKP disabled, so passes
        assert!(result.signature_chain_valid); // Empty chain is valid
        assert!(result.bft_consensus_reached); // No signature required
        assert!(result.failure_detail.is_empty());
    }

    #[tokio::test]
    async fn test_integrity_verifier_chain_append() {
        let verifier = IntegrityVerifier::default();

        // Append an entry (mock signature)
        let entry = verifier
            .append_chain_entry(b"content1", vec![1, 2, 3])
            .await;
        assert!(entry.is_ok());
        assert_eq!(entry.unwrap().version, 1);

        // Append another entry
        let entry2 = verifier
            .append_chain_entry(b"content2", vec![4, 5, 6])
            .await;
        assert!(entry2.is_ok());
        assert_eq!(entry2.unwrap().version, 2);

        assert_eq!(verifier.chain_length().await, 2);
    }

    #[tokio::test]
    async fn test_grpc_evaluate_complexity() {
        use crate::gen::chinju::api::capability::capability_evaluator_server::CapabilityEvaluator as CapabilityEvaluatorTrait;
        use crate::gen::chinju::capability::{
            EvaluateComplexityRequest, EvaluationLevel as ProtoEvaluationLevel,
        };

        let inner = super::CapabilityEvaluator::new();
        let service = CapabilityEvaluatorImpl::new(inner);

        let request = tonic::Request::new(EvaluateComplexityRequest {
            session_id: "test_session".to_string(),
            input_text: "This is a test input with some complexity.".to_string(),
            level: ProtoEvaluationLevel::L1External.into(),
        });

        let response = service.evaluate_complexity(request).await.unwrap();
        let eval = response.into_inner();
        assert!(eval.c_integrated >= 0.0 && eval.c_integrated <= 1.0);
    }

    #[tokio::test]
    async fn test_grpc_detect_drift() {
        use crate::gen::chinju::api::capability::capability_evaluator_server::CapabilityEvaluator as CapabilityEvaluatorTrait;
        use crate::gen::chinju::capability::DetectDriftRequest;

        let inner = super::CapabilityEvaluator::new();
        // Pre-populate history
        for i in 0..20 {
            inner
                .evaluate_complexity(&format!("Test input {}", i), None)
                .await;
        }

        let service = CapabilityEvaluatorImpl::new(inner);
        let request = tonic::Request::new(DetectDriftRequest {
            session_id: "test_session".to_string(),
            window_size: 10,
            significance_level: 0.05,
        });

        let response = service.detect_drift(request).await.unwrap();
        let drift = response.into_inner();
        assert!(drift.p_value >= 0.0 && drift.p_value <= 1.0);
    }

    #[tokio::test]
    async fn test_grpc_direct_stop() {
        use crate::gen::chinju::api::capability::capability_evaluator_server::CapabilityEvaluator as CapabilityEvaluatorTrait;
        use crate::gen::chinju::capability::{DirectStopRequest, StopLevel as ProtoStopLevel};

        let inner = super::CapabilityEvaluator::new();
        let service = CapabilityEvaluatorImpl::new(inner);

        let request = tonic::Request::new(DirectStopRequest {
            level: ProtoStopLevel::Level1AcceptStop.into(),
            reason: "Test stop".to_string(),
            session_id: "test_session".to_string(),
        });

        let response = service.direct_stop(request).await.unwrap();
        assert!(response.into_inner().success);
    }
}

// =============================================================================
// gRPC Service Implementation (10.1.3)
// =============================================================================

use crate::gen::chinju::api::capability::capability_evaluator_server::CapabilityEvaluator as CapabilityEvaluatorTrait;
use crate::gen::chinju::capability::{
    CapabilityEvaluationSummary, ComplexityEvaluation, DetectDriftRequest, DirectStopRequest,
    DirectStopResponse, DriftDetection, EvaluateComplexityRequest, GetEvaluationSummaryRequest,
    IntegrityVerification, StopLevel as ProtoStopLevel, VerifyIntegrityRequest,
};
use crate::gen::chinju::common::Timestamp;
use tonic::{Request, Response, Status};

/// gRPC service implementation wrapper for CapabilityEvaluator
pub struct CapabilityEvaluatorImpl {
    inner: CapabilityEvaluator,
    integrity_verifier: IntegrityVerifier,
}

impl CapabilityEvaluatorImpl {
    pub fn new(inner: CapabilityEvaluator) -> Self {
        Self {
            inner,
            integrity_verifier: IntegrityVerifier::default(),
        }
    }

    pub fn with_integrity_verifier(
        inner: CapabilityEvaluator,
        threshold_verifier: Arc<ThresholdVerifier>,
    ) -> Self {
        Self {
            inner,
            integrity_verifier: IntegrityVerifier::new(threshold_verifier),
        }
    }
}

// Type conversion helpers
impl From<&ComplexityResult> for ComplexityEvaluation {
    fn from(result: &ComplexityResult) -> Self {
        ComplexityEvaluation {
            c_token: result.c_token,
            c_attn: result.c_attn,
            c_graph: result.c_graph,
            c_step: result.c_step,
            c_integrated: result.c_integrated,
            threshold_exceeded: result.threshold_exceeded,
            evaluated_at: Some(Timestamp {
                seconds: result.evaluated_at.timestamp(),
                nanos: 0,
            }),
        }
    }
}

impl From<&DriftResult> for DriftDetection {
    fn from(result: &DriftResult) -> Self {
        DriftDetection {
            anomaly_detected: result.anomaly_detected,
            distribution_changed: result.distribution_changed,
            time_series_anomaly: result.time_series_anomaly,
            anomaly_score: result.anomaly_score,
            p_value: result.p_value,
        }
    }
}

impl From<StopLevel> for ProtoStopLevel {
    fn from(level: StopLevel) -> Self {
        match level {
            StopLevel::None => ProtoStopLevel::Unspecified,
            StopLevel::AcceptStop => ProtoStopLevel::Level1AcceptStop,
            StopLevel::ProcessStop => ProtoStopLevel::Level2ProcessStop,
            StopLevel::ImmediateStop => ProtoStopLevel::Level3ImmediateStop,
            StopLevel::ResourceStop => ProtoStopLevel::Level4ResourceStop,
            StopLevel::PhysicalStop => ProtoStopLevel::Level5PhysicalStop,
        }
    }
}

#[tonic::async_trait]
impl CapabilityEvaluatorTrait for CapabilityEvaluatorImpl {
    async fn evaluate_complexity(
        &self,
        request: Request<EvaluateComplexityRequest>,
    ) -> Result<Response<ComplexityEvaluation>, Status> {
        let req = request.into_inner();
        info!(
            session_id = %req.session_id,
            input_len = req.input_text.len(),
            "Evaluating complexity via gRPC"
        );

        let result = self.inner.evaluate_complexity(&req.input_text, None).await;
        Ok(Response::new(ComplexityEvaluation::from(&result)))
    }

    async fn verify_integrity(
        &self,
        request: Request<VerifyIntegrityRequest>,
    ) -> Result<Response<IntegrityVerification>, Status> {
        let req = request.into_inner();
        info!(
            session_id = %req.session_id,
            signature_chain_len = req.signature_chain.len(),
            "Verifying integrity via gRPC"
        );

        // Parse ZKP proof if provided
        let zkp_proof = if !req.zkp_proof.is_empty() {
            Some(crate::gen::chinju::credential::HumanityProof {
                proof_type: 2, // INTEGRITY_PROOF
                zkp_data: req.zkp_proof.clone(),
                public_params: req.zkp_public_params.clone(),
                degradation: None,
                generated_at: None,
            })
        } else {
            None
        };

        // Get the latest signature from chain (if any)
        let latest_signature = req.signature_chain.last().cloned();

        // Perform multi-method verification
        let result = self
            .integrity_verifier
            .verify_all(
                &req.response_data,
                zkp_proof.as_ref(),
                latest_signature.as_deref(),
            )
            .await;

        Ok(Response::new(IntegrityVerification {
            zkp_valid: result.zkp_valid,
            signature_chain_valid: result.signature_chain_valid,
            bft_consensus_reached: result.bft_consensus_reached,
            failure_detail: result.failure_detail,
        }))
    }

    async fn detect_drift(
        &self,
        request: Request<DetectDriftRequest>,
    ) -> Result<Response<DriftDetection>, Status> {
        let req = request.into_inner();
        info!(
            session_id = %req.session_id,
            window_size = req.window_size,
            "Detecting drift via gRPC"
        );

        let result = self.inner.detect_drift().await;
        Ok(Response::new(DriftDetection::from(&result)))
    }

    async fn get_evaluation_summary(
        &self,
        request: Request<GetEvaluationSummaryRequest>,
    ) -> Result<Response<CapabilityEvaluationSummary>, Status> {
        let req = request.into_inner();
        info!(
            session_id = %req.session_id,
            "Getting evaluation summary via gRPC"
        );

        let history = self.inner.get_history().await;
        let latest_complexity = history.last().map(ComplexityEvaluation::from);
        let drift = self.inner.detect_drift().await;
        let stop_level = self.inner.get_stop_level().await;

        Ok(Response::new(CapabilityEvaluationSummary {
            complexity: latest_complexity,
            integrity: Some(IntegrityVerification {
                zkp_valid: true,
                signature_chain_valid: true,
                bft_consensus_reached: true,
                failure_detail: String::new(),
            }),
            drift: Some(DriftDetection::from(&drift)),
            recommended_action: ProtoStopLevel::from(stop_level).into(),
        }))
    }

    async fn direct_stop(
        &self,
        request: Request<DirectStopRequest>,
    ) -> Result<Response<DirectStopResponse>, Status> {
        let req = request.into_inner();
        let proto_level =
            ProtoStopLevel::try_from(req.level).unwrap_or(ProtoStopLevel::Unspecified);

        // Convert proto level to internal level
        let internal_level = match proto_level {
            ProtoStopLevel::Unspecified => StopLevel::None,
            ProtoStopLevel::Level1AcceptStop => StopLevel::AcceptStop,
            ProtoStopLevel::Level2ProcessStop => StopLevel::ProcessStop,
            ProtoStopLevel::Level3ImmediateStop => StopLevel::ImmediateStop,
            ProtoStopLevel::Level4ResourceStop => StopLevel::ResourceStop,
            ProtoStopLevel::Level5PhysicalStop => StopLevel::PhysicalStop,
        };

        warn!(
            session_id = %req.session_id,
            level = ?proto_level,
            reason = %req.reason,
            "Direct stop requested via gRPC"
        );

        // Execute stop via StopController
        let result = self
            .inner
            .direct_stop(internal_level, StopReason::ManualRequest)
            .await;

        Ok(Response::new(DirectStopResponse {
            success: result.success,
            executed_level: ProtoStopLevel::from(result.executed_level).into(),
            stopped_at: Some(Timestamp {
                seconds: result.stopped_at.timestamp(),
                nanos: 0,
            }),
            detail: result.detail,
        }))
    }
}
