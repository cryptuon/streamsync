# Core Libraries

The foundational libraries powering StreamSync.

---

## Library Overview

| Library | Purpose | Tests |
|---------|---------|-------|
| `networking-core` | Gossip protocol, peer discovery | 45 |
| `sharding-core` | Hash ring, rebalancing, health | 60 |
| `distributed-duckdb` | Distributed SQL queries | 34 |
| `idl-sync` | Behavioral IDL generation | 18 |
| `zk-reconstruction` | Compressed account recovery | 8 |
| `storage-core` | Compression, batch I/O | 3 |
| `solana-indexer` | RPC client, parsing | 6 |
| `program-parser` | SPL/Metaplex parsing | 8 |

---

## networking-core

Handles all node-to-node communication.

### Features

- Push/Pull/Push-Pull gossip modes
- Peer discovery and management
- Heartbeat monitoring
- Failure detection

### Usage

```rust
use networking_core::{GossipProtocol, GossipConfig, GossipMode};

let config = GossipConfig {
    mode: GossipMode::PushPull,
    fanout: 3,
    heartbeat_interval: Duration::from_secs(1),
    failure_threshold: 5,
};

let gossip = GossipProtocol::new(config);
gossip.start().await;

// Broadcast update
gossip.broadcast(StateUpdate::NodeJoined(node_info)).await;
```

---

## sharding-core

Manages data distribution across nodes.

### Features

- Consistent hash ring
- Virtual nodes for balance
- Automatic rebalancing
- Health monitoring

### Usage

```rust
use sharding_core::{HashRing, ShardManager};

let ring = HashRing::new(virtual_nodes: 150);

// Add nodes
ring.add_node(node1);
ring.add_node(node2);

// Find responsible node
let node = ring.get_node(&account_pubkey);

// Rebalance after node changes
let shard_manager = ShardManager::new(ring);
shard_manager.rebalance().await;
```

---

## distributed-duckdb

Distributes queries across node data.

### Features

- Query planning and optimization
- Parallel sub-query execution
- Result aggregation
- Local caching

### Usage

```rust
use distributed_duckdb::{Coordinator, DistributedQuery};

let coordinator = Coordinator::new(nodes);

let query = DistributedQuery::parse(
    "SELECT * FROM accounts WHERE owner = $1"
)?;

let result = coordinator.execute(query, &[owner]).await?;
```

---

## zk-reconstruction

Reconstructs compressed accounts with ZK proofs.

### Features

- Merkle proof verification
- Multiple compression formats
- Proof caching
- Parallel proof generation

### Usage

```rust
use zk_reconstruction::{Reconstructor, CompressionType};

let reconstructor = Reconstructor::new();

let account = reconstructor.reconstruct(
    merkle_root,
    leaf_index,
    CompressionType::SplAccountCompression,
).await?;
```

---

## Building Libraries

```bash
# Build all libraries
cargo build --workspace --release

# Test specific library
cargo test --package networking-core

# Run all tests
cargo test --workspace
```

---

## Adding Dependencies

```toml
# Cargo.toml
[dependencies]
networking-core = { path = "core-libraries/networking-core" }
sharding-core = { path = "core-libraries/sharding-core" }
distributed-duckdb = { path = "core-libraries/distributed-duckdb" }
```

---

## Documentation

Generate and view library docs:

```bash
cargo doc --workspace --open
```
