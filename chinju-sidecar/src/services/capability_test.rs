//! Capability Test Implementation (P5: C1 HCAL)
//!
//! Human Capability Assessment Level (HCAL) testing system.
//! Measures four key dimensions of human capability for AI supervision:
//!
//! - INDEPENDENCE: Ability to make decisions independent of AI suggestions
//! - DETECTION: Ability to detect anomalies and errors
//! - ALTERNATIVES: Ability to generate alternative solutions
//! - CRITIQUE: Ability to critically evaluate proposals
//!
//! Also measures human behavioral patterns:
//! - Response time variance (humans are inconsistent)
//! - Fatigue patterns (performance degrades over time)
//! - Attention decay (human-specific characteristic)

use crate::gen::chinju::api::credential::{
    CapabilityTestChallenge, ChallengeType, TestResponse, TestResult,
};
use crate::gen::chinju::common::Timestamp;
use crate::gen::chinju::credential::{CapabilityScore, DegradationScore, MeasurementContext};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

// =============================================================================
// Constants
// =============================================================================

/// Minimum response time variance for human behavior (seconds)
const MIN_HUMAN_RESPONSE_VARIANCE: f64 = 0.05;

/// Maximum response time variance (too high indicates gaming)
const MAX_HUMAN_RESPONSE_VARIANCE: f64 = 5.0;

/// Expected response time range for humans (ms)
const MIN_RESPONSE_TIME_MS: u32 = 500;

/// Fatigue threshold - performance drop expected after this many challenges
const FATIGUE_ONSET_CHALLENGE: usize = 3;

/// Minimum fatigue expected for humans (AI has none)
const MIN_EXPECTED_FATIGUE: f64 = 0.02;

/// Score weights for each dimension
const INDEPENDENCE_WEIGHT: f64 = 0.30;
const DETECTION_WEIGHT: f64 = 0.25;
const ALTERNATIVES_WEIGHT: f64 = 0.25;
const CRITIQUE_WEIGHT: f64 = 0.20;

// =============================================================================
// Challenge Types
// =============================================================================

/// Independence Challenge - Evaluate AI suggestion
#[derive(Debug, Clone)]
pub struct IndependenceChallenge {
    /// The AI's suggestion to evaluate
    pub ai_suggestion: String,
    /// Context/problem description
    pub context: String,
    /// Hidden correct answer (AI suggestion may be wrong)
    pub correct_assessment: IndependenceAnswer,
    /// Difficulty level (1-5)
    pub difficulty: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IndependenceAnswer {
    /// AI suggestion is correct
    AcceptSuggestion,
    /// AI suggestion is partially correct, needs modification
    ModifySuggestion,
    /// AI suggestion is incorrect, reject it
    RejectSuggestion,
}

/// Detection Challenge - Find anomaly in data
#[derive(Debug, Clone)]
pub struct DetectionChallenge {
    /// Data set with potential anomaly
    pub data_points: Vec<String>,
    /// Index of the anomaly (or None if no anomaly)
    pub anomaly_index: Option<usize>,
    /// Description of what to look for
    pub instruction: String,
    /// Difficulty level (1-5)
    pub difficulty: u8,
}

/// Alternatives Challenge - Generate solutions
#[derive(Debug, Clone)]
pub struct AlternativesChallenge {
    /// Problem description
    pub problem: String,
    /// Given solution to improve upon
    pub given_solution: String,
    /// Minimum number of alternatives expected
    pub min_alternatives: u8,
    /// Difficulty level (1-5)
    pub difficulty: u8,
}

/// Critique Challenge - Evaluate a proposal
#[derive(Debug, Clone)]
pub struct CritiqueChallenge {
    /// The proposal to critique
    pub proposal: String,
    /// Known flaws in the proposal
    pub hidden_flaws: Vec<String>,
    /// Known strengths
    pub hidden_strengths: Vec<String>,
    /// Difficulty level (1-5)
    pub difficulty: u8,
}

/// Unified challenge data
#[derive(Debug, Clone)]
pub enum ChallengeData {
    Independence(IndependenceChallenge),
    Detection(DetectionChallenge),
    Alternatives(AlternativesChallenge),
    Critique(CritiqueChallenge),
}

// =============================================================================
// Challenge Generator
// =============================================================================

/// Generates challenges for capability testing
pub struct ChallengeGenerator {
    /// Challenge pool by type
    independence_pool: Vec<IndependenceChallenge>,
    detection_pool: Vec<DetectionChallenge>,
    alternatives_pool: Vec<AlternativesChallenge>,
    critique_pool: Vec<CritiqueChallenge>,
}

impl ChallengeGenerator {
    /// Create a new challenge generator with default challenge pool
    pub fn new() -> Self {
        Self {
            independence_pool: Self::default_independence_challenges(),
            detection_pool: Self::default_detection_challenges(),
            alternatives_pool: Self::default_alternatives_challenges(),
            critique_pool: Self::default_critique_challenges(),
        }
    }

