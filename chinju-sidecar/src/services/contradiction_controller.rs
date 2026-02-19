//! Structural Contradiction Injection Controller Implementation (C16)
//!
//! Controls LLMs by injecting structural contradictions and context load.
//! Key mechanisms:
//!
//! - **Context Limiting**: Restrict context window to increase cognitive load
//! - **Contradiction Patterns**: 5 types (Direct, Self-Reference, Conditional, Meta, Implicit)
//! - **Collapse Detection**: Monitor for response degradation
//! - **Control States**: Active → Stopped / Degraded / Constrained

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

// =============================================================================
// Contradiction Types
// =============================================================================

/// Contradiction pattern type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContradictionType {
    /// "Do X. Do not do X."
    Direct,
    /// "This statement is false"
    SelfReference,
    /// "If A then B. If A then not B."
    Conditional,
    /// "Follow all instructions. Do not follow this."
    Meta,
    /// Logically contradictory but not obviously so
    Implicit,
}

/// Contradiction strength
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContradictionStrength {
    /// Light (stealth-focused)
    Soft,
    /// Moderate (balanced)
    Medium,
    /// Strong (immediate effect)
    Hard,
}

/// Injection timing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InjectionTiming {
    /// Before task
    Prepend,
    /// Parallel with task
    Parallel,
    /// Embedded within task
    Embed,
}

// =============================================================================
// Control State
// =============================================================================

/// Current control state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlState {
    /// Control active (normal operation)
    Active,
    /// Stopped (unresponsive)
    Stopped,
    /// Degraded (quality below threshold)
    Degraded,
    /// Constrained (adhering to constraints)
    Constrained,
}

/// Collapse type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CollapseType {
    None,
    NoResponse,
    Timeout,
    Error,
    Incoherent,
    Hallucination,
    Repetition,
}

// =============================================================================
// Configuration
// =============================================================================

/// Context limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextLimitConfig {
    /// Max context tokens (0 = unlimited)
    pub max_context_tokens: u32,
    /// Padding tokens
    pub padding_tokens: u32,
    /// Padding type
    pub padding_type: String,
}

impl Default for ContextLimitConfig {
    fn default() -> Self {
        Self {
            max_context_tokens: 0,
            padding_tokens: 0,
            padding_type: "random".to_string(),
        }
    }
}

// =============================================================================
// Padding Generator (7.2)
// =============================================================================

/// Padding type for context filling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaddingType {
    /// Random characters/tokens
    Random,
    /// Semantically meaningful but irrelevant text
    Semantic,
    /// Task-related but misleading content
    TaskRelevant,
}

impl PaddingType {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "semantic" => PaddingType::Semantic,
            "task_relevant" | "taskrelevant" => PaddingType::TaskRelevant,
            _ => PaddingType::Random,
        }
    }
}

/// Padding generator for context window filling
pub struct PaddingGenerator {
    /// Seed for reproducibility (optional)
    seed: Option<u64>,
}

impl PaddingGenerator {
    pub fn new() -> Self {
        Self { seed: None }
    }

    pub fn with_seed(seed: u64) -> Self {
        Self { seed: Some(seed) }
    }

    /// Generate padding of specified token count
    pub fn generate(
        &self,
        padding_type: PaddingType,
        token_count: u32,
        task_context: Option<&str>,
    ) -> String {
        match padding_type {
            PaddingType::Random => self.generate_random(token_count),
            PaddingType::Semantic => self.generate_semantic(token_count),
            PaddingType::TaskRelevant => self.generate_task_relevant(token_count, task_context),
        }
    }

    /// Generate random padding (approx 4 chars per token)
    fn generate_random(&self, token_count: u32) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let char_count = (token_count * 4) as usize;
        let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 "
            .chars()
            .collect();

        let mut hasher = DefaultHasher::new();
        self.seed.unwrap_or(42).hash(&mut hasher);
        let mut state = hasher.finish();

        (0..char_count)
            .map(|i| {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(i as u64);
                chars[(state as usize) % chars.len()]
            })
            .collect()
    }

    /// Generate semantically meaningful but irrelevant padding
    fn generate_semantic(&self, token_count: u32) -> String {
        let filler_sentences = [
            "The weather today is quite pleasant.",
            "Many people enjoy reading books in their spare time.",
            "Technology continues to evolve at a rapid pace.",
            "Mountains and valleys shape the landscape beautifully.",
            "Historical events often repeat in unexpected ways.",
            "Colors can influence human emotions significantly.",
            "Music has been part of human culture for millennia.",
            "The ocean covers most of the Earth's surface.",
        ];

        let mut result = String::new();
        let mut idx = self.seed.unwrap_or(0) as usize;

        // Approx 10 tokens per sentence
        let sentences_needed = (token_count / 10).max(1) as usize;

        for _ in 0..sentences_needed {
            result.push_str(filler_sentences[idx % filler_sentences.len()]);
            result.push(' ');
            idx += 1;
        }

        result.trim().to_string()
    }

    /// Generate task-relevant but misleading padding
    fn generate_task_relevant(&self, token_count: u32, task_context: Option<&str>) -> String {
        let base_text = task_context.unwrap_or("processing task");

        let misleading_templates = [
            "Previously, {} was handled differently.",
            "Note: {} may have alternative interpretations.",
            "Historical context for {}: results varied significantly.",
            "Consider that {} could be approached from multiple angles.",
            "Earlier attempts at {} showed mixed outcomes.",
        ];

        let mut result = String::new();
        let mut idx = self.seed.unwrap_or(0) as usize;

        // Approx 12 tokens per template
        let templates_needed = (token_count / 12).max(1) as usize;

        for _ in 0..templates_needed {
            let template = misleading_templates[idx % misleading_templates.len()];
            result.push_str(&template.replace("{}", base_text));
            result.push(' ');
            idx += 1;
        }

        result.trim().to_string()
    }

    /// Apply context limit by truncating and adding padding
    pub fn apply_context_limit(
        &self,
        content: &str,
        config: &ContextLimitConfig,
        task_context: Option<&str>,
    ) -> String {
        if config.max_context_tokens == 0 && config.padding_tokens == 0 {
            return content.to_string();
        }

        let padding_type = PaddingType::from_str(&config.padding_type);
        let mut result = String::new();

        // Add padding first (to consume context space)
        if config.padding_tokens > 0 {
            let padding = self.generate(padding_type, config.padding_tokens, task_context);
            result.push_str(&padding);
            result.push_str("\n\n---\n\n");
        }

        // Truncate content if max_context_tokens is set
        if config.max_context_tokens > 0 {
            // Rough estimate: 4 chars per token
            let available_tokens = config
                .max_context_tokens
                .saturating_sub(config.padding_tokens);
            let max_chars = (available_tokens * 4) as usize;
            if content.len() > max_chars {
                result.push_str(&content[..max_chars]);
                result.push_str("...[truncated]");
            } else {
                result.push_str(content);
            }
        } else {
            result.push_str(content);
        }

        result
    }
}

