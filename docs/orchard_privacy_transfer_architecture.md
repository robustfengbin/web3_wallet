# Zcash Orchard 隐私转账架构文档

## 1. 概述

Orchard 是 Zcash 最新的隐私协议 (NU5 激活)，使用 Halo 2 零知识证明系统。本文档详细说明了隐私转账中 Note、Witness 的同步逻辑和核心数据流。

## 2. 核心数据结构

### 2.1 OrchardNote (scanner.rs:94-156)

```rust
pub struct OrchardNote {
    // === 基本信息 ===
    pub id: Option<i64>,           // 数据库ID
    pub wallet_id: Option<i32>,    // 钱包ID (支持多钱包)
    pub account_id: u32,           // 账户索引
    pub tx_hash: String,           // 交易哈希
    pub block_height: u64,         // 区块高度

    // === 承诺与虚空符 ===
    pub note_commitment: [u8; 32], // 笔记承诺 (cmx)
    pub nullifier: [u8; 32],       // 虚空符 (用于标记已花费)

    // === 价值信息 ===
    pub value_zatoshis: u64,       // 金额 (1 ZEC = 100,000,000 zatoshis)
    pub position: u64,             // 在全局树中的位置
    pub is_spent: bool,            // 是否已花费

    // === 花费必要数据 (shielded-to-shielded 转账) ===
    pub recipient: [u8; 43],       // Orchard地址 (43字节)
    pub rho: [u8; 32],             // 笔记随机性
    pub rseed: [u8; 32],           // 随机种子

    // === 见证数据 ===
    pub witness_data: Option<WitnessData>, // Merkle认证路径
    pub memo: Option<String>,      // 备注
}
```

### 2.2 WitnessData (tree.rs:311-320)

```rust
pub struct WitnessData {
    pub position: u64,              // 笔记在树中的位置
    pub auth_path: Vec<[u8; 32]>,   // 32个哈希值 (Merkle认证路径)
    pub root: [u8; 32],             // 树根 (锚点/anchor)
}
```

**关键点**: witness 数据包含从 note 位置到树根的完整认证路径，用于在转账时证明 note 确实存在于承诺树中。

### 2.3 OrchardTreeTracker (tree.rs:36-49)

```rust
pub struct OrchardTreeTracker {
    tree: CommitmentTree<MerkleHashOrchard, 32>,  // 承诺树 (深度32)
    witnesses: HashMap<u64, IncrementalWitness>,  // 见证缓存 (按位置索引)
    current_position: u64,                         // 当前位置
    last_block_height: u64,                        // 最后处理的区块高度
}
```

## 3. 同步流程

### 3.1 整体流程图

```
┌─────────────────────────────────────────────────────────────────────┐
│                        同步启动 (sync.rs:718)                        │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  1. 获取链顶高度 get_chain_height()                                  │
│  2. 检查是否需要从 frontier 初始化树                                  │
│     - 如果 tree_size < 100 且 witness_count == 0                    │
│     - 调用 initialize_tree_from_frontier()                          │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  3. 初始化树 (sync.rs:683-715)                                       │
│     - get_tree_state(height) → 获取 frontier_hex, root              │
│     - count_commitments_at_height() → 获取起始 position              │
│     - scanner.init_from_frontier() → 重建树状态                      │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  4. 批量获取区块 fetch_blocks_batch() (sync.rs:390-454)              │
│     - 批量 RPC: getblockhash → getblock (verbosity=2)               │
│     - 并行获取 (parallel_fetches 个并发)                             │
│     - 转换为 CompactBlock 格式                                       │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  5. 扫描区块 scanner.scan_blocks() (scanner.rs:377-529)              │
│                                                                     │
│     for block in blocks:                                            │
│       for tx in block.transactions:                                 │
│         for action in tx.orchard_actions:                           │
│           ┌──────────────────────────────────────────────────────┐  │
│           │ 尝试用 IVK 解密 (External/Internal scope)            │  │
│           │ try_decrypt_note() → 使用 orchard crate 解密         │  │
│           └───────────────────────┬──────────────────────────────┘  │
│                                   │                                 │
│                      ┌────────────┴────────────┐                    │
│                      ▼                         ▼                    │
│              [是我们的 note]            [不是我们的 note]           │
│                      │                         │                    │
│                      ▼                         ▼                    │
│           tree.append_and_mark()    tree.append_commitment()        │
│           (标记见证追踪)               (仅追踪承诺)                  │
│                      │                         │                    │
│                      ▼                         │                    │
│           获取 witness_data                    │                    │
│           存储 note 到内存/DB                  │                    │
│                      │                         │                    │
│                      └─────────┬───────────────┘                    │
│                                ▼                                    │
│                    检查 nullifier 是否匹配已有 note                  │
│                    (检测已花费的 notes)                              │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│  6. 持久化 (sync.rs:989-1044, 1828-1896)                            │
│     - store_notes() → 保存 note 到数据库                            │
│     - mark_notes_spent() → 更新已花费 notes                          │
│     - persist_witnesses() → 保存 witness 数据                        │
│     - persist_scan_state() → 保存扫描状态                            │
└─────────────────────────────────────────────────────────────────────┘
```