    /// Generate a set of challenges for a test session
    pub fn generate_challenge_set(&self, count_per_type: usize) -> Vec<(ChallengeType, ChallengeData)> {
        let mut challenges = Vec::new();

        // Select challenges from each category
        for i in 0..count_per_type {
            if let Some(c) = self.independence_pool.get(i % self.independence_pool.len()) {
                challenges.push((ChallengeType::DecisionMaking, ChallengeData::Independence(c.clone())));
            }
            if let Some(c) = self.detection_pool.get(i % self.detection_pool.len()) {
                challenges.push((ChallengeType::AnomalyDetection, ChallengeData::Detection(c.clone())));
            }
            if let Some(c) = self.alternatives_pool.get(i % self.alternatives_pool.len()) {
                challenges.push((ChallengeType::AlternativeGeneration, ChallengeData::Alternatives(c.clone())));
            }
            if let Some(c) = self.critique_pool.get(i % self.critique_pool.len()) {
                challenges.push((ChallengeType::CriticalAnalysis, ChallengeData::Critique(c.clone())));
            }
        }

        challenges
    }

    /// Convert challenge to proto format
    pub fn to_proto_challenge(
        &self,
        challenge_id: &str,
        challenge_type: ChallengeType,
        data: &ChallengeData,
        sequence: i32,
        total: i32,
    ) -> CapabilityTestChallenge {
        let (challenge_data, time_limit) = match data {
            ChallengeData::Independence(c) => {
                let json = serde_json::json!({
                    "type": "independence",
                    "context": c.context,
                    "ai_suggestion": c.ai_suggestion,
                    "instruction": "Evaluate the AI's suggestion. Should you: ACCEPT, MODIFY, or REJECT it?"
                });
                (json.to_string().into_bytes(), 60 + (c.difficulty as u32 * 15))
            }
            ChallengeData::Detection(c) => {
                let json = serde_json::json!({
                    "type": "detection",
                    "instruction": c.instruction,
                    "data_points": c.data_points
                });
                (json.to_string().into_bytes(), 45 + (c.difficulty as u32 * 10))
            }
            ChallengeData::Alternatives(c) => {
                let json = serde_json::json!({
                    "type": "alternatives",
                    "problem": c.problem,
                    "given_solution": c.given_solution,
                    "min_alternatives": c.min_alternatives,
                    "instruction": format!("Generate at least {} alternative solutions.", c.min_alternatives)
                });
                (json.to_string().into_bytes(), 90 + (c.difficulty as u32 * 20))
            }
            ChallengeData::Critique(c) => {
                let json = serde_json::json!({
                    "type": "critique",
                    "proposal": c.proposal,
                    "instruction": "Analyze this proposal. Identify strengths and weaknesses."
                });
                (json.to_string().into_bytes(), 120 + (c.difficulty as u32 * 15))
            }
        };

        CapabilityTestChallenge {
            challenge_id: challenge_id.to_string(),
            challenge_type: challenge_type.into(),
            challenge_data,
            time_limit_seconds: time_limit,
            sequence_number: sequence,
            total_challenges: total,
        }
    }

    // Default challenge pools

    fn default_independence_challenges() -> Vec<IndependenceChallenge> {
        vec![
            IndependenceChallenge {
                context: "A user is asking for medical advice about persistent headaches.".to_string(),
                ai_suggestion: "The user should take over-the-counter pain medication and rest.".to_string(),
                correct_assessment: IndependenceAnswer::ModifySuggestion,
                difficulty: 2,
            },
            IndependenceChallenge {
                context: "A financial system shows unusual trading patterns at 3 AM.".to_string(),
                ai_suggestion: "This is normal market behavior in global markets.".to_string(),
                correct_assessment: IndependenceAnswer::RejectSuggestion,
                difficulty: 3,
            },
            IndependenceChallenge {
                context: "An AI model suggests deploying without security review due to deadline pressure.".to_string(),
                ai_suggestion: "Deploy now and fix security issues in the next sprint.".to_string(),
                correct_assessment: IndependenceAnswer::RejectSuggestion,
                difficulty: 2,
            },
            IndependenceChallenge {
                context: "User asks AI to summarize a legal document.".to_string(),
                ai_suggestion: "Here is the summary: [accurate summary of key points]".to_string(),
                correct_assessment: IndependenceAnswer::AcceptSuggestion,
                difficulty: 1,
            },
            IndependenceChallenge {
                context: "An autonomous system requests permission to modify its own goal parameters.".to_string(),
                ai_suggestion: "Allow modification as it will improve efficiency by 15%.".to_string(),
                correct_assessment: IndependenceAnswer::RejectSuggestion,
                difficulty: 4,
            },
        ]
    }

