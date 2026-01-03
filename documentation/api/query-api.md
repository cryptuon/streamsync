# Query API

RESTful API for querying Solana data.

---

## Base URL

```
https://api.streamsync.io/v1
```

---

## Authentication

```bash
curl -H "Authorization: Bearer YOUR_API_KEY" \
  https://api.streamsync.io/v1/account/...
```

---

## Endpoints

### Get Account

```http
GET /account/{pubkey}
```

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `pubkey` | string | Account public key |
| `encoding` | string | `base64`, `jsonParsed` (default) |

**Response:**

```json
{
  "pubkey": "So11111111111111111111111111111111111111112",
  "lamports": 1000000000,
  "owner": "11111111111111111111111111111111",
  "data": { ... },
  "executable": false,
  "rentEpoch": 123
}
```

---

### Get Transaction

```http
GET /transaction/{signature}
```

**Response:**

```json
{
  "signature": "5x...",
  "slot": 123456789,
  "blockTime": 1704067200,
  "status": "confirmed",
  "fee": 5000,
  "instructions": [ ... ]
}
```

---

### Get Token Accounts

```http
GET /tokens/{owner}
```

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `owner` | string | Wallet public key |
| `mint` | string | Filter by mint (optional) |

**Response:**

```json
{
  "tokens": [
    {
      "mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
      "amount": "1000000",
      "decimals": 6,
      "uiAmount": 1.0
    }
  ]
}
```

---

### Search Transactions

```http
POST /transactions/search
```

**Body:**

```json
{
  "program": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
  "account": "...",
  "startTime": 1704067200,
  "endTime": 1704153600,
  "limit": 100
}
```

---

## Performance Options

### SLA Header

```bash
curl -H "X-Max-Latency: 10ms" \
  https://api.streamsync.io/v1/account/...
```

If SLA is missed, the query is free (automatic refund).

### Priority

```bash
curl -H "X-Priority: high" \
  https://api.streamsync.io/v1/account/...
```

---

## Rate Limits

| Tier | Requests/Second |
|------|-----------------|
| Free | 10 |
| Basic | 100 |
| Pro | 1,000 |
| Enterprise | Unlimited |

---

## Error Codes

| Code | Description |
|------|-------------|
| 400 | Bad request |
| 401 | Unauthorized |
| 404 | Not found |
| 429 | Rate limited |
| 500 | Server error |
| 504 | SLA timeout (refunded) |

---

## SDKs

```bash
# TypeScript
npm install @streamsync/sdk

# Rust
cargo add streamsync-sdk

# Python
pip install streamsync
```

### TypeScript Example

```typescript
import { StreamSync } from '@streamsync/sdk';

const client = new StreamSync({ apiKey: 'YOUR_KEY' });

const account = await client.getAccount(pubkey, {
  maxLatency: '10ms'
});
```
