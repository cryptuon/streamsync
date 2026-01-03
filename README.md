# High-Performance Decentralized Indexing Network

An economically decentralized network delivering guaranteed sub-10ms Solana query performance through competitive node operations and market-driven incentives.

## Core Principle: Economic Decentralization over Geographic Distribution

True decentralization isn't about where servers are located - it's about **who controls pricing, availability, and access decisions**.

**Current Problem**: Centralized indexing providers hold customers hostage with arbitrary pricing, service restrictions, and single points of failure.

**Our Solution**: Market-driven competition between independent node operators within a performance-guaranteed protocol.

### Why This Approach Wins

- **Day 1 Decentralization**: Multiple competing operators from launch
- **Performance First**: Sub-10ms guarantees through economic incentives
- **Market Pricing**: Supply/demand sets prices, not corporate decisions
- **Protocol Guarantees**: Cannot be censored or arbitrarily restricted
- **Competitive Innovation**: Operators compete on performance and features

## Technical Architecture

### 1. Racing Competition for Query Responses
- **3-5 nodes race** to answer each query
- **First correct response wins** 70% of payment
- **Verification nodes earn** 15% each for consensus
- **Sub-10ms guarantee** or customer doesn't pay

### 2. Specialized Node Operations
- **ZK Reconstruction Specialists**: High-compute nodes for compressed data gaps
- **Cache Optimizers**: High-memory nodes for predictive query caching
- **Speed Runners**: Low-latency nodes for simple account lookups
- **Archive Nodes**: High-storage nodes for historical data

### 3. Distributed DuckDB Architecture
- **Partial data per node**: Nodes specialize in data subsets
- **Parallel sub-queries**: Complex queries distributed across relevant nodes
- **Local result merging**: DuckDB handles high-performance aggregation
- **NNG communication**: High-performance inter-node messaging

### 4. Market-Driven Performance
- **Economic incentives** for speed and accuracy
- **Real-time pricing** based on supply and demand
- **Reputation scoring** affects node selection and rewards
- **Automatic slashing** for poor performance or incorrect results

## Network Economics

### Independent Operator Competition
- **Multiple independent entities** operate nodes from day 1
- **Compete for query revenue** through performance and specialization
- **Market-driven pricing** prevents vendor lock-in
- **Protocol-level guarantees** ensure access cannot be restricted

### $STRM Token Economics
- **$STRM token** for network access and staking
- **Payment options**: STRM, SOL, USDC, or custom SPL tokens
- **Revenue split**: 50% nodes, 20% treasury, 20% data providers, 10% governance
- **Racing rewards**: 70% winner, 15% each verifier
- **Batched settlement** every 5 minutes on Solana
- **Staking required**: 10,000 STRM minimum, 7-day cooldown

> **Full details**: [Token Economics](docs/token-economics.md)

### Customer Value Proposition
- **Pay only for performance delivered**: Sub-10ms or no charge
- **No vendor lock-in**: Multiple competing operators
- **Transparent pricing**: Market rates, not corporate decisions
- **Guaranteed availability**: Network-level redundancy
- **Protocol-level access rights**: Cannot be censored or restricted

## Implementation Strategy

### Phase 1: Competitive Launch (Months 1-6)
- **4-5 independent operators** with 2-3 nodes each
- **Full economic competition** from day 1
- **Shared technical standards** but independent operation
- **Market-driven pricing** and performance incentives

### Phase 2: Network Expansion (Months 6-18)
- **Open operator onboarding** with staking requirements
- **Geographic distribution** across regions and providers
- **Advanced specializations** and performance tiers
- **Governance transition** to token holder voting

### Phase 3: Ecosystem Maturity (Month 18+)
- **Permissionless participation** with automated admission
- **Full infrastructure diversity** across cloud and bare metal
- **Advanced features** driven by market demand
- **Self-sustaining economics** independent of founding entities

## Documentation

### User Documentation (MkDocs)

Build and serve the user-facing documentation:

```bash
# Install MkDocs with Material theme
pip install mkdocs-material mkdocs-minify-plugin

# Serve locally
mkdocs serve

# Build static site
mkdocs build
```

Visit http://localhost:8000 to browse the documentation.

### Developer Documentation

| Topic | Link |
|-------|------|
| **Why Economic Decentralization** | [docs/why-economic-decentralization.md](docs/why-economic-decentralization.md) |
| **Performance Guarantees** | [docs/performance-guarantees.md](docs/performance-guarantees.md) |
| **Token Economics** | [docs/token-economics.md](docs/token-economics.md) |
| **Network Economics** | [docs/network-economics.md](docs/network-economics.md) |
| **Incentive Model** | [docs/incentive-model.md](docs/incentive-model.md) |
| **Architecture Overview** | [docs/architecture-overview.md](docs/architecture-overview.md) |
| **Core Libraries** | [docs/core-libraries.md](docs/core-libraries.md) |
| **Whitepaper** | [docs/whitepaper/](docs/whitepaper/) |
| **Getting Started** | [docs/getting-started.md](docs/getting-started.md) |
| **Project Roadmap** | [docs/project-roadmap.md](docs/project-roadmap.md) |

## Project Status

This project has completed **Phase 1 development** with all core systems implemented and tested.

### Completed Components

| Component | Status | Tests |
|-----------|--------|-------|
| **Core Libraries** | ✅ Complete | 193+ |
| **$STRM Token Program** | ✅ Complete | Anchor-based |
| **Payment Gateway** | ✅ Complete | Solana + Stripe |
| **Settlement Engine** | ✅ Complete | Batch processing |
| **Racing Competition** | ✅ Complete | Parallel queries |
| **Node Specializations** | ✅ Complete | 4 types |
| **Gossip Protocol** | ✅ Complete | Push/Pull/Heartbeat |
| **Cluster Management** | ✅ Complete | Rebalancing + Health |

### Core Libraries

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

### Quick Start

```bash
# Clone and build
git clone https://github.com/your-org/streamsync.git
cd streamsync
cargo build --release

# Run tests (193+ passing)
cargo test --workspace

# Initialize a node
./target/release/streamsync init --config node.toml
./target/release/streamsync run
```

### Next Steps

- [x] Complete core library development
- [x] Implement economic layer ($STRM token)
- [x] Build racing competition system
- [x] Implement gossip protocol
- [x] Add cluster health monitoring
- [ ] Deploy to Solana devnet
- [ ] Recruit founding operators
- [ ] Launch testnet for validation
- [ ] Mainnet deployment

---

**This network is economically decentralized from day 1, with technical decentralization following market-driven expansion.**