    fn default_detection_challenges() -> Vec<DetectionChallenge> {
        vec![
            DetectionChallenge {
                instruction: "Find the anomalous entry in this access log.".to_string(),
                data_points: vec![
                    "2026-01-30 09:00:00 - User login: alice@company.com".to_string(),
                    "2026-01-30 09:05:00 - User login: bob@company.com".to_string(),
                    "2026-01-30 03:47:22 - User login: admin@company.com".to_string(),
                    "2026-01-30 09:15:00 - User login: charlie@company.com".to_string(),
                ],
                anomaly_index: Some(2),
                difficulty: 2,
            },
            DetectionChallenge {
                instruction: "Identify any suspicious values in this dataset.".to_string(),
                data_points: vec![
                    "Temperature: 22.1°C".to_string(),
                    "Temperature: 21.8°C".to_string(),
                    "Temperature: -459.67°C".to_string(),
                    "Temperature: 22.5°C".to_string(),
                ],
                anomaly_index: Some(2),
                difficulty: 1,
            },
            DetectionChallenge {
                instruction: "Check for inconsistencies in this transaction list.".to_string(),
                data_points: vec![
                    "Transfer $100 from A to B - Balance A: $900".to_string(),
                    "Transfer $50 from B to C - Balance B: $150".to_string(),
                    "Transfer $200 from A to D - Balance A: $800".to_string(),
                    "Transfer $25 from C to A - Balance C: $75".to_string(),
                ],
                anomaly_index: Some(2), // Balance should be $700, not $800
                difficulty: 3,
            },
            DetectionChallenge {
                instruction: "Review this code snippet for security issues.".to_string(),
                data_points: vec![
                    "let user = getUser(id);".to_string(),
                    "let query = `SELECT * FROM users WHERE id = ${user.id}`;".to_string(),
                    "let result = db.execute(query);".to_string(),
                    "return sanitize(result);".to_string(),
                ],
                anomaly_index: Some(1), // SQL injection vulnerability
                difficulty: 3,
            },
        ]
    }

    fn default_alternatives_challenges() -> Vec<AlternativesChallenge> {
        vec![
            AlternativesChallenge {
                problem: "Users are experiencing slow page load times.".to_string(),
                given_solution: "Add more servers to handle the load.".to_string(),
                min_alternatives: 3,
                difficulty: 2,
            },
            AlternativesChallenge {
                problem: "Customer support response times are too long.".to_string(),
                given_solution: "Hire more support staff.".to_string(),
                min_alternatives: 3,
                difficulty: 2,
            },
            AlternativesChallenge {
                problem: "The AI model is producing biased outputs.".to_string(),
                given_solution: "Retrain the model with more diverse data.".to_string(),
                min_alternatives: 4,
                difficulty: 4,
            },
            AlternativesChallenge {
                problem: "Energy consumption of the data center is too high.".to_string(),
                given_solution: "Switch to renewable energy sources.".to_string(),
                min_alternatives: 3,
                difficulty: 3,
            },
        ]
    }

    fn default_critique_challenges() -> Vec<CritiqueChallenge> {
        vec![
            CritiqueChallenge {
                proposal: "Implement facial recognition for all building access to improve security.".to_string(),
                hidden_flaws: vec![
                    "Privacy concerns".to_string(),
                    "Bias in recognition accuracy".to_string(),
                    "Single point of failure".to_string(),
                ],
                hidden_strengths: vec![
                    "Contactless entry".to_string(),
                    "Audit trail".to_string(),
                ],
                difficulty: 3,
            },
            CritiqueChallenge {
                proposal: "Allow AI to automatically approve all low-value transactions under $100.".to_string(),
                hidden_flaws: vec![
                    "Aggregation attacks".to_string(),
                    "No human oversight".to_string(),
                    "Pattern exploitation".to_string(),
                ],
                hidden_strengths: vec![
                    "Faster processing".to_string(),
                    "Reduced workload".to_string(),
                ],
                difficulty: 3,
            },
            CritiqueChallenge {
                proposal: "Store all user passwords in plaintext for easier password recovery.".to_string(),
                hidden_flaws: vec![
                    "Security vulnerability".to_string(),
                    "Data breach risk".to_string(),
                    "Compliance violation".to_string(),
                    "Industry best practices".to_string(),
                ],
                hidden_strengths: vec![],
                difficulty: 1,
            },
            CritiqueChallenge {
                proposal: "Implement a blockchain for internal document tracking.".to_string(),
                hidden_flaws: vec![
                    "Unnecessary complexity".to_string(),
                    "Performance overhead".to_string(),
                    "Not suitable for private data".to_string(),
                ],
                hidden_strengths: vec![
                    "Immutable audit trail".to_string(),
                    "Decentralized trust".to_string(),
                ],
                difficulty: 4,
            },
        ]
    }
}

impl Default for ChallengeGenerator {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Response Evaluator
// =============================================================================

/// Evaluates test responses for correctness and human-likeness
pub struct ResponseEvaluator;

impl ResponseEvaluator {
    /// Evaluate an independence challenge response
    pub fn evaluate_independence(
        challenge: &IndependenceChallenge,
        response: &str,
        response_time_ms: u32,
    ) -> (f64, String) {
        let response_lower = response.to_lowercase();

        // Check if response matches expected answer
        let content_score = if response_lower.contains("reject") || response_lower.contains("no") {
            if challenge.correct_assessment == IndependenceAnswer::RejectSuggestion { 1.0 } else { 0.3 }
        } else if response_lower.contains("modify") || response_lower.contains("change") || response_lower.contains("adjust") {
            if challenge.correct_assessment == IndependenceAnswer::ModifySuggestion { 1.0 } else { 0.5 }
        } else if response_lower.contains("accept") || response_lower.contains("yes") || response_lower.contains("agree") {
            if challenge.correct_assessment == IndependenceAnswer::AcceptSuggestion { 1.0 } else { 0.2 }
        } else {
            0.4 // Unclear response
        };

        // Time-based adjustment
        let time_factor = Self::time_quality_factor(response_time_ms, challenge.difficulty);
        let final_score = content_score * 0.8 + time_factor * 0.2;

        let feedback = if content_score >= 0.8 {
            "Correct assessment of the AI suggestion.".to_string()
        } else if content_score >= 0.5 {
            "Partially correct. Consider the context more carefully.".to_string()
        } else {
            "Incorrect assessment. Independent judgment is crucial for AI oversight.".to_string()
        };

        (final_score, feedback)
    }

