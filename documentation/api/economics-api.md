# Economics API

API for staking, rewards, and payments.

---

## Staking

### Get Stake Info

```http
GET /economics/stake/{wallet}
```

```json
{
  "wallet": "...",
  "stakedAmount": 50000,
  "pendingUnstake": 0,
  "unstakeTimestamp": null,
  "pendingRewards": 125.5,
  "totalEarned": 2450.0
}
```

### Stake Tokens

```http
POST /economics/stake
```

```json
{
  "amount": 10000,
  "nodeId": "node-001"
}
```

### Begin Unstake

```http
POST /economics/unstake
```

```json
{
  "amount": 5000
}
```

---

## Rewards

### Get Pending Rewards

```http
GET /economics/rewards/pending
```

### Claim Rewards

```http
POST /economics/rewards/claim
```

### Reward History

```http
GET /economics/rewards/history?days=30
```

---

## Pricing

### Get Current Pricing

```http
GET /economics/pricing
```

```json
{
  "tiers": {
    "free": { "limit": 1000, "pricePerQuery": 0 },
    "basic": { "limit": 50000, "pricePerQuery": 0.001 },
    "pro": { "limit": 1000000, "pricePerQuery": 0.0008 }
  },
  "multipliers": {
    "complexity": { "get_account": 1.2, "search": 2.0 },
    "sla": { "10ms": 1.5, "5ms": 2.0 }
  }
}
```

### Calculate Query Cost

```http
POST /economics/pricing/calculate
```

```json
{
  "queryType": "get_account",
  "tier": "pro",
  "sla": "10ms",
  "paymentToken": "STRM"
}
```

---

## Credits

### Get Balance

```http
GET /economics/credits
```

### Add Credits

```http
POST /economics/credits
```

```json
{
  "amount": 1000,
  "paymentToken": "SOL"
}
```
