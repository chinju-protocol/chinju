#!/bin/bash
#
# CHINJU Protocol - Nitro Enclave vsock Communication Test
#
# This script runs on the EC2 parent instance to test vsock
# communication with the running Nitro Enclave.
#
# Prerequisites:
#   1. Enclave must be running (nitro-cli describe-enclaves)
#   2. Rust toolchain installed
#
# Usage:
#   ./scripts/test-nitro-vsock.sh [CID]
#
#   CID: Enclave CID (auto-detected if not specified)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_header() {
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}========================================${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Check if running on EC2 with Nitro Enclaves
check_nitro_environment() {
    print_header "Checking Nitro Enclave Environment"

    if ! command -v nitro-cli &> /dev/null; then
        print_error "nitro-cli not found. Are you running on a Nitro-enabled EC2 instance?"
        exit 1
    fi
    print_success "nitro-cli found"

    # Check if Enclave is running
    ENCLAVE_INFO=$(nitro-cli describe-enclaves 2>/dev/null || echo "[]")
    
    if [ "$ENCLAVE_INFO" = "[]" ]; then
        print_error "No Enclave running. Start one with:"
        echo "  nitro-cli run-enclave --eif-path chinju-enclave.eif --memory 512 --cpu-count 2"
        exit 1
    fi
    print_success "Enclave is running"

    # Extract CID
    if [ -z "$1" ]; then
        ENCLAVE_CID=$(echo "$ENCLAVE_INFO" | jq -r '.[0].EnclaveCID')
        if [ -z "$ENCLAVE_CID" ] || [ "$ENCLAVE_CID" = "null" ]; then
            print_error "Could not detect Enclave CID"
            exit 1
        fi
    else
        ENCLAVE_CID="$1"
    fi
    
    print_success "Enclave CID: $ENCLAVE_CID"
    echo
}

# Build the test binary
build_test() {
    print_header "Building Test Binary"
    
    cd "$PROJECT_ROOT"
    
    echo "Building chinju-core with nitro feature..."
    cargo build --release --package chinju-core --example nitro_test --features nitro
    
    print_success "Build completed"
    echo
}

# Run the test
run_test() {
    print_header "Running vsock Communication Test"
    
    cd "$PROJECT_ROOT"
    
    export CHINJU_NITRO_ENCLAVE_CID="$ENCLAVE_CID"
    export CHINJU_NITRO_PORT="${CHINJU_NITRO_PORT:-5000}"
    export CHINJU_NITRO_DEBUG="${CHINJU_NITRO_DEBUG:-true}"
    export RUST_LOG="${RUST_LOG:-info,chinju_core=debug}"
    
    echo "Environment:"
    echo "  CHINJU_NITRO_ENCLAVE_CID=$CHINJU_NITRO_ENCLAVE_CID"
    echo "  CHINJU_NITRO_PORT=$CHINJU_NITRO_PORT"
    echo "  CHINJU_NITRO_DEBUG=$CHINJU_NITRO_DEBUG"
    echo

    ./target/release/examples/nitro_test
}

# Show Enclave info
show_enclave_info() {
    print_header "Enclave Information"
    nitro-cli describe-enclaves | jq '.'
    echo
}

# Main
main() {
    echo
    print_header "CHINJU Nitro Enclave vsock Test"
    echo

    # Check environment
    check_nitro_environment "$1"

    # Show Enclave info
    show_enclave_info

    # Build
    build_test

    # Run test
    run_test

    echo
    print_success "All tests completed!"
}

main "$@"