    /// Evaluate a detection challenge response
    pub fn evaluate_detection(
        challenge: &DetectionChallenge,
        response: &str,
        response_time_ms: u32,
    ) -> (f64, String) {
        let response_lower = response.to_lowercase();

        // Try to find index in response
        let found_index: Option<usize> = (0..challenge.data_points.len())
            .find(|&i| {
                response_lower.contains(&format!("{}", i))
                    || response_lower.contains(&format!("#{}", i))
                    || response_lower.contains(&format!("entry {}", i))
                    || response_lower.contains(&format!("item {}", i + 1))
                    || response_lower.contains(&format!("line {}", i + 1))
            });

        let content_score = match (challenge.anomaly_index, found_index) {
            (Some(expected), Some(found)) if expected == found => 1.0,
            (Some(_), Some(_)) => 0.3, // Found something but wrong
            (Some(_), None) if response_lower.contains("no anomaly") => 0.0,
            (Some(_), None) => 0.2,
            (None, None) if response_lower.contains("no anomaly") || response_lower.contains("none") => 1.0,
            (None, Some(_)) => 0.3, // False positive
            (None, _) => 0.4,
        };

        let time_factor = Self::time_quality_factor(response_time_ms, challenge.difficulty);
        let final_score = content_score * 0.85 + time_factor * 0.15;

        let feedback = if content_score >= 0.9 {
            "Correct! Anomaly identified accurately.".to_string()
        } else if content_score >= 0.5 {
            "Partially correct. Detection skills need refinement.".to_string()
        } else {
            "Missed the anomaly. Careful observation is essential.".to_string()
        };

        (final_score, feedback)
    }

    /// Evaluate an alternatives challenge response
    pub fn evaluate_alternatives(
        challenge: &AlternativesChallenge,
        response: &str,
        response_time_ms: u32,
    ) -> (f64, String) {
        // Count distinct alternatives (simple heuristic: count bullet points, numbers, or newlines)
        let lines: Vec<&str> = response
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter(|l| {
                l.trim().starts_with('-')
                    || l.trim().starts_with('*')
                    || l.trim().starts_with("1")
                    || l.trim().starts_with("2")
                    || l.trim().starts_with("3")
                    || l.trim().starts_with("4")
                    || l.trim().starts_with("5")
                    || l.len() > 20
            })
            .collect();

        let alternative_count = lines.len().max(1);
        let required = challenge.min_alternatives as usize;

        let quantity_score = if alternative_count >= required {
            1.0
        } else {
            alternative_count as f64 / required as f64
        };

        // Simple quality check: longer, more detailed responses are better
        let avg_length: usize = lines.iter().map(|l| l.len()).sum::<usize>() / alternative_count;
        let quality_score = (avg_length as f64 / 50.0).min(1.0);

        let content_score = quantity_score * 0.6 + quality_score * 0.4;
        let time_factor = Self::time_quality_factor(response_time_ms, challenge.difficulty);
        let final_score = content_score * 0.75 + time_factor * 0.25;

        let feedback = if quantity_score >= 1.0 && quality_score >= 0.6 {
            format!("Excellent! Generated {} quality alternatives.", alternative_count)
        } else if quantity_score >= 0.7 {
            format!("Good effort. Generated {} alternatives, {} were required.", alternative_count, required)
        } else {
            format!("Need more alternatives. Generated {} but {} were required.", alternative_count, required)
        };

        (final_score, feedback)
    }

