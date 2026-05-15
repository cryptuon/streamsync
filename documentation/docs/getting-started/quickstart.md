# Quick Start

Get up and running with StreamSync in under 5 minutes.

---

## Prerequisites

- Rust 1.75+ (for building from source)
- Solana CLI (optional, for staking operations)
- 8GB RAM minimum

---

## Option 1: Query the Network (Users)

### Install the CLI

```bash
# Install from crates.io
cargo install streamsync-cli

# Or download pre-built binary
curl -sSL https://get.streamsync.io | sh
```

### Make Your First Query

```bash
# Query an account
streamsync query account So11111111111111111111111111111111111111112

# Query with performance guarantee (refund if missed)
streamsync query account So11111111111111111111111111111111111111112 --max-latency 10ms

# Query token accounts
streamsync query tokens --owner <WALLET_PUBKEY>

# Search transactions
streamsync query transactions --program <PROGRAM_ID> --limit 100
```

### API Access

```bash
# Get API key
streamsync auth register --email your@email.com

# Use in requests
curl -H "Authorization: Bearer YOUR_API_KEY" \
  https://api.streamsync.io/v1/account/So11111111111111111111111111111111111111112
```

---

## Option 2: Run a Node (Operators)

### Clone and Build

```bash
# Clone repository
git clone https://github.com/your-org/streamsync.git
cd streamsync

# Build release binary
cargo build --release

# Verify build (193+ tests)
cargo test --workspace
```

### Initialize Node

```bash
# Generate node configuration
./target/release/streamsync init \
  --node-type speed-runner \
  --data-dir ./data \
  --config node.toml

# Edit configuration
vim node.toml
```

### Configure `node.toml`

```toml
[node]
id = "my-node-001"
type = "speed-runner"
region = "us-east-1"

[network]
listen_address = "0.0.0.0:8080"
discovery_nodes = [
    "discovery-1.streamsync.io:7878",
    "discovery-2.streamsync.io:7878"
]

[economics]
stake_account = "YOUR_STAKE_PUBKEY"
reward_address = "YOUR_REWARD_PUBKEY"
```

### Start the Node

```bash
# Run the node
./target/release/streamsync run --config node.toml

# Or with Docker
docker run -d \
  -v ./node.toml:/etc/streamsync/node.toml \
  -v ./data:/data \
  -p 8080:8080 \
  streamsync/node:latest
```

---

## Option 3: Stake STRM (Token Holders)

### Get STRM Tokens

STRM is available on:

- Jupiter (Solana DEX)
- Raydium
- Orca

### Stake Tokens

```bash
# Install Solana CLI if needed
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"

# Stake 10,000 STRM (minimum)
streamsync stake 10000 --node-pubkey <NODE_PUBKEY>

# Check staking status
streamsync stake status --wallet <YOUR_WALLET>

# View pending rewards
streamsync rewards pending
```

### Unstake Tokens

```bash
# Begin unstaking (7-day cooldown)
streamsync unstake 5000

# After cooldown, withdraw
streamsync withdraw
```

---

## Next Steps

<div class="grid cards" markdown>

-   :material-book-open-variant:{ .lg .middle } __Learn Concepts__

    ---

    Understand how StreamSync works

    [:octicons-arrow-right-24: Concepts](../concepts/overview.md)

-   :material-api:{ .lg .middle } __API Reference__

    ---

    Explore the full API

    [:octicons-arrow-right-24: API Docs](../api/query-api.md)

-   :material-server:{ .lg .middle } __Run a Node__

    ---

    Join the network as an operator

    [:octicons-arrow-right-24: Operator Guide](../operators/running-a-node.md)

-   :material-currency-usd:{ .lg .middle } __Token Economics__

    ---

    Understand STRM tokenomics

    [:octicons-arrow-right-24: Tokenomics](../tokenomics/strm-token.md)

</div>

---

## Getting Help

- **Discord**: [discord.gg/streamsync](https://discord.gg/streamsync)
- **GitHub Issues**: [github.com/your-org/streamsync/issues](https://github.com/your-org/streamsync/issues)
- **Documentation**: You're here!

!!! question "Need help?"
    Join our Discord community for real-time support from the team and other users.
