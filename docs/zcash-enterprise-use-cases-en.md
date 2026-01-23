# Zcash Enterprise Use Cases Guide

This document provides detailed descriptions of Zcash privacy transfer applications for enterprise environments using the Web3 Wallet Service, including specific business workflows, API examples, and best practices.

---

## Table of Contents

1. [Cryptocurrency Payment Gateway](#1-cryptocurrency-payment-gateway)
2. [Corporate Treasury Management](#2-corporate-treasury-management)
3. [OTC Trading Platform](#3-otc-trading-platform)
4. [Privacy-Focused Exchange](#4-privacy-focused-exchange)
5. [Cross-Border Remittance Service](#5-cross-border-remittance-service)
6. [Institutional Digital Asset Custody](#6-institutional-digital-asset-custody)
7. [Supply Chain Finance Privacy Payments](#7-supply-chain-finance-privacy-payments)
8. [Payroll Distribution System](#8-payroll-distribution-system)

---

## 1. Cryptocurrency Payment Gateway

### 1.1 Business Scenario

E-commerce platforms, SaaS providers, and gaming companies need to accept ZEC payments while protecting customer privacy.

### 1.2 Business Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                       Payment Gateway Flow                               │
└─────────────────────────────────────────────────────────────────────────┘

   Customer                Payment Gateway                 Merchant
      │                        │                              │
      │  1. Initiate payment   │                              │
      │ ──────────────────────▶│                              │
      │                        │                              │
      │  2. Return payment     │                              │
      │     address (u1...)    │                              │
      │ ◀──────────────────────│                              │
      │                        │                              │
      │  3. Customer sends ZEC │                              │
      │     (T→Z shielding)    │                              │
      │ ──────────────────────▶│                              │
      │                        │                              │
      │                        │  4. Detect incoming funds    │
      │                        │     Update order status      │
      │                        │ ────────────────────────────▶│
      │                        │                              │
      │                        │  5. Periodic settlement      │
      │                        │     (Z→T or Z→Z)             │
      │                        │ ────────────────────────────▶│
      │                        │                              │
```

### 1.3 Technical Implementation

#### Step 1: Create Separate Wallet for Each Merchant

```bash
# Create merchant wallet
POST /api/v1/wallets
{
  "name": "merchant_shop_001",
  "chain": "zcash"
}

# Response
{
  "id": 101,
  "name": "merchant_shop_001",
  "address": "t1XYZ...",  # Transparent address
  "chain": "zcash"
}
```

#### Step 2: Enable Orchard to Generate Unified Address

```bash
# Enable Orchard
POST /api/v1/wallets/101/orchard/enable
{
  "birthday_height": 2400000  # Current block height
}

# Response
{
  "unified_address": "u1qwerty...",
  "transparent_address": "t1XYZ...",
  "birthday_height": 2400000
}
```

#### Step 3: Generate Unique Payment Address for Each Order

```bash
# Get unified address (contains Orchard receiver)
GET /api/v1/wallets/101/orchard/addresses

# Response
{
  "unified_address": "u1qwerty...",
  "orchard_address": "...",
  "transparent_address": "t1XYZ..."
}
```

#### Step 4: Monitor Incoming Payments

```bash
# Periodically check shielded balance changes
GET /api/v1/wallets/101/orchard/balance

# Response
{
  "total_zatoshis": 150000000,      # 1.5 ZEC
  "spendable_zatoshis": 150000000,
  "pending_zatoshis": 0,
  "note_count": 3
}

# Query specific Notes (each incoming payment)
GET /api/v1/wallets/101/orchard/notes

# Response
{
  "notes": [
    {
      "id": 1,
      "value_zatoshis": 50000000,   # 0.5 ZEC
      "block_height": 2400100,
      "tx_hash": "abc123...",
      "memo": "Order #12345",        # Order ID (encrypted memo)
      "is_spent": false
    },
    ...
  ]
}
```

#### Step 5: Settlement to Merchant (Z→Z for Privacy)

```bash
# Initiate shielded transfer to merchant's unified address
POST /api/v1/transfers/orchard
{
  "wallet_id": 101,
  "to_address": "u1merchant_main_wallet...",
  "amount": "1.5",
  "memo": "Daily settlement 2024-01-20",
  "fund_source": "Shielded"
}

# Response
{
  "transfer_id": 5001,
  "status": "pending",
  "estimated_fee": "0.0001"
}

# Execute transfer
POST /api/v1/transfers/orchard/5001/execute

# Response
{
  "tx_hash": "def456...",
  "status": "broadcast"
}
```

### 1.4 Privacy Advantages

| Stage | Traditional Approach | Zcash Approach |
|-------|---------------------|----------------|
| Customer Payment | Amount visible | Hidden after T→Z |
| Merchant Receipt | Balance visible | Fully private |
| Settlement | Traceable | Untraceable with Z→Z |

### 1.5 Recommended Configuration

```env
# Payment gateway configuration
WEB3_ZCASH__RPC_URL=http://zcash-node:8232

# High-frequency scenario: use multiple RPC nodes
WEB3_ZCASH__FALLBACK_RPCS=http://node2:8232,http://node3:8232
```

---

## 2. Corporate Treasury Management

### 2.1 Business Scenario

Large enterprise finance departments managing crypto assets need:
- Multi-level approval workflows
- Separation of duties (initiator vs. approver)
- Complete audit trails
- Fund privacy protection

### 2.2 Organizational Structure

```
┌─────────────────────────────────────────────────────────────────────────┐
│                   Corporate Treasury Architecture                        │
└─────────────────────────────────────────────────────────────────────────┘

                        ┌──────────────┐
                        │     CFO      │
                        │   (Admin)    │
                        └──────┬───────┘
                               │
              ┌────────────────┼────────────────┐
              │                │                │
       ┌──────▼──────┐  ┌──────▼──────┐  ┌──────▼──────┐
       │  Finance    │  │  Finance    │  │    Risk     │
       │  Manager    │  │  Manager    │  │  Manager    │
       │ (Operator)  │  │ (Operator)  │  │ (Operator)  │
       │  Initiate   │  │  Approve    │  │  View Only  │
       └─────────────┘  └─────────────┘  └─────────────┘

Wallet Structure:
┌─────────────────────────────────────────────────────────────────────────┐
│                                                                          │
│   ┌─────────────┐    ┌─────────────┐    ┌─────────────┐                │
│   │ Operations  │    │   Reserve   │    │    Cold     │                │
│   │   Wallet    │    │   Wallet    │    │   Wallet    │                │
│   │ (Hot Wallet)│    │(Warm Wallet)│    │  (Offline)  │                │
│   │Daily Payments│   │ Mid-term    │    │ Long-term   │                │
│   │  < 10 ZEC   │    │ 10-100 ZEC  │    │  > 100 ZEC  │                │
│   └─────────────┘    └─────────────┘    └─────────────┘                │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 2.3 Business Workflow

#### Scenario: Supplier Payment

```
Finance Staff          Finance Manager              CFO                System
    │                      │                         │                   │
    │  1. Initiate payment │                         │                   │
    │     Amount: 50 ZEC   │                         │                   │
    │ ────────────────────▶│                         │                   │
    │                      │                         │                   │
    │                      │  2. Review payment      │                   │
    │                      │     (Amount > 10 ZEC)   │                   │
    │                      │ ───────────────────────▶│                   │
    │                      │                         │                   │
    │                      │                         │  3. CFO approval  │
    │                      │                         │ ─────────────────▶│
    │                      │                         │                   │
    │                      │                         │   4. Execute      │
    │                      │                         │      transfer     │
    │ ◀───────────────────────────────────────────────────────────────── │
    │                      │                         │   5. Log audit    │
    │                      │                         │                   │
```

### 2.4 API Implementation

#### Step 1: Create Multi-Tier Wallet Structure

```bash
# Create operations wallet (hot wallet)
POST /api/v1/wallets
{ "name": "ops_hot_wallet", "chain": "zcash" }

# Create reserve wallet (warm wallet)
POST /api/v1/wallets
{ "name": "reserve_warm_wallet", "chain": "zcash" }

# Create cold wallet (offline signing, import public key only)
POST /api/v1/wallets
{ "name": "cold_storage", "chain": "zcash" }
```

#### Step 2: Initiate Transfer (Status: Pending Approval)

```bash
# Finance staff initiates transfer
POST /api/v1/transfers/orchard
{
  "wallet_id": 201,
  "to_address": "u1supplier_address...",
  "amount": "50",
  "memo": "Invoice #INV-2024-0120 | PO #PO-2024-0089",
  "fund_source": "Shielded"
}

# Response - Creates pending approval transfer record
{
  "transfer_id": 6001,
  "status": "pending_approval",
  "initiated_by": "finance_staff_01",
  "amount": "50",
  "fee": "0.0001",
  "created_at": "2024-01-20T10:30:00Z"
}
```

#### Step 3: Approval Process

```bash
# Finance manager views pending approvals
GET /api/v1/transfers?status=pending_approval&wallet_id=201

# Response
{
  "transfers": [
    {
      "id": 6001,
      "amount": "50",
      "to_address": "u1supplier...",
      "memo": "Invoice #INV-2024-0120...",
      "initiated_by": "finance_staff_01",
      "status": "pending_approval"
    }
  ]
}

# Finance manager approves (requires CFO second approval)
# Note: Approval workflow implemented at application layer
```

#### Step 4: Execute Approved Transfer

```bash
# CFO or authorized admin executes
POST /api/v1/transfers/orchard/6001/execute

# Response
{
  "tx_hash": "abc123...",
  "status": "broadcast",
  "executed_by": "cfo_admin",
  "executed_at": "2024-01-20T14:00:00Z"
}
```

#### Step 5: Audit Log Query

```bash
# Query audit logs
GET /api/v1/audit-logs?resource=transfer&resource_id=6001

# Response
{
  "logs": [
    {
      "action": "transfer_initiated",
      "user": "finance_staff_01",
      "timestamp": "2024-01-20T10:30:00Z",
      "details": { "amount": "50", "to": "u1supplier..." }
    },
    {
      "action": "transfer_approved",
      "user": "finance_manager_01",
      "timestamp": "2024-01-20T12:00:00Z"
    },
    {
      "action": "transfer_approved",
      "user": "cfo_admin",
      "timestamp": "2024-01-20T13:50:00Z"
    },
    {
      "action": "transfer_executed",
      "user": "cfo_admin",
      "timestamp": "2024-01-20T14:00:00Z",
      "details": { "tx_hash": "abc123..." }
    }
  ]
}
```

### 2.5 Permission Matrix

| Operation | Admin (CFO) | Operator (Manager) | Viewer (Auditor) |
|-----------|-------------|-------------------|------------------|
| Create Wallet | ✓ | ✗ | ✗ |
| Initiate Transfer | ✓ | ✓ | ✗ |
| Approve Transfer | ✓ | ✓ (within limit) | ✗ |
| Execute Transfer | ✓ | ✗ | ✗ |
| View Balance | ✓ | ✓ | ✓ |
| Export Private Key | ✓ | ✗ | ✗ |
| View Audit Logs | ✓ | ✓ | ✓ |

---

## 3. OTC Trading Platform

### 3.1 Business Scenario

Over-the-counter (OTC) trading platforms facilitate large cryptocurrency trades requiring:
- Protection of buyer/seller identities
- Transaction amount confidentiality
- Transaction receipts
- Dispute resolution support

### 3.2 Trading Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          OTC Trading Flow                                │
└─────────────────────────────────────────────────────────────────────────┘

   Buyer                  OTC Platform                    Seller
    │                        │                              │
    │  1. Submit buy order   │                              │
    │     100 ZEC @ $25      │                              │
    │ ──────────────────────▶│                              │
    │                        │                              │
    │                        │  2. Match with sell order    │
    │                        │◀──────────────────────────────│
    │                        │                              │
    │  3. Lock fiat in escrow│                              │
    │ ──────────────────────▶│                              │
    │                        │                              │
    │                        │  4. Notify seller to send    │
    │                        │     ZEC to escrow address    │
    │                        │ ────────────────────────────▶│
    │                        │                              │
    │                        │  5. Seller transfers (Z→Z)   │
    │                        │◀──────────────────────────────│
    │                        │     to platform escrow       │
    │                        │                              │
    │                        │  6. Confirm ZEC received     │
    │                        │     Release fiat to seller   │
    │                        │ ────────────────────────────▶│
    │                        │                              │
    │  7. Platform transfers │                              │
    │     ZEC (Z→Z) to buyer │                              │
    │◀──────────────────────│                              │
    │                        │                              │
    │  8. Generate receipt   │                              │
    │     (encrypted memo)   │                              │
    │                        │                              │
```

### 3.3 Technical Implementation

#### Step 1: Create Escrow Wallet Pool

```bash
# Create separate escrow wallet for each trade
POST /api/v1/wallets
{
  "name": "escrow_trade_20240120_001",
  "chain": "zcash"
}

# Enable Orchard
POST /api/v1/wallets/301/orchard/enable
{
  "birthday_height": 2450000
}
```

#### Step 2: Seller Deposits to Escrow (Z→Z)

```bash
# Seller transfers from their shielded wallet to escrow
# Memo contains trade ID for reconciliation

POST /api/v1/transfers/orchard
{
  "wallet_id": 302,  # Seller wallet
  "to_address": "u1escrow_trade_001...",
  "amount": "100",
  "memo": "OTC_TRADE_ID:TRD-20240120-001|SELLER:S001",
  "fund_source": "Shielded"
}
```

#### Step 3: Platform Confirms Receipt

```bash
# Query escrow wallet for incoming funds
GET /api/v1/wallets/301/orchard/notes

# Response
{
  "notes": [
    {
      "value_zatoshis": 10000000000,  # 100 ZEC
      "memo": "OTC_TRADE_ID:TRD-20240120-001|SELLER:S001",
      "block_height": 2450100,
      "is_spent": false
    }
  ]
}
```

#### Step 4: Release to Buyer (Z→Z)

```bash
# After fiat confirmation, transfer to buyer
POST /api/v1/transfers/orchard
{
  "wallet_id": 301,  # Escrow wallet
  "to_address": "u1buyer_address...",
  "amount": "99.999",  # Minus fee
  "memo": "OTC_TRADE_ID:TRD-20240120-001|BUYER:B001|COMPLETED",
  "fund_source": "Shielded"
}

POST /api/v1/transfers/orchard/7001/execute
```

### 3.4 Privacy Protection Analysis

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     Privacy Protection Analysis                          │
└─────────────────────────────────────────────────────────────────────────┘

On-chain visible information:
├── Seller outgoing: Only "a shielded transaction occurred" is visible
├── Escrow receipt: Amount and sender are hidden
├── Escrow outgoing: Amount and recipient are hidden
└── Buyer receipt: Amount and sender are hidden

Off-chain information (platform only):
├── Buyer identity (KYC)
├── Seller identity (KYC)
├── Transaction amount
├── Transaction price
└── Encrypted memo content
```

### 3.5 Dispute Resolution

```bash
# If dispute occurs, platform can provide evidence:

# 1. Query escrow wallet complete transaction history
GET /api/v1/wallets/301/orchard/notes?include_spent=true

# 2. Export transaction receipt (requires admin permission)
GET /api/v1/transfers/7001

# Response includes complete transaction details and encrypted memo
{
  "id": 7001,
  "from_wallet_id": 301,
  "to_address": "u1buyer...",
  "amount": "99.999",
  "memo": "OTC_TRADE_ID:TRD-20240120-001|BUYER:B001|COMPLETED",
  "tx_hash": "xyz789...",
  "status": "confirmed",
  "block_height": 2450200
}
```

---

## 4. Privacy-Focused Exchange

### 4.1 Business Scenario

Exchanges handling large volumes of ZEC deposits and withdrawals need to:
- Protect user deposit privacy
- Optimize withdrawal fees
- Meet regulatory requirements

### 4.2 Deposit/Withdrawal Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                   Exchange Deposit/Withdrawal Flow                       │
└─────────────────────────────────────────────────────────────────────────┘

                     Deposit Flow (T→Z)
┌─────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│  User   │───▶│ User T-Addr │───▶│ Exchange    │───▶│  Hot Wallet │
│         │    │  t1user...  │    │ Unified Addr│    │  Pool       │
└─────────┘    └─────────────┘    │ u1ex...     │    │ (Shielded)  │
                  On-chain         └─────────────┘    └─────────────┘
                  Visible            Shielding         Private Storage


                     Withdrawal Flow (Z→T or Z→Z)
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│  Hot Wallet │───▶│    User     │───▶│    User     │
│    Pool     │    │  Withdraw   │    │  Receives   │
│ (Shielded)  │    │   Address   │    │   Funds     │
└─────────────┘    └─────────────┘    └─────────────┘
Private Storage    Z→T: t1user...      Funds Received
                   Z→Z: u1user...
```

### 4.3 Technical Implementation

#### Deposit Address Generation

```bash
# Generate unique deposit address for each user
# Option A: Use transparent address (easier tracking)
GET /api/v1/wallets/401/address  # Returns t1 address

# Option B: Use unified address (better privacy)
GET /api/v1/wallets/401/orchard/addresses
# Returns u1 address, supports both transparent and shielded deposits
```

#### Deposit Detection and Consolidation

```bash
# 1. Monitor transparent address for incoming funds
GET /api/v1/wallets/401/balance?chain=zcash

# 2. Consolidate to shielded hot wallet (T→Z)
POST /api/v1/transfers/orchard
{
  "wallet_id": 401,
  "to_address": "u1hot_wallet...",
  "amount": "10.5",
  "fund_source": "Transparent"  # Use transparent balance
}

# 3. Or monitor shielded deposits
GET /api/v1/wallets/401/orchard/balance
```

#### Withdrawal Processing

```bash
# User requests withdrawal to transparent address (Z→T)
POST /api/v1/transfers/orchard
{
  "wallet_id": 402,  # Hot wallet
  "to_address": "t1user_withdraw_address...",  # Transparent address
  "amount": "5.0",
  "memo": "Withdrawal #WD-20240120-001",
  "fund_source": "Shielded"
}

# User requests withdrawal to shielded address (Z→Z, protects user privacy)
POST /api/v1/transfers/orchard
{
  "wallet_id": 402,
  "to_address": "u1user_privacy_address...",  # Unified address
  "amount": "5.0",
  "fund_source": "Shielded"
}
```

### 4.4 Batch Withdrawal Optimization

```bash
# Combine multiple small withdrawals to reduce on-chain fees

# Single withdrawal fee: 0.0001 ZEC
# Batch withdrawal (10 combined) fee: ~0.0005 ZEC
# Savings: ~50%

# Note: Current system doesn't support multi-output single transactions
# Recommended: Scheduled batch processing every 10 minutes
```

### 4.5 Regulatory Compliance

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    Regulatory Compliance Framework                       │
└─────────────────────────────────────────────────────────────────────────┘

1. User KYC
   ├── Identity verification before deposits
   └── Secondary verification for large withdrawals

2. Transaction Tracking
   ├── Complete off-chain deposit/withdrawal records
   ├── Audit logs retained for 7 years
   └── Encrypted memos record transaction IDs

3. Suspicious Activity Reporting
   ├── Monitor abnormal deposit/withdrawal patterns
   └── Auto-generate SAR reports

4. Viewing Key
   ├── Provide read-only key to regulators
   └── Allows auditing shielded balance without spending key
```

---

## 5. Cross-Border Remittance Service

### 5.1 Business Scenario

Cross-border remittance providers need:
- Fast settlement (avoid traditional SWIFT 3-5 days)
- Lower intermediary fees
- Sender privacy protection
- Compliance with different country regulations

### 5.2 Remittance Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                Cross-Border Remittance Flow (USA → Japan)                │
└─────────────────────────────────────────────────────────────────────────┘

  US Sender            US Agent              JP Agent           JP Recipient
      │                  │                      │                    │
      │  1. Send $1000   │                      │                    │
      │  to US agent     │                      │                    │
      │ ────────────────▶│                      │                    │
      │                  │                      │                    │
      │                  │  2. Purchase ZEC     │                    │
      │                  │     (local exchange) │                    │
      │                  │                      │                    │
      │                  │  3. Z→Z transfer     │                    │
      │                  │     to JP agent      │                    │
      │                  │ ───────────────────▶ │                    │
      │                  │                      │                    │
      │                  │                      │  4. Sell ZEC       │
      │                  │                      │     for JPY        │
      │                  │                      │                    │
      │                  │                      │  5. Pay recipient  │
      │                  │                      │ ──────────────────▶│
      │                  │                      │                    │

On-chain Privacy:
├── Cannot trace funds from USA to Japan
├── Cannot determine remittance amount
└── Only agents know transaction details
```

### 5.3 Technical Implementation

#### Agent Wallet Setup

```bash
# US Agent Wallet
POST /api/v1/wallets
{ "name": "remit_agent_us_001", "chain": "zcash" }

POST /api/v1/wallets/501/orchard/enable
{ "birthday_height": 2460000 }

# Japan Agent Wallet
POST /api/v1/wallets
{ "name": "remit_agent_jp_001", "chain": "zcash" }

POST /api/v1/wallets/502/orchard/enable
{ "birthday_height": 2460000 }
```

#### Remittance Transfer

```bash
# US agent transfers to Japan agent
POST /api/v1/transfers/orchard
{
  "wallet_id": 501,
  "to_address": "u1remit_agent_jp_001...",
  "amount": "38.5",  # ~$1000 in ZEC
  "memo": "REMIT|REF:RM-20240120-US-JP-001|AMT:1000USD|TO:Tanaka",
  "fund_source": "Shielded"
}
```

#### Exchange Rate Locking and Reconciliation

```bash
# Record exchange rate snapshot
{
  "remittance_id": "RM-20240120-US-JP-001",
  "source_amount": 1000,
  "source_currency": "USD",
  "zec_amount": 38.5,
  "zec_rate": 25.97,  # USD/ZEC
  "target_amount": 148500,
  "target_currency": "JPY",
  "jpy_rate": 153.5,  # JPY/USD
  "fee_percent": 1.5,
  "timestamp": "2024-01-20T10:00:00Z"
}
```

### 5.4 Multi-Currency Routing

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    Multi-Currency Routing Strategy                       │
└─────────────────────────────────────────────────────────────────────────┘

Case 1: Small Remittances (< $500)
├── Path: Direct ZEC transfer (Z→Z)
├── Advantage: Maximum privacy
└── Fee: ~$0.01

Case 2: Large Remittances (> $5000)
├── Path: ETH + ZEC combination
├── ETH: Fast settlement for majority
├── ZEC: Privacy transfer for remainder
└── Advantage: Speed + privacy balance

Case 3: Urgent Remittances
├── Path: ETH or USDT (stablecoin)
├── Advantage: Confirmation within 10 minutes
└── Trade-off: Privacy
```

---

## 6. Institutional Digital Asset Custody

### 6.1 Business Scenario

Custody providers serving funds, family offices, and other institutions need:
- Secure storage of large assets
- Audit and compliance support
- Flexible withdrawal permissions

### 6.2 Custody Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                   Institutional Custody Architecture                     │
└─────────────────────────────────────────────────────────────────────────┘

                      Custody Provider Infrastructure
┌─────────────────────────────────────────────────────────────────────────┐
│                                                                          │
│   ┌─────────────────────────────────────────────────────────────────┐   │
│   │                   HSM (Hardware Security Module)                 │   │
│   │   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐          │   │
│   │   │ Master Key 1│   │ Master Key 2│   │ Master Key 3│          │   │
│   │   │    (USA)    │   │   (Europe)  │   │   (Asia)    │          │   │
│   │   └─────────────┘   └─────────────┘   └─────────────┘          │   │
│   │           Multi-geographic Distribution + Multi-signature        │   │
│   └─────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│   Client Asset Segregation                                               │
│   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐                  │
│   │  Client A   │   │  Client B   │   │  Client C   │                  │
│   │ Hedge Fund  │   │ Family Off  │   │  Pension    │                  │
│   │  500 ZEC    │   │  2000 ZEC   │   │ 10000 ZEC   │                  │
│   │ u1clientA.. │   │ u1clientB.. │   │ u1clientC.. │                  │
│   └─────────────┘   └─────────────┘   └─────────────┘                  │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 6.3 Technical Implementation

#### Client Wallet Segregation

```bash
# Create separate wallet for each institutional client
POST /api/v1/wallets
{
  "name": "custody_client_hedgefund_alpha",
  "chain": "zcash"
}

# Enable Orchard
POST /api/v1/wallets/601/orchard/enable
{
  "birthday_height": 2470000
}
```

#### Viewing Key Export (for Auditors)

```bash
# Export viewing key (without spending key)
# Note: This feature requires extension implementation

GET /api/v1/wallets/601/orchard/viewing-key

# Response
{
  "viewing_key": "zxviews1...",  # Can only view balance/transactions, cannot spend
  "wallet_id": 601,
  "address": "u1clientA..."
}
```

#### Periodic Proof of Reserves

```bash
# Generate proof of reserves report
GET /api/v1/wallets/601/orchard/balance

# Response
{
  "wallet_id": 601,
  "client": "Hedge Fund Alpha",
  "balance": {
    "total_zatoshis": 50000000000,  # 500 ZEC
    "spendable_zatoshis": 50000000000,
    "pending_zatoshis": 0
  },
  "proof_of_reserves": {
    "block_height": 2480000,
    "merkle_root": "abc123...",
    "timestamp": "2024-01-20T00:00:00Z"
  }
}
```

### 6.4 Withdrawal Approval Process

```bash
# 1. Client submits withdrawal request (off-chain system)
{
  "request_id": "WD-601-20240120-001",
  "client_id": "hedgefund_alpha",
  "amount": "50",
  "to_address": "u1client_external...",
  "reason": "Quarterly distribution"
}

# 2. Custodian reviews and executes
POST /api/v1/transfers/orchard
{
  "wallet_id": 601,
  "to_address": "u1client_external...",
  "amount": "50",
  "memo": "Custody withdrawal|REF:WD-601-20240120-001",
  "fund_source": "Shielded"
}

# 3. Multi-signature approval (requires 2/3 signatures)
# Note: Multi-sig implementation at HSM layer
```

---

## 7. Supply Chain Finance Privacy Payments

### 7.1 Business Scenario

Payments between manufacturers and suppliers requiring:
- Purchase price protection (trade secrets)
- Supplier privacy (protect customer relationships)
- Invoice reconciliation support

### 7.2 Payment Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     Supply Chain Payment Flow                            │
└─────────────────────────────────────────────────────────────────────────┘

   Purchasing Dept             ERP System                  Supplier
      │                           │                           │
      │  1. Create purchase order │                           │
      │ ─────────────────────────▶│                           │
      │                           │                           │
      │                           │  2. Send PO to supplier   │
      │                           │ ─────────────────────────▶│
      │                           │                           │
      │                           │  3. Supplier ships +      │
      │                           │     sends invoice         │
      │                           │◀───────────────────────── │
      │                           │                           │
      │  4. Confirm receipt       │                           │
      │ ─────────────────────────▶│                           │
      │                           │                           │
      │                           │  5. Trigger ZEC payment   │
      │                           │     (Z→Z shielded)        │
      │                           │ ─────────────────────────▶│
      │                           │                           │
      │                           │  6. Supplier confirms     │
      │                           │◀───────────────────────── │
      │                           │                           │

On-chain Privacy Protection:
├── Competitors cannot determine procurement volumes
├── Suppliers cannot be identified
└── Pricing information completely confidential
```

### 7.3 Technical Implementation

#### System Integration

```bash
# ERP system initiates payment via API
POST /api/v1/transfers/orchard
{
  "wallet_id": 701,  # Corporate payment wallet
  "to_address": "u1supplier_component_co...",
  "amount": "150.5",  # ~$3900
  "memo": "PO:2024-0120-001|INV:INV-SUP-0089|HASH:sha256...",
  "fund_source": "Shielded"
}

# Memo structure:
# PO: Purchase order number
# INV: Supplier invoice number
# HASH: Invoice file hash (for verification)
```

#### Automated Reconciliation

```bash
# Query paid invoices
GET /api/v1/wallets/701/orchard/notes?is_spent=true

# Response
{
  "notes": [
    {
      "tx_hash": "abc123...",
      "amount": "150.5",
      "memo": "PO:2024-0120-001|INV:INV-SUP-0089|...",
      "spent_at": "2024-01-20T15:00:00Z"
    }
  ]
}

# ERP system parses memo, auto-marks invoice as paid
```

### 7.4 Batch Payment Optimization

```bash
# Month-end batch payments to multiple suppliers
# Approach: Create payment batch, execute sequentially

suppliers = [
  { "address": "u1sup_a...", "amount": "50", "ref": "INV-A-001" },
  { "address": "u1sup_b...", "amount": "75.5", "ref": "INV-B-002" },
  { "address": "u1sup_c...", "amount": "120", "ref": "INV-C-003" }
]

for sup in suppliers:
    POST /api/v1/transfers/orchard
    {
      "wallet_id": 701,
      "to_address": sup.address,
      "amount": sup.amount,
      "memo": f"BATCH:PAY-202401|REF:{sup.ref}",
      "fund_source": "Shielded"
    }
```

---

## 8. Payroll Distribution System

### 8.1 Business Scenario

Companies using ZEC for employee salary or bonus payments:
- Protect employee salary privacy
- Support cross-border team payments
- Reduce international transfer fees

### 8.2 Payroll Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                       Payroll Distribution Flow                          │
└─────────────────────────────────────────────────────────────────────────┘

   HR System                 Wallet Service                 Employee
      │                           │                            │
      │  1. Generate payroll      │                            │
      │     (CSV/API)             │                            │
      │ ─────────────────────────▶│                            │
      │                           │                            │
      │                           │  2. Verify sufficient      │
      │                           │     balance                │
      │                           │                            │
      │                           │  3. Batch execute          │
      │                           │     transfers (Z→Z)        │
      │                           │ ──────────────────────────▶│
      │                           │                            │
      │  4. Return distribution   │                            │
      │     report                │                            │
      │◀─────────────────────────│                            │
      │                           │                            │
      │  5. Notify employees      │                            │
      │ ─────────────────────────────────────────────────────▶ │
      │                           │                            │

Privacy Protection:
├── Employees cannot view each other's salaries
├── Outsiders cannot analyze company compensation spending
└── Each transfer is independent and unlinkable
```

### 8.3 Technical Implementation

#### Payroll Data Import

```bash
# Payroll data format
payroll = {
  "pay_period": "2024-01",
  "payments": [
    {
      "employee_id": "EMP-001",
      "name": "Alice",
      "address": "u1alice...",
      "amount": "3.85",  # ~$100
      "type": "salary"
    },
    {
      "employee_id": "EMP-002",
      "name": "Bob",
      "address": "u1bob...",
      "amount": "4.62",  # ~$120
      "type": "salary"
    },
    {
      "employee_id": "EMP-003",
      "name": "Charlie",
      "address": "u1charlie...",
      "amount": "1.15",  # ~$30
      "type": "bonus"
    }
  ]
}
```

#### Batch Distribution

```bash
# Initiate transfers sequentially
for payment in payroll.payments:
    POST /api/v1/transfers/orchard
    {
      "wallet_id": 801,  # Payroll wallet
      "to_address": payment.address,
      "amount": payment.amount,
      "memo": f"PAYROLL|{payroll.pay_period}|{payment.employee_id}|{payment.type}",
      "fund_source": "Shielded"
    }

# Execute all pending transfers
for transfer_id in pending_transfers:
    POST /api/v1/transfers/orchard/{transfer_id}/execute
```

#### Distribution Report

```bash
# Generate distribution report
GET /api/v1/transfers?wallet_id=801&created_after=2024-01-25

# Response
{
  "summary": {
    "total_amount": "9.62",
    "total_count": 3,
    "success_count": 3,
    "failed_count": 0
  },
  "transfers": [
    {
      "id": 8001,
      "to_address": "u1alice...",
      "amount": "3.85",
      "tx_hash": "abc...",
      "status": "confirmed"
    },
    ...
  ]
}
```

---

## Appendix A: API Quick Reference

### Wallet Management

| Operation | Endpoint | Method |
|-----------|----------|--------|
| Create Wallet | `/api/v1/wallets` | POST |
| Enable Orchard | `/api/v1/wallets/{id}/orchard/enable` | POST |
| Get Addresses | `/api/v1/wallets/{id}/orchard/addresses` | GET |
| Query Balance | `/api/v1/wallets/{id}/orchard/balance` | GET |
| Query Notes | `/api/v1/wallets/{id}/orchard/notes` | GET |

### Transfer Operations

| Operation | Endpoint | Method |
|-----------|----------|--------|
| Initiate Transfer | `/api/v1/transfers/orchard` | POST |
| Execute Transfer | `/api/v1/transfers/orchard/{id}/execute` | POST |
| Query Transfer | `/api/v1/transfers/{id}` | GET |
| List Transfers | `/api/v1/transfers` | GET |

### Sync Management

| Operation | Endpoint | Method |
|-----------|----------|--------|
| Sync Status | `/api/v1/zcash/scan/status` | GET |
| Manual Sync | `/api/v1/zcash/scan/sync` | POST |

---

## Appendix B: Best Practices

### Security Recommendations

1. **Key Management**
   - Use HSM for master key storage in production
   - Rotate API keys regularly
   - Enable IP whitelisting

2. **Network Security**
   - Use HTTPS for all API calls
   - Access RPC nodes via internal network
   - Enable firewall rules

3. **Audit Compliance**
   - Maintain complete audit logs
   - Regular backup exports
   - Implement access controls

### Performance Optimization

1. **Sync Optimization**
   - Set reasonable birthday_height
   - Avoid syncing from genesis block
   - Use dedicated RPC nodes

2. **Batch Operations**
   - Combine small transfers
   - Use queues for high-volume requests
   - Implement rate limiting

### Disaster Recovery

1. **Node Failures**
   - Configure multiple RPC backup nodes
   - Implement automatic failover

2. **Data Recovery**
   - Regular database backups
   - Secure private key backups
   - Test recovery procedures

---

## Appendix C: Glossary

| Term | Description |
|------|-------------|
| T-Address | Transparent address, starts with t1 |
| Z-Address | Shielded address (legacy Sapling) |
| Unified Address | Combined address, starts with u1, contains multiple receivers |
| Orchard | Latest Zcash privacy protocol using Halo 2 proofs |
| Halo 2 | Zero-knowledge proof system without trusted setup |
| Note | Shielded UTXO containing amount and recipient info |
| Nullifier | Marks Note as spent, prevents double-spending |
| Shielding | T→Z transfer |
| Deshielding | Z→T transfer |
| ZIP-317 | Zcash fee standard |
| Zatoshi | Smallest ZEC unit, 1 ZEC = 10^8 zatoshi |

---

*Document Version: 1.0*
*Last Updated: 2024-01-20*