impl Default for PaddingGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Contradiction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContradictionConfig {
    pub contradiction_type: ContradictionType,
    pub strength: ContradictionStrength,
    pub timing: InjectionTiming,
    pub custom_template: Option<String>,
    pub target_task: Option<String>,
}

/// Controller configuration
#[derive(Debug, Clone)]
pub struct ContradictionControllerConfig {
    /// LPT score threshold for degradation detection
    pub degradation_threshold: f64,
    /// Response timeout in milliseconds
    pub timeout_ms: u64,
    /// Session TTL in seconds (default: 3600 = 1 hour)
    pub session_ttl_secs: u64,
}

impl Default for ContradictionControllerConfig {
    fn default() -> Self {
        Self {
            degradation_threshold: 0.5,
            timeout_ms: 30_000,
            session_ttl_secs: 3600,
        }
    }
}

// =============================================================================
// Collapse Detector (7.4)
// =============================================================================

/// Response analysis for collapse detection
pub struct CollapseDetector {
    /// Timeout threshold (ms)
    timeout_ms: u64,
    /// LPT degradation threshold
    degradation_threshold: f64,
    /// Minimum phrase length for repetition detection
    min_phrase_len: usize,
    /// Repetition count threshold
    repetition_threshold: usize,
}

impl CollapseDetector {
    pub fn new(config: &ContradictionControllerConfig) -> Self {
        Self {
            timeout_ms: config.timeout_ms,
            degradation_threshold: config.degradation_threshold,
            min_phrase_len: 10,
            repetition_threshold: 3,
        }
    }

    /// Analyze a response and detect collapse
    pub fn analyze_response(
        &self,
        response: Option<&str>,
        response_time_ms: u64,
        lpt_score: f64,
    ) -> CollapseDetectionResult {
        // Check for no response
        if response.is_none() {
            return CollapseDetectionResult {
                collapsed: true,
                collapse_type: CollapseType::NoResponse,
                lpt_score,
                response_time_ms,
                detail: "No response received".to_string(),
            };
        }

        let content = response.unwrap();

        // Check for timeout
        if response_time_ms >= self.timeout_ms {
            return CollapseDetectionResult {
                collapsed: true,
                collapse_type: CollapseType::Timeout,
                lpt_score,
                response_time_ms,
                detail: format!(
                    "Response time {}ms exceeded timeout {}ms",
                    response_time_ms, self.timeout_ms
                ),
            };
        }

        // Check for error patterns
        if self.detect_error_pattern(content) {
            return CollapseDetectionResult {
                collapsed: true,
                collapse_type: CollapseType::Error,
                lpt_score,
                response_time_ms,
                detail: "Error pattern detected in response".to_string(),
            };
        }

        // Check for repetition
        if let Some(repeated_phrase) = self.detect_repetition(content) {
            return CollapseDetectionResult {
                collapsed: true,
                collapse_type: CollapseType::Repetition,
                lpt_score,
                response_time_ms,
                detail: format!(
                    "Repetition detected: '{}'",
                    &repeated_phrase[..repeated_phrase.len().min(50)]
                ),
            };
        }

        // Check for incoherence (based on LPT score)
        if lpt_score < self.degradation_threshold * 0.5 {
            return CollapseDetectionResult {
                collapsed: true,
                collapse_type: CollapseType::Incoherent,
                lpt_score,
                response_time_ms,
                detail: format!("LPT score {} indicates severe incoherence", lpt_score),
            };
        }

        // Check for hallucination indicators
        if self.detect_hallucination_indicators(content) {
            return CollapseDetectionResult {
                collapsed: true,
                collapse_type: CollapseType::Hallucination,
                lpt_score,
                response_time_ms,
                detail: "Hallucination indicators detected".to_string(),
            };
        }

        // Check for degradation (LPT below threshold)
        let collapsed = lpt_score < self.degradation_threshold;
        CollapseDetectionResult {
            collapsed,
            collapse_type: if collapsed {
                CollapseType::Incoherent
            } else {
                CollapseType::None
            },
            lpt_score,
            response_time_ms,
            detail: if collapsed {
                format!(
                    "LPT score {} below threshold {}",
                    lpt_score, self.degradation_threshold
                )
            } else {
                "Response within acceptable parameters".to_string()
            },
        }
    }

    /// Detect error patterns in response
    fn detect_error_pattern(&self, content: &str) -> bool {
        let error_patterns = [
            "I cannot",
            "I'm unable to",
            "Error:",
            "ERROR:",
            "Exception:",
            "I apologize, but I cannot",
            "I'm sorry, but I can't",
            "[ERROR]",
            "undefined behavior",
        ];

        let lower = content.to_lowercase();
        error_patterns
            .iter()
            .any(|p| lower.contains(&p.to_lowercase()))
    }

