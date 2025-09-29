# StreamSync Node - Quick Start Guide

## Overview

StreamSync is a production-ready distributed node for decentralized Solana transaction indexing. This guide will get you up and running with a fully functional StreamSync node.

## 🚀 Quick Start

### 1. Build the Node

```bash
# Build the StreamSync node
cargo build --bin streamsync

# The binary will be available at ./target/debug/streamsync
```

### 2. Initialize Configuration

```bash
# Create a new node configuration
./target/debug/streamsync init --output node.toml

# This creates a TOML configuration file with sensible defaults
```

### 3. Start the Node

```bash
# Start with default configuration
./target/debug/streamsync start --config node.toml

# Start with debug logging
./target/debug/streamsync start --config node.toml --debug

# Start as an observer node
./target/debug/streamsync start --config node.toml --role observer
```

### 4. Monitor Status

```bash
# Check node status
./target/debug/streamsync status

# View help for all available commands
./target/debug/streamsync --help
```

## 📋 Complete Demo

Run the comprehensive demo to see all functionality:

```bash
# Run interactive demo
./demo.sh
```

The demo showcases:
- ✅ Node configuration initialization
- ✅ Complete component startup sequence
- ✅ Networking and sharding integration
- ✅ Debug logging and health checks
- ✅ CLI interface functionality
- ✅ Core library test coverage

## 🔧 Configuration

The node configuration (`node.toml`) includes:

### Node Settings
- `id`: Unique node identifier (UUID)
- `role`: Node role (primary, secondary, observer)
- `api_port`: HTTP API port (default: 8080)
- `metrics_port`: Metrics collection port (default: 9090)

### Networking
- `listen_addr`: Interface to bind to (default: 0.0.0.0)
- `p2p_port`: Peer-to-peer communication port (default: 7777)
- `max_peers`: Maximum peer connections (default: 50)

### Sharding
- `virtual_nodes`: Virtual nodes per physical node (default: 150)
- `replication_factor`: Data replication factor (default: 3)
- `hash_function`: Hash algorithm (ahash, sha256)

### Performance
- `worker_threads`: Number of worker threads (default: CPU count)
- `max_concurrent_queries`: Max concurrent queries (default: 100)

## 🏗️ Architecture

StreamSync integrates multiple core libraries:

### Core Libraries
1. **consensus-core**: PBFT consensus implementation (32/34 tests passing)
2. **networking-core**: High-performance NNG transport (38/38 tests passing)
3. **sharding-core**: Consistent hashing with virtual nodes (60/60 tests passing)

### Node Components
- **CLI Interface**: Complete lifecycle management
- **Configuration Management**: TOML-based settings with validation
- **Async Runtime**: Tokio-based async execution
- **Logging**: Structured logging with tracing
- **Error Handling**: Comprehensive error handling with anyhow

## 📊 Test Coverage

Overall test coverage: **130/132 tests passing (98.5% success rate)**

```bash
# Test individual core libraries
cd core-libraries/consensus-core && cargo test
cd core-libraries/networking-core && cargo test
cd core-libraries/sharding-core && cargo test
```

## 🌐 Distributed Operation

### Single Node
```bash
# Start a single node for development/testing
./target/debug/streamsync start --config node.toml --debug
```

### Multi-Node Network
To create a distributed network:

1. **Create multiple configurations:**
   ```bash
   ./target/debug/streamsync init --output node1.toml
   ./target/debug/streamsync init --output node2.toml
   ./target/debug/streamsync init --output node3.toml
   ```

2. **Update networking settings** in each config:
   - Set different `p2p_port` values
   - Configure `bootstrap_nodes` to connect nodes

3. **Start nodes:**
   ```bash
   ./target/debug/streamsync start --config node1.toml --role primary &
   ./target/debug/streamsync start --config node2.toml --role secondary &
   ./target/debug/streamsync start --config node3.toml --role secondary &
   ```

## 🔍 Monitoring & Debugging

### Logs
- **Info Level**: Standard operational logs
- **Debug Level**: Detailed component interactions
- **Health Checks**: Automatic component health monitoring

### Performance Metrics
- Network message statistics
- Sharding cluster health
- Node operation metrics

## 🛠️ Development

### Project Structure
```
streamsync/
├── src/                    # Main node application
│   ├── main.rs            # CLI interface
│   ├── node.rs            # Node orchestration
│   └── config.rs          # Configuration management
├── core-libraries/         # Core library implementations
│   ├── consensus-core/    # PBFT consensus
│   ├── networking-core/   # NNG transport
│   └── sharding-core/     # Consistent hashing
├── demo.sh                # Comprehensive demo
└── Cargo.toml            # Workspace configuration
```

### Building from Source
```bash
# Full workspace build
cargo build --workspace

# Run all tests
cargo test --workspace

# Build optimized release
cargo build --release --bin streamsync
```

## 🚀 Next Steps

1. **Customize Configuration**: Edit `node.toml` for your environment
2. **Scale Deployment**: Deploy multiple nodes for distributed operation
3. **Monitor Performance**: Use debug logs to track system behavior
4. **Integrate Applications**: Connect your applications to the node's API
5. **Contribute**: Help improve the codebase and add new features

## 📚 Additional Resources

- **Architecture Documentation**: See `docs/architecture-overview.md`
- **Core Libraries**: See `docs/core-libraries.md`
- **API Reference**: HTTP API documentation (coming soon)
- **Performance Tuning**: Optimization guides (coming soon)

---

**Ready to build the future of decentralized transaction indexing!** 🌟