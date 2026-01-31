# CHINJU Sidecar

AI Gateway and Policy Enforcement Service for the CHINJU Protocol.

## Overview

The CHINJU Sidecar provides a secure proxy layer between AI applications and AI model backends (OpenAI, etc.). It implements the CHINJU Protocol's security features:

- **C5: Survival Token** - Token-based resource control
- **C6: Audit Trail** - Comprehensive logging with tamper detection
- **C9: Policy Engine** - Configurable policy evaluation
- **C11: LPT Monitor** - LLM quality degradation detection
- **C12: Human Credential** - Human verification
- **C13: Model Containment** - Extraction deterrent, output sanitization, side-channel blocking, Dead Man's Switch

## Quick Start

### Running with Docker

```bash
# Development mode (mock AI)
docker-compose up

# With OpenAI API
OPENAI_API_KEY=sk-xxx docker-compose up
```

### Running locally

```bash
# Build
cargo build --release

# Run (mock mode)
cargo run --release --bin chinju-sidecar

# Run with OpenAI
OPENAI_API_KEY=sk-xxx cargo run --release --bin chinju-sidecar
```

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | `info` | Log level (trace, debug, info, warn, error) |
| `CHINJU_GRPC_PORT` | `50051` | gRPC server port |
| `CHINJU_HTTP_PORT` | `8080` | HTTP server port (OpenAI compatible) |
| `CHINJU_AUDIT_PATH` | `./audit.jsonl` | Audit log file path |
| `OPENAI_API_KEY` | - | OpenAI API key (optional, enables real API) |
| `OPENAI_BASE_URL` | `https://api.openai.com/v1` | OpenAI API base URL |

### C13 Model Containment

| Variable | Default | Description |
|----------|---------|-------------|
| `CHINJU_C13_EXTRACTION_DETERRENT` | `true` | Enable extraction deterrent |
| `CHINJU_C13_OUTPUT_SANITIZATION` | `true` | Enable output sanitization |
| `CHINJU_C13_SIDE_CHANNEL_BLOCKING` | `true` | Enable side-channel blocking |
| `CHINJU_C13_DEAD_MANS_SWITCH` | `true` | Enable Dead Man's Switch |
| `CHINJU_C13_SANITIZATION_MODE` | `Standard` | Sanitization mode (Light, Standard, Strong) |
| `CHINJU_C13_PARAPHRASE_ENABLED` | `false` | Enable semantic paraphrasing |
| `CHINJU_C13_PARAPHRASE_MODEL` | `gpt-4o-mini` | Model for paraphrasing |

### Dead Man's Switch

| Variable | Default | Description |
|----------|---------|-------------|
| `CHINJU_DMS_HEARTBEAT_INTERVAL` | `30` | Heartbeat interval in seconds |
| `CHINJU_DMS_HEARTBEAT_TIMEOUT` | `90` | Heartbeat timeout in seconds |
| `CHINJU_DMS_MIN_TEMPERATURE` | `0.0` | Min temperature (Celsius) |
| `CHINJU_DMS_MAX_TEMPERATURE` | `50.0` | Max temperature (Celsius) |
| `CHINJU_DMS_MAX_ACCELERATION` | `1.0` | Max acceleration (G) |
| `CHINJU_DMS_GRACE_PERIOD` | `10` | Grace period in seconds |

## API Endpoints

### HTTP (OpenAI Compatible)

```
POST /v1/chat/completions  - Chat completion (OpenAI compatible)
GET  /v1/models            - List available models
GET  /health               - Health check
GET  /metrics              - Prometheus metrics
```

### gRPC

```
ProcessRequest       - Process AI request
ProcessRequestStream - Streaming response
ValidateRequest      - Validate request before processing
GetAIStatus          - Get system status
EmergencyHalt        - Emergency halt (requires threshold signature)
ResumeFromHalt       - Resume from halt (requires threshold signature)
```

## CLI Tool

```bash
# Check status
cargo run --bin chinju-cli -- status

# Send a request
cargo run --bin chinju-cli -- ask "Hello, world"

# Stream response
cargo run --bin chinju-cli -- stream "Tell me a story"

# View audit logs
cargo run --bin chinju-cli -- audit --last 10

# Check health
cargo run --bin chinju-cli -- health

# View metrics
cargo run --bin chinju-cli -- metrics
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Client Application                        │
└─────────────────────────┬───────────────────────────────────┘
                          │ HTTP / gRPC
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                   CHINJU Sidecar                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │   Gateway    │──│  Credential  │──│   Policy     │       │
│  │   Service    │  │   Service    │  │   Engine     │       │
│  └──────┬───────┘  └──────────────┘  └──────────────┘       │
│         │                                                    │
│  ┌──────┴───────┐  ┌──────────────┐  ┌──────────────┐       │
│  │ C13 Containment                                   │       │
│  │ - Extraction Deterrent                            │       │
│  │ - Output Sanitizer                                │       │
│  │ - Side Channel Blocker                            │       │
│  │ - Dead Man's Switch                               │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                    AI Model Backend                          │
│              (OpenAI, Anthropic, Local LLM)                  │
└─────────────────────────────────────────────────────────────┘
```

## Testing

```bash
# Run all tests
cargo test

# Run C13 integration tests
cargo test --test c13_integration

# Run with coverage
cargo tarpaulin --out Html
```

## License

Apache 2.0
