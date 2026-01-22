# Orchard Witness 增量同步方案

## 一、背景

### 1.1 问题描述

Zcash Orchard 隐私转账需要提供 Witness（Merkle 证明路径）来证明 note 存在于 Commitment Tree 中。

**核心挑战**：
- Witness 需要随着每个新 commitment 的添加而更新
- 如果每次都从 note 诞生时重新扫描，随着时间推移会越来越慢
- 需要实现真正的增量同步

### 1.2 关键概念

| 概念 | 说明 |
|------|------|
| **Commitment Tree** | 全局 Merkle 树，所有 note 的 commitment (cmx) 按顺序添加 |
| **Position** | note 在树中的全局位置（从 0 开始递增） |
| **Witness** | 从 note 位置到树根的证明路径（auth_path + root） |
| **Frontier** | 树在某个高度的压缩状态，可从 RPC 获取 |
| **IncrementalWitness** | 可增量更新的 witness 跟踪状态 |

### 1.3 树深度与存储

- Orchard 树深度固定为 **32 层**
- 最多支持 2³² ≈ 43 亿个叶子
- **关键**：无论树有多大，witness 大小固定（32 × 32 = 1024 字节 + 元数据 ≈ 1-2 KB）

---

## 二、方案设计

### 2.1 核心思想

**保存 witness 的跟踪状态，而不仅仅是结果**

| | 旧方案（保存结果） | 新方案（保存状态） |
|--|------------------|------------------|
| 保存内容 | auth_path, root（静态快照） | IncrementalWitness（可更新状态） |
| 新区块来了 | 结果作废，需重新计算 | 追加更新，O(32) 操作 |
| 每次 sync | 从 min_note_height 开始 | 从上次高度继续 |
| 扫描范围 | 可能几十万区块 | 只有新区块 |

### 2.2 数据存储

```
┌─────────────────────────────────────────────────────────────┐
│  orchard_tree_state 表（全局，只有 1 行）                     │
├─────────────────────────────────────────────────────────────┤
│  - tree_data: 序列化的 CommitmentTree                       │
│  - tree_height: 树对应的区块高度                             │
│  - tree_size: 树中 commitment 数量                          │
│                                                             │
│  作用：知道"整棵树长什么样"，可以追加新 commitments           │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  orchard_notes 表（每个 note 一行）                          │
├─────────────────────────────────────────────────────────────┤
│  - witness_state: 序列化的 IncrementalWitness               │
│  - witness_position: note 在树中的位置                       │
│  - ... 其他字段                                             │
│                                                             │
│  作用：知道"我的 note 在树里的证明路径"                       │
└─────────────────────────────────────────────────────────────┘
```

---

## 三、详细流程

### 3.1 定时 Sync（每 5 分钟）

```
┌─────────────────────────────────────────────────────────────────┐
│  Step 1: 加载状态                                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  从 DB 加载 tree_state（orchard_tree_state 表）                  │
│  从 DB 加载所有未花费 notes 的 witness_state                      │
│                                                                 │
│  如果 tree_state 不存在（首次启动）：                             │
│    - 找最早 note 的 block_height                                 │
│    - 如果没有 notes，用 birthday_height                          │
│    - 调用 RPC: z_gettreestate(height - 1) 获取 frontier         │
│    - 用 frontier 初始化 tree                                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│  Step 2: 扫描新区块 (tree_height + 1 → chain_tip)               │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  for block in range(tree_height + 1, chain_tip):                │
│      for action in block.orchard_actions:                       │
│          cmx = action.cmx                                       │
│          current_pos = tree.position()                          │
│                                                                 │
│          # 1. 更新所有已有 witnesses                             │
│          for witness in witnesses.values():                     │
│              witness.append(cmx)                                │
│                                                                 │
│          # 2. 尝试解密，发现新 note                              │
│          if try_decrypt(action) → new_note:                     │
│              tree.append_and_mark(cmx)                          │
│              new_witness = IncrementalWitness.from_tree(tree)   │
│              save_note_to_db(new_note, current_pos)             │
│              witnesses[current_pos] = new_witness               │
│          else:                                                  │
│              tree.append(cmx)                                   │
│                                                                 │
│          # 3. 检查 nullifier，标记已花费                         │
│          check_spent(action.nullifier)                          │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│  Step 3: 保存状态                                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  # 保存全局 tree 状态                                            │
│  tree_data = serialize(tree)                                    │
│  UPDATE orchard_tree_state SET tree_data=?, tree_height=?       │
│                                                                 │
│  # 保存每个 note 的 witness 状态                                 │
│  for (nullifier, witness) in witnesses:                         │
│      witness_data = serialize(witness)                          │
│      UPDATE orchard_notes SET witness_state=? WHERE nullifier=? │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 3.2 首次发现 Note

```
场景：在区块 3000500 发现了一个 note

Step 1: 获取 frontier（如果 tree 还没初始化）

        调用 RPC: z_gettreestate(3000499)  ← note 所在区块的前一块

        返回：
          frontier_hex = "01a2b3c4..."   ← 树在 3000499 时的边缘状态
          tree_size = 45000              ← 此时树有 45000 个 commitments

        这个 frontier 已经"浓缩"了从创世到 3000499 的所有历史！

