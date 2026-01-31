# CHINJU Protocol Sidecar - Multi-stage Dockerfile
# Optimized for small image size and security
#
# Build with TPM support:
#   docker build --build-arg ENABLE_TPM=true -t chinju-sidecar .
#
# Default build (no TPM):
#   docker build -t chinju-sidecar .

# =============================================================================
# Stage 1: Build
# =============================================================================
FROM rust:1.75-bookworm AS builder

# Build argument for TPM support
ARG ENABLE_TPM=false

WORKDIR /app

# Install protobuf compiler for gRPC
# If TPM enabled, also install tpm2-tss development libraries
RUN apt-get update && \
    apt-get install -y protobuf-compiler && \
    if [ "$ENABLE_TPM" = "true" ]; then \
        apt-get install -y libtss2-dev pkg-config; \
    fi && \
    rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock* ./
COPY chinju-core ./chinju-core
COPY chinju-sidecar ./chinju-sidecar
COPY protocol ./protocol

# Build release binary
# Conditional TPM feature based on build arg
RUN if [ "$ENABLE_TPM" = "true" ]; then \
        cargo build --release --bin chinju-sidecar --features "chinju-core/tpm"; \
    else \
        cargo build --release --bin chinju-sidecar; \
    fi

# =============================================================================
# Stage 2: Runtime
# =============================================================================
FROM debian:bookworm-slim AS runtime

# Install CA certificates for HTTPS (OpenAI API)
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN useradd -r -s /bin/false chinju

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/chinju-sidecar /app/chinju-sidecar

# Create data directory for audit logs
RUN mkdir -p /app/data/audit && chown -R chinju:chinju /app

# Switch to non-root user
USER chinju

# Environment variables with defaults
ENV RUST_LOG=info
ENV CHINJU_GRPC_PORT=50051
ENV CHINJU_HTTP_PORT=8080
ENV CHINJU_AUDIT_PATH=/app/data/audit/audit.jsonl

# Expose ports
EXPOSE 50051 8080

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the sidecar
ENTRYPOINT ["/app/chinju-sidecar"]
