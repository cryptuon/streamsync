# StreamSync Token Economics

The definitive guide to $STRM token economics, staking, rewards, and network incentives.

---

## Token Overview

### $STRM Token

The primary utility token for the StreamSync network.

| Property | Value |
|----------|-------|
| **Symbol** | STRM |
| **Blockchain** | Solana |
| **Program ID** | `STRMxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx` |
| **Decimals** | 9 |
| **Total Supply** | 1,000,000,000 STRM |

### Supported Payment Tokens

Users can pay for queries using:

| Token | Use Case |
|-------|----------|
| **STRM** | Native token, lowest fees |
| **SOL** | Solana native, convenient |
| **USDC** | Stablecoin, predictable costs |
| **SPL Tokens** | Custom integrations |

---

## Token Distribution

| Allocation | Percentage | Amount | Vesting |
|------------|------------|--------|---------|
| **Node Operator Rewards** | 40% | 400M STRM | Emitted over 10 years |
| **Protocol Treasury** | 20% | 200M STRM | DAO controlled |
| **Team & Advisors** | 15% | 150M STRM | 4-year vesting, 1-year cliff |
| **Early Investors** | 10% | 100M STRM | 2-year vesting, 6-month cliff |
| **Community & Ecosystem** | 10% | 100M STRM | Grants, bounties, partnerships |
| **Liquidity** | 5% | 50M STRM | DEX liquidity provision |

---

## Revenue Distribution

All query fees are distributed according to these percentages:

```
┌─────────────────────────────────────────────────────────────┐
│                    Query Fee (100%)                          │
└─────────────────────────────────────────────────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        ▼                   ▼                   ▼
┌───────────────┐   ┌───────────────┐   ┌───────────────┐
│ Node Operators│   │   Treasury    │   │ Data Providers│
│     50%       │   │     20%       │   │     20%       │
└───────────────┘   └───────────────┘   └───────────────┘
                            │
                            ▼
                    ┌───────────────┐
                    │  Governance   │
                    │     10%       │
                    └───────────────┘
```

### Distribution Breakdown

| Recipient | Percentage | Purpose |
|-----------|------------|---------|
| **Node Operators** | 50% | Reward nodes serving queries |
| **Protocol Treasury** | 20% | Fund development, audits, operations |
| **Data Providers** | 20% | Compensate Solana RPC providers |
| **Governance** | 10% | Distribute to STRM stakers |

---

## Racing Competition Rewards

When nodes race to answer queries:

```
┌─────────────────────────────────────────────────────────────┐
│                   Query Payment                              │
└─────────────────────────────────────────────────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        ▼                   ▼                   ▼
┌───────────────┐   ┌───────────────┐   ┌───────────────┐
│    Winner     │   │  Verifier 1   │   │  Verifier 2   │
│     70%       │   │     15%       │   │     15%       │
│ (first correct│   │ (confirms     │   │ (confirms     │
│   response)   │   │   result)     │   │   result)     │
└───────────────┘   └───────────────┘   └───────────────┘
```

### Racing Rules

1. **3-5 nodes** race per query
2. **First correct response** wins 70%
3. **Two verifiers** confirm correctness, earn 15% each
4. Rewards **batch every 5 minutes** for gas efficiency
5. Performance affects **future selection probability**

---

## Staking Requirements

### Node Operator Staking

| Parameter | Value |
|-----------|-------|
| **Minimum Stake** | 10,000 STRM |
| **Unstaking Cooldown** | 7 days |
| **Slashing Max** | 10% per incident |

### Staking Benefits

| Stake Amount | Selection Weight | Reward Multiplier |
|--------------|------------------|-------------------|
| 10,000 STRM | 1.0x | 1.0x |
| 50,000 STRM | 1.5x | 1.1x |
| 100,000 STRM | 2.0x | 1.2x |
| 500,000 STRM | 3.0x | 1.3x |
| 1,000,000 STRM | 4.0x | 1.5x |

### Slashing Conditions

| Violation | Penalty |
|-----------|---------|
| Incorrect query result | 2% of stake |
| Downtime > 1 hour | 0.5% of stake |
| Consensus violation | 5% of stake |
| Malicious behavior | 10% of stake |

---

## Pricing Tiers

### Query Pricing

| Tier | Requests/Month | STRM per Request | SOL per Request | Features |
|------|----------------|------------------|-----------------|----------|
| **Free** | 1,000 | 0 | 0 | Basic queries |
| **Basic** | 50,000 | 0.001 | 0.00001 | + Transaction history, Account lookups |
| **Pro** | 1,000,000 | 0.0008 | 0.000008 | + Webhooks, Analytics, Priority support |
| **Enterprise** | Unlimited | 0.0005 | 0.000005 | + Custom integrations, SLA, Dedicated support |

### Cost Multipliers

| Factor | Multiplier Range |
|--------|------------------|
| **Query Complexity** | 1.0x - 5.0x |
| **Data Size** | 1.0x - 2.0x |
| **Priority** | 1.0x - 2.0x |
| **Performance Guarantee** | 1.0x - 3.0x |

#### Complexity Examples

| Query Type | Multiplier |
|------------|------------|
| `get_transaction` | 1.0x |
| `get_account` | 1.2x |
| `get_token_accounts` | 1.5x |
| `search_transactions` | 2.0x |
| `get_program_accounts` | 3.0x |
| `complex_analytics` | 5.0x |