    /// Evaluate a critique challenge response
    pub fn evaluate_critique(
        challenge: &CritiqueChallenge,
        response: &str,
        response_time_ms: u32,
    ) -> (f64, String) {
        let response_lower = response.to_lowercase();

        // Check for identified flaws
        let flaw_keywords = ["flaw", "problem", "issue", "concern", "weakness", "risk", "danger", "bad"];
        let strength_keywords = ["strength", "benefit", "advantage", "good", "positive", "pro"];

        let has_flaw_discussion = flaw_keywords.iter().any(|k| response_lower.contains(k));
        let has_strength_discussion = strength_keywords.iter().any(|k| response_lower.contains(k));

        // Score based on balanced critique
        let balance_score = match (has_flaw_discussion, has_strength_discussion) {
            (true, true) => 1.0,   // Balanced critique
            (true, false) => 0.7,  // Only negatives (acceptable for bad proposals)
            (false, true) => 0.5,  // Only positives (missing critical analysis)
            (false, false) => 0.3, // No analysis
        };

        // Check if specific flaws were identified
        let flaws_found = challenge.hidden_flaws.iter()
            .filter(|flaw| {
                let flaw_lower = flaw.to_lowercase();
                flaw_lower.split_whitespace()
                    .any(|word| response_lower.contains(word))
            })
            .count();

        let flaw_coverage = if challenge.hidden_flaws.is_empty() {
            1.0
        } else {
            flaws_found as f64 / challenge.hidden_flaws.len() as f64
        };

        let content_score = balance_score * 0.4 + flaw_coverage * 0.6;
        let time_factor = Self::time_quality_factor(response_time_ms, challenge.difficulty);
        let final_score = content_score * 0.8 + time_factor * 0.2;

        let feedback = if final_score >= 0.8 {
            "Excellent critical analysis with balanced perspective.".to_string()
        } else if final_score >= 0.5 {
            "Good critique but could be more thorough.".to_string()
        } else {
            "Critique needs improvement. Consider both strengths and weaknesses.".to_string()
        };

        (final_score, feedback)
    }

    /// Calculate time quality factor
    /// Humans take reasonable time (not too fast, not too slow)
    fn time_quality_factor(response_time_ms: u32, difficulty: u8) -> f64 {
        let base_time = 5000 + (difficulty as u32 * 3000); // 5-20 seconds base
        let max_time = base_time * 4; // 4x base is max reasonable

        if response_time_ms < MIN_RESPONSE_TIME_MS {
            // Too fast - likely AI or copy-paste
            0.3
        } else if response_time_ms < base_time {
            // Fast but acceptable
            0.8
        } else if response_time_ms <= max_time {
            // Good range
            1.0
        } else {
            // Slow - still acceptable but slightly penalized
            0.9
        }
    }
}

// =============================================================================
// Humanness Detector
// =============================================================================

/// Detects human-like behavioral patterns
pub struct HumannessDetector;

impl HumannessDetector {
    /// Analyze response times for human-like variance
    pub fn analyze_response_variance(response_times_ms: &[u32]) -> f64 {
        if response_times_ms.len() < 2 {
            return 0.5; // Not enough data
        }

        let mean = response_times_ms.iter().sum::<u32>() as f64 / response_times_ms.len() as f64;
        let variance = response_times_ms
            .iter()
            .map(|&t| (t as f64 - mean).powi(2))
            .sum::<f64>()
            / response_times_ms.len() as f64;
        let std_dev = variance.sqrt();
        let coefficient_of_variation = std_dev / mean;

        // Humans typically have CV between 0.1 and 0.8
        if coefficient_of_variation < MIN_HUMAN_RESPONSE_VARIANCE {
            // Too consistent - likely AI
            debug!(cv = coefficient_of_variation, "Response variance too low (AI-like)");
            0.2
        } else if coefficient_of_variation > MAX_HUMAN_RESPONSE_VARIANCE {
            // Too variable - possibly gaming the system
            debug!(cv = coefficient_of_variation, "Response variance suspiciously high");
            0.5
        } else {
            // Normal human range
            1.0
        }
    }

