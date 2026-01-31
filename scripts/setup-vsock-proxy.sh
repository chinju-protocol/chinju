#!/bin/bash
#
# CHINJU Protocol - vsock-proxy Setup Script
#
# This script sets up vsock-proxy on the EC2 parent instance to allow
# the Nitro Enclave to communicate with AWS KMS.
#
# Usage:
#   ./scripts/setup-vsock-proxy.sh [REGION]
#
# The script will:
# 1. Install aws-nitro-enclaves-cli if not present
# 2. Start vsock-proxy for KMS endpoint
# 3. Optionally start proxy for STS (for IAM credentials)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

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

# Default region
REGION="${1:-$(aws configure get region 2>/dev/null || echo 'us-east-1')}"

# Ports
KMS_PROXY_PORT=8000
STS_PROXY_PORT=8001

print_header "CHINJU vsock-proxy Setup"
echo "Region: $REGION"
echo "KMS Proxy Port: $KMS_PROXY_PORT"
echo "STS Proxy Port: $STS_PROXY_PORT"
echo

# Check if running on EC2
check_ec2() {
    if ! curl -s -m 1 http://169.254.169.254/latest/meta-data/instance-id &>/dev/null; then
        print_warning "Not running on EC2. vsock-proxy requires EC2 with Nitro Enclaves."
        return 1
    fi
    print_success "Running on EC2"
    return 0
}

# Check if vsock-proxy is available
check_vsock_proxy() {
    if ! command -v vsock-proxy &>/dev/null; then
        print_warning "vsock-proxy not found. Installing aws-nitro-enclaves-cli..."
        
        # Install on Amazon Linux 2
        if [ -f /etc/amazon-linux-release ]; then
            sudo amazon-linux-extras install aws-nitro-enclaves-cli -y
            sudo yum install aws-nitro-enclaves-cli-devel -y
        else
            print_error "Please install aws-nitro-enclaves-cli manually"
            return 1
        fi
    fi
    print_success "vsock-proxy available"
}

# Start vsock-proxy for KMS
start_kms_proxy() {
    local kms_endpoint="kms.${REGION}.amazonaws.com"
    
    # Check if already running
    if pgrep -f "vsock-proxy $KMS_PROXY_PORT $kms_endpoint" &>/dev/null; then
        print_warning "KMS proxy already running on port $KMS_PROXY_PORT"
        return 0
    fi
    
    echo "Starting KMS proxy: port $KMS_PROXY_PORT -> $kms_endpoint:443"
    
    # Start in background
    nohup vsock-proxy $KMS_PROXY_PORT $kms_endpoint 443 \
        --config /etc/nitro_enclaves/vsock-proxy.yaml \
        > /var/log/vsock-proxy-kms.log 2>&1 &
    
    sleep 1
    
    if pgrep -f "vsock-proxy $KMS_PROXY_PORT" &>/dev/null; then
        print_success "KMS proxy started (port $KMS_PROXY_PORT)"
    else
        print_error "Failed to start KMS proxy"
        return 1
    fi
}

# Start vsock-proxy for STS (optional, for IAM role credentials)
start_sts_proxy() {
    local sts_endpoint="sts.${REGION}.amazonaws.com"
    
    # Check if already running
    if pgrep -f "vsock-proxy $STS_PROXY_PORT $sts_endpoint" &>/dev/null; then
        print_warning "STS proxy already running on port $STS_PROXY_PORT"
        return 0
    fi
    
    echo "Starting STS proxy: port $STS_PROXY_PORT -> $sts_endpoint:443"
    
    # Start in background
    nohup vsock-proxy $STS_PROXY_PORT $sts_endpoint 443 \
        --config /etc/nitro_enclaves/vsock-proxy.yaml \
        > /var/log/vsock-proxy-sts.log 2>&1 &
    
    sleep 1
    
    if pgrep -f "vsock-proxy $STS_PROXY_PORT" &>/dev/null; then
        print_success "STS proxy started (port $STS_PROXY_PORT)"
    else
        print_warning "Failed to start STS proxy (optional)"
    fi
}

# Create vsock-proxy config if not exists
create_config() {
    local config_file="/etc/nitro_enclaves/vsock-proxy.yaml"
    
    if [ -f "$config_file" ]; then
        print_success "vsock-proxy config exists"
        return 0
    fi
    
    echo "Creating vsock-proxy config..."
    
    sudo mkdir -p /etc/nitro_enclaves
    sudo tee "$config_file" > /dev/null << EOF
# vsock-proxy configuration for CHINJU Protocol
# Allows Enclave to access KMS and STS endpoints

allowlist:
  - { address: "kms.${REGION}.amazonaws.com", port: 443 }
  - { address: "sts.${REGION}.amazonaws.com", port: 443 }
  - { address: "kms.amazonaws.com", port: 443 }
  - { address: "sts.amazonaws.com", port: 443 }
EOF

    print_success "Created vsock-proxy config"
}

# Show status
show_status() {
    print_header "vsock-proxy Status"
    
    echo "Running proxies:"
    pgrep -a -f "vsock-proxy" || echo "  (none)"
    
    echo
    echo "Port mappings:"
    echo "  $KMS_PROXY_PORT -> kms.${REGION}.amazonaws.com:443 (KMS)"
    echo "  $STS_PROXY_PORT -> sts.${REGION}.amazonaws.com:443 (STS)"
}

# Stop all proxies
stop_proxies() {
    print_header "Stopping vsock-proxy"
    
    pkill -f "vsock-proxy" || true
    print_success "All proxies stopped"
}

# Main
case "${2:-start}" in
    start)
        echo
        if check_ec2; then
            check_vsock_proxy
            create_config
            start_kms_proxy
            start_sts_proxy
            echo
            show_status
        fi
        ;;
    stop)
        stop_proxies
        ;;
    status)
        show_status
        ;;
    restart)
        stop_proxies
        sleep 1
        check_ec2 && start_kms_proxy && start_sts_proxy
        ;;
    *)
        echo "Usage: $0 [REGION] [start|stop|status|restart]"
        exit 1
        ;;
esac

echo
print_header "Environment Variables for Enclave"
cat << EOF
export VSOCK_PROXY_CID=3
export VSOCK_PROXY_PORT=$KMS_PROXY_PORT
export AWS_REGION=$REGION
export AWS_KMS_KEY_ID=<your-kms-key-id>
EOF
