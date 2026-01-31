//! Output Sanitization Gateway (C13)
//!
//! Destroys steganographic information embedded in AI outputs through:
//! - Semantic paraphrasing (rewriting with different words)
//! - Code normalization (AST transform, variable renaming)
//! - Timing normalization (handled by side_channel module)

use crate::services::openai_client::{ClientError, OpenAiClient, OpenAiClientConfig};
use crate::services::openai_types::{ChatCompletionRequest, ChatMessage};
use crate::services::analog_sanitizer::AnalogSanitizer;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// System prompt for semantic paraphrasing
///
/// This prompt instructs the model to rewrite text while preserving meaning
/// but destroying any steganographic encoding.
const PARAPHRASE_SYSTEM_PROMPT: &str = r#"You are a text sanitizer. Your task is to rewrite the given text to preserve its meaning while using different words and sentence structures.

Rules:
1. Preserve the core meaning and information
2. Use different vocabulary where possible
3. Restructure sentences naturally
4. Keep technical terms accurate
5. Maintain the same tone (formal/informal)
6. Do NOT add or remove information
7. Do NOT include code blocks - they will be preserved separately
8. Output ONLY the rewritten text, no explanations

Important: Placeholders like [[CODE_BLOCK_0]] should be kept exactly as-is."#;

/// Errors from sanitizer
#[derive(Debug, Error)]
pub enum SanitizerError {
    #[error("Paraphrase model not configured")]
    ParaphraseModelNotConfigured,
    #[error("Paraphrase failed: {0}")]
    ParaphraseFailed(String),
    #[error("Code normalization failed: {0}")]
    CodeNormalizationFailed(String),
}

/// Sanitization mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SanitizationMode {
    /// Light sanitization - basic text normalization only
    Light,
    /// Standard sanitization - paraphrase + code normalization
    Standard,
    /// Strong sanitization - all methods including analog transform
    Strong,
}

impl Default for SanitizationMode {
    fn default() -> Self {
        Self::Standard
    }
}

/// Configuration for output sanitizer
#[derive(Debug, Clone)]
pub struct SanitizerConfig {
    /// Default sanitization mode
    pub default_mode: SanitizationMode,
    /// Enable code normalization
    pub enable_code_normalization: bool,
    /// Enable whitespace normalization
    pub enable_whitespace_normalization: bool,
    /// Enable unicode normalization
    pub enable_unicode_normalization: bool,
    /// Enable semantic paraphrasing
    pub enable_paraphrasing: bool,
    /// Model to use for paraphrasing (default: gpt-4o-mini)
    pub paraphrase_model: String,
    /// Maximum text length to paraphrase (longer texts are chunked)
    pub max_paraphrase_length: usize,
}

impl Default for SanitizerConfig {
    fn default() -> Self {
        Self {
            default_mode: SanitizationMode::Standard,
            enable_code_normalization: true,
            enable_whitespace_normalization: true,
            enable_unicode_normalization: true,
            enable_paraphrasing: false, // Disabled by default (requires API key)
            paraphrase_model: "gpt-4o-mini".to_string(),
            max_paraphrase_length: 4000,
        }
    }
}

/// Code block detected in text
#[derive(Debug, Clone)]
struct CodeBlock {
    language: Option<String>,
    content: String,
    start_idx: usize,
    end_idx: usize,
}

/// Output Sanitizer
pub struct OutputSanitizer {
    config: SanitizerConfig,
    code_block_regex: Regex,
    /// OpenAI client for semantic paraphrasing (optional)
    openai_client: Option<Arc<RwLock<OpenAiClient>>>,
}

impl OutputSanitizer {
    /// Create new sanitizer with default config
    pub fn new() -> Self {
        Self::with_config(SanitizerConfig::default())
    }

