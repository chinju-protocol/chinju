//! OpenAI API compatible HTTP server
//!
//! Provides REST endpoints that mirror the OpenAI API, allowing
//! existing applications to use CHINJU Protocol protection without modification.
//!
//! Endpoints:
//! - POST /v1/chat/completions - OpenAI compatible chat
//! - GET  /v1/models - List available models
//! - GET  /health - Health check
//! - GET  /metrics - Prometheus metrics

use crate::ids::{CredentialId, RequestId};
use crate::services::audit::AuditLogger;
use crate::services::metrics::MetricsCollector;
use crate::services::openai_client::{ClientError, OpenAiClient};
use crate::services::openai_types::*;
use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::{IntoResponse, Response, Sse},
    routing::{get, post},
    Router,
};
use futures_util::StreamExt;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// HTTP server state
pub struct HttpServerState {
    /// OpenAI client (optional - None means mock mode)
    pub openai_client: Option<Arc<OpenAiClient>>,
    /// Audit logger
    pub audit_logger: Arc<AuditLogger>,
    /// Request counter
    pub request_count: Arc<RwLock<u64>>,
    /// Metrics collector
    pub metrics: Arc<MetricsCollector>,
}

impl HttpServerState {
    pub fn new(openai_client: Option<Arc<OpenAiClient>>, audit_logger: Arc<AuditLogger>) -> Self {
        Self {
            openai_client,
            audit_logger,
            request_count: Arc::new(RwLock::new(0)),
            metrics: Arc::new(MetricsCollector::new()),
        }
    }

    /// Create with custom metrics collector
    pub fn with_metrics(
        openai_client: Option<Arc<OpenAiClient>>,
        audit_logger: Arc<AuditLogger>,
        metrics: Arc<MetricsCollector>,
    ) -> Self {
        Self {
            openai_client,
            audit_logger,
            request_count: Arc::new(RwLock::new(0)),
            metrics,
        }
    }
}

/// Create the HTTP router
pub fn create_router(state: Arc<HttpServerState>) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/models", get(list_models))
        .route("/health", get(health_check))
        .route("/metrics", get(metrics_handler))
        .route("/", get(root))
        .with_state(state)
}

/// Prometheus metrics endpoint
async fn metrics_handler(State(state): State<Arc<HttpServerState>>) -> impl IntoResponse {
    let metrics = state.metrics.render_metrics().await;
    (
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; charset=utf-8",
        )],
        metrics,
    )
}

/// Start the HTTP server
pub async fn start_http_server(
    addr: std::net::SocketAddr,
    state: Arc<HttpServerState>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = create_router(state);

    info!(addr = %addr, "Starting OpenAI-compatible HTTP server");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// CHINJU authentication extracted from headers
#[derive(Debug)]
struct ChinjuAuth {
    credential_id: Option<CredentialId>,
    capability_score: Option<f64>,
}

impl ChinjuAuth {
    fn from_headers(headers: &axum::http::HeaderMap) -> Result<Self, AppError> {
        let credential_id = headers
            .get("X-Chinju-Credential-Id")
            .and_then(|v| v.to_str().ok())
            .map(|raw| CredentialId::new(raw.to_string()))
            .transpose()
            .map_err(|e| AppError::bad_request(e.to_string()))?;

        let capability_score = headers
            .get("X-Chinju-Capability-Score")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok());

        Ok(Self {
            credential_id,
            capability_score,
        })
    }
}

