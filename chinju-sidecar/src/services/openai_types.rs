//! OpenAI API compatible type definitions
//!
//! These types mirror the OpenAI API for compatibility with existing applications.

use serde::{Deserialize, Serialize};

/// Chat completion request (OpenAI compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    /// Model identifier
    pub model: String,

    /// Messages in the conversation
    pub messages: Vec<ChatMessage>,

    /// Sampling temperature (0.0 - 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    /// Maximum tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Top-p sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,

    /// Frequency penalty
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,

    /// Presence penalty
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,

    /// Stop sequences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<StopSequence>,

    /// Whether to stream the response
    #[serde(default)]
    pub stream: bool,

    /// User identifier for abuse detection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Role: "system", "user", "assistant"
    pub role: String,

    /// Message content
    pub content: String,

    /// Optional name for the participant
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Stop sequence (single or multiple)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StopSequence {
    Single(String),
    Multiple(Vec<String>),
}

impl StopSequence {
    pub fn to_vec(&self) -> Vec<String> {
        match self {
            StopSequence::Single(s) => vec![s.clone()],
            StopSequence::Multiple(v) => v.clone(),
        }
    }
}

/// Chat completion response (OpenAI compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    /// Response ID
    pub id: String,

    /// Object type: "chat.completion"
    pub object: String,

    /// Unix timestamp of creation
    pub created: i64,

    /// Model used
    pub model: String,

    /// Response choices
    pub choices: Vec<Choice>,

    /// Token usage statistics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,

    /// System fingerprint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
}

/// Response choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    /// Choice index
    pub index: u32,

    /// Message content
    pub message: ChatMessage,

    /// Reason for stopping
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    /// Tokens in the prompt
    pub prompt_tokens: u32,

    /// Tokens in the completion
    pub completion_tokens: u32,

    /// Total tokens
    pub total_tokens: u32,
}

/// Streaming chunk (SSE format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChunk {
    /// Chunk ID
    pub id: String,

    /// Object type: "chat.completion.chunk"
    pub object: String,

    /// Unix timestamp of creation
    pub created: i64,

    /// Model used
    pub model: String,

    /// Choices with deltas
    pub choices: Vec<ChunkChoice>,
}

/// Streaming choice delta
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkChoice {
    /// Choice index
    pub index: u32,

    /// Delta content
    pub delta: Delta,

    /// Reason for finishing (only in final chunk)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// Delta content in streaming
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Delta {
    /// Role (only in first chunk)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,

    /// Content fragment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// Models list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsResponse {
    /// Object type: "list"
    pub object: String,

    /// Available models
    pub data: Vec<Model>,
}

/// Model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    /// Model ID
    pub id: String,

    /// Object type: "model"
    pub object: String,

    /// Unix timestamp of creation
    pub created: i64,

    /// Owner
    pub owned_by: String,
}

/// OpenAI API error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiError {
    pub error: OpenAiErrorDetail,
}

/// Error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiErrorDetail {
    /// Error message
    pub message: String,

    /// Error type
    #[serde(rename = "type")]
    pub error_type: String,

    /// Parameter that caused the error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param: Option<String>,

    /// Error code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

impl OpenAiError {
    pub fn new(message: impl Into<String>, error_type: impl Into<String>) -> Self {
        Self {
            error: OpenAiErrorDetail {
                message: message.into(),
                error_type: error_type.into(),
                param: None,
                code: None,
            },
        }
    }

    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(message, "invalid_request_error")
    }

    pub fn authentication_error(message: impl Into<String>) -> Self {
        Self::new(message, "authentication_error")
    }

    pub fn rate_limit_error(message: impl Into<String>) -> Self {
        Self::new(message, "rate_limit_error")
    }

    pub fn server_error(message: impl Into<String>) -> Self {
        Self::new(message, "server_error")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_request_serialization() {
        let request = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: "You are a helpful assistant.".to_string(),
                    name: None,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: "Hello!".to_string(),
                    name: None,
                },
            ],
            temperature: Some(0.7),
            max_tokens: Some(100),
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            stream: false,
            user: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("gpt-4"));
        assert!(json.contains("temperature"));

        let parsed: ChatCompletionRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.model, "gpt-4");
        assert_eq!(parsed.messages.len(), 2);
    }

    #[test]
    fn test_chat_response_serialization() {
        let response = ChatCompletionResponse {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion".to_string(),
            created: 1677652288,
            model: "gpt-4".to_string(),
            choices: vec![Choice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: "Hello! How can I help you?".to_string(),
                    name: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(Usage {
                prompt_tokens: 10,
                completion_tokens: 8,
                total_tokens: 18,
            }),
            system_fingerprint: Some("fp_123".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        let parsed: ChatCompletionResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "chatcmpl-123");
        assert_eq!(parsed.choices[0].finish_reason, Some("stop".to_string()));
    }

    #[test]
    fn test_stop_sequence() {
        let single = StopSequence::Single("stop".to_string());
        assert_eq!(single.to_vec(), vec!["stop"]);

        let multiple = StopSequence::Multiple(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(multiple.to_vec(), vec!["a", "b"]);
    }
}
