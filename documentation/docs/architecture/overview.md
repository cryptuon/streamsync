# Architecture Overview

System design and component interaction.

---

## High-Level Architecture

```mermaid
graph TB
    subgraph "Clients"
        C1[Web App]
        C2[Trading Bot]
        C3[Analytics]
    end

    subgraph "StreamSync Network"
        R[Query Router]

        subgraph "Node Pool"
            N1[Speed Runner]
            N2[Cache Optimizer]
            N3[Archive Node]
            N4[Reconstruction Spec]
        end

        G[Gossip Protocol]
        S[Settlement Engine]
    end

    subgraph "Solana"
        RPC[RPC Nodes]
        SM[$STRM Program]
    end

    C1 --> R
    C2 --> R
    C3 --> R

    R --> N1
    R --> N2
    R --> N3
    R --> N4

    N1 <--> G
    N2 <--> G
    N3 <--> G
    N4 <--> G

    N1 --> RPC
    N2 --> RPC
    N3 --> RPC
    N4 --> RPC

    S --> SM
```

---

## Core Components

### 1. Query Router

Routes incoming queries to appropriate nodes:

```rust
pub struct QueryRouter {
    node_registry: NodeRegistry,
    load_balancer: LoadBalancer,
    racing_manager: RacingManager,
}

impl QueryRouter {
    pub async fn route_query(&self, query: Query) -> Result<QueryResult> {
        // 1. Find capable nodes
        let candidates = self.node_registry
            .find_capable(&query.query_type);

        // 2. Select racing participants
        let racers = self.load_balancer
            .select_weighted(candidates, 5);

        // 3. Execute race
        self.racing_manager
            .execute_race(racers, query)
            .await
    }
}
```

### 2. Node Registry

Tracks all active nodes and their capabilities:

```rust
pub struct NodeRegistry {
    nodes: DashMap<NodeId, NodeInfo>,
    capabilities: HashMap<QueryType, Vec<NodeId>>,
    health_checker: HealthChecker,
}

pub struct NodeInfo {
    id: NodeId,
    specialization: NodeSpecialization,
    endpoint: SocketAddr,
    stake: u64,
    reputation: ReputationScore,
    last_heartbeat: Instant,
}
```

### 3. Racing Manager

Orchestrates query racing:

```rust
pub struct RacingManager {
    verifier_pool: VerifierPool,
    reward_recorder: RewardRecorder,
}

impl RacingManager {
    pub async fn execute_race(&self, nodes: Vec<NodeId>, query: Query)
        -> Result<RaceResult>
    {
        // Parallel dispatch
        let responses = self.dispatch_parallel(nodes, &query).await;

        // Find winner (first valid)
        let winner = self.find_winner(responses)?;

        // Verify result
        let verifiers = self.verifier_pool.verify(&winner.result).await?;

        // Record rewards
        self.reward_recorder.record(
            winner.node,
            verifiers,
            query.payment
        ).await?;

        Ok(winner)
    }
}
```

### 4. Gossip Protocol

Maintains network state across nodes:

```rust
pub struct GossipProtocol {
    local_state: Arc<RwLock<NetworkState>>,
    peers: Vec<PeerConnection>,
    mode: GossipMode, // Push, Pull, or PushPull
}

impl GossipProtocol {
    // Push updates to peers
    pub async fn push(&self, update: StateUpdate) {
        for peer in self.random_peers(3) {
            peer.send(GossipMessage::Push(update.clone())).await;
        }
    }

    // Pull state from peers
    pub async fn pull(&self) {
        for peer in self.random_peers(3) {
            let remote = peer.request_state().await;
            self.merge_state(remote);
        }
    }
}
```

### 5. Settlement Engine

Batches and settles rewards:

```rust
pub struct SettlementEngine {
    pending: DashMap<NodeId, Vec<Reward>>,
    batch_interval: Duration,
    anchor_client: AnchorClient,
}

impl SettlementEngine {
    pub async fn run(&self) {
        loop {
            tokio::time::sleep(self.batch_interval).await;
            self.process_batch().await;
        }
    }

    async fn process_batch(&self) {
        let batch = self.collect_pending();

        // Single Solana transaction for all rewards
        self.anchor_client
            .process_settlement_batch(batch)
            .await
            .expect("Settlement failed");
    }
}
```

