//! CHINJU Protocol Sidecar
//!
//! AI Gateway server that enforces the CHINJU Protocol.
//!
//! ## Features
//! - C5: Survival Token management
//! - C6: Audit logging with hash chain
//! - C9: Policy enforcement
//! - C12: Human credential verification
//! - P3: OpenAI API compatible HTTP server
//!
//! ## Usage
//!
//! ```bash
//! # Mock mode (no OpenAI API key required)
//! cargo run --bin chinju-sidecar
//!
//! # With OpenAI API (set in .env file or environment)
//! # Create .env file with: OPENAI_API_KEY=sk-xxx
//! cargo run --bin chinju-sidecar
//! ```

use chinju_sidecar::gen::chinju::api::credential::credential_service_server::CredentialServiceServer;
use chinju_sidecar::gen::chinju::api::gateway::ai_gateway_service_server::AiGatewayServiceServer;
use chinju_sidecar::services::{
    create_audit_system_with_restore, CredentialServiceImpl, FileStorage, GatewayService,
    HttpServerState, OpenAiClient, OpenAiClientConfig, PolicyEngine, SigningService,
    StorageBackend, TokenService, start_http_server,
};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::transport::Server;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env file if present (before any other initialization)
    dotenvy::dotenv().ok();

    // Initialize logging
    let _subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(true)
        .with_thread_ids(false)
        .compact()
        .init();

    println!(
        r#"
    ╔═══════════════════════════════════════════════════════════╗
    ║                                                           ║
    ║   ██████╗██╗  ██╗██╗███╗   ██╗     ██╗██╗   ██╗           ║
    ║  ██╔════╝██║  ██║██║████╗  ██║     ██║██║   ██║           ║
    ║  ██║     ███████║██║██╔██╗ ██║     ██║██║   ██║           ║
    ║  ██║     ██╔══██║██║██║╚██╗██║██   ██║██║   ██║           ║
    ║  ╚██████╗██║  ██║██║██║ ╚████║╚█████╔╝╚██████╔╝           ║
    ║   ╚═════╝╚═╝  ╚═╝╚═╝╚═╝  ╚═══╝ ╚════╝  ╚═════╝            ║
    ║                                                           ║
    ║   CHINJU Protocol Sidecar v0.1.0                         ║
    ║   AI Safety & Governance Gateway                          ║
    ║                                                           ║
    ║   Features:                                               ║
    ║   • C5:  Survival Token Management                        ║
    ║   • C6:  Audit Logging (Hash Chain)                       ║
    ║   • C9:  Policy Engine                                    ║
    ║   • C12: Human Credential Verification                    ║
    ║   • P3:  OpenAI API Compatible Proxy                      ║
    ║                                                           ║
    ╚═══════════════════════════════════════════════════════════╝
    "#
    );

    // Initialize services
    info!("Initializing services...");

    // C5: Token Service
    let token_service = Arc::new(RwLock::new(TokenService::new(10000)));
    info!("  Token Service initialized (balance: 10000)");

    // Phase 4.4: Signing Service (hardware-backed if configured)
    let signing_service = match SigningService::from_env() {
        Ok(svc) => {
            info!(
                "  Signing Service initialized (trust level: {:?})",
                svc.trust_level()
            );
            Some(svc)
        }
        Err(e) => {
            info!("  Signing Service using mock ({})", e);
            Some(SigningService::mock())
        }
    };

    // C12: Credential Service with hardware-backed signing
    let credential_service = Arc::new(if let Some(svc) = signing_service {
        CredentialServiceImpl::with_signing_service(svc)
    } else {
        CredentialServiceImpl::new()
    });
    info!("  Credential Service initialized");

    // C9: Policy Engine
    let policy_engine = Arc::new(PolicyEngine::new());
    info!("  Policy Engine initialized (2 policies loaded)");

    // C6: Audit Service
    let audit_path = std::env::var("CHINJU_AUDIT_PATH")
        .unwrap_or_else(|_| "./data/audit/audit.jsonl".to_string());
    let archive_dir = std::env::var("CHINJU_AUDIT_ARCHIVE")
        .unwrap_or_else(|_| "./data/audit/archive".to_string());

    let audit_storage: Arc<dyn StorageBackend> =
        Arc::new(FileStorage::new(PathBuf::from(&audit_path), PathBuf::from(&archive_dir))?);

    let (audit_logger, audit_persister) =
        create_audit_system_with_restore(audit_storage, "chinju-sidecar-001", 10000).await?;

    // Start audit persister in background
    tokio::spawn(audit_persister.run());
    info!("  Audit Service initialized (path: {})", audit_path);

    // P3: OpenAI Client (optional)
    let openai_client = match OpenAiClientConfig::from_env() {
        Ok(config) if config.is_valid() => {
            match OpenAiClient::new(config) {
                Ok(client) => {
                    info!("  OpenAI Client configured (API mode)");
                    Some(Arc::new(client))
                }
                Err(e) => {
                    warn!("  OpenAI Client failed to initialize: {}", e);
                    None
                }
            }
        }
        _ => {
            info!("  OpenAI Client not configured (mock mode)");
            info!("    Set OPENAI_API_KEY to enable real API calls");
            None
        }
    };

    // AI Gateway Service (integrates all services)
    let gateway_service = if let Some(ref client) = openai_client {
        GatewayService::with_openai_client(
            token_service,
            credential_service.clone(),
            policy_engine,
            audit_logger.clone(),
            client.clone(),
        ).await
    } else {
        GatewayService::with_audit_logger(
            token_service,
            credential_service.clone(),
            policy_engine,
            audit_logger.clone(),
        ).await
    };
    info!("  Gateway Service initialized");

    // Server addresses
    let grpc_port: u16 = std::env::var("CHINJU_GRPC_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(50051);
    let http_port: u16 = std::env::var("CHINJU_HTTP_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);

    let grpc_addr: SocketAddr = format!("[::1]:{}", grpc_port).parse()?;
    let http_addr: SocketAddr = format!("0.0.0.0:{}", http_port).parse()?;

    let trust_level = if openai_client.is_some() {
        "SOFTWARE (OpenAI API Mode)"
    } else {
        "MOCK (Development Mode)"
    };

    info!("Starting CHINJU Sidecar");
    info!("  gRPC Server: {}", grpc_addr);
    info!("  HTTP Server: {} (OpenAI compatible)", http_addr);
    info!("  Trust Level: {}", trust_level);

    // Create HTTP server state
    let http_state = Arc::new(HttpServerState::new(openai_client, audit_logger));

    // Start HTTP server in background
    let http_handle = tokio::spawn(async move {
        if let Err(e) = start_http_server(http_addr, http_state).await {
            tracing::error!("HTTP server error: {}", e);
        }
    });

    // Start gRPC server
    let grpc_handle = tokio::spawn(async move {
        if let Err(e) = Server::builder()
            .add_service(AiGatewayServiceServer::new(gateway_service))
            .add_service(CredentialServiceServer::new(
                credential_service.as_ref().clone(),
            ))
            .serve(grpc_addr)
            .await
        {
            tracing::error!("gRPC server error: {}", e);
        }
    });

    info!("CHINJU Sidecar is ready!");
    info!("");
    info!("Test endpoints:");
    info!("  curl http://localhost:{}/health", http_port);
    info!("  curl http://localhost:{}/v1/models", http_port);
    info!("");

    // Wait for either server to finish
    tokio::select! {
        _ = http_handle => {
            warn!("HTTP server stopped");
        }
        _ = grpc_handle => {
            warn!("gRPC server stopped");
        }
    }

    Ok(())
}
