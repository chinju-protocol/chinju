//! CHINJU Nitro Enclave Application
//!
//! This binary runs inside the Nitro Enclave and provides:
//! - Cryptographic key management
//! - Data sealing/unsealing
//! - Remote attestation
//!
//! Communication with the parent EC2 instance is via vsock.

use chinju_enclave::{ServerConfig, VsockServer};
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

fn main() {
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // Initialize logging
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(env_filter)
        .init();

    info!("CHINJU Enclave starting...");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));

    // Get port from environment or use default
    let port: u32 = match std::env::var("ENCLAVE_PORT") {
        Ok(value) => match value.parse() {
            Ok(port) => port,
            Err(e) => {
                eprintln!("Invalid ENCLAVE_PORT '{}': {}", value, e);
                std::process::exit(2);
            }
        },
        Err(_) => 5000,
    };

    let config = ServerConfig { port, backlog: 128 };

    let server = VsockServer::new(config);

    info!("Starting vsock server on port {}", port);

    if let Err(e) = server.run() {
        eprintln!("Server error: {}", e);
        std::process::exit(1);
    }
}
