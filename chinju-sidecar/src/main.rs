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

use chinju_core::hardware::{
    physical_kill_switch, DeadMansSwitchConfig, HardwareConfig, SoftDeadMansSwitch,
};
use chinju_core::types::TrustLevel;
use chinju_sidecar::gen::chinju::api::capability::capability_evaluator_server::CapabilityEvaluatorServer;
use chinju_sidecar::gen::chinju::api::contradiction::contradiction_controller_server::ContradictionControllerServer;
use chinju_sidecar::gen::chinju::api::credential::credential_service_server::CredentialServiceServer;
use chinju_sidecar::gen::chinju::api::gateway::ai_gateway_service_server::AiGatewayServiceServer;
use chinju_sidecar::gen::chinju::api::survival_attention::survival_attention_service_server::SurvivalAttentionServiceServer;
use chinju_sidecar::gen::chinju::api::value_neuron::value_neuron_monitor_server::ValueNeuronMonitorServer;
use chinju_sidecar::services::{
    create_audit_system_with_restore,
    create_router,
    // C14-C17 services
    CapabilityEvaluator,
    CapabilityEvaluatorImpl,
    ContradictionController,
    ContradictionControllerImpl,
    CredentialServiceImpl,
    FileStorage,
    GatewayService,
    HttpServerState,
    OpenAiClient,
    OpenAiClientConfig,
    PolicyEngine,
    SanitizationMode,
    SigningService,
    StorageBackend,
    SurvivalAttentionService,
    SurvivalAttentionServiceImpl,
    TokenService,
    ValueNeuronMonitor,
    ValueNeuronMonitorImpl,
};
use chinju_sidecar::ChinjuConfig;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::transport::Server;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

