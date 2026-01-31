#!/bin/bash
#
# CHINJU Protocol - TPM Integration Test Script
#
# This script runs the TPM integration tests using Docker Compose.
# It starts swtpm (software TPM) and runs the test suite.
#
# Usage:
#   ./scripts/test-tpm.sh          # Run all TPM tests
#   ./scripts/test-tpm.sh shell    # Interactive shell for debugging
#   ./scripts/test-tpm.sh clean    # Clean up containers

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_ROOT"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_header() {
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}$1${NC}"
    echo -e "${GREEN}========================================${NC}"
}

print_warning() {
    echo -e "${YELLOW}WARNING: $1${NC}"
}

print_error() {
    echo -e "${RED}ERROR: $1${NC}"
}

# Clean up function
cleanup() {
    print_header "Cleaning up containers..."
    docker-compose -f docker-compose.yml -f docker-compose.test.yml down --remove-orphans 2>/dev/null || true
}

# Wait for swtpm to be ready
wait_for_swtpm() {
    print_header "Waiting for swtpm to be ready..."
    local max_attempts=30
    local attempt=1
    
    while [ $attempt -le $max_attempts ]; do
        if docker-compose -f docker-compose.yml exec -T swtpm nc -z localhost 2321 2>/dev/null; then
            echo -e "${GREEN}swtpm is ready!${NC}"
            return 0
        fi
        echo "Attempt $attempt/$max_attempts: swtpm not ready yet..."
        sleep 1
        ((attempt++))
    done
    
    print_error "swtpm did not become ready in time"
    return 1
}

case "${1:-test}" in
    test)
        print_header "CHINJU Protocol - TPM Integration Tests"
        
        # Start swtpm
        print_header "Starting swtpm..."
        docker-compose -f docker-compose.yml up -d swtpm
        
        # Wait for swtpm
        sleep 3
        wait_for_swtpm
        
        # Run tests
        print_header "Running TPM integration tests..."
        docker-compose -f docker-compose.yml -f docker-compose.test.yml run --rm chinju-tpm-test
        
        # Show results
        print_header "Tests completed!"
        ;;
        
    shell)
        print_header "CHINJU Protocol - TPM Debug Shell"
        
        # Start swtpm
        print_header "Starting swtpm..."
        docker-compose -f docker-compose.yml up -d swtpm
        
        # Wait for swtpm
        sleep 3
        wait_for_swtpm
        
        # Start interactive shell
        print_header "Starting interactive shell..."
        echo "TPM connection: TPM_HOST=swtpm TPM_PORT=2321"
        echo "Run tests with: cargo test --features tpm -p chinju-core -- --ignored --test-threads=1"
        docker-compose -f docker-compose.yml -f docker-compose.test.yml run --rm chinju-tpm-shell
        ;;
        
    integration)
        print_header "CHINJU Protocol - Full Integration Test"
        
        # Start all services
        print_header "Starting swtpm and sidecar..."
        docker-compose -f docker-compose.yml -f docker-compose.test.yml up -d swtpm chinju-integration-test
        
        # Wait for services
        sleep 5
        
        # Show logs
        print_header "Service logs..."
        docker-compose -f docker-compose.yml -f docker-compose.test.yml logs --tail=50
        
        # Health check
        print_header "Health check..."
        curl -s http://localhost:8081/health || print_warning "Health check failed (may still be starting)"
        ;;
        
    clean)
        cleanup
        echo -e "${GREEN}Cleanup complete!${NC}"
        ;;
        
    logs)
        docker-compose -f docker-compose.yml -f docker-compose.test.yml logs -f
        ;;
        
    *)
        echo "Usage: $0 {test|shell|integration|clean|logs}"
        echo ""
        echo "Commands:"
        echo "  test        Run TPM integration tests (default)"
        echo "  shell       Start interactive shell for debugging"
        echo "  integration Start full integration test environment"
        echo "  clean       Clean up Docker containers"
        echo "  logs        Follow Docker logs"
        exit 1
        ;;
esac