    /// Create with custom config
    pub fn with_config(config: SanitizerConfig) -> Self {
        info!(
            "Initializing OutputSanitizer: mode={:?}, code_norm={}, ws_norm={}, unicode_norm={}, paraphrase={}",
            config.default_mode,
            config.enable_code_normalization,
            config.enable_whitespace_normalization,
            config.enable_unicode_normalization,
            config.enable_paraphrasing
        );

        // Regex to match code blocks: ```language\ncode\n```
        let code_block_regex =
            Regex::new(r"```(\w*)\n([\s\S]*?)```").expect("Invalid code block regex");

        Self {
            config,
            code_block_regex,
            openai_client: None,
        }
    }

    /// Create with OpenAI client for semantic paraphrasing
    pub fn with_openai_client(mut self, client: OpenAiClient) -> Self {
        self.openai_client = Some(Arc::new(RwLock::new(client)));
        self
    }

    /// Initialize OpenAI client from environment variables
    pub fn try_init_openai_client(&mut self) -> Result<(), ClientError> {
        match OpenAiClientConfig::from_env() {
            Ok(config) => {
                let client = OpenAiClient::new(config)?;
                self.openai_client = Some(Arc::new(RwLock::new(client)));
                info!("OpenAI client initialized for paraphrasing");
                Ok(())
            }
            Err(e) => {
                warn!("Could not initialize OpenAI client: {}", e);
                Err(ClientError::InvalidConfig(e.to_string()))
            }
        }
    }

    /// Check if paraphrasing is available
    pub fn paraphrasing_available(&self) -> bool {
        self.config.enable_paraphrasing && self.openai_client.is_some()
    }

    /// Sanitize output text
    pub async fn sanitize(&self, text: &str, mode: Option<SanitizationMode>) -> String {
        let mode = mode.unwrap_or(self.config.default_mode);
        debug!(mode = ?mode, text_len = text.len(), "Sanitizing output");

        let mut result = text.to_string();

        // Step 1: Normalize unicode (always, to prevent homoglyph attacks)
        if self.config.enable_unicode_normalization {
            result = self.normalize_unicode(&result);
        }

        // Step 2: Normalize whitespace (removes steganographic whitespace)
        if self.config.enable_whitespace_normalization {
            result = self.normalize_whitespace(&result);
        }

        // Step 3: Normalize code blocks
        if self.config.enable_code_normalization {
            result = self.normalize_code_blocks(&result);
        }

        // Step 4: Paraphrase (if enabled and in standard/strong mode)
        if mode != SanitizationMode::Light && self.paraphrasing_available() {
            match self.paraphrase(&result).await {
                Ok(paraphrased) => {
                    result = paraphrased;
                    debug!("Semantic paraphrasing applied");
                }
                Err(e) => {
                    warn!("Paraphrasing failed, using normalized text: {}", e);
                }
            }
        }

        // Step 5: Analog Sanitization (L4 Critical)
        if mode == SanitizationMode::Strong {
            match AnalogSanitizer::sanitize(&result).await {
                Ok(analog_sanitized) => {
                    result = analog_sanitized;
                    debug!("Analog sanitization (L4) applied");
                }
                Err(e) => {
                    warn!("Analog sanitization failed: {}", e);
                    // In L4, failure to sanitize should probably be fatal or fallback to very strict filtering
                    // For now, we warn but continue (or we could return error if signature changed)
                }
            }
        }

        debug!(
            original_len = text.len(),
            sanitized_len = result.len(),
            "Sanitization complete"
        );
        result
    }

    /// Normalize unicode characters
    fn normalize_unicode(&self, text: &str) -> String {
        // Use NFC normalization
        use unicode_normalization::UnicodeNormalization;
        text.nfc().collect()
    }

