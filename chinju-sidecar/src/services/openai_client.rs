//! OpenAI API client
//!
//! Provides a client for calling the OpenAI API with support for
//! both regular and streaming responses.

use crate::services::openai_types::*;
use futures_util::Stream;
use reqwest::{header, Client};
use std::pin::Pin;
use tracing::{debug, warn};

/// OpenAI client configuration
#[derive(Debug, Clone)]
pub struct OpenAiClientConfig {
    /// API key
    pub api_key: String,
    /// Base URL (default: https://api.openai.com/v1)
    pub base_url: String,
    /// Organization ID (optional)
    pub organization_id: Option<String>,
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    /// Maximum retries
    pub max_retries: u32,
}

impl Default for OpenAiClientConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://api.openai.com/v1".to_string(),
            organization_id: None,
            timeout_seconds: 60,
            max_retries: 3,
        }
    }
}

impl OpenAiClientConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, ConfigError> {
        let api_key =
            std::env::var("OPENAI_API_KEY").map_err(|_| ConfigError::MissingApiKey)?;

        let base_url = std::env::var("OPENAI_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());

        let organization_id = std::env::var("OPENAI_ORG_ID").ok();

        let timeout_seconds = std::env::var("OPENAI_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(60);

        Ok(Self {
            api_key,
            base_url,
            organization_id,
            timeout_seconds,
            max_retries: 3,
        })
    }

    /// Check if the configuration is valid
    pub fn is_valid(&self) -> bool {
        !self.api_key.is_empty()
    }
}

/// OpenAI API client
pub struct OpenAiClient {
    client: Client,
    config: OpenAiClientConfig,
}

impl OpenAiClient {
    /// Create a new client
    pub fn new(config: OpenAiClientConfig) -> Result<Self, ClientError> {
        let mut headers = header::HeaderMap::new();

        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {}", config.api_key))
                .map_err(|_| ClientError::InvalidConfig("Invalid API key format".into()))?,
        );

        if let Some(ref org_id) = config.organization_id {
            headers.insert(
                "OpenAI-Organization",
                header::HeaderValue::from_str(org_id)
                    .map_err(|_| ClientError::InvalidConfig("Invalid org ID".into()))?,
            );
        }

        let client = Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| ClientError::HttpClientError(e.to_string()))?;

        Ok(Self { client, config })
    }

    /// Send a chat completion request (non-streaming)
    pub async fn chat_completion(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, ClientError> {
        let url = format!("{}/chat/completions", self.config.base_url);

        debug!(
            model = %request.model,
            messages = request.messages.len(),
            "Sending chat completion request"
        );

        let response = self
            .client
            .post(&url)
            .json(request)
            .send()
            .await
            .map_err(|e| ClientError::RequestFailed(e.to_string()))?;

        let status = response.status();

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(self.parse_error(status.as_u16(), &error_body));
        }

        response
            .json()
            .await
            .map_err(|e| ClientError::ParseError(e.to_string()))
    }

    /// Send a chat completion request with streaming
    pub async fn chat_completion_stream(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk, ClientError>> + Send>>, ClientError>
    {
        let url = format!("{}/chat/completions", self.config.base_url);

        let mut stream_request = request.clone();
        stream_request.stream = true;

        debug!(
            model = %request.model,
            "Starting streaming chat completion"
        );

        let response = self
            .client
            .post(&url)
            .json(&stream_request)
            .send()
            .await
            .map_err(|e| ClientError::RequestFailed(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(self.parse_error(status.as_u16(), &error_body));
        }

        // Parse SSE stream
        let stream = async_stream::stream! {
            use futures_util::StreamExt;

            let mut byte_stream = response.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk) = byte_stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));

                        // Process SSE events
                        while let Some(pos) = buffer.find("\n\n") {
                            let event = buffer[..pos].to_string();
                            buffer = buffer[pos + 2..].to_string();

                            for line in event.lines() {
                                if let Some(data) = line.strip_prefix("data: ") {
                                    if data == "[DONE]" {
                                        return;
                                    }

                                    match serde_json::from_str::<ChatCompletionChunk>(data) {
                                        Ok(chunk) => yield Ok(chunk),
                                        Err(e) => {
                                            warn!(error = %e, data = %data, "Failed to parse SSE chunk");
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(ClientError::StreamError(e.to_string()));
                        return;
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }

    /// List available models
    pub async fn list_models(&self) -> Result<ModelsResponse, ClientError> {
        let url = format!("{}/models", self.config.base_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ClientError::RequestFailed(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(self.parse_error(status.as_u16(), &error_body));
        }

        response
            .json()
            .await
            .map_err(|e| ClientError::ParseError(e.to_string()))
    }

    /// Parse error response
    fn parse_error(&self, status: u16, body: &str) -> ClientError {
        if let Ok(error) = serde_json::from_str::<OpenAiError>(body) {
            ClientError::OpenAiError {
                status,
                message: error.error.message,
                error_type: error.error.error_type,
            }
        } else {
            ClientError::UnexpectedError {
                status,
                body: body.to_string(),
            }
        }
    }
}

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("OPENAI_API_KEY environment variable not set")]
    MissingApiKey,
}

/// Client errors
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("HTTP client error: {0}")]
    HttpClientError(String),

    #[error("Request failed: {0}")]
    RequestFailed(String),

    #[error("OpenAI API error (status {status}): {message} (type: {error_type})")]
    OpenAiError {
        status: u16,
        message: String,
        error_type: String,
    },

    #[error("Unexpected error (status {status}): {body}")]
    UnexpectedError { status: u16, body: String },

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Stream error: {0}")]
    StreamError(String),
}

impl ClientError {
    /// Convert to HTTP status code for proxying
    pub fn to_status_code(&self) -> u16 {
        match self {
            ClientError::OpenAiError { status, .. } => *status,
            ClientError::UnexpectedError { status, .. } => *status,
            ClientError::InvalidConfig(_) => 400,
            ClientError::HttpClientError(_) => 502,
            ClientError::RequestFailed(_) => 502,
            ClientError::ParseError(_) => 500,
            ClientError::StreamError(_) => 500,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = OpenAiClientConfig::default();
        assert_eq!(config.base_url, "https://api.openai.com/v1");
        assert!(!config.is_valid());
    }

    #[test]
    fn test_config_valid() {
        let config = OpenAiClientConfig {
            api_key: "sk-test".to_string(),
            ..Default::default()
        };
        assert!(config.is_valid());
    }

    #[test]
    fn test_error_status_code() {
        let error = ClientError::OpenAiError {
            status: 429,
            message: "Rate limit".to_string(),
            error_type: "rate_limit_error".to_string(),
        };
        assert_eq!(error.to_status_code(), 429);
    }
}