---

## Data Flow

### Query Execution

```mermaid
sequenceDiagram
    participant C as Client
    participant R as Router
    participant N as Nodes
    participant V as Verifiers
    participant S as Settlement

    C->>R: Query + Payment
    R->>R: Select racing nodes

    par Racing
        R->>N: Execute query (x5)
    end

    N-->>R: First response wins

    par Verification
        R->>V: Verify result (x2)
    end

    V-->>R: Confirmed

    R-->>C: Result

    R->>S: Record rewards
    S->>S: Batch (5 min)
    S->>S: Settle on-chain
```

### State Synchronization

```mermaid
sequenceDiagram
    participant N1 as Node 1
    participant N2 as Node 2
    participant N3 as Node 3

    loop Every 1s
        N1->>N2: Heartbeat
        N1->>N3: Heartbeat
    end

    Note over N1,N3: Push-Pull Gossip
    N1->>N2: Push state update
    N2->>N1: Pull request
    N1-->>N2: State digest
    N2->>N2: Merge states

    Note over N1,N3: Health Detection
    N3--xN1: Missed heartbeats
    N1->>N1: Mark N3 suspect
    N1->>N2: Gossip: N3 suspect
```

---

## Storage Architecture

### Per-Node Storage

```
data/
├── duckdb/
│   ├── accounts.duckdb     # Account state
│   ├── transactions.duckdb # Transaction history
│   └── tokens.duckdb       # Token data
├── cache/
│   ├── hot/                # In-memory LRU
│   └── warm/               # SSD-backed
└── logs/
    └── streamsync.log
```

### Distributed Data

```mermaid
graph TB
    subgraph "Data Partitioning"
        D[All Solana Data]
        D --> P1[Partition 1<br/>Accounts A-M]
        D --> P2[Partition 2<br/>Accounts N-Z]
        D --> P3[Partition 3<br/>Recent Txs]
        D --> P4[Partition 4<br/>Historical]
    end

    subgraph "Node Assignment"
        P1 --> N1[Node 1 Primary]
        P1 --> N2[Node 2 Replica]
        P2 --> N3[Node 3 Primary]
        P2 --> N4[Node 4 Replica]
    end
```

---

## Security Architecture

### Network Security

```mermaid
graph TB
    subgraph "External"
        C[Client]
    end

    subgraph "DMZ"
        LB[Load Balancer]
        WAF[Web Application Firewall]
    end

    subgraph "Internal"
        R[Router]
        N[Nodes]
    end

    C --> WAF --> LB --> R --> N
```

### Authentication

| Layer | Method |
|-------|--------|
| Client → Router | API Key + HMAC |
| Router → Node | mTLS |
| Node → Node | Signed messages |
| Node → Solana | Wallet signature |

---

## Scalability

### Horizontal Scaling

```mermaid
graph LR
    subgraph "Current"
        R1[Router]
        N1[Node 1]
        N2[Node 2]
    end

    subgraph "Scaled"
        R2[Router Cluster]
        N3[Node 3]
        N4[Node 4]
        N5[Node 5]
        N6[Node 6]
    end

    Current --> Scaled
```

### Capacity Planning

| Nodes | Queries/Second | Latency (p99) |
|-------|---------------|---------------|
| 10 | 10,000 | 15ms |
| 50 | 50,000 | 12ms |
| 100 | 100,000 | 10ms |
| 500 | 500,000 | 8ms |

---

## Failure Handling

### Node Failure

```mermaid
stateDiagram-v2
    [*] --> Healthy
    Healthy --> Suspect: 3 missed heartbeats
    Suspect --> Healthy: Heartbeat received
    Suspect --> Failed: 5 missed heartbeats
    Failed --> Healthy: Node recovers
    Failed --> [*]: Node removed
```

### Data Recovery

1. **Hot standby** - Replica takes over immediately
2. **Rebalancing** - Data redistributed to healthy nodes
3. **Reconstruction** - Rebuild from Solana RPC if needed

---

## Next Steps

- [Core Libraries](core-libraries.md) - Implementation details
- [Gossip Protocol](gossip-protocol.md) - Network coordination
- [Distributed Queries](distributed-queries.md) - Query execution
