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
    // Initialize logging
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();

    info!("CHINJU Enclave starting...");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));

    // Get port from environment or use default
    let port: u32 = std::env::var("ENCLAVE_PORT")
        .unwrap_or_else(|_| "5000".to_string())
        .parse()
        .expect("Invalid ENCLAVE_PORT");

    let config = ServerConfig {
        port,
        backlog: 128,
    };

    let server = VsockServer::new(config);

    info!("Starting vsock server on port {}", port);

    if let Err(e) = server.run() {
        eprintln!("Server error: {}", e);
        std::process::exit(1);
    }
}
