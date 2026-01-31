#!/bin/bash
#
# CHINJU Nitro Enclave Setup Script
#
# Run this on an EC2 Nitro-enabled instance (c5, m5, r5, etc.)
# with --enclave-options Enabled=true
#
# Usage:
#   chmod +x setup-nitro-enclave.sh
#   ./setup-nitro-enclave.sh

set -e

echo "=== CHINJU Nitro Enclave Setup ==="

# Check if running on EC2
if ! curl -s --max-time 2 http://169.254.169.254/latest/meta-data/instance-id > /dev/null 2>&1; then
    echo "ERROR: This script must be run on an EC2 instance"
    exit 1
fi

# Check if Nitro Enclaves are enabled
if [ ! -e /dev/nitro_enclaves ]; then
    echo "ERROR: Nitro Enclaves not enabled on this instance"
    echo "Launch with: --enclave-options Enabled=true"
    exit 1
fi

echo "[1/6] Installing Nitro Enclaves CLI..."
if command -v amazon-linux-extras &> /dev/null; then
    # Amazon Linux 2
    sudo amazon-linux-extras install aws-nitro-enclaves-cli -y
    sudo yum install aws-nitro-enclaves-cli-devel -y
elif command -v dnf &> /dev/null; then
    # Amazon Linux 2023
    sudo dnf install aws-nitro-enclaves-cli aws-nitro-enclaves-cli-devel -y
else
    echo "Unsupported OS. Please install Nitro CLI manually."
    exit 1
fi

echo "[2/6] Adding user to ne group..."
sudo usermod -aG ne $USER
sudo usermod -aG docker $USER

echo "[3/6] Configuring Nitro Enclave allocator..."
# Allocate 512MB for Enclave (adjust as needed)
sudo tee /etc/nitro_enclaves/allocator.yaml > /dev/null <<EOF
---
memory_mib: 512
cpu_count: 2
EOF

echo "[4/6] Starting Nitro Enclave allocator service..."
sudo systemctl enable nitro-enclaves-allocator.service
sudo systemctl start nitro-enclaves-allocator.service

echo "[5/6] Installing Rust..."
if ! command -v rustup &> /dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
fi

# Add musl target for static linking
rustup target add x86_64-unknown-linux-musl

echo "[6/6] Installing Docker..."
if ! command -v docker &> /dev/null; then
    sudo yum install docker -y || sudo dnf install docker -y
    sudo systemctl enable docker
    sudo systemctl start docker
fi

echo ""
echo "=== Setup Complete ==="
echo ""
echo "IMPORTANT: Log out and log back in for group changes to take effect."
echo ""
echo "Next steps:"
echo "  1. Log out and log back in"
echo "  2. Clone your repository"
echo "  3. Build the Enclave image:"
echo ""
echo "     cd chinju-protocol/chinju-enclave"
echo "     docker build -t chinju-enclave -f Dockerfile.enclave ."
echo "     nitro-cli build-enclave --docker-uri chinju-enclave:latest --output-file chinju-enclave.eif"
echo ""
echo "  4. Run the Enclave:"
echo ""
echo "     nitro-cli run-enclave --cpu-count 2 --memory 512 --enclave-cid 16 --eif-path chinju-enclave.eif"
echo ""
echo "  5. Check status:"
echo ""
echo "     nitro-cli describe-enclaves"
echo ""