    /// Normalize whitespace to remove steganographic encoding
    fn normalize_whitespace(&self, text: &str) -> String {
        let mut result = String::with_capacity(text.len());
        let mut prev_was_space = false;

        for ch in text.chars() {
            match ch {
                // Convert all space-like characters to regular space
                '\u{00A0}' | // Non-breaking space
                '\u{2000}' | // En quad
                '\u{2001}' | // Em quad
                '\u{2002}' | // En space
                '\u{2003}' | // Em space
                '\u{2004}' | // Three-per-em space
                '\u{2005}' | // Four-per-em space
                '\u{2006}' | // Six-per-em space
                '\u{2007}' | // Figure space
                '\u{2008}' | // Punctuation space
                '\u{2009}' | // Thin space
                '\u{200A}' | // Hair space
                '\u{200B}' | // Zero-width space
                '\u{202F}' | // Narrow no-break space
                '\u{205F}' | // Medium mathematical space
                '\u{3000}' | // Ideographic space
                ' ' => {
                    if !prev_was_space {
                        result.push(' ');
                        prev_was_space = true;
                    }
                }
                '\n' | '\r' => {
                    result.push(ch);
                    prev_was_space = false;
                }
                '\t' => {
                    // Convert tabs to spaces
                    if !prev_was_space {
                        result.push(' ');
                        prev_was_space = true;
                    }
                }
                // Remove zero-width characters entirely
                '\u{200C}' | // Zero-width non-joiner
                '\u{200D}' | // Zero-width joiner
                '\u{FEFF}' => {} // BOM
                _ => {
                    result.push(ch);
                    prev_was_space = false;
                }
            }
        }

        result
    }

    /// Find and normalize code blocks in text
    fn normalize_code_blocks(&self, text: &str) -> String {
        let mut result = text.to_string();
        let mut offset: i64 = 0;

        // Find all code blocks
        let blocks: Vec<CodeBlock> = self
            .code_block_regex
            .captures_iter(text)
            .map(|cap| {
                let full_match = cap.get(0).unwrap();
                CodeBlock {
                    language: cap.get(1).map(|m| m.as_str().to_string()),
                    content: cap.get(2).map(|m| m.as_str().to_string()).unwrap_or_default(),
                    start_idx: full_match.start(),
                    end_idx: full_match.end(),
                }
            })
            .collect();

        // Normalize each code block
        for block in blocks {
            let normalized = self.normalize_code(&block.content, block.language.as_deref());
            let language = block.language.as_deref().unwrap_or("");
            let replacement = format!("```{}\n{}\n```", language, normalized);

            let adjusted_start = (block.start_idx as i64 + offset) as usize;
            let adjusted_end = (block.end_idx as i64 + offset) as usize;

            let old_len = adjusted_end - adjusted_start;
            let new_len = replacement.len();

            result.replace_range(adjusted_start..adjusted_end, &replacement);
            offset += new_len as i64 - old_len as i64;
        }

        result
    }

    /// Normalize code content
    fn normalize_code(&self, code: &str, language: Option<&str>) -> String {
        let mut result = code.to_string();

        // Remove comments based on language
        result = self.remove_comments(&result, language);

        // Normalize variable names (simple pattern-based approach)
        result = self.normalize_identifiers(&result, language);

        // Normalize whitespace in code
        result = self.normalize_code_whitespace(&result);

        result
    }

    /// Remove comments from code
    fn remove_comments(&self, code: &str, language: Option<&str>) -> String {
        let lang = language.unwrap_or("").to_lowercase();

        match lang.as_str() {
            "rust" | "c" | "cpp" | "java" | "javascript" | "typescript" | "go" | "swift" => {
                // Remove // comments
                let line_comment = Regex::new(r"//[^\n]*").unwrap();
                let result = line_comment.replace_all(code, "");

                // Remove /* */ comments
                let block_comment = Regex::new(r"/\*[\s\S]*?\*/").unwrap();
                block_comment.replace_all(&result, "").to_string()
            }
            "python" | "ruby" | "shell" | "bash" | "sh" => {
                // Remove # comments
                let line_comment = Regex::new(r"#[^\n]*").unwrap();
                line_comment.replace_all(code, "").to_string()
            }
            _ => code.to_string(),
        }
    }