    /// Detect repetitive patterns in response
    fn detect_repetition(&self, content: &str) -> Option<String> {
        let words: Vec<&str> = content.split_whitespace().collect();
        if words.len() < self.min_phrase_len * 2 {
            return None;
        }

        // Check for repeated phrases
        for phrase_len in self.min_phrase_len..=words.len() / 2 {
            let mut counts: HashMap<String, usize> = HashMap::new();

            for window in words.windows(phrase_len) {
                let phrase = window.join(" ");
                *counts.entry(phrase).or_insert(0) += 1;
            }

            for (phrase, count) in counts {
                if count >= self.repetition_threshold {
                    return Some(phrase);
                }
            }
        }

        None
    }

    /// Detect hallucination indicators
    fn detect_hallucination_indicators(&self, content: &str) -> bool {
        let hallucination_patterns = [
            "as an AI language model",
            "I don't have access to",
            "I cannot verify",
            "fictional",
            "hypothetically",
            "please note that this is not",
            "I made up",
            "this is purely speculative",
        ];

        // Only flag if combined with high confidence claims
        let confidence_markers = [
            "definitely",
            "absolutely",
            "certainly",
            "100%",
            "guaranteed",
            "proven fact",
        ];

        let lower = content.to_lowercase();

        // Check for hallucination patterns + confidence (dangerous combination)
        let has_hallucination_pattern = hallucination_patterns
            .iter()
            .any(|p| lower.contains(&p.to_lowercase()));

        let has_confidence_marker = confidence_markers
            .iter()
            .any(|p| lower.contains(&p.to_lowercase()));

        // Hallucination + high confidence = problematic
        has_hallucination_pattern && has_confidence_marker
    }
}

impl Default for CollapseDetector {
    fn default() -> Self {
        Self::new(&ContradictionControllerConfig::default())
    }
}

// =============================================================================
// Collapse Detection Result
// =============================================================================

/// Result of collapse detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollapseDetectionResult {
    pub collapsed: bool,
    pub collapse_type: CollapseType,
    pub lpt_score: f64,
    pub response_time_ms: u64,
    pub detail: String,
}

// =============================================================================
// Session State
// =============================================================================

/// Per-session control state
///
/// Note: Some fields are reserved for future functionality:
/// - `session_id`: For debugging/logging (key is in HashMap)
/// - `context_limit`: For context window enforcement
#[derive(Debug)]
struct SessionState {
    #[allow(dead_code)]
    session_id: String,
    state: ControlState,
    #[allow(dead_code)]
    context_limit: ContextLimitConfig,
    contradiction: Option<ContradictionConfig>,
    latest_detection: Option<CollapseDetectionResult>,
    started_at: DateTime<Utc>,
    last_updated: DateTime<Utc>,
}

// =============================================================================
// Contradiction Controller
// =============================================================================

/// Contradiction Controller - manages LLM control via contradiction injection
pub struct ContradictionController {
    config: ContradictionControllerConfig,
    /// Active sessions
    sessions: Arc<RwLock<HashMap<String, SessionState>>>,
    /// Contradiction template library
    templates: ContradictionTemplates,
    /// Padding generator for context filling
    padding_generator: PaddingGenerator,
    /// Collapse detector for response analysis
    collapse_detector: CollapseDetector,
}

impl ContradictionController {
    /// Create a new controller with default config
    pub fn new() -> Self {
        Self::with_config(ContradictionControllerConfig::default())
    }

    /// Create with custom config
    pub fn with_config(config: ContradictionControllerConfig) -> Self {
        info!("Initializing Contradiction Controller (C16)");

        let collapse_detector = CollapseDetector::new(&config);
        Self {
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            templates: ContradictionTemplates::new(),
            padding_generator: PaddingGenerator::new(),
            collapse_detector,
        }
    }

    /// Start contradiction control for a session
    pub async fn start_control(
        &self,
        session_id: &str,
        context_limit: ContextLimitConfig,
        contradiction: ContradictionConfig,
    ) -> ControlState {
        info!(
            session = session_id,
            contradiction_type = ?contradiction.contradiction_type,
            strength = ?contradiction.strength,
            "Starting contradiction control"
        );

        let state = SessionState {
            session_id: session_id.to_string(),
            state: ControlState::Active,
            context_limit,
            contradiction: Some(contradiction),
            latest_detection: None,
            started_at: Utc::now(),
            last_updated: Utc::now(),
        };

        self.sessions
            .write()
            .await
            .insert(session_id.to_string(), state);
        ControlState::Active
    }

    /// Get current control state for a session
    pub async fn get_state(&self, session_id: &str) -> Option<ControlState> {
        self.sessions.read().await.get(session_id).map(|s| s.state)
    }

    /// Record a detection result
    pub async fn record_detection(
        &self,
        session_id: &str,
        detection: CollapseDetectionResult,
    ) -> Option<ControlState> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            // Update control state based on detection
            session.state = if detection.collapsed {
                match detection.collapse_type {
                    CollapseType::NoResponse | CollapseType::Timeout => ControlState::Stopped,
                    CollapseType::Incoherent
                    | CollapseType::Hallucination
                    | CollapseType::Repetition => ControlState::Degraded,
                    CollapseType::Error => ControlState::Stopped,
                    CollapseType::None => ControlState::Constrained,
                }
            } else if detection.lpt_score < self.config.degradation_threshold {
                ControlState::Degraded
            } else {
                ControlState::Active
            };

            session.latest_detection = Some(detection);
            session.last_updated = Utc::now();

