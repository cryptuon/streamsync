# StreamSync Roadmap

**[🌐 Site](https://streamsync.cryptuon.com/) · [📚 Docs](https://docs.cryptuon.com/streamsync/) · [← README](README.md)**

> **Active development.** Dates below are directional, not commitments. On-chain
> layouts, economic parameters, and API shapes may change between releases.
> Issues and PRs that sharpen this plan are welcome.

## Vision

StreamSync is **data infrastructure for the agent economy**. The read side of
Solana — account state, decoded program data, transaction traces, and analytical
history — is now consumed by autonomous agents, trading systems, and real-time
dApps that cannot tolerate unpredictable latency or a single point of control.
Those consumers need reads that are **fast, verifiable, and vendor-neutral**.

StreamSync's answer is a DePIN-style indexing network: independent operators run
on commodity hardware, **race** to answer each query in under 10ms, and get paid
on outcome. Solana is the coordination and settlement layer; DuckDB shards are
the query layer; economic decentralization — not geographic spread — is the
guarantee. The goal is a network where no single party sets pricing, restricts
access, or lies about the SLA, and where correctness is backed by staked capital
rather than a marketing promise.

## Where we are

Phase 1 is code-complete and tested. The workspace ships nine Rust crates
(`networking-core`, `sharding-core`, `distributed-duckdb`, `idl-sync`,
`zk-reconstruction`, `storage-core`, `solana-indexer`, `program-parser`,
`consensus-core`) plus the node binary. The on-chain side is Anchor-based: the
$STRM token program, payment gateway, settlement engine, and racing-competition
contract. Core-library tests pass at 193+.

What is *not* yet done is the hard part of turning working code into a live
network: an on-chain deployment, real operators on real hardware, and the
operational machinery that makes an SLA trustworthy in production.

## Milestones

### M1 — Devnet coordination (near term)
- Deploy the $STRM token, payment gateway, settlement, and racing contracts to Solana **devnet**.
- Stand up a reference multi-node cluster across two or three operator profiles.
- Publish a public gateway endpoint and a getting-started path for early integrators.
- Wire end-to-end telemetry: per-query latency, win/verify accounting, settlement traces.

### M2 — Founding operator cohort (near–mid term)
- Recruit 4–5 independent operators running 2–3 nodes each, spanning the four specializations.
- Exercise staking, slashing, and reputation scoring under real, adversarial conditions.
- Harden the gossip mesh and hash-ring rebalancing under operator churn.
- Validate the sub-10ms SLA at a customer-facing gateway across multiple regions.

### M3 — Public testnet (mid term)
- Open operator onboarding with staking requirements and automated admission checks.
- Run sustained load with agent-style and dApp-style query mixes.
- Publish honest benchmarks vs. centralized RPC/indexers under identical workloads.
- Freeze v1 economic parameters (revenue split, cooldown, slashing curve) after testnet data.

### M4 — Mainnet + agent-native surfaces (later)
- Mainnet deployment of contracts and settlement.
- Permissionless participation with automated admission.
- Agent-facing conveniences: signed/attestable query responses for verifiable AI and
  autonomous settlement, batched read endpoints for high-frequency agent loops.
- Governance transition toward token-holder voting on parameters.

## Cheapest path to production

StreamSync is deliberately cheap to run because it **coordinates on Solana and
pushes the actual work to operators on commodity hardware**. There is no
StreamSync-owned datacenter to fund — the network's cost floor is a handful of
VPS or bare-metal boxes plus Solana transaction fees. The recommended minimum to
get from "code-complete" to "live, useful network" is below.

### Recommended operator infra tiers

| Tier | Role fit | Suggested spec | Rough monthly cost | Notes |
|------|----------|----------------|--------------------|-------|
| **Entry VPS** | speed-runner, verifier | 4 vCPU / 8 GB RAM / NVMe / 1 Gbps | ~$20–40 | Cheapest way to join the race on hot queries; latency to the gateway region matters more than raw cores. |
| **Cache node** | cache-optimizer | 8 vCPU / 32 GB RAM / NVMe | ~$60–120 | Memory is the point — predictive caching for popular reads. |
| **Compute node** | zk-reconstruction | 8–16 vCPU / 16–32 GB RAM | ~$80–160 | Bursty CPU for rebuilding compressed/ZK account state; spot/preemptible instances work well. |
| **Archive node** | archive | 4–8 vCPU / 32 GB RAM / multi-TB storage | ~$40–100 + storage | Bare-metal or storage-optimized VPS; deep historical shards and backfills. |

Guidance: **start on VPS, graduate hot roles to bare-metal only when the race
math justifies it.** A single operator can run one entry node profitably; the
network only needs a handful of operators to be useful. Coordination and
settlement run against **Solana mainnet** (devnet for M1) — batched every five
minutes so on-chain fees stay a rounding error rather than a per-query tax.

### Production-viability checklist

The gap between the current codebase and a network teams will trust is
operational, not architectural. Each item below is required before StreamSync
can carry production traffic:

- **Operator incentive & settlement contract** — deployed and audited on-chain
  payment/racing/settlement so winners, verifiers, and data providers are paid
  correctly and slashing is enforceable, not theoretical.
- **Latency-SLA verification** — independent, tamper-resistant measurement of
  the sub-10ms SLA at the gateway, so "pay on outcome" is auditable rather than
  self-reported by operators.
- **Shard rebalancing** — automated, low-disruption hash-ring rebalancing as
  operators join and leave, so no partition of Solana state goes cold or
  overloaded.
- **Failover** — structural redundancy proven under operator churn: because every
  query is raced to 3–5 nodes, an operator dropping mid-query must be a non-event,
  validated under fault injection.
- **Monitoring** — network-wide observability (per-operator latency, accuracy,
  uptime, reputation, settlement health) with alerting, so operators and
  integrators can trust the numbers without running their own probes.

Delivering that checklist across M1–M3 is what turns StreamSync from
code-complete into production-viable.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). The most valuable early contributions are
operator tooling, SLA-verification hardening, and honest benchmarks against
centralized RPC/indexers.