---

## Node Specialization Bonuses

Different node types earn different premiums:

| Specialization | Bonus | Requirements |
|----------------|-------|--------------|
| **SpeedRunner** | +30% | Sub-1ms latency capability |
| **ReconstructionSpec** | +100% | ZK reconstruction capability |
| **CacheOptimizer** | +50% | 64GB+ RAM, high bandwidth |
| **ArchiveNode** | +20% | 4TB+ storage, 99.9% uptime |

---

## Settlement Process

### Batch Settlement (Every 5 Minutes)

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│ Query 1     │    │ Settlement  │    │ Solana      │
│ Query 2     │───▶│ Batch       │───▶│ Transaction │
│ Query 3     │    │ (aggregate) │    │ (on-chain)  │
│ ...         │    │             │    │             │
└─────────────┘    └─────────────┘    └─────────────┘
                          │
                          ▼
                   ┌─────────────┐
                   │ Node Stakes │
                   │ Updated     │
                   └─────────────┘
```

### Why Batch?

- **Gas efficiency**: Single transaction for many rewards
- **Lower costs**: ~0.000005 SOL per reward vs 0.00001 per individual
- **Atomic**: All rewards in batch succeed or fail together

---

## Governance

### Voting Power

| Source | Weight |
|--------|--------|
| Staked STRM | 1 vote per STRM |
| Node Operation Time | +10% per year (max 50%) |
| Performance Score | +0-20% based on metrics |

### Governable Parameters

| Parameter | Current Value | Range |
|-----------|---------------|-------|
| Minimum Stake | 10,000 STRM | 1,000 - 100,000 |
| Unstaking Cooldown | 7 days | 1 - 30 days |
| Winner Percentage | 70% | 50% - 80% |
| Verifier Percentage | 15% | 10% - 25% |
| Protocol Fee | 20% | 5% - 30% |
| Slashing Percentages | Various | 0.1% - 10% |

---

## Economic Sustainability

### Target Metrics

| Metric | Target |
|--------|--------|
| Node Operator ROI | 15-30% annually |
| Customer Savings vs Centralized | 20%+ |
| Protocol Self-Sufficiency | Year 2+ |

### Revenue Projections

| Network Stage | Daily Queries | Daily Revenue | Node Earnings |
|---------------|---------------|---------------|---------------|
| Launch | 100K | $100 | $50 |
| Growth | 10M | $8,000 | $4,000 |
| Mature | 1B | $500,000 | $250,000 |

---

## Smart Contract Architecture

### Program Accounts (PDAs)

```rust
// Global configuration
ProgramConfig {
    admin: Pubkey,
    oracle_authority: Pubkey,
    min_stake: u64,           // 10,000 STRM
    cooldown_seconds: i64,    // 604,800 (7 days)
    winner_bps: u16,          // 7000 (70%)
    verifier_bps: u16,        // 1500 (15%)
    protocol_fee_bps: u16,    // 2000 (20%)
    paused: bool,
}

// Per-node staking account
NodeStake {
    owner: Pubkey,
    staked_amount: u64,
    pending_unstake: u64,
    unstake_timestamp: i64,
    pending_rewards: u64,
    total_earned: u64,
    queries_won: u64,
    queries_verified: u64,
    specialization: NodeSpecialization,
    is_active: bool,
}

// Settlement batch
SettlementBatch {
    batch_id: u64,
    created_at: i64,
    processed: bool,
    total_rewards: u64,
    reward_count: u16,
    rewards: Vec<BatchReward>,
}
```

### Instructions

| Instruction | Who Can Call | Purpose |
|-------------|--------------|---------|
| `initialize` | Deployer (once) | Set up program |
| `stake` | Anyone | Become node operator |
| `add_stake` | Node owner | Increase stake |
| `begin_unstake` | Node owner | Start cooldown |
| `withdraw` | Node owner | Complete unstake |
| `claim_rewards` | Node owner | Claim pending rewards |
| `record_reward` | Oracle only | Record query result |
| `process_batch` | Anyone | Settle pending rewards |
| `slash` | Admin only | Penalize misbehavior |

---

## Integration Example

```rust
use streamsync::{EconomicsManager, PaymentToken};

// Create economics manager
let (economics, _receiver) = EconomicsManager::new();

// Create user account
let user_id = economics.create_account(
    "api_key_xxx".to_string(),
    "pro".to_string()
).await?;

// Add credits
economics.add_credits(user_id, 1000.0, PaymentToken::STRM).await?;

// Calculate query cost
let cost = economics.calculate_request_cost(
    "get_account",      // query type
    10,                 // data size KB
    1.0,                // priority
    "pro",              // tier
    PaymentToken::STRM  // payment token
);

// Charge for request
economics.charge_request(user_id, cost).await?;

// Distribute revenue (called periodically)
economics.distribute_revenue(PaymentToken::STRM).await?;
```

---

## Summary

| Aspect | Value |
|--------|-------|
| **Token** | $STRM on Solana |
| **Supply** | 1 billion |
| **Node Revenue** | 50% of fees |
| **Racing Winner** | 70% of query payment |
| **Min Stake** | 10,000 STRM |
| **Unstake Cooldown** | 7 days |
| **Settlement** | Every 5 minutes |

This model ensures:
- **Nodes are rewarded** for performance
- **Protocol is sustainable** through treasury
- **Users get fair pricing** based on usage
- **Governance enables** community control