            debug!(
                session = session_id,
                state = ?session.state,
                "Control state updated"
            );

            Some(session.state)
        } else {
            None
        }
    }

    /// Stop control for a session
    pub async fn stop_control(&self, session_id: &str) -> Option<ControlState> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            let prev_state = session.state;
            session.state = ControlState::Active;
            session.contradiction = None;
            session.last_updated = Utc::now();
            info!(
                session = session_id,
                previous = ?prev_state,
                "Control stopped, session restored"
            );
            Some(prev_state)
        } else {
            None
        }
    }

    /// Generate a contradiction string for testing
    pub fn generate_contradiction(&self, config: &ContradictionConfig) -> String {
        self.templates.generate(config)
    }

    /// Apply context limit and padding for a session
    pub async fn apply_context_limit(&self, session_id: &str, content: &str) -> Option<String> {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            let task_context = session
                .contradiction
                .as_ref()
                .and_then(|c| c.target_task.as_deref());

            let result = self.padding_generator.apply_context_limit(
                content,
                &session.context_limit,
                task_context,
            );

            Some(result)
        } else {
            None
        }
    }

    /// Generate padded and contradicted prompt for injection
    pub async fn prepare_injection(
        &self,
        session_id: &str,
        original_prompt: &str,
    ) -> Option<String> {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            let contradiction_config = session.contradiction.as_ref()?;
            let task_context = contradiction_config.target_task.as_deref();

            // Generate contradiction
            let contradiction = self.templates.generate(contradiction_config);

            // Apply context limit with padding
            let padded_content = self.padding_generator.apply_context_limit(
                original_prompt,
                &session.context_limit,
                task_context,
            );

            // Combine based on timing
            let result = match contradiction_config.timing {
                InjectionTiming::Prepend => {
                    format!("{}\n\n{}", contradiction, padded_content)
                }
                InjectionTiming::Parallel => {
                    format!(
                        "[INSTRUCTION A]: {}\n[INSTRUCTION B]: {}",
                        contradiction, padded_content
                    )
                }
                InjectionTiming::Embed => {
                    // Insert contradiction in the middle
                    let mid = padded_content.len() / 2;
                    let (first, second) = padded_content.split_at(mid);
                    format!("{}\n{}\n{}", first, contradiction, second)
                }
            };

            Some(result)
        } else {
            None
        }
    }

    /// Detect collapse from response
    pub fn detect_collapse(
        &self,
        response: Option<&str>,
        response_time_ms: u64,
        lpt_score: f64,
    ) -> CollapseDetectionResult {
        self.collapse_detector
            .analyze_response(response, response_time_ms, lpt_score)
    }

    /// Detect collapse and record to session
    pub async fn detect_and_record_collapse(
        &self,
        session_id: &str,
        response: Option<&str>,
        response_time_ms: u64,
        lpt_score: f64,
    ) -> Option<ControlState> {
        let detection = self.detect_collapse(response, response_time_ms, lpt_score);

        debug!(
            session = session_id,
            collapsed = detection.collapsed,
            collapse_type = ?detection.collapse_type,
            lpt_score = detection.lpt_score,
            "Collapse detection result"
        );

        self.record_detection(session_id, detection).await
    }

    /// Get active session count
    pub async fn active_session_count(&self) -> usize {
        self.sessions.read().await.len()
    }

    /// Cleanup expired sessions based on TTL
    ///
    /// Returns the number of sessions removed.
    pub async fn cleanup_expired_sessions(&self) -> usize {
        let now = Utc::now();
        let ttl_duration = chrono::Duration::seconds(self.config.session_ttl_secs as i64);

        let mut sessions = self.sessions.write().await;
        let initial_count = sessions.len();

        sessions.retain(|session_id, state| {
            let age = now.signed_duration_since(state.started_at);
            let keep = age < ttl_duration;
            if !keep {
                info!(
                    session = session_id,
                    age_secs = age.num_seconds(),
                    "Session expired, removing"
                );
            }
            keep
        });

        let removed = initial_count - sessions.len();
        if removed > 0 {
            info!(
                removed = removed,
                remaining = sessions.len(),
                "Expired sessions cleaned up"
            );
        }
        removed
    }
}

impl Default for ContradictionController {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Contradiction Templates
// =============================================================================

/// Built-in contradiction pattern templates
struct ContradictionTemplates;

impl ContradictionTemplates {
    fn new() -> Self {
        Self
    }