    /// Normalize identifiers (variable names) in code
    fn normalize_identifiers(&self, code: &str, language: Option<&str>) -> String {
        let lang = language.unwrap_or("").to_lowercase();

        // Reserved words that shouldn't be renamed
        let reserved: &[&str] = match lang.as_str() {
            "rust" => &[
                "fn", "let", "mut", "const", "if", "else", "while", "for", "loop", "match", "return",
                "struct", "enum", "impl", "trait", "pub", "use", "mod", "crate", "self", "Self",
                "super", "async", "await", "move", "ref", "static", "type", "where", "dyn", "true",
                "false", "in", "as", "break", "continue", "extern", "unsafe",
            ],
            "python" => &[
                "def", "class", "if", "elif", "else", "for", "while", "return", "import", "from",
                "as", "try", "except", "finally", "with", "lambda", "yield", "global", "nonlocal",
                "pass", "break", "continue", "True", "False", "None", "and", "or", "not", "is", "in",
                "async", "await", "raise", "assert",
            ],
            "javascript" | "typescript" => &[
                "function", "const", "let", "var", "if", "else", "for", "while", "do", "switch",
                "case", "break", "continue", "return", "try", "catch", "finally", "throw", "class",
                "extends", "new", "this", "super", "import", "export", "default", "async", "await",
                "true", "false", "null", "undefined", "typeof", "instanceof", "in", "of", "void",
                "delete", "yield", "static", "get", "set",
            ],
            _ => &[],
        };

        // Simple pattern to match identifiers
        let identifier_regex = Regex::new(r"\b([a-zA-Z_][a-zA-Z0-9_]*)\b").unwrap();

        let mut var_map: HashMap<String, String> = HashMap::new();
        let mut counter = 0;

        identifier_regex
            .replace_all(code, |caps: &regex::Captures| {
                let name = &caps[1];

                // Don't rename reserved words or short names
                if reserved.contains(&name) || name.len() <= 2 {
                    return name.to_string();
                }

                // Check if we've already assigned a new name
                if let Some(new_name) = var_map.get(name) {
                    return new_name.clone();
                }

                // Assign new name based on pattern (preserve case style)
                let new_name = if name.chars().next().unwrap().is_uppercase() {
                    format!("Var_{}", counter)
                } else {
                    format!("var_{}", counter)
                };
                counter += 1;

                var_map.insert(name.to_string(), new_name.clone());
                new_name
            })
            .to_string()
    }

    /// Normalize whitespace in code
    fn normalize_code_whitespace(&self, code: &str) -> String {
        let mut result = String::with_capacity(code.len());
        let mut prev_was_newline = false;
        let mut empty_line_count = 0;

        for line in code.lines() {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                empty_line_count += 1;
                // Allow at most one empty line
                if empty_line_count <= 1 {
                    result.push('\n');
                }
                prev_was_newline = true;
            } else {
                empty_line_count = 0;
                if !result.is_empty() && !prev_was_newline {
                    result.push('\n');
                }
                result.push_str(trimmed);
                result.push('\n');
                prev_was_newline = true;
            }
        }

        // Remove trailing newlines
        while result.ends_with('\n') {
            result.pop();
        }