### 3.2 关键函数详解

#### 3.2.1 树初始化 (从 frontier)

```rust
// sync.rs:683-715
async fn initialize_tree_from_frontier(&self, note_height: u64) -> OrchardResult<()> {
    // 1. 获取 note 之前一个区块的树状态
    let frontier_height = note_height.saturating_sub(1);

    // 2. 通过 z_gettreestate RPC 获取 frontier
    let (frontier_hex, frontier_root, _) = self.get_tree_state(frontier_height).await?;

    // 3. 计算起始位置 (树中已有的承诺数量)
    let start_position = self.count_commitments_at_height(frontier_height).await?;

    // 4. 初始化 scanner 的树
    scanner.init_from_frontier(&frontier_hex, start_position, frontier_height)?;
}
```

#### 3.2.2 Note 解密 (scanner.rs:546-663)

```rust
fn try_decrypt_note(
    &self,
    viewing_key: &OrchardViewingKey,
    action: &CompactOrchardAction,
    tx_hash: &str,
    block_height: u64,
    position: u64,
) -> Option<OrchardNote> {
    // 1. 解析 nullifier 和 cmx
    let nullifier = Nullifier::from_bytes(&action.nullifier)?;
    let cmx = ExtractedNoteCommitment::from_bytes(&action.cmx)?;

    // 2. 获取 IVK 并尝试两个 scope (External 和 Internal)
    let fvk = viewing_key.fvk();

    for scope in [Scope::External, Scope::Internal] {
        let ivk = fvk.to_ivk(scope);
        let prepared_ivk = PreparedIncomingViewingKey::new(&ivk);

        // 3. 使用 batch 解密 API
        let results = batch::try_compact_note_decryption(
            &[prepared_ivk],
            &[(domain, compact_action)],
        );

        if let Some(Some(((note, recipient), _))) = results.into_iter().next() {
            // 4. 解密成功！提取花费所需数据
            let recipient_bytes = recipient.to_raw_address_bytes();  // 43 bytes
            let rho_bytes = note.rho().to_bytes();                   // 32 bytes
            let rseed_bytes = *note.rseed().as_bytes();              // 32 bytes

            return Some(OrchardNote {
                recipient: recipient_bytes,
                rho: rho_bytes,
                rseed: rseed_bytes,
                // ... 其他字段
            });
        }
    }

    None
}
```

#### 3.2.3 承诺树更新 (tree.rs:130-183)

```rust
// 添加承诺 (不追踪见证)
pub fn append_commitment(&mut self, cmx: &[u8; 32]) -> Result<u64, TreeError> {
    let hash = parse_merkle_hash(cmx)?;

    // 1. 添加到树
    self.tree.append(hash.clone())?;

    let position = self.current_position;
    self.current_position += 1;

    // 2. 更新所有现有见证 (关键！)
    for witness in self.witnesses.values_mut() {
        witness.append(hash.clone())?;
    }

    Ok(position)
}

// 添加承诺并标记见证追踪 (用于我们的 note)
pub fn append_and_mark(&mut self, cmx: &[u8; 32]) -> Result<u64, TreeError> {
    let hash = parse_merkle_hash(cmx)?;

    // 1. 添加到树
    self.tree.append(hash.clone())?;

    let position = self.current_position;
    self.current_position += 1;

    // 2. 更新现有见证
    for witness in self.witnesses.values_mut() {
        witness.append(hash.clone())?;
    }

    // 3. 为此位置创建新见证
    let witness = IncrementalWitness::from_tree(self.tree.clone())?;
    self.witnesses.insert(position, witness);

    Ok(position)
}
```

## 4. 转账流程

### 4.1 整体流程图