fn load_env_file() {
    // 1) Explicit env file path (best for CI/ops consistency)
    if let Ok(path) = std::env::var("CHINJU_ENV_FILE") {
        let _ = dotenvy::from_path(path);
        return;
    }

    // 2) Standard behavior: current directory or its parents
    if dotenvy::dotenv().is_ok() {
        return;
    }

    // 3) Monorepo root fallback when running from repository top
    let _ = dotenvy::from_path("chinju-sidecar/.env");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env file if present (before any other initialization)
    load_env_file();

    // Validate global configuration early to fail fast on inconsistent settings.
    let config = ChinjuConfig::from_env();
    if let Err(e) = config.validate() {
        return Err(format!("Invalid CHINJU configuration: {}", e).into());
    }

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

    let audit_storage: Arc<dyn StorageBackend> = Arc::new(FileStorage::new(
        PathBuf::from(&audit_path),
        PathBuf::from(&archive_dir),
    )?);

    let (audit_logger, audit_persister) =
        create_audit_system_with_restore(audit_storage, "chinju-sidecar-001", 10000).await?;

    // Start audit persister in background
    tokio::spawn(audit_persister.run());
    info!("  Audit Service initialized (path: {})", audit_path);

    // P3: OpenAI Client (optional)
    let openai_client = match OpenAiClientConfig::from_env() {
        Ok(config) if config.is_valid() => match OpenAiClient::new(config) {
            Ok(client) => {
                info!("  OpenAI Client configured (API mode)");
                Some(Arc::new(client))
            }
            Err(e) => {
                warn!("  OpenAI Client failed to initialize: {}", e);
                None
            }
        },
        _ => {
            info!("  OpenAI Client not configured (mock mode)");
            info!("    Set OPENAI_API_KEY to enable real API calls");
            None
        }
    };

    // AI Gateway Service (integrates all services)
    // Initialize Hardware & Dead Man's Switch (C13)
    let hardware_config = HardwareConfig::from_env().unwrap_or_else(|e| {
        warn!("Failed to load hardware config: {}, using mock", e);
        HardwareConfig::mock()
    });

    let dead_mans_switch: Arc<dyn chinju_core::hardware::DeadMansSwitch> = {
        let config = DeadMansSwitchConfig::default();
        Arc::new(SoftDeadMansSwitch::new(config))
    };

    // L4 Critical: Register physical kill switch
    if hardware_config.security_level() == TrustLevel::HardwareCritical {
        info!("SECURITY LEVEL: L4 CRITICAL - Physical Kill Switch Enabled");
        dead_mans_switch.on_emergency(Arc::new(|| {
            tokio::spawn(async {
                physical_kill_switch::execute_physical_shutdown().await;
            });
        }));
    }

    // Configure Containment (C13)
    let mut containment_config = config.containment.clone();

    // L4 Critical: Force Analog Sanitization
    if hardware_config.security_level() == TrustLevel::HardwareCritical {
        info!("SECURITY LEVEL: L4 CRITICAL - Analog Sanitization Enabled");
        containment_config.sanitization_mode = SanitizationMode::Strong;
        containment_config.sanitizer_config.default_mode = SanitizationMode::Strong;
    }

    let gateway_service = GatewayService::with_containment(
        token_service,
        credential_service.clone(),
        policy_engine,
        Some(audit_logger.clone()),
        openai_client.clone(),
        containment_config,
        dead_mans_switch,
    )
    .await;
    info!("  Gateway Service initialized");

    // C14: Capability Evaluator
    let capability_evaluator = CapabilityEvaluator::new();
    let capability_service = CapabilityEvaluatorImpl::new(capability_evaluator);
    info!("  Capability Evaluator (C14) initialized");

    // C16: Contradiction Controller
    let contradiction_controller = ContradictionController::new();
    let contradiction_service = ContradictionControllerImpl::new(contradiction_controller);
    info!("  Contradiction Controller (C16) initialized");

    // C15: Value Neuron Monitor
    let value_neuron_monitor = ValueNeuronMonitor::new();
    let value_neuron_service = ValueNeuronMonitorImpl::new(value_neuron_monitor);
    info!("  Value Neuron Monitor (C15) initialized");

    // C17: Survival Attention
    let survival_attention = SurvivalAttentionService::new();
    let survival_attention_service = SurvivalAttentionServiceImpl::new(survival_attention);
    info!("  Survival Attention Service (C17) initialized");

    // Server addresses
    let grpc_port = config.server.grpc_port;
    let http_port = config.server.http_port;

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

    let (shutdown_tx, _) = tokio::sync::broadcast::channel::<()>(1);

    // Start HTTP server in background (graceful shutdown enabled)
    let mut http_shutdown_rx = shutdown_tx.subscribe();
    let http_handle = tokio::spawn(async move {
        let app = create_router(http_state);
        let listener = match tokio::net::TcpListener::bind(http_addr).await {
            Ok(listener) => listener,
            Err(e) => {
                tracing::error!("HTTP server bind error: {}", e);
                return;
            }
        };

        let server = axum::serve(listener, app).with_graceful_shutdown(async move {
            let _ = http_shutdown_rx.recv().await;
        });

        if let Err(e) = server.await {
            tracing::error!("HTTP server error: {}", e);
        }
    });

    // Start gRPC server (graceful shutdown enabled)
    let mut grpc_shutdown_rx = shutdown_tx.subscribe();
    let grpc_handle = tokio::spawn(async move {
        if let Err(e) = Server::builder()
            .add_service(AiGatewayServiceServer::new(gateway_service))
            .add_service(CredentialServiceServer::new(
                credential_service.as_ref().clone(),
            ))
            // C14-C17 services
            .add_service(CapabilityEvaluatorServer::new(capability_service))
            .add_service(ContradictionControllerServer::new(contradiction_service))
            .add_service(ValueNeuronMonitorServer::new(value_neuron_service))
            .add_service(SurvivalAttentionServiceServer::new(
                survival_attention_service,
            ))
            .serve_with_shutdown(grpc_addr, async move {
                let _ = grpc_shutdown_rx.recv().await;
            })
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

    let mut http_handle = http_handle;
    let mut grpc_handle = grpc_handle;

    // Wait for signal or unexpected server stop
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Shutdown signal received");
        }
        res = &mut http_handle => {
            if let Err(e) = res {
                warn!("HTTP server task join error: {}", e);
            } else {
                warn!("HTTP server stopped");
            }
        }
        res = &mut grpc_handle => {
            if let Err(e) = res {
                warn!("gRPC server task join error: {}", e);
            } else {
                warn!("gRPC server stopped");
            }
        }
    }

    // Trigger graceful shutdown and wait for both servers.
    let _ = shutdown_tx.send(());
    if let Err(e) = http_handle.await {
        warn!("HTTP server shutdown join error: {}", e);
    }
    if let Err(e) = grpc_handle.await {
        warn!("gRPC server shutdown join error: {}", e);
    }

    info!("CHINJU Sidecar shutdown complete");

    Ok(())
}
