# Configuration

Complete configuration reference for StreamSync nodes.

---

## Configuration File

StreamSync uses TOML configuration files. Generate a default config:

```bash
./target/release/streamsync init --config node.toml
```

---

## Full Configuration Reference

```toml
# =============================================================================
# StreamSync Node Configuration
# =============================================================================

# -----------------------------------------------------------------------------
# Node Identity
# -----------------------------------------------------------------------------
[node]
# Unique node identifier (auto-generated if not set)
id = "my-node-001"

# Node type: speed-runner, cache-optimizer, archive-node, reconstruction-spec
type = "speed-runner"

# Geographic region for routing optimization
region = "us-east-1"

# Human-readable name
name = "My StreamSync Node"

# -----------------------------------------------------------------------------
# Network Configuration
# -----------------------------------------------------------------------------
[network]
# Address to listen on for queries
listen_address = "0.0.0.0:8080"

# Address for gossip protocol
gossip_address = "0.0.0.0:7878"

# Discovery/bootstrap nodes
discovery_nodes = [
    "discovery-1.streamsync.io:7878",
    "discovery-2.streamsync.io:7878",
]

# Maximum concurrent connections
max_connections = 1000

# Connection timeout
connection_timeout_ms = 5000

# Keep-alive interval
keepalive_interval_seconds = 30

# -----------------------------------------------------------------------------
# Specialization Settings
# -----------------------------------------------------------------------------
[specialization]
# For speed-runner nodes
target_latency_ms = 5
cache_capacity_gb = 32
supported_query_types = [
    "simple_account_lookup",
    "token_balance",
    "basic_aggregation"
]

# For cache-optimizer nodes (uncomment if type = "cache-optimizer")
# hot_data_threshold_queries = 100
# eviction_policy = "lru"  # lru, lfu, adaptive
# predictive_caching = true

# For archive nodes (uncomment if type = "archive-node")
# retention_days = 365
# compression_level = 6  # 1-9
# index_everything = true

# For reconstruction-spec nodes (uncomment if type = "reconstruction-spec")
# merkle_cache_size_mb = 4096
# zk_proof_workers = 8

# -----------------------------------------------------------------------------
# Database Configuration
# -----------------------------------------------------------------------------
[database]
# Path to DuckDB data file
path = "./data/streamsync.duckdb"

# Memory limit (percentage of system RAM or absolute)
memory_limit = "80%"

# Number of worker threads (0 = auto-detect)
threads = 0

# Query result cache size
cache_size_mb = 2048

# WAL mode for durability
wal_mode = true

# Checkpoint interval
checkpoint_interval_seconds = 300

# -----------------------------------------------------------------------------
# Performance Configuration
# -----------------------------------------------------------------------------
[performance]
# Query timeout (node-side)
query_timeout_ms = 15

# Cache TTL for query results
cache_ttl_seconds = 5

# Metrics collection interval
metrics_interval_seconds = 10

# Maximum concurrent queries
max_concurrent_queries = 100

# Query queue size
query_queue_size = 1000

# -----------------------------------------------------------------------------
# Economics Configuration
# -----------------------------------------------------------------------------
[economics]
# Your staking account public key
stake_account = "YOUR_STAKE_ACCOUNT_PUBKEY"

# Address to receive rewards
reward_address = "YOUR_REWARD_WALLET_PUBKEY"

# Minimum query fee to accept (in STRM lamports)
min_query_fee = 1000

# Auto-claim rewards threshold (in STRM)
auto_claim_threshold = 100.0

# -----------------------------------------------------------------------------
# Solana RPC Configuration
# -----------------------------------------------------------------------------
[solana]
# RPC endpoint for reading chain data
rpc_url = "https://api.mainnet-beta.solana.com"

# WebSocket endpoint for subscriptions
ws_url = "wss://api.mainnet-beta.solana.com"

# Commitment level: processed, confirmed, finalized
commitment = "confirmed"

# RPC request timeout
request_timeout_seconds = 30

# Rate limit (requests per second)
rate_limit = 100

# -----------------------------------------------------------------------------
# Gossip Protocol Configuration
# -----------------------------------------------------------------------------
[gossip]
# Gossip protocol variant: push, pull, push-pull
protocol = "push-pull"

# Fanout for push gossip
fanout = 3

# Pull interval
pull_interval_seconds = 5

# Heartbeat interval
heartbeat_interval_seconds = 1

# Node failure detection threshold
failure_threshold_missed_heartbeats = 5

# -----------------------------------------------------------------------------
# Logging Configuration
# -----------------------------------------------------------------------------
[logging]
# Log level: trace, debug, info, warn, error
level = "info"

# Log format: json, pretty
format = "json"

# Log file path (empty = stdout only)
file = "./logs/streamsync.log"

# Log rotation
max_file_size_mb = 100
max_files = 10

# -----------------------------------------------------------------------------
# Metrics & Monitoring
# -----------------------------------------------------------------------------
[metrics]
# Enable Prometheus metrics endpoint
enabled = true

# Metrics endpoint address
address = "0.0.0.0:9090"

# Include detailed query metrics
detailed_query_metrics = true

# -----------------------------------------------------------------------------
# TLS Configuration (Optional)
# -----------------------------------------------------------------------------
[tls]
# Enable TLS
enabled = false

# Certificate file path
cert_file = "./certs/server.crt"

# Private key file path
key_file = "./certs/server.key"

# CA certificate for client verification (optional)
ca_file = "./certs/ca.crt"
```