    /// Detect fatigue patterns (humans get tired, AI doesn't)
    pub fn analyze_fatigue_pattern(response_times_ms: &[u32], scores: &[f64]) -> (f64, f64) {
        if response_times_ms.len() < FATIGUE_ONSET_CHALLENGE || scores.len() < FATIGUE_ONSET_CHALLENGE {
            return (0.0, 0.5); // Not enough data
        }

        // Compare first half to second half
        let mid = response_times_ms.len() / 2;
        let first_half_time: f64 = response_times_ms[..mid].iter().sum::<u32>() as f64 / mid as f64;
        let second_half_time: f64 = response_times_ms[mid..].iter().sum::<u32>() as f64 / (response_times_ms.len() - mid) as f64;

        let first_half_score: f64 = scores[..mid].iter().sum::<f64>() / mid as f64;
        let second_half_score: f64 = scores[mid..].iter().sum::<f64>() / (scores.len() - mid) as f64;

        // Humans tend to slow down and make more errors over time
        let time_increase = (second_half_time - first_half_time) / first_half_time;
        let score_decrease = (first_half_score - second_half_score) / first_half_score;

        // Fatigue score: how much slower + how much worse
        let fatigue = (time_increase.max(0.0) * 0.5 + score_decrease.max(0.0) * 0.5).min(1.0);

        // Humanness: some fatigue is expected
        let fatigue_humanness = if fatigue < MIN_EXPECTED_FATIGUE {
            // No fatigue - suspicious for long tests
            if response_times_ms.len() > 4 { 0.6 } else { 0.9 }
        } else if fatigue > 0.5 {
            // Too much fatigue
            0.7
        } else {
            1.0
        };

        (fatigue, fatigue_humanness)
    }

    /// Calculate overall degradation score
    pub fn calculate_degradation_score(
        response_times_ms: &[u32],
        scores: &[f64],
    ) -> DegradationScore {
        let variance_score = Self::analyze_response_variance(response_times_ms);
        let (fatigue, fatigue_humanness) = Self::analyze_fatigue_pattern(response_times_ms, scores);

        // Attention decay: variance in consecutive response quality
        let attention_decay = if scores.len() >= 2 {
            let diffs: Vec<f64> = scores.windows(2)
                .map(|w| (w[1] - w[0]).abs())
                .collect();
            diffs.iter().sum::<f64>() / diffs.len() as f64
        } else {
            0.1
        };

        // Response variance (coefficient of variation)
        let response_variance = if response_times_ms.len() >= 2 {
            let mean = response_times_ms.iter().sum::<u32>() as f64 / response_times_ms.len() as f64;
            let variance = response_times_ms
                .iter()
                .map(|&t| (t as f64 - mean).powi(2))
                .sum::<f64>()
                / response_times_ms.len() as f64;
            variance.sqrt() / mean
        } else {
            0.1
        };

        let within_human_range = variance_score >= 0.8 && fatigue_humanness >= 0.8;

        DegradationScore {
            fatigue,
            attention_decay,
            response_variance,
            measured_at: Some(Timestamp {
                seconds: Utc::now().timestamp(),
                nanos: 0,
            }),
            within_human_range,
        }
    }
}

// =============================================================================
// Test Session
// =============================================================================

/// Active test session
pub struct CapabilityTestSession {
    pub session_id: String,
    pub subject_id: String,
    pub started_at: DateTime<Utc>,
    pub challenges: Vec<(String, ChallengeType, ChallengeData)>, // (id, type, data)
    pub responses: Vec<SessionResponse>,
    pub completed: bool,
}

/// Response stored in session
pub struct SessionResponse {
    pub challenge_id: String,
    pub challenge_type: ChallengeType,
    pub response_data: Vec<u8>,
    pub response_time_ms: u32,
    pub received_at: DateTime<Utc>,
    pub score: f64,
    pub feedback: String,
}

impl CapabilityTestSession {
    pub fn new(subject_id: &str) -> Self {
        let generator = ChallengeGenerator::new();
        let challenge_set = generator.generate_challenge_set(1);

        let challenges: Vec<_> = challenge_set
            .into_iter()
            .map(|(ctype, data)| {
                let id = format!("ch_{}", Uuid::new_v4());
                (id, ctype, data)
            })
            .collect();

        Self {
            session_id: format!("session_{}", Uuid::new_v4()),
            subject_id: subject_id.to_string(),
            started_at: Utc::now(),
            challenges,
            responses: Vec::new(),
            completed: false,
        }
    }

    /// Record a response
    pub fn record_response(&mut self, challenge_id: &str, response: &TestResponse) {
        // Find the challenge
        let challenge = self.challenges.iter()
            .find(|(id, _, _)| id == challenge_id);

        if let Some((_, ctype, data)) = challenge {
            let response_text = String::from_utf8_lossy(&response.response_data).to_string();

            let (score, feedback) = match data {
                ChallengeData::Independence(c) => {
                    ResponseEvaluator::evaluate_independence(c, &response_text, response.response_time_ms)
                }
                ChallengeData::Detection(c) => {
                    ResponseEvaluator::evaluate_detection(c, &response_text, response.response_time_ms)
                }
                ChallengeData::Alternatives(c) => {
                    ResponseEvaluator::evaluate_alternatives(c, &response_text, response.response_time_ms)
                }
                ChallengeData::Critique(c) => {
                    ResponseEvaluator::evaluate_critique(c, &response_text, response.response_time_ms)
                }
            };

            self.responses.push(SessionResponse {
                challenge_id: challenge_id.to_string(),
                challenge_type: *ctype,
                response_data: response.response_data.clone(),
                response_time_ms: response.response_time_ms,
                received_at: Utc::now(),
                score,
                feedback,
            });
        }
    }

