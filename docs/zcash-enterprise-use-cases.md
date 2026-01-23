# Zcash 企业级应用场景详解

本文档详细描述 Web3 Wallet Service 在企业环境中的 Zcash 隐私转账应用场景，包括具体业务流程、API 调用示例和最佳实践。

---

## 目录

1. [加密货币支付网关](#1-加密货币支付网关)
2. [企业资金管理系统](#2-企业资金管理系统)
3. [OTC 大宗交易平台](#3-otc-大宗交易平台)
4. [隐私交易所](#4-隐私交易所)
5. [跨境支付与汇款服务](#5-跨境支付与汇款服务)
6. [机构数字资产托管](#6-机构数字资产托管)
7. [供应链金融隐私支付](#7-供应链金融隐私支付)
8. [薪酬发放系统](#8-薪酬发放系统)

---

## 1. 加密货币支付网关

### 1.1 业务场景

电商平台、SaaS 服务商、游戏公司等需要接受 ZEC 支付，同时保护客户隐私。

### 1.2 业务流程

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          支付网关流程                                    │
└─────────────────────────────────────────────────────────────────────────┘

    客户                    支付网关                    商户
      │                        │                        │
      │  1. 发起支付请求       │                        │
      │ ──────────────────────▶│                        │
      │                        │                        │
      │  2. 返回收款地址       │                        │
      │    (统一地址 u1...)    │                        │
      │ ◀──────────────────────│                        │
      │                        │                        │
      │  3. 客户转账 ZEC       │                        │
      │    (T→Z 屏蔽化)        │                        │
      │ ──────────────────────▶│                        │
      │                        │                        │
      │                        │  4. 检测到入账         │
      │                        │     更新订单状态       │
      │                        │ ──────────────────────▶│
      │                        │                        │
      │                        │  5. 定期结算           │
      │                        │    (Z→T 或 Z→Z)       │
      │                        │ ──────────────────────▶│
      │                        │                        │
```

### 1.3 技术实现

#### 步骤 1：为每个商户创建独立钱包

```bash
# 创建商户钱包
POST /api/v1/wallets
{
  "name": "merchant_shop_001",
  "chain": "zcash"
}

# 响应
{
  "id": 101,
  "name": "merchant_shop_001",
  "address": "t1XYZ...",  # 透明地址
  "chain": "zcash"
}
```

#### 步骤 2：启用 Orchard 生成统一地址

```bash
# 启用 Orchard
POST /api/v1/wallets/101/orchard/enable
{
  "birthday_height": 2400000  # 当前区块高度
}

# 响应
{
  "unified_address": "u1qwerty...",
  "transparent_address": "t1XYZ...",
  "birthday_height": 2400000
}
```

#### 步骤 3：为每笔订单生成唯一收款地址

```bash
# 获取统一地址（包含 Orchard 接收器）
GET /api/v1/wallets/101/orchard/addresses

# 响应
{
  "unified_address": "u1qwerty...",
  "orchard_address": "...",
  "transparent_address": "t1XYZ..."
}
```

#### 步骤 4：监听入账

```bash
# 定期查询隐私余额变化
GET /api/v1/wallets/101/orchard/balance

# 响应
{
  "total_zatoshis": 150000000,      # 1.5 ZEC
  "spendable_zatoshis": 150000000,
  "pending_zatoshis": 0,
  "note_count": 3
}

# 查询具体 Notes（每笔入账）
GET /api/v1/wallets/101/orchard/notes

# 响应
{
  "notes": [
    {
      "id": 1,
      "value_zatoshis": 50000000,   # 0.5 ZEC
      "block_height": 2400100,
      "tx_hash": "abc123...",
      "memo": "Order #12345",        # 订单号（加密备忘录）
      "is_spent": false
    },
    ...
  ]
}
```

#### 步骤 5：结算到商户（Z→Z 保持隐私）

```bash
# 发起隐私转账到商户统一地址
POST /api/v1/transfers/orchard
{
  "wallet_id": 101,
  "to_address": "u1merchant_main_wallet...",
  "amount": "1.5",
  "memo": "Daily settlement 2024-01-20",
  "fund_source": "Shielded"
}

# 响应
{
  "transfer_id": 5001,
  "status": "pending",
  "estimated_fee": "0.0001"
}

# 执行转账
POST /api/v1/transfers/orchard/5001/execute

# 响应
{
  "tx_hash": "def456...",
  "status": "broadcast"
}
```

### 1.4 隐私优势

| 环节 | 传统方案 | Zcash 方案 |
|------|---------|-----------|
| 客户付款 | 金额公开 | T→Z 后金额隐藏 |
| 商户收款 | 余额公开 | 完全隐私 |
| 结算 | 可追踪 | Z→Z 不可追踪 |

### 1.5 配置建议

```env
# 支付网关推荐配置
WEB3_ZCASH__RPC_URL=http://zcash-node:8232

# 高频场景使用多个 RPC 节点
WEB3_ZCASH__FALLBACK_RPCS=http://node2:8232,http://node3:8232
```

---

## 2. 企业资金管理系统

### 2.1 业务场景

大型企业财务部门管理加密资产，需要：
- 多级审批流程
- 职责分离（发起人 vs 审批人）
- 完整审计追踪
- 资金隐私保护

### 2.2 组织架构

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        企业资金管理架构                                  │
└─────────────────────────────────────────────────────────────────────────┘

                        ┌──────────────┐
                        │   CFO        │
                        │  (Admin)     │
                        └──────┬───────┘
                               │
              ┌────────────────┼────────────────┐
              │                │                │
       ┌──────▼──────┐  ┌──────▼──────┐  ┌──────▼──────┐
       │  财务经理   │  │  财务经理   │  │  风控经理   │
       │ (Operator)  │  │ (Operator)  │  │ (Operator)  │
       │  发起转账   │  │  审批转账   │  │  只读查看   │
       └─────────────┘  └─────────────┘  └─────────────┘

钱包结构：
┌─────────────────────────────────────────────────────────────────────────┐
│                                                                          │
│   ┌─────────────┐    ┌─────────────┐    ┌─────────────┐                │
│   │  运营钱包   │    │  储备钱包   │    │  冷钱包     │                │
│   │  (热钱包)   │    │  (温钱包)   │    │  (离线)     │                │
│   │  日常支付   │    │  中期储备   │    │  长期存储   │                │
│   │  < 10 ZEC   │    │  10-100 ZEC │    │  > 100 ZEC  │                │
│   └─────────────┘    └─────────────┘    └─────────────┘                │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 2.3 业务流程

#### 场景：支付供应商款项

```
财务专员                 财务经理                 CFO                  系统
    │                      │                      │                      │
    │  1. 发起付款申请     │                      │                      │
    │  金额: 50 ZEC        │                      │                      │
    │ ────────────────────▶│                      │                      │
    │                      │                      │                      │
    │                      │  2. 审核付款信息     │                      │
    │                      │  (金额 > 10 ZEC)     │                      │
    │                      │ ────────────────────▶│                      │
    │                      │                      │                      │
    │                      │                      │  3. CFO 最终审批     │
    │                      │                      │ ────────────────────▶│
    │                      │                      │                      │
    │                      │                      │      4. 执行转账     │
    │                      │                      │      (Z→Z 隐私)      │
    │ ◀────────────────────────────────────────────────────────────────│
    │                      │                      │      5. 记录审计日志 │
    │                      │                      │                      │
```

### 2.4 API 实现

#### 步骤 1：创建多级钱包结构

```bash
# 创建运营钱包（热钱包）
POST /api/v1/wallets
{ "name": "ops_hot_wallet", "chain": "zcash" }

# 创建储备钱包（温钱包）
POST /api/v1/wallets
{ "name": "reserve_warm_wallet", "chain": "zcash" }

# 创建冷钱包（离线签名，仅导入公钥）
POST /api/v1/wallets
{ "name": "cold_storage", "chain": "zcash" }
```

#### 步骤 2：发起转账（状态：待审批）

```bash
# 财务专员发起转账
POST /api/v1/transfers/orchard
{
  "wallet_id": 201,
  "to_address": "u1supplier_address...",
  "amount": "50",
  "memo": "Invoice #INV-2024-0120 | PO #PO-2024-0089",
  "fund_source": "Shielded"
}

# 响应 - 创建待审批的转账记录
{
  "transfer_id": 6001,
  "status": "pending_approval",
  "initiated_by": "finance_staff_01",
  "amount": "50",
  "fee": "0.0001",
  "created_at": "2024-01-20T10:30:00Z"
}
```

#### 步骤 3：审批流程

```bash
# 财务经理查看待审批列表
GET /api/v1/transfers?status=pending_approval&wallet_id=201

# 响应
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

# 财务经理审批（需要 CFO 二次审批）
# 注：此为业务层逻辑，需在应用层实现审批工作流
```

#### 步骤 4：执行已审批的转账

```bash
# CFO 或有权限的管理员执行
POST /api/v1/transfers/orchard/6001/execute

# 响应
{
  "tx_hash": "abc123...",
  "status": "broadcast",
  "executed_by": "cfo_admin",
  "executed_at": "2024-01-20T14:00:00Z"
}
```

#### 步骤 5：审计日志查询

```bash
# 查询审计日志
GET /api/v1/audit-logs?resource=transfer&resource_id=6001

# 响应
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

### 2.5 权限矩阵

| 操作 | Admin (CFO) | Operator (经理) | Viewer (审计) |
|------|-------------|-----------------|---------------|
| 创建钱包 | ✓ | ✗ | ✗ |
| 发起转账 | ✓ | ✓ | ✗ |
| 审批转账 | ✓ | ✓ (限额内) | ✗ |
| 执行转账 | ✓ | ✗ | ✗ |
| 查看余额 | ✓ | ✓ | ✓ |
| 导出私钥 | ✓ | ✗ | ✗ |
| 查看审计日志 | ✓ | ✓ | ✓ |

---

## 3. OTC 大宗交易平台

### 3.1 业务场景

场外交易（OTC）平台撮合大额加密货币交易，需要：
- 保护买卖双方身份
- 保护交易金额
- 提供交易凭证
- 支持争议仲裁

### 3.2 交易流程

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         OTC 交易流程                                     │
└─────────────────────────────────────────────────────────────────────────┘

   买家                    OTC 平台                    卖家
    │                        │                         │
    │  1. 提交购买意向       │                         │
    │     100 ZEC @ $25      │                         │
    │ ──────────────────────▶│                         │
    │                        │                         │
    │                        │  2. 匹配卖家意向        │
    │                        │◀─────────────────────────│
    │                        │                         │
    │  3. 锁定法币到托管     │                         │
    │ ──────────────────────▶│                         │
    │                        │                         │
    │                        │  4. 通知卖家转 ZEC     │
    │                        │     到平台托管地址      │
    │                        │ ─────────────────────▶ │
    │                        │                         │
    │                        │  5. 卖家转账 (Z→Z)     │
    │                        │◀─────────────────────────│
    │                        │     到平台隐私托管      │
    │                        │                         │
    │                        │  6. 确认 ZEC 到账      │
    │                        │     释放法币给卖家      │
    │                        │ ─────────────────────▶ │
    │                        │                         │
    │  7. 平台转 ZEC (Z→Z)  │                         │
    │     到买家隐私地址     │                         │
    │◀──────────────────────│                         │
    │                        │                         │
    │  8. 生成交易凭证       │                         │
    │     (加密备忘录)       │                         │
    │                        │                         │
```

### 3.3 技术实现

#### 步骤 1：创建托管钱包池

```bash
# 为每笔交易创建独立托管钱包
POST /api/v1/wallets
{
  "name": "escrow_trade_20240120_001",
  "chain": "zcash"
}

# 启用 Orchard
POST /api/v1/wallets/301/orchard/enable
{
  "birthday_height": 2450000
}
```

#### 步骤 2：卖家转入托管（Z→Z）

```bash
# 卖家从自己的隐私钱包转到托管地址
# 备忘录包含交易 ID，用于后续对账

POST /api/v1/transfers/orchard
{
  "wallet_id": 302,  # 卖家钱包
  "to_address": "u1escrow_trade_001...",
  "amount": "100",
  "memo": "OTC_TRADE_ID:TRD-20240120-001|SELLER:S001",
  "fund_source": "Shielded"
}
```

#### 步骤 3：平台确认到账

```bash
# 查询托管钱包入账
GET /api/v1/wallets/301/orchard/notes

# 响应
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

#### 步骤 4：释放到买家（Z→Z）

```bash
# 法币确认后，转给买家
POST /api/v1/transfers/orchard
{
  "wallet_id": 301,  # 托管钱包
  "to_address": "u1buyer_address...",
  "amount": "99.999",  # 扣除手续费
  "memo": "OTC_TRADE_ID:TRD-20240120-001|BUYER:B001|COMPLETED",
  "fund_source": "Shielded"
}

POST /api/v1/transfers/orchard/7001/execute
```

### 3.4 隐私保护分析

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        隐私保护分析                                      │
└─────────────────────────────────────────────────────────────────────────┘

链上可见信息：
├── 卖家转出：仅可见"有一笔隐私交易发生"
├── 托管接收：金额、发送方均隐藏
├── 托管转出：金额、接收方均隐藏
└── 买家接收：金额、发送方均隐藏

链下信息（仅平台知晓）：
├── 买家身份（KYC）
├── 卖家身份（KYC）
├── 交易金额
├── 交易价格
└── 加密备忘录内容
```

### 3.5 争议处理

```bash
# 如果发生争议，平台可以提供证据：

# 1. 查询托管钱包完整交易记录
GET /api/v1/wallets/301/orchard/notes?include_spent=true

# 2. 导出交易凭证（需要管理员权限）
GET /api/v1/transfers/7001

# 响应包含完整的交易详情和加密备忘录
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

## 4. 隐私交易所

### 4.1 业务场景

交易所需要处理大量 ZEC 充值和提现，同时：
- 保护用户充值隐私
- 优化提现手续费
- 满足监管要求

### 4.2 充提流程

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        交易所充提流程                                    │
└─────────────────────────────────────────────────────────────────────────┘

                     充值流程 (T→Z)
┌─────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│  用户   │───▶│ 用户透明地址 │───▶│ 交易所统一  │───▶│  热钱包池   │
│         │    │  t1user...   │    │ 地址 u1ex.. │    │  (隐私)     │
└─────────┘    └─────────────┘    └─────────────┘    └─────────────┘
                  链上可见            屏蔽化             隐私存储


                     提现流程 (Z→T 或 Z→Z)
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│  热钱包池   │───▶│  用户提现   │───▶│   用户      │
│  (隐私)     │    │  地址       │    │   收款      │
└─────────────┘    └─────────────┘    └─────────────┘
   隐私存储        Z→T: t1user...      资金到账
                   Z→Z: u1user...
```

### 4.3 技术实现

#### 充值地址生成

```bash
# 为每个用户生成唯一充值地址
# 方案 A：使用透明地址（便于追踪）
GET /api/v1/wallets/401/address  # 返回 t1 地址

# 方案 B：使用统一地址（更好的隐私）
GET /api/v1/wallets/401/orchard/addresses
# 返回 u1 地址，支持透明和隐私两种转入方式
```

#### 充值检测与归集

```bash
# 1. 监控透明地址入账
GET /api/v1/wallets/401/balance?chain=zcash

# 2. 归集到隐私热钱包 (T→Z)
POST /api/v1/transfers/orchard
{
  "wallet_id": 401,
  "to_address": "u1hot_wallet...",
  "amount": "10.5",
  "fund_source": "Transparent"  # 使用透明余额
}

# 3. 或者监控隐私入账
GET /api/v1/wallets/401/orchard/balance
```

#### 提现处理

```bash
# 用户申请提现到透明地址（Z→T）
POST /api/v1/transfers/orchard
{
  "wallet_id": 402,  # 热钱包
  "to_address": "t1user_withdraw_address...",  # 透明地址
  "amount": "5.0",
  "memo": "Withdrawal #WD-20240120-001",
  "fund_source": "Shielded"
}

# 用户申请提现到隐私地址（Z→Z，保护用户隐私）
POST /api/v1/transfers/orchard
{
  "wallet_id": 402,
  "to_address": "u1user_privacy_address...",  # 统一地址
  "amount": "5.0",
  "fund_source": "Shielded"
}
```

### 4.4 批量提现优化

```bash
# 合并多笔小额提现，减少链上手续费

# 单笔提现手续费：0.0001 ZEC
# 批量提现（10笔合并）手续费：约 0.0005 ZEC
# 节省：约 50%

# 注意：当前系统暂不支持单交易多输出
# 建议方案：定时批量处理，每 10 分钟执行一批
```

### 4.5 监管合规

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        监管合规方案                                      │
└─────────────────────────────────────────────────────────────────────────┘

1. 用户 KYC
   ├── 充值前完成身份验证
   └── 大额提现二次验证

2. 交易追踪
   ├── 链下记录完整充提记录
   ├── 审计日志保存 7 年
   └── 加密备忘录记录交易 ID

3. 可疑交易报告
   ├── 监控异常充提模式
   └── 自动生成 SAR 报告

4. 查看密钥（Viewing Key）
   ├── 向监管机构提供只读密钥
   └── 允许审计隐私余额，不泄露支出密钥
```

---

## 5. 跨境支付与汇款服务

### 5.1 业务场景

跨境汇款服务商需要：
- 快速结算（避免传统 SWIFT 的 3-5 天）
- 降低中间费用
- 保护汇款人隐私
- 满足不同国家监管要求

### 5.2 汇款流程

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     跨境汇款流程（美国 → 日本）                          │
└─────────────────────────────────────────────────────────────────────────┘

  美国汇款人          美国代理商           日本代理商          日本收款人
      │                  │                    │                   │
      │  1. 汇款 $1000   │                    │                   │
      │  到美国代理      │                    │                   │
      │ ────────────────▶│                    │                   │
      │                  │                    │                   │
      │                  │  2. 购买 ZEC       │                   │
      │                  │     (当地交易所)   │                   │
      │                  │                    │                   │
      │                  │  3. Z→Z 转账       │                   │
      │                  │     到日本代理     │                   │
      │                  │ ──────────────────▶│                   │
      │                  │                    │                   │
      │                  │                    │  4. 卖出 ZEC      │
      │                  │                    │     获得 JPY      │
      │                  │                    │                   │
      │                  │                    │  5. 支付给收款人  │
      │                  │                    │ ─────────────────▶│
      │                  │                    │                   │

链上隐私：
├── 无法追踪资金从美国到日本
├── 无法确定汇款金额
└── 仅代理商知晓交易详情
```

### 5.3 技术实现

#### 代理商钱包设置

```bash
# 美国代理商钱包
POST /api/v1/wallets
{ "name": "remit_agent_us_001", "chain": "zcash" }

POST /api/v1/wallets/501/orchard/enable
{ "birthday_height": 2460000 }

# 日本代理商钱包
POST /api/v1/wallets
{ "name": "remit_agent_jp_001", "chain": "zcash" }

POST /api/v1/wallets/502/orchard/enable
{ "birthday_height": 2460000 }
```

#### 汇款转账

```bash
# 美国代理商转给日本代理商
POST /api/v1/transfers/orchard
{
  "wallet_id": 501,
  "to_address": "u1remit_agent_jp_001...",
  "amount": "38.5",  # 约 $1000 的 ZEC
  "memo": "REMIT|REF:RM-20240120-US-JP-001|AMT:1000USD|TO:Tanaka",
  "fund_source": "Shielded"
}
```

#### 汇率锁定与对账

```bash
# 记录汇率快照
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

### 5.4 多币种路由

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        多币种路由策略                                    │
└─────────────────────────────────────────────────────────────────────────┘

情况 1：小额汇款（< $500）
├── 路径：ZEC 直接转账（Z→Z）
├── 优势：隐私最大化
└── 费用：约 $0.01

情况 2：大额汇款（> $5000）
├── 路径：ETH + ZEC 组合
├── ETH：快速结算大部分金额
├── ZEC：隐私转账剩余部分
└── 优势：速度 + 隐私平衡

情况 3：紧急汇款
├── 路径：ETH 或 USDT（稳定币）
├── 优势：10 分钟内确认
└── 牺牲：隐私性
```

---

## 6. 机构数字资产托管

### 6.1 业务场景

托管服务商为基金、家族办公室等机构客户托管 ZEC 资产：
- 安全存储大额资产
- 支持审计和合规
- 提供灵活的提款权限

### 6.2 托管架构

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        机构托管架构                                      │
└─────────────────────────────────────────────────────────────────────────┘

                         托管服务商基础设施
┌─────────────────────────────────────────────────────────────────────────┐
│                                                                          │
│   ┌─────────────────────────────────────────────────────────────────┐   │
│   │                      HSM (硬件安全模块)                          │   │
│   │   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐          │   │
│   │   │  主私钥 1   │   │  主私钥 2   │   │  主私钥 3   │          │   │
│   │   │  (美国)     │   │  (欧洲)     │   │  (亚洲)     │          │   │
│   │   └─────────────┘   └─────────────┘   └─────────────┘          │   │
│   │              多地理位置分布 + 多签名                             │   │
│   └─────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│   客户资产隔离                                                           │
│   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐                  │
│   │  客户 A     │   │  客户 B     │   │  客户 C     │                  │
│   │  Hedge Fund │   │  Family Off │   │  Pension    │                  │
│   │  500 ZEC    │   │  2000 ZEC   │   │  10000 ZEC  │                  │
│   │  u1clientA..│   │  u1clientB..│   │  u1clientC..│                  │
│   └─────────────┘   └─────────────┘   └─────────────┘                  │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 6.3 技术实现

#### 客户钱包隔离

```bash
# 为每个机构客户创建独立钱包
POST /api/v1/wallets
{
  "name": "custody_client_hedgefund_alpha",
  "chain": "zcash"
}

# 启用 Orchard
POST /api/v1/wallets/601/orchard/enable
{
  "birthday_height": 2470000
}
```

#### 只读密钥导出（供审计使用）

```bash
# 导出查看密钥（不含支出密钥）
# 注：此功能需要扩展实现

GET /api/v1/wallets/601/orchard/viewing-key

# 响应
{
  "viewing_key": "zxviews1...",  # 仅能查看余额和交易，不能花费
  "wallet_id": 601,
  "address": "u1clientA..."
}
```

#### 定期资产证明

```bash
# 生成资产证明报告
GET /api/v1/wallets/601/orchard/balance

# 响应
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

### 6.4 提款审批流程

```bash
# 1. 客户发起提款请求（链下系统）
{
  "request_id": "WD-601-20240120-001",
  "client_id": "hedgefund_alpha",
  "amount": "50",
  "to_address": "u1client_external...",
  "reason": "Quarterly distribution"
}

# 2. 托管方审核并执行
POST /api/v1/transfers/orchard
{
  "wallet_id": 601,
  "to_address": "u1client_external...",
  "amount": "50",
  "memo": "Custody withdrawal|REF:WD-601-20240120-001",
  "fund_source": "Shielded"
}

# 3. 多签名批准（需要 2/3 签名）
# 注：多签名需要在 HSM 层实现
```

---

## 7. 供应链金融隐私支付

### 7.1 业务场景

制造企业与供应商之间的支付，需要：
- 保护采购价格（商业机密）
- 供应商隐私（不暴露客户关系）
- 支持发票对账

### 7.2 支付流程

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     供应链支付流程                                       │
└─────────────────────────────────────────────────────────────────────────┘

   采购企业                 ERP 系统                    供应商
      │                        │                         │
      │  1. 创建采购订单       │                         │
      │ ──────────────────────▶│                         │
      │                        │                         │
      │                        │  2. 发送 PO 到供应商    │
      │                        │ ───────────────────────▶│
      │                        │                         │
      │                        │  3. 供应商发货+发票    │
      │                        │◀─────────────────────── │
      │                        │                         │
      │  4. 收货确认           │                         │
      │ ──────────────────────▶│                         │
      │                        │                         │
      │                        │  5. 触发 ZEC 支付      │
      │                        │     (Z→Z 隐私转账)     │
      │                        │ ───────────────────────▶│
      │                        │                         │
      │                        │  6. 供应商确认收款     │
      │                        │◀─────────────────────── │
      │                        │                         │

链上隐私保护：
├── 竞争对手无法得知采购量
├── 供应商无法被识别
└── 价格信息完全保密
```

### 7.3 技术实现

#### 系统集成

```bash
# ERP 系统通过 API 发起支付
POST /api/v1/transfers/orchard
{
  "wallet_id": 701,  # 企业付款钱包
  "to_address": "u1supplier_component_co...",
  "amount": "150.5",  # 约 $3900
  "memo": "PO:2024-0120-001|INV:INV-SUP-0089|HASH:sha256...",
  "fund_source": "Shielded"
}

# 备忘录结构：
# PO: 采购订单号
# INV: 供应商发票号
# HASH: 发票文件哈希（用于验证）
```

#### 自动对账

```bash
# 查询已支付的发票
GET /api/v1/wallets/701/orchard/notes?is_spent=true

# 响应
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

# ERP 系统解析备忘录，自动标记发票已付
```

### 7.4 批量支付优化

```bash
# 月末批量支付多个供应商
# 方案：创建支付批次，逐一执行

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

## 8. 薪酬发放系统

### 8.1 业务场景

企业使用 ZEC 发放员工薪酬或奖金：
- 保护员工薪资隐私
- 满足跨境团队支付需求
- 减少国际转账费用

### 8.2 发薪流程

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        薪酬发放流程                                      │
└─────────────────────────────────────────────────────────────────────────┘

   HR 系统                  钱包服务                    员工
      │                        │                         │
      │  1. 生成工资单         │                         │
      │     (CSV/API)          │                         │
      │ ──────────────────────▶│                         │
      │                        │                         │
      │                        │  2. 验证余额充足        │
      │                        │                         │
      │                        │  3. 批量执行转账        │
      │                        │     (Z→Z 隐私)          │
      │                        │ ───────────────────────▶│
      │                        │                         │
      │  4. 返回发放报告       │                         │
      │◀──────────────────────│                         │
      │                        │                         │
      │  5. 通知员工           │                         │
      │ ─────────────────────────────────────────────▶ │
      │                        │                         │

隐私保护：
├── 员工之间无法互相查看薪资
├── 外部无法统计公司薪酬支出
└── 每笔转账独立，不可关联
```

### 8.3 技术实现

#### 工资单导入

```bash
# 工资单数据格式
payroll = {
  "pay_period": "2024-01",
  "payments": [
    {
      "employee_id": "EMP-001",
      "name": "Alice",
      "address": "u1alice...",
      "amount": "3.85",  # 约 $100
      "type": "salary"
    },
    {
      "employee_id": "EMP-002",
      "name": "Bob",
      "address": "u1bob...",
      "amount": "4.62",  # 约 $120
      "type": "salary"
    },
    {
      "employee_id": "EMP-003",
      "name": "Charlie",
      "address": "u1charlie...",
      "amount": "1.15",  # 约 $30
      "type": "bonus"
    }
  ]
}
```

#### 批量发放

```bash
# 逐一发起转账
for payment in payroll.payments:
    POST /api/v1/transfers/orchard
    {
      "wallet_id": 801,  # 薪酬钱包
      "to_address": payment.address,
      "amount": payment.amount,
      "memo": f"PAYROLL|{payroll.pay_period}|{payment.employee_id}|{payment.type}",
      "fund_source": "Shielded"
    }

# 执行所有待处理转账
for transfer_id in pending_transfers:
    POST /api/v1/transfers/orchard/{transfer_id}/execute
```

#### 发放报告

```bash
# 生成发放报告
GET /api/v1/transfers?wallet_id=801&created_after=2024-01-25

# 响应
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

## 附录 A：API 快速参考

### 钱包管理

| 操作 | 端点 | 方法 |
|------|------|------|
| 创建钱包 | `/api/v1/wallets` | POST |
| 启用 Orchard | `/api/v1/wallets/{id}/orchard/enable` | POST |
| 获取地址 | `/api/v1/wallets/{id}/orchard/addresses` | GET |
| 查询余额 | `/api/v1/wallets/{id}/orchard/balance` | GET |
| 查询 Notes | `/api/v1/wallets/{id}/orchard/notes` | GET |

### 转账操作

| 操作 | 端点 | 方法 |
|------|------|------|
| 发起转账 | `/api/v1/transfers/orchard` | POST |
| 执行转账 | `/api/v1/transfers/orchard/{id}/execute` | POST |
| 查询转账 | `/api/v1/transfers/{id}` | GET |
| 转账列表 | `/api/v1/transfers` | GET |

### 同步管理

| 操作 | 端点 | 方法 |
|------|------|------|
| 同步状态 | `/api/v1/zcash/scan/status` | GET |
| 手动同步 | `/api/v1/zcash/scan/sync` | POST |

---

## 附录 B：最佳实践

### 安全建议

1. **密钥管理**
   - 生产环境使用 HSM 存储主密钥
   - 定期轮换 API 密钥
   - 启用 IP 白名单

2. **网络安全**
   - 所有 API 调用使用 HTTPS
   - RPC 节点使用内网访问
   - 启用防火墙规则

3. **审计合规**
   - 保留完整审计日志
   - 定期导出备份
   - 实施访问控制

### 性能优化

1. **同步优化**
   - 设置合理的 birthday_height
   - 避免从创世块同步
   - 使用专用 RPC 节点

2. **批量操作**
   - 合并小额转账
   - 使用队列处理大量请求
   - 实施限流保护

### 故障恢复

1. **节点故障**
   - 配置多个 RPC 备用节点
   - 实施自动故障转移

2. **数据恢复**
   - 定期备份数据库
   - 保留私钥安全备份
   - 测试恢复流程

---

## 附录 C：术语表

| 术语 | 说明 |
|------|------|
| T-Address | 透明地址，以 t1 开头 |
| Z-Address | 隐私地址（旧版 Sapling） |
| Unified Address | 统一地址，以 u1 开头，包含多种接收器 |
| Orchard | Zcash 最新隐私协议，使用 Halo 2 证明 |
| Halo 2 | 无需可信设置的零知识证明系统 |
| Note | 隐私币的 UTXO，包含金额和接收者信息 |
| Nullifier | 用于标记 Note 已花费，防止双花 |
| Shielding | 屏蔽化，T→Z 转账 |
| Deshielding | 去隐蔽化，Z→T 转账 |
| ZIP-317 | Zcash 手续费标准 |
| Zatoshi | ZEC 最小单位，1 ZEC = 10^8 zatoshi |

---

*文档版本：1.0*
*更新日期：2024-01-20*