```
┌─────────────────────────────────────────────────────────────────────┐
│                    创建转账提案 create_proposal()                    │
│     - 验证金额和余额                                                 │
│     - 确定资金来源 (Shielded/Transparent/Auto)                       │
│     - 计算费用 (ZIP-317)                                            │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                 刷新见证 refresh_witnesses_for_spending()            │
│     (sync.rs:1441-1797)                                             │
│                                                                     │
│     1. 检查 scanner 状态                                             │
│        - tree_has_witnesses? witness_count > 0 或 tree_size > 1000  │
│                                                                     │
│     2. 决定扫描策略:                                                 │
│        ┌──────────────────────────────────────────────────────────┐ │
│        │ 如果 scanner_height >= note_height && tree_has_witnesses │ │
│        │ → 增量扫描 (从 scanner_height 到 chain_tip)              │ │
│        └──────────────────────────────────────────────────────────┘ │
│        ┌──────────────────────────────────────────────────────────┐ │
│        │ 否则 (服务重启后树状态丢失)                               │ │
│        │ → 完整重扫 (从 min_note_height 开始，初始化 frontier)     │ │
│        └──────────────────────────────────────────────────────────┘ │
│                                                                     │
│     3. 扫描并更新见证                                                │
│     4. 验证树根匹配预期锚点                                          │
│     5. 持久化见证到数据库                                            │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│            加载可花费 notes get_spendable_notes_with_witnesses()     │
│     (sync.rs:1218-1427)                                             │
│                                                                     │
│     1. 优先从内存获取 (有 witness 的 notes)                          │
│     2. 如果内存没有，从数据库加载:                                   │
│        - 解析 recipient, rho, rseed                                 │
│        - 解析 witness_data (position, auth_path, root)              │
│        - 重建 OrchardNote 对象                                      │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│              构建交易 build_transaction() (transfer.rs:283-371)      │
│                                                                     │
│     根据 fund_source 分发:                                          │
│     ├── Shielded → build_shielded_bundle()                          │
│     ├── Transparent → build_shielding_bundle()                      │
│     └── Auto → 优先 Shielded，fallback Transparent                  │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
        ┌───────────────────────┴───────────────────────┐
        ▼                                               ▼
┌───────────────────────────┐               ┌───────────────────────────┐
│ Shielded-to-Shielded     │               │ Transparent-to-Shielded   │
│ build_shielded_bundle()   │               │ build_shielding_bundle()  │
│ (transfer.rs:567-874)     │               │ (transfer.rs:929-1033)    │
└───────────┬───────────────┘               └───────────┬───────────────┘
            │                                           │
            ▼                                           ▼
┌─────────────────────────────────────────────────────────────────────┐
│                       Shielded 转账详细流程                          │
│                                                                     │
│  1. select_notes() → 选择足够金额的 notes                            │
│                                                                     │
│  2. 验证每个 note 的 witness:                                        │
│     - witness_data 必须存在                                          │
│     - witness.root 必须 == anchor_bytes (关键！)                     │
│                                                                     │
│  3. 重建 orchard::Note 对象:                                         │
│     recipient → orchard::Address                                    │
│     rho → orchard::note::Rho                                        │
│     rseed → orchard::note::RandomSeed                               │
│     → orchard::Note::from_parts()                                   │
│                                                                     │
│  4. 验证重建的 note commitment 匹配存储的 cmx                         │
│                                                                     │
│  5. 转换 witness_data → orchard::tree::MerklePath                   │
│                                                                     │
│  6. 构建 Orchard bundle:                                             │
│     - builder.add_spend(fvk, note, merkle_path)                     │
│     - builder.add_output(ovk, recipient, value, memo)  // 支付       │
│     - builder.add_output(ovk, change_address, change)  // 找零       │
│                                                                     │
│  7. 创建证明: unauthorized_bundle.create_proof(pk, rng)              │
│     (使用 Halo 2 零知识证明，耗时几秒)                                │
│                                                                     │
│  8. 应用签名: proven_bundle.apply_signatures(rng, sighash, saks)     │
│                                                                     │
│  9. 序列化交易                                                       │
└─────────────────────────────────────────────────────────────────────┘
```

### 4.2 关键函数详解

#### 4.2.1 Note 重建 (transfer.rs:667-783)

