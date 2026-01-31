//! CHINJU Nitro Enclave Application Library
//!
//! This crate implements the Enclave-side of the CHINJU Protocol's
//! secure execution environment using AWS Nitro Enclaves.
//!
//! # Architecture
//!
//! The Enclave application runs inside an isolated environment and
//! communicates with the parent EC2 instance via vsock.
//!
//! ```text
//! Parent EC2 Instance         Nitro Enclave
//! +------------------+        +------------------+
//! | chinju-sidecar   |        | chinju-enclave   |
//! |                  |        |                  |
//! | NitroEnclaveClient  <--vsock-->  VsockServer |
//! |                  |        |                  |
//! +------------------+        | NSM API          |
//!                             | Key Manager      |
//!                             +------------------+
//! ```

pub mod handlers;
pub mod key_manager;
pub mod kms_client;
pub mod nsm;
pub mod protocol;

pub use kms_client::{KmsClient, KmsClientConfig};

use handlers::RequestHandler;
use protocol::EnclaveRequest;
use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, error, info, trace};

/// Enclave application errors
#[derive(Debug, Error)]
pub enum EnclaveError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("NSM error: {0}")]
    NsmError(String),

    #[error("Key manager error: {0}")]
    KeyManagerError(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Socket error: {0}")]
    SocketError(String),
}

/// vsock server configuration
pub struct ServerConfig {
    /// vsock port to listen on
    pub port: u32,
    /// Connection backlog
    pub backlog: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 5000,
            backlog: 128,
        }
    }
}

/// vsock server for handling parent requests
pub struct VsockServer {
    config: ServerConfig,
    handler: Arc<RequestHandler>,
}

impl VsockServer {
    /// Create a new vsock server
    pub fn new(config: ServerConfig) -> Self {
        let handler = Arc::new(RequestHandler::new());
        Self { config, handler }
    }

    /// Run the server (blocking)
    pub fn run(&self) -> Result<(), EnclaveError> {
        use nix::sys::socket::{
            accept, bind, listen, socket, AddressFamily, SockFlag, SockType, VsockAddr,
        };

        info!("Starting vsock server on port {}", self.config.port);

        // Create vsock socket
        let socket_fd = socket(
            AddressFamily::Vsock,
            SockType::Stream,
            SockFlag::empty(),
            None,
        )
        .map_err(|e: nix::Error| EnclaveError::SocketError(e.to_string()))?;

        // Bind to VMADDR_CID_ANY (accept connections from any CID)
        let addr = VsockAddr::new(nix::libc::VMADDR_CID_ANY, self.config.port);
        bind(socket_fd.as_raw_fd(), &addr)
            .map_err(|e: nix::Error| EnclaveError::SocketError(e.to_string()))?;

        // Listen for connections
        listen(&socket_fd, self.config.backlog)
            .map_err(|e: nix::Error| EnclaveError::SocketError(e.to_string()))?;

        info!("Server listening for connections...");

        // Accept connections in a loop
        loop {
            match accept(socket_fd.as_raw_fd()) {
                Ok(client_fd) => {
                    debug!("Accepted connection");
                    let handler = self.handler.clone();

                    // Handle connection in a new thread
                    std::thread::spawn(move || {
                        if let Err(e) = handle_connection(client_fd, handler) {
                            error!("Connection handler error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Accept error: {}", e);
                }
            }
        }
    }
}

/// Handle a single client connection
fn handle_connection(fd: RawFd, handler: Arc<RequestHandler>) -> Result<(), EnclaveError> {
    // Wrap fd in a File for Read/Write traits
    let mut stream = unsafe { std::fs::File::from_raw_fd(fd) };

    loop {
        // Read request length (4 bytes, big-endian)
        let mut len_buf = [0u8; 4];
        match stream.read_exact(&mut len_buf) {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                debug!("Client disconnected");
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        }
        let request_len = u32::from_be_bytes(len_buf) as usize;
        trace!("Receiving {} byte request", request_len);

        // Read request body
        let mut request_bytes = vec![0u8; request_len];
        stream.read_exact(&mut request_bytes)?;

        // Deserialize request
        let request: EnclaveRequest = serde_cbor::from_slice(&request_bytes)
            .map_err(|e| EnclaveError::SerializationError(e.to_string()))?;

        // Handle request
        let response = handler.handle(request);

        // Serialize response
        let response_bytes = serde_cbor::to_vec(&response)
            .map_err(|e| EnclaveError::SerializationError(e.to_string()))?;

        trace!("Sending {} byte response", response_bytes.len());

        // Send response length
        let len = (response_bytes.len() as u32).to_be_bytes();
        stream.write_all(&len)?;
        stream.write_all(&response_bytes)?;
        stream.flush()?;
    }
}