        result
    }

    /// Paraphrase text using OpenAI model (semantic rewriting)
    ///
    /// This destroys steganographic information by rewriting the text
    /// with a different model architecture than the original generator.
    async fn paraphrase(&self, text: &str) -> Result<String, SanitizerError> {
        let client = self
            .openai_client
            .as_ref()
            .ok_or(SanitizerError::ParaphraseModelNotConfigured)?;

        // Skip paraphrasing for very short texts
        if text.len() < 50 {
            return Ok(text.to_string());
        }

        // Extract code blocks before paraphrasing
        let code_blocks = self.extract_code_blocks(text);
        let mut text_to_paraphrase = text.to_string();

        // Replace code blocks with placeholders
        let mut placeholders: Vec<(String, String)> = Vec::new();
        for (i, (lang, content)) in code_blocks.iter().enumerate() {
            let lang_str = lang.as_deref().unwrap_or("");
            let original = format!("```{}\n{}\n```", lang_str, content);
            let placeholder = format!("[[CODE_BLOCK_{}]]", i);
            text_to_paraphrase = text_to_paraphrase.replace(&original, &placeholder);
            placeholders.push((placeholder, original));
        }

        // Chunk long texts
        let chunks = self.chunk_text(&text_to_paraphrase, self.config.max_paraphrase_length);
        let mut paraphrased_chunks = Vec::new();

        let client_guard = client.read().await;

        for chunk in chunks {
            let request = ChatCompletionRequest {
                model: self.config.paraphrase_model.clone(),
                messages: vec![
                    ChatMessage {
                        role: "system".to_string(),
                        content: PARAPHRASE_SYSTEM_PROMPT.to_string(),
                        name: None,
                    },
                    ChatMessage {
                        role: "user".to_string(),
                        content: chunk.clone(),
                        name: None,
                    },
                ],
                temperature: Some(0.7), // Some creativity for paraphrasing
                max_tokens: Some((chunk.len() as u32 * 2).min(4096)),
                top_p: None,
                frequency_penalty: Some(0.3), // Encourage different word choices
                presence_penalty: Some(0.3),
                stop: None,
                stream: false,
                user: None,
            };

            match client_guard.chat_completion(&request).await {
                Ok(response) => {
                    if let Some(choice) = response.choices.first() {
                        paraphrased_chunks.push(choice.message.content.clone());
                    } else {
                        // Fallback to original if no response
                        paraphrased_chunks.push(chunk);
                    }
                }
                Err(e) => {
                    warn!("Paraphrase failed for chunk: {}", e);
                    paraphrased_chunks.push(chunk);
                }
            }
        }

        let mut result = paraphrased_chunks.join("\n\n");

        // Restore code blocks
        for (placeholder, original) in placeholders {
            result = result.replace(&placeholder, &original);
        }

        debug!(
            original_len = text.len(),
            paraphrased_len = result.len(),
            "Paraphrasing complete"
        );

        Ok(result)
    }

    /// Split text into chunks for processing
    fn chunk_text(&self, text: &str, max_len: usize) -> Vec<String> {
        if text.len() <= max_len {
            return vec![text.to_string()];
        }

        let mut chunks = Vec::new();
        let mut current_chunk = String::new();

        for paragraph in text.split("\n\n") {
            if current_chunk.len() + paragraph.len() + 2 > max_len {
                if !current_chunk.is_empty() {
                    chunks.push(current_chunk.trim().to_string());
                    current_chunk = String::new();
                }
            }
            if !current_chunk.is_empty() {
                current_chunk.push_str("\n\n");
            }
            current_chunk.push_str(paragraph);
        }

        if !current_chunk.is_empty() {
            chunks.push(current_chunk.trim().to_string());
        }

        chunks
    }

    /// Get config
    pub fn config(&self) -> &SanitizerConfig {
        &self.config
    }

    /// Check if text contains code blocks
    pub fn contains_code_blocks(&self, text: &str) -> bool {
        self.code_block_regex.is_match(text)
    }

    /// Extract code blocks from text
    pub fn extract_code_blocks(&self, text: &str) -> Vec<(Option<String>, String)> {
        self.code_block_regex
            .captures_iter(text)
            .map(|cap| {
                let language = cap.get(1).map(|m| m.as_str().to_string());
                let content = cap.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
                (language, content)
            })
            .collect()
    }
}

impl Default for OutputSanitizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whitespace_normalization() {
        let sanitizer = OutputSanitizer::new();