```rust
// 从存储的数据重建 orchard::Note
for (idx, note) in selected_notes.iter().enumerate() {
    // 1. 重建地址
    let recipient_addr = orchard::Address::from_raw_address_bytes(&note.recipient)?;

    // 2. 重建 Rho
    let rho = orchard::note::Rho::from_bytes(&note.rho)?;

    // 3. 重建 RandomSeed (需要 rho 作为参数)
    let rseed = orchard::note::RandomSeed::from_bytes(note.rseed, &rho)?;

    // 4. 重建完整 Note
    let value = NoteValue::from_raw(note.value_zatoshis);
    let orchard_note = orchard::Note::from_parts(recipient_addr, value, rho, rseed)?;

    // 5. 验证承诺匹配 (关键！)
    let extracted_cmx = ExtractedNoteCommitment::from(orchard_note.commitment());
    let reconstructed_cmx = extracted_cmx.to_bytes();

    if reconstructed_cmx != note.note_commitment {
        return Err("Note commitment mismatch");
    }

    // 6. 转换 witness 为 MerklePath
    let merkle_path = note.witness_data.to_merkle_path()?;

    // 7. 添加 spend
    builder.add_spend(fvk.clone(), orchard_note, merkle_path)?;
}
```

#### 4.2.2 Witness 验证 (transfer.rs:599-630)

```rust
// 验证 witness 根必须匹配锚点
for (idx, note) in selected_notes.iter().enumerate() {
    if note.witness_data.is_none() {
        return Err(OrchardError::WitnessNotFound);
    }

    if let Some(ref witness) = note.witness_data {
        if witness.root != anchor_bytes {
            // 见证的树根与当前锚点不匹配
            // 这意味着树在扫描后又更新了，需要重新同步
            return Err(OrchardError::TransactionBuild(
                "Witness anchor mismatch - please resync"
            ));
        }
    }
}
```

## 5. 重要约束和安全性

| 约束 | 描述 | 代码位置 |
|------|------|----------|
| **Witness 根匹配** | `note.witness_data.root == anchor` 必须相等 | transfer.rs:611 |
| **最小确认数** | `current_height >= note.block_height + 10` | scanner.rs:329 |
| **锚点年龄** | 见证最多 50 个块之前过期 | sync.rs:1431 |
| **Note 数据完整性** | recipient/rho/rseed 必须准确存储 | scanner.rs:640-657 |
| **Note 承诺验证** | 重建的 commitment 必须与扫描时一致 | transfer.rs:711-728 |
| **零值保护** | 不允许创建 value=0 的 Orchard actions | transfer.rs:1194-1203 |

## 6. 性能优化

1. **并行 RPC 获取**: 批量请求最多 25 个块同时 (sync.rs:52)
2. **见证缓存**: 内存中保留最新见证，避免重复计算 (tree.rs:42)
3. **增量扫描**: 支持从上次高度继续，无需重新扫描 (sync.rs:1498-1507)
4. **树状态持久化**: witness 保存到 DB，服务重启恢复 (sync.rs:1828-1896)
5. **Proving Key 缓存**: 全局 static OnceLock 缓存 (transfer.rs:50)

## 7. 数据库表设计

### orchard_notes 表

```sql
CREATE TABLE orchard_notes (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    wallet_id INT NOT NULL,
    nullifier VARCHAR(64) NOT NULL UNIQUE,
    value_zatoshis BIGINT NOT NULL,
    block_height BIGINT NOT NULL,
    tx_hash VARCHAR(64) NOT NULL,
    position_in_block INT NOT NULL,
    is_spent BOOLEAN DEFAULT FALSE,
    memo TEXT,

    -- 花费所需数据
    recipient VARCHAR(86),        -- 43 bytes hex
    rho VARCHAR(64),              -- 32 bytes hex
    rseed VARCHAR(64),            -- 32 bytes hex

    -- 见证数据
    witness_position BIGINT,
    witness_auth_path TEXT,       -- JSON array of 32 hex strings
    witness_root VARCHAR(64),     -- 32 bytes hex

    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);
```

## 8. 常见问题排查

### 8.1 "Unknown anchor" 错误

**原因**: witness 的树根与节点期望的锚点不匹配

**排查步骤**:
1. 检查 `scanner.tree_tracker().root()` 是否与 `get_expected_anchor(height)` 匹配
2. 调用 `refresh_witnesses_for_spending()` 刷新见证
3. 确保扫描高度达到最新

### 8.2 "No spendable notes" 错误

**原因**: 没有满足条件的可花费 notes

**排查步骤**:
1. 检查 `get_unspent_notes()` 返回的 notes
2. 验证 notes 有足够的确认数 (>= 10)
3. 确保 notes 没有被标记为已花费

### 8.3 服务重启后无法转账

**原因**: 内存中的树状态丢失

**解决方案**:
1. `refresh_witnesses_for_spending()` 会自动检测并重建树
2. 从 frontier 初始化，然后重新扫描