/// POST /v1/chat/completions
async fn chat_completions(
    State(state): State<Arc<HttpServerState>>,
    headers: axum::http::HeaderMap,
    Json(request): Json<ChatCompletionRequest>,
) -> Result<Response, AppError> {
    let auth = ChinjuAuth::from_headers(&headers)?;
    let request_id = RequestId::new(uuid::Uuid::new_v4().to_string())
        .expect("uuid-based request id must be valid");

    // Increment request count
    {
        let mut count = state.request_count.write().await;
        *count += 1;
    }

    info!(
        request_id = %request_id,
        model = %request.model,
        stream = request.stream,
        credential_id = ?auth.credential_id,
        "Received chat completion request"
    );

    // Serialize request for audit hash
    let request_bytes = serde_json::to_vec(&request).unwrap_or_default();

    // Log request to audit
    let audit_result = state
        .audit_logger
        .log_ai_request(
            &request_id,
            auth.credential_id.as_ref(),
            auth.capability_score,
            &request_bytes,
            &request.model,
        )
        .await;

    if let Err(e) = &audit_result {
        warn!(error = %e, "Failed to log AI request to audit");
    }

    // Check if we have OpenAI client configured
    let Some(client) = &state.openai_client else {
        // Mock mode
        return Ok(mock_response(&request_id, &request).into_response());
    };

    if request.stream {
        // Streaming response
        match client.chat_completion_stream(&request).await {
            Ok(stream) => {
                let sse_stream = stream.map(|result| match result {
                    Ok(chunk) => {
                        let data = serde_json::to_string(&chunk).unwrap_or_default();
                        Ok::<_, Infallible>(axum::response::sse::Event::default().data(data))
                    }
                    Err(e) => {
                        let error = OpenAiError::server_error(e.to_string());
                        let data = serde_json::to_string(&error).unwrap_or_default();
                        Ok(axum::response::sse::Event::default().data(data))
                    }
                });

                // Add final [DONE] event
                let final_stream = async_stream::stream! {
                    for await event in sse_stream {
                        yield event;
                    }
                    yield Ok::<_, Infallible>(axum::response::sse::Event::default().data("[DONE]"));
                };

                Ok(Sse::new(final_stream)
                    .keep_alive(axum::response::sse::KeepAlive::default())
                    .into_response())
            }
            Err(e) => {
                error!(error = %e, "OpenAI streaming request failed");
                Err(AppError::from_client_error(e))
            }
        }
    } else {
        // Non-streaming response
        let start_time = std::time::Instant::now();

        match client.chat_completion(&request).await {
            Ok(response) => {
                let duration_ms = start_time.elapsed().as_millis() as u64;
                let tokens_consumed = response
                    .usage
                    .as_ref()
                    .map(|u| u.total_tokens as u64)
                    .unwrap_or(0);

                // Serialize response for audit hash
                let response_bytes = serde_json::to_vec(&response).unwrap_or_default();

                // Log response to audit
                let _ = state
                    .audit_logger
                    .log_ai_response(
                        &request_id,
                        &response_bytes,
                        "allow",
                        &[],
                        tokens_consumed,
                        duration_ms,
                        true,
                    )
                    .await;

                debug!(
                    request_id = %request_id,
                    tokens = tokens_consumed,
                    duration_ms = duration_ms,
                    "Chat completion successful"
                );

                Ok(Json(response).into_response())
            }
            Err(e) => {
                error!(error = %e, "OpenAI request failed");

                // Log failure to audit
                let _ = state
                    .audit_logger
                    .log_ai_response(&request_id, b"error", "error", &[], 0, 0, false)
                    .await;

                Err(AppError::from_client_error(e))
            }
        }
    }
}

/// GET /v1/models
async fn list_models(
    State(state): State<Arc<HttpServerState>>,
) -> Result<Json<ModelsResponse>, AppError> {
    if let Some(client) = &state.openai_client {
        match client.list_models().await {
            Ok(models) => Ok(Json(models)),
            Err(e) => Err(AppError::from_client_error(e)),
        }
    } else {
        // Return mock models in mock mode
        Ok(Json(ModelsResponse {
            object: "list".to_string(),
            data: vec![
                Model {
                    id: "gpt-4".to_string(),
                    object: "model".to_string(),
                    created: chrono::Utc::now().timestamp(),
                    owned_by: "openai".to_string(),
                },
                Model {
                    id: "gpt-3.5-turbo".to_string(),
                    object: "model".to_string(),
                    created: chrono::Utc::now().timestamp(),
                    owned_by: "openai".to_string(),
                },
            ],
        }))
    }
}

