# FAQ

Frequently asked questions about StreamSync.

---

## General

??? question "What is StreamSync?"
    StreamSync is a decentralized indexing network for Solana that provides fast, reliable query access with performance guarantees. Multiple independent operators compete to serve queries, ensuring fair pricing and high availability.

??? question "How is it different from Helius, QuickNode, etc.?"
    Traditional providers are centralized - one company controls pricing, access, and availability. StreamSync is economically decentralized from day one: multiple independent operators compete, market forces set prices, and protocol-level guarantees ensure access.

??? question "What performance can I expect?"
    Most queries complete in under 10ms. You can specify SLA requirements, and if they're not met, you don't pay.

---

## For Users

??? question "How do I pay for queries?"
    You can pay with STRM (lowest fees), SOL, or USDC. Payments are handled automatically through the network.

??? question "Is there a free tier?"
    Yes! 1,000 free queries per month to get started.

??? question "What if the query fails or is slow?"
    If the query doesn't meet your specified SLA, you're automatically refunded. No action required.

??? question "Which query types are supported?"
    - Account lookups
    - Transaction history
    - Token balances
    - Program accounts
    - Complex analytics

---

## For Node Operators

??? question "How much can I earn?"
    Earnings depend on your performance, stake, and network demand. Top operators can earn significant rewards. See [Rewards](../tokenomics/rewards.md) for projections.

??? question "What hardware do I need?"
    Minimum: 8 cores, 32GB RAM, 500GB SSD. Recommended: 32+ cores, 128GB RAM, 4TB NVMe. See [Running a Node](../operators/running-a-node.md).

??? question "How much stake is required?"
    Minimum 10,000 STRM. Higher stakes improve your selection probability and provide reward multipliers.

??? question "What happens if my node goes down?"
    Short outages (<1 hour) result in minor reputation impact. Extended outages may result in slashing. Use monitoring and redundancy.

---

## Token & Economics

??? question "What is $STRM?"
    STRM is the native utility token for the StreamSync network. It's used for payments, staking, and governance.

??? question "Where can I buy STRM?"
    STRM is available on Jupiter, Raydium, and Orca (Solana DEXs).

??? question "How are rewards distributed?"
    50% to node operators, 20% to treasury, 20% to data providers, 10% to governance stakers.

??? question "What's the unstaking period?"
    7 days. You can begin unstaking anytime, but tokens are locked during the cooldown.

---

## Technical

??? question "What database does StreamSync use?"
    DuckDB for local query execution, distributed across nodes using consistent hashing.

??? question "How does the gossip protocol work?"
    Nodes use push-pull gossip for state synchronization and failure detection. See [Gossip Protocol](../architecture/gossip-protocol.md).

??? question "Is the code open source?"
    Yes! Available at [github.com/your-org/streamsync](https://github.com/your-org/streamsync).

??? question "How do I report bugs?"
    Open an issue on GitHub or reach out on Discord.

---

## Security

??? question "Is StreamSync audited?"
    Smart contract audits are in progress. See [Security](../architecture/overview.md#security-architecture).

??? question "What happens if a node provides wrong data?"
    Incorrect results are detected by verifiers, and the node's stake is slashed. The query is re-routed to other nodes.

??? question "Can operators censor queries?"
    No. Queries are routed to multiple nodes, and any operator can serve them. Protocol-level access cannot be revoked.