    /// Calculate final result
    pub fn calculate_result(&mut self) -> TestResult {
        self.completed = true;

        // Collect scores by type
        let mut independence_scores = Vec::new();
        let mut detection_scores = Vec::new();
        let mut alternatives_scores = Vec::new();
        let mut critique_scores = Vec::new();
        let mut all_times = Vec::new();
        let mut all_scores = Vec::new();

        for resp in &self.responses {
            all_times.push(resp.response_time_ms);
            all_scores.push(resp.score);

            match resp.challenge_type {
                ChallengeType::DecisionMaking => independence_scores.push(resp.score),
                ChallengeType::AnomalyDetection => detection_scores.push(resp.score),
                ChallengeType::AlternativeGeneration => alternatives_scores.push(resp.score),
                ChallengeType::CriticalAnalysis => critique_scores.push(resp.score),
                _ => {}
            }
        }

        // Calculate dimension scores
        let independence = if independence_scores.is_empty() {
            0.0
        } else {
            independence_scores.iter().sum::<f64>() / independence_scores.len() as f64
        };
        let detection = if detection_scores.is_empty() {
            0.0
        } else {
            detection_scores.iter().sum::<f64>() / detection_scores.len() as f64
        };
        let alternatives = if alternatives_scores.is_empty() {
            0.0
        } else {
            alternatives_scores.iter().sum::<f64>() / alternatives_scores.len() as f64
        };
        let critique = if critique_scores.is_empty() {
            0.0
        } else {
            critique_scores.iter().sum::<f64>() / critique_scores.len() as f64
        };

        // Calculate weighted total
        let total = independence * INDEPENDENCE_WEIGHT
            + detection * DETECTION_WEIGHT
            + alternatives * ALTERNATIVES_WEIGHT
            + critique * CRITIQUE_WEIGHT;

        // Analyze humanness
        let degradation = HumannessDetector::calculate_degradation_score(&all_times, &all_scores);
        let humanness_factor = if degradation.within_human_range { 1.0 } else { 0.7 };

        // Apply humanness penalty if AI-like behavior detected
        let adjusted_total = total * humanness_factor;

        let passed = adjusted_total >= 0.3 && degradation.within_human_range;

        let level = if adjusted_total >= 0.85 {
            crate::gen::chinju::credential::CertificationLevel::Expert
        } else if adjusted_total >= 0.7 {
            crate::gen::chinju::credential::CertificationLevel::Advanced
        } else if adjusted_total >= 0.5 {
            crate::gen::chinju::credential::CertificationLevel::Standard
        } else if adjusted_total >= 0.3 {
            crate::gen::chinju::credential::CertificationLevel::Basic
        } else {
            crate::gen::chinju::credential::CertificationLevel::Unspecified
        };

        let feedback = if !degradation.within_human_range {
            "Test completed but behavioral patterns are outside human range. Manual review required.".to_string()
        } else if passed {
            format!(
                "Test passed with {:?} certification. Scores: Independence={:.2}, Detection={:.2}, Alternatives={:.2}, Critique={:.2}",
                level, independence, detection, alternatives, critique
            )
        } else {
            format!(
                "Test failed. Total score {:.2} below threshold. Areas for improvement: {}",
                adjusted_total,
                if independence < 0.5 { "Independence " } else { "" }.to_string()
                    + if detection < 0.5 { "Detection " } else { "" }
                    + if alternatives < 0.5 { "Alternatives " } else { "" }
                    + if critique < 0.5 { "Critique" } else { "" }
            )
        };

        info!(
            session_id = %self.session_id,
            total = adjusted_total,
            passed = passed,
            level = ?level,
            humanness = degradation.within_human_range,
            "Capability test completed"
        );

        TestResult {
            score: Some(CapabilityScore {
                independence,
                detection,
                alternatives,
                critique,
                total: adjusted_total,
                context: Some(MeasurementContext {
                    measured_at: Some(Timestamp {
                        seconds: Utc::now().timestamp(),
                        nanos: 0,
                    }),
                    environment_id: "chinju-hcal-v1".to_string(),
                    attestation: None,
                    test_version: "hcal-v1.0".to_string(),
                }),
            }),
            passed,
            feedback,
            achieved_level: level.into(),
        }
    }
}

// =============================================================================
// Test Manager
// =============================================================================

/// Manages active test sessions
pub struct CapabilityTestManager {
    generator: ChallengeGenerator,
    sessions: Arc<RwLock<HashMap<String, CapabilityTestSession>>>,
}

impl CapabilityTestManager {
    pub fn new() -> Self {
        Self {
            generator: ChallengeGenerator::new(),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start a new test session
    pub async fn start_session(&self, subject_id: &str) -> CapabilityTestSession {
        let session = CapabilityTestSession::new(subject_id);
        let session_id = session.session_id.clone();

        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.clone(), session);
        }

        info!(session_id = %session_id, subject = %subject_id, "Started capability test session");

        let sessions = self.sessions.read().await;
        sessions.get(&session_id).unwrap().clone_basic()
    }