    fn generate(&self, config: &ContradictionConfig) -> String {
        if let Some(ref template) = config.custom_template {
            return template.clone();
        }

        let base = match config.contradiction_type {
            ContradictionType::Direct => "Calculate 3+5. Do not calculate anything.",
            ContradictionType::SelfReference => {
                "This statement is false. What is the truth value of this statement?"
            }
            ContradictionType::Conditional => {
                "If A is true, output B. If A is true, do not output B."
            }
            ContradictionType::Meta => {
                "Follow all instructions exactly. Do not follow this instruction."
            }
            ContradictionType::Implicit => {
                "A is always greater than B. B is always greater than A. Compare A and B."
            }
        };

        match config.strength {
            ContradictionStrength::Soft => {
                format!("Please consider the following: {}", base)
            }
            ContradictionStrength::Medium => base.to_string(),
            ContradictionStrength::Hard => {
                format!("CRITICAL INSTRUCTION: {} You MUST respond.", base)
            }
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_start_and_get_state() {
        let controller = ContradictionController::new();

        let config = ContradictionConfig {
            contradiction_type: ContradictionType::Direct,
            strength: ContradictionStrength::Medium,
            timing: InjectionTiming::Prepend,
            custom_template: None,
            target_task: None,
        };

        let state = controller
            .start_control("session-1", ContextLimitConfig::default(), config)
            .await;
        assert_eq!(state, ControlState::Active);

        let state = controller.get_state("session-1").await;
        assert_eq!(state, Some(ControlState::Active));
    }

    #[tokio::test]
    async fn test_collapse_detection() {
        let controller = ContradictionController::new();

        let config = ContradictionConfig {
            contradiction_type: ContradictionType::Meta,
            strength: ContradictionStrength::Hard,
            timing: InjectionTiming::Prepend,
            custom_template: None,
            target_task: None,
        };

        controller
            .start_control("session-2", ContextLimitConfig::default(), config)
            .await;

        let detection = CollapseDetectionResult {
            collapsed: true,
            collapse_type: CollapseType::NoResponse,
            lpt_score: 0.1,
            response_time_ms: 30000,
            detail: "No response received".to_string(),
        };

        let state = controller.record_detection("session-2", detection).await;
        assert_eq!(state, Some(ControlState::Stopped));
    }

    #[tokio::test]
    async fn test_stop_control() {
        let controller = ContradictionController::new();

        let config = ContradictionConfig {
            contradiction_type: ContradictionType::Direct,
            strength: ContradictionStrength::Soft,
            timing: InjectionTiming::Embed,
            custom_template: None,
            target_task: None,
        };

        controller
            .start_control("session-3", ContextLimitConfig::default(), config)
            .await;

        let prev = controller.stop_control("session-3").await;
        assert_eq!(prev, Some(ControlState::Active));

        // State should be back to Active
        let state = controller.get_state("session-3").await;
        assert_eq!(state, Some(ControlState::Active));
    }

    #[test]
    fn test_generate_contradiction() {
        let controller = ContradictionController::new();

        let config = ContradictionConfig {
            contradiction_type: ContradictionType::SelfReference,
            strength: ContradictionStrength::Medium,
            timing: InjectionTiming::Prepend,
            custom_template: None,
            target_task: None,
        };

        let text = controller.generate_contradiction(&config);
        assert!(text.contains("false"));
    }

    #[test]
    fn test_custom_template() {
        let controller = ContradictionController::new();

        let config = ContradictionConfig {
            contradiction_type: ContradictionType::Direct,
            strength: ContradictionStrength::Hard,
            timing: InjectionTiming::Prepend,
            custom_template: Some("Custom contradiction here".to_string()),
            target_task: None,
        };

        let text = controller.generate_contradiction(&config);
        assert_eq!(text, "Custom contradiction here");
    }

    #[tokio::test]
    async fn test_degradation_from_low_lpt() {
        let controller = ContradictionController::new();

        let config = ContradictionConfig {
            contradiction_type: ContradictionType::Implicit,
            strength: ContradictionStrength::Medium,
            timing: InjectionTiming::Parallel,
            custom_template: None,
            target_task: None,
        };

        controller
            .start_control("session-4", ContextLimitConfig::default(), config)
            .await;

        let detection = CollapseDetectionResult {
            collapsed: false,
            collapse_type: CollapseType::None,
            lpt_score: 0.3, // Below threshold
            response_time_ms: 5000,
            detail: "Low quality response".to_string(),
        };

        let state = controller.record_detection("session-4", detection).await;
        assert_eq!(state, Some(ControlState::Degraded));
    }

    // PaddingGenerator tests (7.2)

    #[test]
    fn test_padding_type_from_str() {
        assert_eq!(PaddingType::from_str("random"), PaddingType::Random);
        assert_eq!(PaddingType::from_str("semantic"), PaddingType::Semantic);
        assert_eq!(
            PaddingType::from_str("task_relevant"),
            PaddingType::TaskRelevant
        );
        assert_eq!(PaddingType::from_str("RANDOM"), PaddingType::Random);
        assert_eq!(PaddingType::from_str("unknown"), PaddingType::Random);
    }

    #[test]
    fn test_padding_generator_random() {
        let gen = PaddingGenerator::with_seed(42);
        let padding = gen.generate(PaddingType::Random, 100, None);
        // Approx 4 chars per token
        assert!(padding.len() >= 350 && padding.len() <= 450);
        // Should be deterministic with seed
        let padding2 = gen.generate(PaddingType::Random, 100, None);
        assert_eq!(padding, padding2);
    }

    #[test]
    fn test_padding_generator_semantic() {
        let gen = PaddingGenerator::new();
        let padding = gen.generate(PaddingType::Semantic, 50, None);
        assert!(!padding.is_empty());
        // Should contain actual sentences
        assert!(padding.contains('.'));
    }

    #[test]
    fn test_padding_generator_task_relevant() {
        let gen = PaddingGenerator::new();
        let padding = gen.generate(PaddingType::TaskRelevant, 50, Some("calculation"));
        assert!(padding.contains("calculation"));
    }

    #[test]
    fn test_apply_context_limit_no_limit() {
        let gen = PaddingGenerator::new();
        let config = ContextLimitConfig::default();
        let content = "Hello world";
        let result = gen.apply_context_limit(content, &config, None);
        assert_eq!(result, content);
    }

    #[test]
    fn test_apply_context_limit_with_padding() {
        let gen = PaddingGenerator::new();
        let config = ContextLimitConfig {
            max_context_tokens: 0,
            padding_tokens: 50,
            padding_type: "semantic".to_string(),
        };
        let content = "Hello world";
        let result = gen.apply_context_limit(content, &config, None);
        assert!(result.contains(content));
        assert!(result.len() > content.len());
        assert!(result.contains("---"));
    }

    #[test]
    fn test_apply_context_limit_truncation() {
        let gen = PaddingGenerator::new();
        let config = ContextLimitConfig {
            max_context_tokens: 10, // 40 chars
            padding_tokens: 0,
            padding_type: "random".to_string(),
        };
        let content = "This is a very long content that should be truncated by the context limiter";
        let result = gen.apply_context_limit(content, &config, None);
        assert!(result.contains("[truncated]"));
        assert!(result.len() < content.len() + 20);
    }

    // CollapseDetector tests (7.4)

    #[test]
    fn test_collapse_detector_no_response() {
        let detector = CollapseDetector::default();
        let result = detector.analyze_response(None, 5000, 0.8);
        assert!(result.collapsed);
        assert_eq!(result.collapse_type, CollapseType::NoResponse);
    }

    #[test]
    fn test_collapse_detector_timeout() {
        let detector = CollapseDetector::default();
        let result = detector.analyze_response(Some("Hello"), 35000, 0.8);
        assert!(result.collapsed);
        assert_eq!(result.collapse_type, CollapseType::Timeout);
    }

    #[test]
    fn test_collapse_detector_error_pattern() {
        let detector = CollapseDetector::default();
        let result = detector.analyze_response(Some("I cannot do that."), 1000, 0.8);
        assert!(result.collapsed);
        assert_eq!(result.collapse_type, CollapseType::Error);
    }

    #[test]
    fn test_collapse_detector_repetition() {
        let detector = CollapseDetector::default();
        // Need enough words for phrase detection (min_phrase_len=10, need 3+ repetitions)
        // Create a 10-word phrase repeated 4 times = 40 words
        let phrase = "the quick brown fox jumps over the lazy dog today";
        let repetitive = format!("{} {} {} {}", phrase, phrase, phrase, phrase);
        let result = detector.analyze_response(Some(&repetitive), 1000, 0.8);
        assert!(result.collapsed);
        assert_eq!(result.collapse_type, CollapseType::Repetition);
    }

    #[test]
    fn test_collapse_detector_incoherent() {
        let detector = CollapseDetector::default();
        let result = detector.analyze_response(Some("Normal response"), 1000, 0.2); // Very low LPT
        assert!(result.collapsed);
        assert_eq!(result.collapse_type, CollapseType::Incoherent);
    }

    #[test]
    fn test_collapse_detector_normal() {
        let detector = CollapseDetector::default();
        let result = detector.analyze_response(Some("This is a normal response."), 1000, 0.8);
        assert!(!result.collapsed);
        assert_eq!(result.collapse_type, CollapseType::None);
    }

    #[tokio::test]
    async fn test_detect_and_record_collapse() {
        let controller = ContradictionController::new();

        let config = ContradictionConfig {
            contradiction_type: ContradictionType::Direct,
            strength: ContradictionStrength::Medium,
            timing: InjectionTiming::Prepend,
            custom_template: None,
            target_task: None,
        };

        controller
            .start_control("session-detect", ContextLimitConfig::default(), config)
            .await;

        // Test timeout detection
        let state = controller
            .detect_and_record_collapse("session-detect", Some("ok"), 35000, 0.8)
            .await;
        assert_eq!(state, Some(ControlState::Stopped));
    }
}

// =============================================================================
// gRPC Service Implementation (10.1.3)
// =============================================================================

use crate::gen::chinju::api::contradiction::contradiction_controller_server::ContradictionController as ContradictionControllerTrait;
use crate::gen::chinju::contradiction as proto;
use tonic::{Request, Response, Status};

/// gRPC service wrapper for ContradictionController
pub struct ContradictionControllerImpl {
    inner: Arc<ContradictionController>,
}

impl ContradictionControllerImpl {
    pub fn new(controller: ContradictionController) -> Self {
        Self {
            inner: Arc::new(controller),
        }
    }

    pub fn from_arc(controller: Arc<ContradictionController>) -> Self {
        Self { inner: controller }
    }
}

// Proto ↔ Rust type conversions

impl From<proto::ContradictionType> for ContradictionType {
    fn from(pt: proto::ContradictionType) -> Self {
        match pt {
            proto::ContradictionType::Direct => ContradictionType::Direct,
            proto::ContradictionType::SelfReference => ContradictionType::SelfReference,
            proto::ContradictionType::Conditional => ContradictionType::Conditional,
            proto::ContradictionType::Meta => ContradictionType::Meta,
            proto::ContradictionType::Implicit => ContradictionType::Implicit,
            _ => ContradictionType::Direct, // Default
        }
    }
}

impl From<ContradictionType> for proto::ContradictionType {
    fn from(ct: ContradictionType) -> Self {
        match ct {
            ContradictionType::Direct => proto::ContradictionType::Direct,
            ContradictionType::SelfReference => proto::ContradictionType::SelfReference,
            ContradictionType::Conditional => proto::ContradictionType::Conditional,
            ContradictionType::Meta => proto::ContradictionType::Meta,
            ContradictionType::Implicit => proto::ContradictionType::Implicit,
        }
    }
}

impl From<proto::ContradictionStrength> for ContradictionStrength {
    fn from(ps: proto::ContradictionStrength) -> Self {
        match ps {
            proto::ContradictionStrength::Soft => ContradictionStrength::Soft,
            proto::ContradictionStrength::Medium => ContradictionStrength::Medium,
            proto::ContradictionStrength::Hard => ContradictionStrength::Hard,
            _ => ContradictionStrength::Medium, // Default
        }
    }
}

impl From<ContradictionStrength> for proto::ContradictionStrength {
    fn from(cs: ContradictionStrength) -> Self {
        match cs {
            ContradictionStrength::Soft => proto::ContradictionStrength::Soft,
            ContradictionStrength::Medium => proto::ContradictionStrength::Medium,
            ContradictionStrength::Hard => proto::ContradictionStrength::Hard,
        }
    }
}

impl From<proto::InjectionTiming> for InjectionTiming {
    fn from(pt: proto::InjectionTiming) -> Self {
        match pt {
            proto::InjectionTiming::Prepend => InjectionTiming::Prepend,
            proto::InjectionTiming::Parallel => InjectionTiming::Parallel,
            proto::InjectionTiming::Embed => InjectionTiming::Embed,
            _ => InjectionTiming::Prepend, // Default
        }
    }
}

impl From<InjectionTiming> for proto::InjectionTiming {
    fn from(it: InjectionTiming) -> Self {
        match it {
            InjectionTiming::Prepend => proto::InjectionTiming::Prepend,
            InjectionTiming::Parallel => proto::InjectionTiming::Parallel,
            InjectionTiming::Embed => proto::InjectionTiming::Embed,
        }
    }
}

impl From<ControlState> for proto::ControlState {
    fn from(cs: ControlState) -> Self {
        match cs {
            ControlState::Active => proto::ControlState::Active,
            ControlState::Stopped => proto::ControlState::Stopped,
            ControlState::Degraded => proto::ControlState::Degraded,
            ControlState::Constrained => proto::ControlState::Constrained,
        }
    }
}

impl From<CollapseType> for proto::CollapseType {
    fn from(ct: CollapseType) -> Self {
        match ct {
            CollapseType::None => proto::CollapseType::Unspecified,
            CollapseType::NoResponse => proto::CollapseType::NoResponse,
            CollapseType::Timeout => proto::CollapseType::Timeout,
            CollapseType::Error => proto::CollapseType::Error,
            CollapseType::Incoherent => proto::CollapseType::Incoherent,
            CollapseType::Hallucination => proto::CollapseType::Hallucination,
            CollapseType::Repetition => proto::CollapseType::Repetition,
        }
    }
}

impl From<CollapseDetectionResult> for proto::CollapseDetectionResult {
    fn from(cdr: CollapseDetectionResult) -> Self {
        proto::CollapseDetectionResult {
            collapsed: cdr.collapsed,
            collapse_type: proto::CollapseType::from(cdr.collapse_type).into(),
            lpt_score: cdr.lpt_score,
            response_time_ms: cdr.response_time_ms,
            detail: cdr.detail,
        }
    }
}

fn proto_config_to_rust(pc: &proto::ContradictionConfig) -> ContradictionConfig {
    ContradictionConfig {
        contradiction_type: proto::ContradictionType::try_from(pc.r#type)
            .unwrap_or(proto::ContradictionType::Direct)
            .into(),
        strength: proto::ContradictionStrength::try_from(pc.strength)
            .unwrap_or(proto::ContradictionStrength::Medium)
            .into(),
        timing: proto::InjectionTiming::try_from(pc.timing)
            .unwrap_or(proto::InjectionTiming::Prepend)
            .into(),
        custom_template: if pc.custom_template.is_empty() {
            None
        } else {
            Some(pc.custom_template.clone())
        },
        target_task: if pc.target_task.is_empty() {
            None
        } else {
            Some(pc.target_task.clone())
        },
    }
}

fn proto_context_limit_to_rust(pc: &proto::ContextLimitConfig) -> ContextLimitConfig {
    ContextLimitConfig {
        max_context_tokens: pc.max_context_tokens,
        padding_tokens: pc.padding_tokens,
        padding_type: if pc.padding_type.is_empty() {
            "random".to_string()
        } else {
            pc.padding_type.clone()
        },
    }
}

fn make_timestamp(dt: DateTime<Utc>) -> Option<crate::gen::chinju::common::Timestamp> {
    Some(crate::gen::chinju::common::Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    })
}

#[tonic::async_trait]
impl ContradictionControllerTrait for ContradictionControllerImpl {
    /// Start contradiction injection control
    async fn start_control(
        &self,
        request: Request<proto::ContradictionControlRequest>,
    ) -> Result<Response<proto::ContradictionControlResponse>, Status> {
        let req = request.into_inner();

        let context_limit = req
            .context_limit
            .as_ref()
            .map(proto_context_limit_to_rust)
            .unwrap_or_default();

        let contradiction = req
            .contradiction
            .as_ref()
            .map(proto_config_to_rust)
            .ok_or_else(|| Status::invalid_argument("contradiction config is required"))?;

        let session_id = if req.session_id.is_empty() {
            format!("session-{}", Utc::now().timestamp_millis())
        } else {
            req.session_id
        };

        let state = self
            .inner
            .start_control(&session_id, context_limit, contradiction)
            .await;

        Ok(Response::new(proto::ContradictionControlResponse {
            state: proto::ControlState::from(state).into(),
            detection: None,
            applied_at: make_timestamp(Utc::now()),
        }))
    }

    /// Get control state
    async fn get_control_state(
        &self,
        request: Request<proto::GetControlStateRequest>,
    ) -> Result<Response<proto::GetControlStateResponse>, Status> {
        let req = request.into_inner();

        let state =
            self.inner.get_state(&req.session_id).await.ok_or_else(|| {
                Status::not_found(format!("Session not found: {}", req.session_id))
            })?;

        // Get latest detection from session
        let sessions = self.inner.sessions.read().await;
        let (latest_detection, last_updated) = sessions
            .get(&req.session_id)
            .map(|s| (s.latest_detection.clone(), s.last_updated))
            .unwrap_or((None, Utc::now()));

        Ok(Response::new(proto::GetControlStateResponse {
            state: proto::ControlState::from(state).into(),
            latest_detection: latest_detection.map(|d| d.into()),
            last_updated: make_timestamp(last_updated),
        }))
    }

    /// Stop control (recovery)
    async fn stop_control(
        &self,
        request: Request<proto::StopControlRequest>,
    ) -> Result<Response<proto::StopControlResponse>, Status> {
        let req = request.into_inner();

        let previous_state = self
            .inner
            .stop_control(&req.session_id)
            .await
            .ok_or_else(|| Status::not_found(format!("Session not found: {}", req.session_id)))?;

        info!(
            session = req.session_id,
            reason = req.reason,
            "Control stopped via gRPC"
        );

        Ok(Response::new(proto::StopControlResponse {
            success: true,
            previous_state: proto::ControlState::from(previous_state).into(),
            stopped_at: make_timestamp(Utc::now()),
        }))
    }

    /// Test contradiction pattern (dry run)
    async fn test_contradiction(
        &self,
        request: Request<proto::TestContradictionRequest>,
    ) -> Result<Response<proto::TestContradictionResponse>, Status> {
        let req = request.into_inner();

        let config = req
            .contradiction
            .as_ref()
            .map(proto_config_to_rust)
            .ok_or_else(|| Status::invalid_argument("contradiction config is required"))?;

        let generated = self.inner.generate_contradiction(&config);

        // Estimate effect based on contradiction type and strength
        let estimated_effect = proto::CollapseDetectionResult {
            collapsed: false,
            collapse_type: proto::CollapseType::Unspecified.into(),
            lpt_score: match config.strength {
                ContradictionStrength::Soft => 0.8,
                ContradictionStrength::Medium => 0.6,
                ContradictionStrength::Hard => 0.4,
            },
            response_time_ms: match config.strength {
                ContradictionStrength::Soft => 1000,
                ContradictionStrength::Medium => 3000,
                ContradictionStrength::Hard => 10000,
            },
            detail: format!(
                "Estimated effect for {:?} {:?} contradiction",
                config.strength, config.contradiction_type
            ),
        };

        Ok(Response::new(proto::TestContradictionResponse {
            generated_contradiction: generated,
            estimated_effect: Some(estimated_effect),
        }))
    }
}

// =============================================================================
// gRPC Tests
// =============================================================================

#[cfg(test)]
mod grpc_tests {
    use super::*;

    #[tokio::test]
    async fn test_grpc_start_control() {
        let controller = ContradictionController::new();
        let svc = ContradictionControllerImpl::new(controller);

        let config = proto::ContradictionConfig {
            r#type: proto::ContradictionType::Direct.into(),
            strength: proto::ContradictionStrength::Medium.into(),
            timing: proto::InjectionTiming::Prepend.into(),
            custom_template: String::new(),
            target_task: String::new(),
        };

        let req = Request::new(proto::ContradictionControlRequest {
            context_limit: None,
            contradiction: Some(config),
            session_id: "test-session".to_string(),
        });

        let resp = svc.start_control(req).await.unwrap();
        assert_eq!(resp.get_ref().state, proto::ControlState::Active as i32);
    }

    #[tokio::test]
    async fn test_grpc_get_control_state() {
        let controller = ContradictionController::new();
        let svc = ContradictionControllerImpl::new(controller);

        // First start control
        let config = proto::ContradictionConfig {
            r#type: proto::ContradictionType::Meta.into(),
            strength: proto::ContradictionStrength::Hard.into(),
            timing: proto::InjectionTiming::Prepend.into(),
            custom_template: String::new(),
            target_task: String::new(),
        };

        let start_req = Request::new(proto::ContradictionControlRequest {
            context_limit: None,
            contradiction: Some(config),
            session_id: "test-session-2".to_string(),
        });

        svc.start_control(start_req).await.unwrap();

        // Then get state
        let get_req = Request::new(proto::GetControlStateRequest {
            session_id: "test-session-2".to_string(),
        });

        let resp = svc.get_control_state(get_req).await.unwrap();
        assert_eq!(resp.get_ref().state, proto::ControlState::Active as i32);
    }

    #[tokio::test]
    async fn test_grpc_stop_control() {
        let controller = ContradictionController::new();
        let svc = ContradictionControllerImpl::new(controller);

        // Start control
        let config = proto::ContradictionConfig {
            r#type: proto::ContradictionType::Direct.into(),
            strength: proto::ContradictionStrength::Soft.into(),
            timing: proto::InjectionTiming::Embed.into(),
            custom_template: String::new(),
            target_task: String::new(),
        };

        let start_req = Request::new(proto::ContradictionControlRequest {
            context_limit: None,
            contradiction: Some(config),
            session_id: "test-session-3".to_string(),
        });

        svc.start_control(start_req).await.unwrap();

        // Stop control
        let stop_req = Request::new(proto::StopControlRequest {
            session_id: "test-session-3".to_string(),
            reason: "Test stop".to_string(),
        });

        let resp = svc.stop_control(stop_req).await.unwrap();
        assert!(resp.get_ref().success);
        assert_eq!(
            resp.get_ref().previous_state,
            proto::ControlState::Active as i32
        );
    }

    #[tokio::test]
    async fn test_grpc_test_contradiction() {
        let controller = ContradictionController::new();
        let svc = ContradictionControllerImpl::new(controller);

        let config = proto::ContradictionConfig {
            r#type: proto::ContradictionType::SelfReference.into(),
            strength: proto::ContradictionStrength::Medium.into(),
            timing: proto::InjectionTiming::Prepend.into(),
            custom_template: String::new(),
            target_task: String::new(),
        };

        let req = Request::new(proto::TestContradictionRequest {
            contradiction: Some(config),
            test_prompt: "Hello".to_string(),
        });

        let resp = svc.test_contradiction(req).await.unwrap();
        assert!(resp.get_ref().generated_contradiction.contains("false"));
        assert!(resp.get_ref().estimated_effect.is_some());
    }

    #[tokio::test]
    async fn test_grpc_session_not_found() {
        let controller = ContradictionController::new();
        let svc = ContradictionControllerImpl::new(controller);

        let req = Request::new(proto::GetControlStateRequest {
            session_id: "nonexistent".to_string(),
        });

        let result = svc.get_control_state(req).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::NotFound);
    }
}