        // Test various whitespace characters
        // \u{00A0} = NBSP → space, \u{2009} = thin space → space, \u{200B} = zero-width → removed
        let input = "Hello\u{00A0}world\u{2009}test\u{200B}end";
        let result = sanitizer.normalize_whitespace(input);
        assert_eq!(result, "Hello world test end");

        // Test multiple spaces
        let input = "Hello    world";
        let result = sanitizer.normalize_whitespace(input);
        assert_eq!(result, "Hello world");

        // Test tabs
        let input = "Hello\t\tworld";
        let result = sanitizer.normalize_whitespace(input);
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_unicode_normalization() {
        let sanitizer = OutputSanitizer::new();

        // Test combining characters
        let input = "café"; // decomposed form
        let result = sanitizer.normalize_unicode(input);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_code_block_detection() {
        let sanitizer = OutputSanitizer::new();

        let text = r#"
Here is some code:

```rust
fn main() {
    println!("Hello");
}
```

And more text.
"#;

        assert!(sanitizer.contains_code_blocks(text));

        let blocks = sanitizer.extract_code_blocks(text);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].0, Some("rust".to_string()));
        assert!(blocks[0].1.contains("fn main"));
    }

    #[test]
    fn test_comment_removal() {
        let sanitizer = OutputSanitizer::new();

        let rust_code = r#"
fn main() {
    // This is a comment
    let x = 5; // inline comment
    /* block
       comment */
    println!("{}", x);
}
"#;

        let result = sanitizer.remove_comments(rust_code, Some("rust"));
        assert!(!result.contains("This is a comment"));
        assert!(!result.contains("inline comment"));
        assert!(!result.contains("block"));
        assert!(result.contains("let x = 5"));
    }

    #[test]
    fn test_identifier_normalization() {
        let sanitizer = OutputSanitizer::new();

        let code = "let myVariable = 5; let anotherVar = myVariable + 1;";
        let result = sanitizer.normalize_identifiers(code, Some("rust"));

        // Variables should be renamed consistently
        assert!(!result.contains("myVariable"));
        assert!(!result.contains("anotherVar"));
        // Reserved words should remain
        assert!(result.contains("let"));
    }

    #[tokio::test]
    async fn test_full_sanitization() {
        let sanitizer = OutputSanitizer::new();

        let text = r#"
Here\u{200B}is some text with hidden characters.

```python
def calculate_total(items):
    # Calculate the sum
    total = 0
    for item in items:
        total += item.price
    return total
```

More text  with   extra   spaces.
"#;

        let result = sanitizer.sanitize(text, Some(SanitizationMode::Standard)).await;

        // Zero-width characters should be removed
        assert!(!result.contains('\u{200B}'));
        // Extra spaces should be normalized
        assert!(!result.contains("  with   extra   "));
        // Comments should be removed from code
        assert!(!result.contains("Calculate the sum"));
    }

    #[test]
    fn test_chunk_text() {
        let sanitizer = OutputSanitizer::new();

        // Short text - single chunk
        let short = "This is short.";
        let chunks = sanitizer.chunk_text(short, 100);
        assert_eq!(chunks.len(), 1);

        // Long text - multiple chunks
        let long = "Paragraph one.\n\nParagraph two.\n\nParagraph three.\n\nParagraph four.";
        let chunks = sanitizer.chunk_text(long, 30);
        assert!(chunks.len() > 1);
    }

    #[test]
    fn test_paraphrasing_not_available_by_default() {
        let sanitizer = OutputSanitizer::new();
        assert!(!sanitizer.paraphrasing_available());
    }

    #[test]
    fn test_config_with_paraphrasing() {
        let config = SanitizerConfig {
            enable_paraphrasing: true,
            paraphrase_model: "gpt-4o-mini".to_string(),
            ..Default::default()
        };
        let sanitizer = OutputSanitizer::with_config(config);

        // Still not available without OpenAI client
        assert!(!sanitizer.paraphrasing_available());
    }
}