---

## Environment Variables

Configuration can be overridden with environment variables:

| Variable | Description | Example |
|----------|-------------|---------|
| `STREAMSYNC_NODE_ID` | Node identifier | `my-node-001` |
| `STREAMSYNC_NODE_TYPE` | Node specialization | `speed-runner` |
| `STREAMSYNC_LISTEN_ADDR` | Query API address | `0.0.0.0:8080` |
| `STREAMSYNC_GOSSIP_ADDR` | Gossip address | `0.0.0.0:7878` |
| `STREAMSYNC_SOLANA_RPC` | Solana RPC URL | `https://api.mainnet...` |
| `STREAMSYNC_LOG_LEVEL` | Log level | `info` |
| `RUST_LOG` | Rust logging filter | `streamsync=debug` |

```bash
# Example
STREAMSYNC_NODE_ID=prod-node-1 \
STREAMSYNC_LOG_LEVEL=debug \
./target/release/streamsync run --config node.toml
```

---

## Configuration by Node Type

### Speed Runner

Optimized for lowest latency queries:

```toml
[node]
type = "speed-runner"

[specialization]
target_latency_ms = 1
cache_capacity_gb = 64
supported_query_types = [
    "simple_account_lookup",
    "token_balance",
]

[database]
memory_limit = "90%"
cache_size_mb = 8192

[performance]
query_timeout_ms = 10
cache_ttl_seconds = 2
```

### Cache Optimizer

Optimized for high cache hit rates:

```toml
[node]
type = "cache-optimizer"

[specialization]
hot_data_threshold_queries = 50
eviction_policy = "adaptive"
predictive_caching = true

[database]
memory_limit = "85%"
cache_size_mb = 16384

[performance]
cache_ttl_seconds = 30
```

### Archive Node

Optimized for historical data access:

```toml
[node]
type = "archive-node"

[specialization]
retention_days = 730  # 2 years
compression_level = 9
index_everything = true

[database]
path = "./data/archive.duckdb"
memory_limit = "60%"

[performance]
query_timeout_ms = 60000  # 60 seconds for large queries
```

### ZK Reconstruction Specialist

Optimized for compressed account reconstruction:

```toml
[node]
type = "reconstruction-spec"

[specialization]
merkle_cache_size_mb = 8192
zk_proof_workers = 16

[database]
memory_limit = "70%"
threads = 16

[performance]
query_timeout_ms = 30000
```

---

## Validation

Validate your configuration:

```bash
# Check configuration syntax
./target/release/streamsync config validate --config node.toml

# Show effective configuration (with defaults)
./target/release/streamsync config show --config node.toml
```

---

## Next Steps

- [Run your node](../operators/running-a-node.md)
- [Stake STRM tokens](../tokenomics/staking.md)
- [Monitor performance](../operators/monitoring.md)
