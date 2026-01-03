# StreamSync Token Custody & Wallet Management

## Overview

StreamSync implements a **hybrid custody model** that balances security, user control, and operational efficiency. Here's where tokens are held:

## 🏦 **Token Storage Locations**

### 1. **Treasury Wallet (Cold Storage)**
- **Location**: Multisig cold storage wallet (5-of-3 threshold)
- **Holds**: Protocol treasury funds, emergency reserves
- **Security**: Offline storage, multiple authorized signers required
- **Address**: `TREASURYxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx` (placeholder)
- **Purpose**: Long-term protocol sustainability, major upgrades

### 2. **Rewards Distribution Wallet (Hot Wallet)**
- **Location**: Encrypted hot wallet on secure servers
- **Holds**: Node operator reward pools, operational funds
- **Security**: Hardware Security Module (HSM) protected
- **File**: `/secure/rewards_wallet.json` (encrypted)
- **Purpose**: Automated reward distribution to node operators

### 3. **User Wallets (Non-Custodial)**
- **Location**: User's own wallets (Phantom, Solflare, Ledger, etc.)
- **Holds**: User's personal token balances
- **Security**: User controls private keys
- **Purpose**: Users maintain full custody of their tokens

### 4. **Escrow Accounts (Smart Contracts)**
- **Location**: On-chain escrow programs
- **Holds**: Pending payments, staked tokens, dispute funds
- **Security**: Smart contract controlled release conditions
- **Purpose**: Secure intermediary for payments and staking

## 💰 **Token Flow Architecture**

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   User Wallet   │────│  Escrow Account  │────│ StreamSync APIs │
│  (Phantom etc.) │    │  (Smart Contract)│    │                 │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                              │
                              ▼
                       ┌──────────────────┐
                       │ Rewards Wallet   │
                       │   (Hot Wallet)   │
                       └──────────────────┘
                              │
                              ▼
                       ┌──────────────────┐
                       │ Treasury Wallet  │
                       │  (Cold Storage)  │
                       └──────────────────┘
```

## 🔄 **Payment & Custody Process**

### **1. User Deposits Credits**
```rust
// User sends tokens to escrow account
let deposit_tx = user_wallet.transfer_to_escrow(amount, token);

// StreamSync verifies transaction
let verified = wallet_manager.verify_deposit_transaction(
    deposit_tx.signature,
    &token,
    amount
).await?;

// Credits added to user account
economics.add_credits(user_id, amount, token).await?;
```

### **2. API Usage & Payment**
```rust
// User makes API request (credits deducted from internal balance)
let cost = economics.calculate_request_cost(query_type, data_size, priority, tier, token);
let charged = economics.charge_request(user_id, cost).await?;

// Tokens remain in escrow until distribution
```

### **3. Revenue Distribution**
```rust
// Periodic distribution to stakeholders
let rewards = revenue_manager.distribute_revenue(period_id, total_revenue).await?;

// Automated transfers from hot wallet
let signatures = wallet_manager.distribute_node_rewards(rewards).await?;
```

## 🔒 **Security Model**

### **Multi-Signature Treasury**
- **5 authorized signers**, **3 signatures required**
- Geographic distribution of signers
- Time-locked transactions for large amounts
- Emergency recovery procedures

### **Hot Wallet Security**
- Hardware Security Module (HSM) protected
- Encrypted at rest with AES-256
- Rate limiting on withdrawals
- Real-time monitoring and alerts

### **Smart Contract Escrow**
- Immutable release conditions
- Time-based unlocks
- Dispute resolution mechanisms
- Slashing for malicious behavior

## 📍 **Token Mint Addresses**

| Token | Mint Address | Network |
|-------|--------------|---------|
| **STRM** | `STRMxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx` | Mainnet |
| **SOL** | `11111111111111111111111111111112` | Native |
| **USDC** | `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` | Mainnet |

## 🚨 **Emergency Procedures**

### **Circuit Breakers**
- Automatic withdrawal suspension on anomalies
- Multi-sig required for emergency fund access
- User account freezing for suspicious activity

### **Recovery Process**
1. **Detection**: Automated monitoring alerts
2. **Assessment**: Security team investigation
3. **Response**: Coordinated response plan execution
4. **Recovery**: Asset recovery through backup systems

## 💼 **Operational Wallets**

### **Node Operator Wallets**
- Each node operator provides their own wallet address
- Rewards distributed directly to operator wallets
- No custody of operator funds by StreamSync

### **Governance Wallet**
- Controlled by governance token holders
- Time-locked for major protocol changes
- Transparent voting and execution process

## 🔄 **Audit & Compliance**

### **Regular Audits**
- Smart contract security audits
- Treasury balance reconciliation
- Hot wallet security assessments

### **Transparency**
- Real-time treasury balance monitoring
- Public transaction logs
- Quarterly financial reports

## 📱 **User Experience**

### **Wallet Integration**
```typescript
// Connect user wallet
const wallet = await window.solana.connect();

// Add credits to StreamSync account
const depositTx = await streamSync.addCredits({
  amount: 100,
  token: 'STRM',
  userWallet: wallet.publicKey
});
```

### **Non-Custodial Benefits**
- ✅ Users maintain full control of their tokens
- ✅ No risk of centralized exchange hacks
- ✅ Transparent on-chain transactions
- ✅ Self-custody security model

## 🎯 **Key Advantages**

1. **Security**: Multi-layer security with cold storage
2. **Transparency**: All transactions on-chain and auditable
3. **User Control**: Non-custodial model preserves user sovereignty
4. **Operational Efficiency**: Automated distributions via hot wallets
5. **Compliance**: Clear audit trails and governance processes

## 🔧 **Implementation Status**

- ✅ **Wallet Manager**: Core custody logic implemented
- ✅ **Multi-signature**: Treasury wallet configuration
- ✅ **Escrow System**: Smart contract integration ready
- ✅ **User Integration**: Wallet connection and verification
- 🔄 **Smart Contracts**: Escrow contracts in development
- 🔄 **STRM Token**: Token deployment planned

The StreamSync custody model ensures that user funds are secure, protocol operations are efficient, and all stakeholders maintain appropriate control over their assets.