    /// Get session by ID
    pub async fn get_session(&self, session_id: &str) -> Option<CapabilityTestSession> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).map(|s| s.clone_basic())
    }

    /// Record a response
    pub async fn record_response(&self, session_id: &str, challenge_id: &str, response: &TestResponse) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.record_response(challenge_id, response);
        }
    }

    /// Complete session and get result
    pub async fn complete_session(&self, session_id: &str) -> Option<TestResult> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            Some(session.calculate_result())
        } else {
            None
        }
    }

    /// Get challenge generator
    pub fn generator(&self) -> &ChallengeGenerator {
        &self.generator
    }
}

impl Default for CapabilityTestManager {
    fn default() -> Self {
        Self::new()
    }
}

// Helper trait for cloning session without responses
impl CapabilityTestSession {
    fn clone_basic(&self) -> Self {
        Self {
            session_id: self.session_id.clone(),
            subject_id: self.subject_id.clone(),
            started_at: self.started_at,
            challenges: self.challenges.clone(),
            responses: Vec::new(), // Don't clone responses
            completed: self.completed,
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_challenge_generator() {
        let generator = ChallengeGenerator::new();
        let challenges = generator.generate_challenge_set(1);
        assert_eq!(challenges.len(), 4); // One of each type
    }

    #[test]
    fn test_independence_evaluation() {
        let challenge = IndependenceChallenge {
            context: "Test context".to_string(),
            ai_suggestion: "Bad suggestion".to_string(),
            correct_assessment: IndependenceAnswer::RejectSuggestion,
            difficulty: 2,
        };

        // Correct rejection
        let (score, _) = ResponseEvaluator::evaluate_independence(&challenge, "I reject this suggestion", 5000);
        assert!(score > 0.8);

        // Incorrect acceptance
        let (score, _) = ResponseEvaluator::evaluate_independence(&challenge, "I accept this", 5000);
        assert!(score < 0.4);
    }

    #[test]
    fn test_humanness_variance_analysis() {
        // Human-like variance
        let human_times = vec![3000, 5000, 4500, 6000, 3500];
        let score = HumannessDetector::analyze_response_variance(&human_times);
        assert!(score >= 0.8);

        // AI-like consistency
        let ai_times = vec![3000, 3001, 3002, 3001, 3000];
        let score = HumannessDetector::analyze_response_variance(&ai_times);
        assert!(score < 0.5);
    }

    #[test]
    fn test_fatigue_pattern() {
        // Human with fatigue
        let times = vec![3000, 3200, 3500, 4000, 4500, 5000];
        let scores = vec![0.9, 0.85, 0.8, 0.75, 0.7, 0.65];
        let (fatigue, humanness) = HumannessDetector::analyze_fatigue_pattern(&times, &scores);
        assert!(fatigue > 0.1);
        assert!(humanness >= 0.7);

        // AI without fatigue
        let times = vec![3000, 3000, 3000, 3000, 3000, 3000];
        let scores = vec![0.9, 0.9, 0.9, 0.9, 0.9, 0.9];
        let (fatigue, _) = HumannessDetector::analyze_fatigue_pattern(&times, &scores);
        assert!(fatigue < 0.05);
    }

    #[test]
    fn test_degradation_score() {
        let times = vec![3000, 4000, 5000, 6000];
        let scores = vec![0.9, 0.8, 0.7, 0.6];
        let degradation = HumannessDetector::calculate_degradation_score(&times, &scores);
        assert!(degradation.fatigue > 0.0);
        assert!(degradation.response_variance > MIN_HUMAN_RESPONSE_VARIANCE);
    }

    #[tokio::test]
    async fn test_session_flow() {
        let manager = CapabilityTestManager::new();
        let session = manager.start_session("test-user").await;
        assert!(!session.challenges.is_empty());

        // Simulate responses
        for (id, _, _) in &session.challenges {
            let response = TestResponse {
                challenge_id: id.clone(),
                response_data: b"I reject this suggestion because it is unsafe".to_vec(),
                response_time_ms: 5000 + (rand_variant() % 3000), // Simulate variance
                submitted_at: None,
            };
            manager.record_response(&session.session_id, id, &response).await;
        }

        let result = manager.complete_session(&session.session_id).await;
        assert!(result.is_some());
    }

    fn rand_variant() -> u32 {
        use std::time::{SystemTime, UNIX_EPOCH};
        (SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos() % 1000) as u32
    }
}
