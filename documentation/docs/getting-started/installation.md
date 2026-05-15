# Installation

Detailed installation instructions for StreamSync components.

---

## System Requirements

### Minimum Requirements

| Component | Requirement |
|-----------|-------------|
| **CPU** | 8 cores |
| **RAM** | 32 GB |
| **Storage** | 500 GB SSD |
| **Network** | 100 Mbps |
| **OS** | Linux (Ubuntu 20.04+), macOS 12+ |

### Recommended (Production Node)

| Component | Requirement |
|-----------|-------------|
| **CPU** | 32+ cores |
| **RAM** | 128 GB (256 GB for ZK Reconstruction) |
| **Storage** | 4 TB+ NVMe SSD |
| **Network** | 10 Gbps |
| **OS** | Ubuntu 22.04 LTS |

---

## Install from Source

### Prerequisites

```bash
# Ubuntu/Debian
sudo apt update
sudo apt install -y \
    build-essential \
    libssl-dev \
    pkg-config \
    protobuf-compiler \
    git

# macOS
brew install openssl protobuf
```

### Install Rust

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add to path
source $HOME/.cargo/env

# Install stable toolchain
rustup default stable
rustup update

# Verify installation
rustc --version  # Should be 1.75+
cargo --version
```

### Build StreamSync

```bash
# Clone repository
git clone https://github.com/your-org/streamsync.git
cd streamsync

# Build release binary
cargo build --release

# Binary location
ls -la ./target/release/streamsync
```

### Verify Installation

```bash
# Run tests
cargo test --workspace

# Expected output:
# networking-core:     45 passed
# sharding-core:       60 passed
# distributed-duckdb:  34 passed
# ...
# Total: 193+ tests passed

# Check version
./target/release/streamsync --version
```

---

## Install via Docker

### Pull Official Image

```bash
# Pull latest
docker pull streamsync/node:latest

# Or specific version
docker pull streamsync/node:1.0.0
```

### Run with Docker

```bash
# Create data directory
mkdir -p ./streamsync-data

# Run node
docker run -d \
    --name streamsync-node \
    -v ./node.toml:/etc/streamsync/node.toml:ro \
    -v ./streamsync-data:/data \
    -p 8080:8080 \
    -p 7878:7878 \
    streamsync/node:latest

# Check logs
docker logs -f streamsync-node
```

### Docker Compose

```yaml
# docker-compose.yml
version: '3.8'

services:
  streamsync:
    image: streamsync/node:latest
    container_name: streamsync-node
    restart: unless-stopped
    volumes:
      - ./node.toml:/etc/streamsync/node.toml:ro
      - streamsync-data:/data
    ports:
      - "8080:8080"   # Query API
      - "7878:7878"   # Gossip protocol
    environment:
      - RUST_LOG=info
      - STREAMSYNC_NODE_ID=my-node

volumes:
  streamsync-data:
```

```bash
# Start
docker-compose up -d

# View logs
docker-compose logs -f
```

---

## Install CLI Only

If you only need to query the network (not run a node):

```bash
# Install from crates.io
cargo install streamsync-cli

# Verify
streamsync --version
streamsync --help
```

---

## Solana Tools (Optional)

Required for staking operations:

```bash
# Install Solana CLI
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"

# Add to PATH
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"

# Verify
solana --version

# Configure for mainnet
solana config set --url https://api.mainnet-beta.solana.com

# Or devnet for testing
solana config set --url https://api.devnet.solana.com
```

---

## Anchor (For Development)

If developing on the $STRM token program:

```bash
# Install Anchor
cargo install --git https://github.com/coral-xyz/anchor avm --locked
avm install 0.30.0
avm use 0.30.0

# Verify
anchor --version
```

---

## Troubleshooting

### Build Errors

??? failure "OpenSSL not found"
    ```bash
    # Ubuntu
    sudo apt install libssl-dev pkg-config

    # macOS
    brew install openssl
    export OPENSSL_DIR=$(brew --prefix openssl)
    ```

??? failure "Protobuf compiler not found"
    ```bash
    # Ubuntu
    sudo apt install protobuf-compiler

    # macOS
    brew install protobuf
    ```

??? failure "Out of memory during build"
    ```bash
    # Reduce parallelism
    cargo build --release -j 2
    ```

### Runtime Errors

??? failure "Permission denied on port 8080"
    ```bash
    # Use non-privileged port
    # Edit node.toml: listen_address = "0.0.0.0:18080"

    # Or run with sudo (not recommended)
    sudo ./target/release/streamsync run
    ```

??? failure "Connection refused to discovery nodes"
    Check firewall settings:
    ```bash
    # Allow outbound on port 7878
    sudo ufw allow out 7878/tcp
    ```

---

## Next Steps

After installation:

1. [Configure your node](configuration.md)
2. [Run the node](../operators/running-a-node.md)
3. [Stake STRM tokens](../tokenomics/staking.md)
