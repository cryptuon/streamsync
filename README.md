# StreamSync — Decentralized Data Infrastructure for the Agent Economy

**[🌐 Site](https://streamsync.cryptuon.com/) · [📚 Docs](https://docs.cryptuon.com/streamsync/) · [🗺️ Roadmap](ROADMAP.md) · [🔬 Cryptuon Research](https://github.com/cryptuon)**

> **Active development.** StreamSync is under active development. APIs,
> schemas, and on-chain layouts may change between releases.
> Production use at your own risk. Issues and PRs welcome.

StreamSync is a **high-performance decentralized indexing network for Solana** — a DePIN
that delivers guaranteed sub-10ms queries through racing competition between independent
operators and distributed DuckDB sharding. It is the fast, verifiable read layer that
autonomous agents, trading systems, and real-time dApps depend on. Built in Rust, MIT-licensed.

- Documentation: <https://docs.cryptuon.com/streamsync/>
- Marketing site: <https://streamsync.cryptuon.com/>
- Roadmap & cheapest path to production: [ROADMAP.md](ROADMAP.md)

## Why this matters in 2026

The consumers of on-chain data have changed. It is no longer just dashboards and
front-ends polling an RPC — it is **autonomous agents making payments, on-chain and
verifiable AI systems reading state to act on it, RWA platforms reconciling ledgers, and
real-time apps that render on every block**. These workloads share one requirement:
**reads that are fast, correct, and not gated by a single vendor.**

An agent that settles a trade, or an AI that acts on decoded program state, cannot afford
tail-latency spikes, mid-quarter rate limits, or a provider that quietly degrades. It needs
a read layer with an *enforceable* latency SLA and correctness that is backed by capital,
not a marketing promise. That is exactly the gap StreamSync fills:

- **Real-time data for agents & apps** — sub-10ms answers to account lookups, decoded
  state, transaction traces, and analytical SQL, so agent loops and live UIs never wait.
- **Verifiable reads** — every answer is raced across independent operators and confirmed by
  verifiers; wrong results are slashed from staked $STRM. Correctness is collateralized.
- **DePIN economics** — operators run on commodity hardware and compete for query revenue.
  No StreamSync-owned datacenter, no single throat to choke, no vendor lock-in.
- **Solana-native settlement** — coordination and payment settle on Solana in 5-minute
  batches, making the network cheap to run and its incentives programmable.

StreamSync sits at the intersection of two 2026 infrastructure narratives — **DePIN** and
**real-time / verifiable data for the agent economy** — and treats them as one problem: the
fast, honest reads that everything else composes on top of.

## Core Principle: Economic Decentralization over Geographic Distribution

True decentralization isn't about where servers are located - it's about **who controls pricing, availability, and access decisions**.

**Current Problem**: Centralized indexing providers hold customers hostage with arbitrary pricing, service restrictions, and single points of failure. As autonomous agents become primary consumers of on-chain data, that single-vendor dependency becomes a systemic risk, not just an inconvenience.

**Our Solution**: Market-driven competition between independent node operators within a performance-guaranteed protocol.

### Why This Approach Wins

- **Day 1 Decentralization**: Multiple competing operators from launch
- **Performance First**: Sub-10ms guarantees through economic incentives
- **Market Pricing**: Supply/demand sets prices, not corporate decisions
- **Protocol Guarantees**: Cannot be censored or arbitrarily restricted
- **Competitive Innovation**: Operators compete on performance and features
- **Agent-Ready**: Verifiable, low-latency reads that autonomous systems can settle on

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
| **Roadmap & Cheapest Path to Production** | [ROADMAP.md](ROADMAP.md) |
| **Project Roadmap (detailed)** | [docs/project-roadmap.md](docs/project-roadmap.md) |

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
git clone https://github.com/cryptuon/streamsync.git
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

See [ROADMAP.md](ROADMAP.md) for the vision, milestones (devnet → founding operators →
testnet → mainnet), the **cheapest path to production**, and the production-viability
checklist (incentive/settlement contract, latency-SLA verification, shard rebalancing,
failover, monitoring).

---

**This network is economically decentralized from day 1, with technical decentralization following market-driven expansion — the fast, verifiable read layer the agent economy runs on.**

---

## Part of Cryptuon Research

`streamsync` is one of [20 open-source blockchain-infrastructure projects](https://www.cryptuon.com/projects) from **[Cryptuon Research](https://www.cryptuon.com)** — blockchain theory, shipped as protocols.

**Related projects:** [SolanaVault](https://solanavault.cryptuon.com/) · [Switchboard](https://switchboard.cryptuon.com/) · [SolanaLM](https://solanalm.cryptuon.com/)

Docs: [docs.cryptuon.com/streamsync](https://docs.cryptuon.com/streamsync/) · Contact: [contact@cryptuon.com](mailto:contact@cryptuon.com)