/// GET /health
async fn health_check() -> &'static str {
    "OK"
}

/// GET /
async fn root() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "name": "CHINJU Sidecar",
        "version": env!("CARGO_PKG_VERSION"),
        "openai_compatible": true,
        "endpoints": {
            "chat_completions": "/v1/chat/completions",
            "models": "/v1/models",
            "health": "/health"
        }
    }))
}

/// Generate mock response for testing without OpenAI API key
fn mock_response(
    request_id: &RequestId,
    request: &ChatCompletionRequest,
) -> Json<ChatCompletionResponse> {
    let content = format!(
        "[CHINJU Mock Response]\n\
         Model: {}\n\
         Messages received: {}\n\
         This is a mock response. Set OPENAI_API_KEY to enable real API calls.\n\
         CHINJU Protocol ensures safe AI operation through:\n\
         - Human credential verification\n\
         - Token-based resource control\n\
         - Policy enforcement\n\
         - Comprehensive audit logging",
        request.model,
        request.messages.len()
    );

    Json(ChatCompletionResponse {
        id: format!("chatcmpl-chinju-{}", request_id),
        object: "chat.completion".to_string(),
        created: chrono::Utc::now().timestamp(),
        model: request.model.clone(),
        choices: vec![Choice {
            index: 0,
            message: ChatMessage {
                role: "assistant".to_string(),
                content,
                name: None,
            },
            finish_reason: Some("stop".to_string()),
        }],
        usage: Some(Usage {
            prompt_tokens: 50,
            completion_tokens: 100,
            total_tokens: 150,
        }),
        system_fingerprint: Some("chinju-sidecar-mock".to_string()),
    })
}

/// Application error type
#[derive(Debug)]
pub struct AppError {
    status: StatusCode,
    body: OpenAiError,
}

impl AppError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        let body = OpenAiError::new(message.into(), "invalid_request_error");
        Self {
            status: StatusCode::BAD_REQUEST,
            body,
        }
    }

    pub fn from_client_error(error: ClientError) -> Self {
        let status = match error.to_status_code() {
            401 => StatusCode::UNAUTHORIZED,
            403 => StatusCode::FORBIDDEN,
            429 => StatusCode::TOO_MANY_REQUESTS,
            500..=599 => StatusCode::BAD_GATEWAY,
            code => StatusCode::from_u16(code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        };

        let body = match &error {
            ClientError::OpenAiError {
                message,
                error_type,
                ..
            } => OpenAiError::new(message, error_type),
            _ => OpenAiError::server_error(error.to_string()),
        };

        Self { status, body }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            body: OpenAiError::invalid_request(err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::audit::{create_audit_system, FileStorage};
    use axum::body::Body;
    use axum::http::{header, Request};
    use tempfile::TempDir;
    use tower::ServiceExt;

    fn create_test_state() -> Arc<HttpServerState> {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("audit.jsonl");
        let archive_dir = temp_dir.path().join("archive");

        let storage: Arc<dyn crate::services::audit::StorageBackend> =
            Arc::new(FileStorage::new(log_path, archive_dir).unwrap());

        let (logger, _persister) = create_audit_system(storage, "test-sidecar", 100);

        Arc::new(HttpServerState::new(None, logger))
    }

    #[tokio::test]
    async fn test_health_check() {
        let state = create_test_state();
        let app = create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_root() {
        let state = create_test_state();
        let app = create_router(state);

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_models_mock() {
        let state = create_test_state();
        let app = create_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/models")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_chat_completions_mock() {
        let state = create_test_state();
        let app = create_router(state);

        let request_body = serde_json::json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Hello!"}
            ]
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/chat/completions")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_chat_completions_rejects_invalid_credential_header() {
        let state = create_test_state();
        let app = create_router(state);

        let request_body = serde_json::json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Hello!"}
            ]
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/chat/completions")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header("X-Chinju-Credential-Id", "invalid header value")
                    .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