Step 2: 初始化本地 tree

        tree = Tree.from_frontier(frontier_hex)
        不需要扫描之前的几百万个区块！

Step 3: 扫描区块 3000500

        遇到 commitment_45001 (cmx_1)  ← 这是我们的 note！

        tree.append_and_mark(cmx_1)
        witness = IncrementalWitness.from_tree(tree)  ← 从当前树状态创建

Step 4: 保存 note 和 witness_state 到数据库
```

### 3.3 转账时获取 Witness

```
┌─────────────────────────────────────────────────────────────────┐
│  get_witness_for_spending(note_id)                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. 从 DB 加载 note 的 witness_state                            │
│  2. 从 DB 加载 tree_height                                      │
│  3. 获取 chain_tip                                              │
│                                                                 │
│  if tree_height < chain_tip:                                    │
│      # 需要追加最近的 commitments（通常很少）                     │
│      从 DB 加载 tree_state                                      │
│      扫描 tree_height+1 → chain_tip                             │
│      for cmx in new_commitments:                                │
│          tree.append(cmx)                                       │
│          witness.append(cmx)                                    │
│                                                                 │
│  4. 提取结果                                                    │
│     auth_path = witness.path()                                  │
│     root = witness.root()                                       │
│     return WitnessData { position, auth_path, root }            │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 四、数据库表结构

### 4.1 新增表：orchard_tree_state

```sql
CREATE TABLE IF NOT EXISTS orchard_tree_state (
    id INT PRIMARY KEY DEFAULT 1,
    tree_data MEDIUMBLOB NOT NULL COMMENT '序列化的 CommitmentTree',
    tree_height BIGINT UNSIGNED NOT NULL COMMENT '树对应的区块高度',
    tree_size BIGINT UNSIGNED NOT NULL COMMENT '树中 commitment 数量',
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);
```

### 4.2 修改表：orchard_notes

```sql
ALTER TABLE orchard_notes
ADD COLUMN witness_state MEDIUMBLOB NULL
COMMENT '序列化的 IncrementalWitness，用于增量更新';
```

### 4.3 现有字段保留

`orchard_notes` 表现有的 witness 相关字段保留，用于快速查询：
- `witness_position` - note 在树中的位置
- `witness_auth_path` - 最新的 auth_path（JSON）
- `witness_root` - 最新的 root

---

## 五、代码结构

### 5.1 文件改动

```
backend/src/
├── db/
│   ├── mod.rs                    # [修改] 添加新表创建
│   └── repositories/
│       └── orchard_repo.rs       # [修改] 添加新的 DB 操作方法
│
└── blockchain/zcash/orchard/
    ├── mod.rs                    # [修改] 导出新模块
    ├── tree.rs                   # [修改] 添加序列化/反序列化方法
    ├── sync.rs                   # [修改] 重构使用新的同步逻辑
    └── scanner.rs                # [修改] 支持外部传入 witnesses
```

### 5.2 主要方法

**tree.rs 新增：**
- `serialize_tree(tree) -> Vec<u8>`
- `deserialize_tree(data) -> CommitmentTree`
- `serialize_witness(witness) -> Vec<u8>`
- `deserialize_witness(data) -> IncrementalWitness`

**orchard_repo.rs 新增：**
- `save_tree_state(tree_data, height, size)`
- `load_tree_state() -> Option<TreeState>`
- `save_witness_state(nullifier, witness_data)`
- `load_witness_states(wallet_id) -> HashMap<String, Vec<u8>>`

**sync.rs 重构：**
- `sync()` - 加载状态 → 扫描 → 保存状态
- `get_witness_for_spending(note_id)` - 获取用于转账的 witness

---

## 六、性能分析

### 6.1 时间复杂度

| 操作 | 旧方案 | 新方案 |
|------|--------|--------|
| 定时 sync | O(note_age × blocks) | O(new_blocks) |
| 转账获取 witness | O(note_age × blocks) | O(since_last_sync) |

### 6.2 存储空间

| 数据 | 大小 |
|------|------|
| tree_state | 几 KB（固定） |
| 每个 witness_state | 1-2 KB（固定） |
| 100 个 notes | ~200 KB |

### 6.3 示例场景

```
场景：note 已存在 30 天，sync 每 5 分钟运行

旧方案：
  - 每次 sync 扫描 30 天的区块（约 4000 个）
  - 转账时也要等待完整扫描

新方案：
  - 每次 sync 只扫描 5 分钟的新区块（约 2-3 个）
  - 转账时最多追加 5 分钟的数据
```

---

## 七、实现顺序

1. **数据库改动** - `db/mod.rs`
2. **Repository 方法** - `orchard_repo.rs`
3. **序列化方法** - `tree.rs`
4. **重构 sync** - `sync.rs`
5. **测试验证**

---

## 八、注意事项

1. **首次启动**：如果没有 tree_state，需要从 frontier 初始化
2. **状态丢失**：如果 DB 数据损坏，可以从 frontier 重建
3. **并发安全**：sync 过程中加锁，避免同时读写
4. **错误恢复**：保存状态失败时回滚，下次重试
