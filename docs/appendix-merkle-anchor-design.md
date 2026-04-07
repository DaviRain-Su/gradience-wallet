# 附录：Merkle 审计日志链上锚定设计

> Status: Draft — Hackathon 专项设计
> Date: 2026-04-07
> Target: HashKey Chain Horizon Hackathon (PayFi / AI 赛道)
> Chain: HashKey Chain (EVM 兼容, OP Stack)

---

## 1. 设计目标

将 Gradience Wallet 的审计日志通过 **Merkle Tree** 定期锚定到 **HashKey Chain** 上，实现：

1. **不可篡改证明** —— 任何对审计日志的修改都会导致 Merkle root 不匹配
2. **时间戳证明** —— 链上交易时间戳证明某时刻日志已存在
3. **合规友好** —— 审计员可独立验证日志完整性，无需信任 Gradience 服务
4. **Hackathon 加分** —— 在 HashKey Chain 上部署合约，满足参赛要求

---

## 2. Merkle Tree 构建

### 2.1 日志批处理策略

```
audit_logs 表 (SQLite/PostgreSQL)
    │
    ├── 每 N 条日志 (默认 1,000) = 1 个批次
    │
    ├── 每个批次内:
    │   Leaf_i = keccak256(
    │       id || wallet_id || action || decision || tx_hash || created_at
    │   )
    │
    ├── Merkle Tree 构建:
    │                       Root
    │                    /        \
    │                 H(A,B)      H(C,D)
    │                /      \    /      \
    │              Leaf_A  Leaf_B Leaf_C Leaf_D
    │
    └── Root 提交到 HashKey Chain
```

### 2.2 Leaf 哈希计算

```rust
use sha3::{Keccak256, Digest};

fn compute_audit_leaf(log: &AuditLogEntry) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(log.id.to_le_bytes());
    hasher.update(log.wallet_id.as_bytes());
    hasher.update(log.action.as_bytes());
    hasher.update(log.decision.as_bytes());
    if let Some(tx_hash) = &log.tx_hash {
        hasher.update(tx_hash.as_bytes());
    }
    hasher.update(log.created_at.to_le_bytes());
    hasher.finalize().into()
}
```

### 2.3 增量锚定

```
批次 1: [log_1 .. log_1000] → Root_1 → 提交到 HashKey
批次 2: [log_1001 .. log_2000] → Root_2 → 提交到 HashKey (prev = Root_1)
批次 3: [log_2001 .. log_3000] → Root_3 → 提交到 HashKey (prev = Root_2)

链式连接: Root_n = f(data_n, Root_{n-1})
→ 任何历史批次被改 → 后续所有 root 失效
```

---

## 3. HashKey Chain 智能合约

### 3.1 合约接口

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/**
 * @title AuditAnchor — Gradience Wallet 审计日志 Merkle 根锚定合约
 * @notice 部署在 HashKey Chain 上，用于审计日志不可篡改证明
 */
contract AuditAnchor {
    struct Anchor {
        bytes32 root;          // Merkle root
        bytes32 prevRoot;      // 前一个 root (链式连接)
        uint256 logStartIndex; // 起始日志 ID
        uint256 logEndIndex;   // 结束日志 ID
        uint256 leafCount;     // 叶子节点数
        uint256 timestamp;     // 锚定时间戳
        address submittedBy;   // 提交者
    }

    // root 哈希 → 锚定记录
    mapping(bytes32 => Anchor) public anchors;

    // root 是否已锚定
    mapping(bytes32 => bool) public isAnchored;

    // 最新锚定的 root
    bytes32 public latestRoot;

    // 事件
    event Anchored(
        bytes32 indexed root,
        bytes32 indexed prevRoot,
        uint256 logStartIndex,
        uint256 logEndIndex,
        uint256 leafCount,
        uint256 timestamp,
        address indexed submittedBy
    );

    /**
     * @notice 锚定新的 Merkle root
     * @param root Merkle 树根
     * @param prevRoot 前一个 root (第一个批次为 0x0)
     * @param logStartIndex 起始日志序号
     * @param logEndIndex 结束日志序号
     * @param leafCount 叶子节点数
     */
    function anchor(
        bytes32 root,
        bytes32 prevRoot,
        uint256 logStartIndex,
        uint256 logEndIndex,
        uint256 leafCount
    ) external {
        require(!isAnchored[root], "Root already anchored");
        require(root != bytes32(0), "Invalid root");

        anchors[root] = Anchor({
            root: root,
            prevRoot: prevRoot,
            logStartIndex: logStartIndex,
            logEndIndex: logEndIndex,
            leafCount: leafCount,
            timestamp: block.timestamp,
            submittedBy: msg.sender
        });

        isAnchored[root] = true;
        latestRoot = root;

        emit Anchored(root, prevRoot, logStartIndex, logEndIndex, leafCount, block.timestamp, msg.sender);
    }

    /**
     * @notice 验证 Merkle proof
     * @param root Merkle 根
     * @param leaf 叶子节点
     * @param proof Merkle 证明路径
     * @return 是否有效
     */
    function verifyProof(
        bytes32 root,
        bytes32 leaf,
        bytes32[] calldata proof
    ) external pure returns (bool) {
        bytes32 computedHash = leaf;
        for (uint256 i = 0; i < proof.length; i++) {
            bytes32 proofElement = proof[i];
            if (computedHash <= proofElement) {
                computedHash = keccak256(abi.encodePacked(computedHash, proofElement));
            } else {
                computedHash = keccak256(abi.encodePacked(proofElement, computedHash));
            }
        }
        return computedHash == root && isAnchored[root];
    }

    /**
     * @notice 获取最新锚定信息
     */
    function getLatestAnchor() external view returns (Anchor memory) {
        return anchors[latestRoot];
    }

    /**
     * @notice 验证锚定链完整性
     * @param root 要验证的 root
     * @return 链式连接是否有效
     */
    function verifyChainIntegrity(bytes32 root) external view returns (bool) {
        if (!isAnchored[root]) return false;

        bytes32 current = root;
        while (current != bytes32(0)) {
            Anchor memory anchor = anchors[current];
            if (anchor.prevRoot != bytes32(0) && !isAnchored[anchor.prevRoot]) {
                return false; // 链断裂
            }
            current = anchor.prevRoot;
        }
        return true;
    }
}
```

### 3.2 合约 Gas 估算

| 操作 | Gas (估算) | HashKey Chain 费用 (HSK) |
|---|---|---|
| anchor() | ~60,000 | < $0.01 |
| verifyProof() (view) | 0 (call) | 0 |
| verifyChainIntegrity() (view) | 0 (call) | 0 |

HashKey Chain 基于 OP Stack，Gas 成本极低，锚定操作成本可忽略。

---

## 4. Rust 实现 (`audit/anchor.rs`)

### 4.1 Merkle Tree 实现

```rust
use sha3::{Keccak256, Digest};

#[derive(Debug, Clone)]
pub struct MerkleTree {
    pub leaves: Vec<[u8; 32]>,
    pub layers: Vec<Vec<[u8; 32]>>,
    pub root: [u8; 32],
}

impl MerkleTree {
    pub fn new(leaves: Vec<[u8; 32]>) -> Self {
        let mut layers: Vec<Vec<[u8; 32]>> = Vec::new();
        layers.push(leaves.clone());

        let mut current_layer = leaves;
        while current_layer.len() > 1 {
            let mut next_layer = Vec::new();
            let chunks = current_layer.chunks(2);
            for chunk in chunks {
                if chunk.len() == 2 {
                    let hash = Self::hash_pair(chunk[0], chunk[1]);
                    next_layer.push(hash);
                } else {
                    // 奇数节点，复制最后一个
                    next_layer.push(chunk[0]);
                }
            }
            layers.push(next_layer.clone());
            current_layer = next_layer;
        }

        let root = current_layer.first().copied().unwrap_or([0u8; 32]);

        Self {
            leaves,
            layers,
            root,
        }
    }

    fn hash_pair(left: [u8; 32], right: [u8; 32]) -> [u8; 32] {
        let mut hasher = Keccak256::new();
        if left <= right {
            hasher.update(left);
            hasher.update(right);
        } else {
            hasher.update(right);
            hasher.update(left);
        }
        hasher.finalize().into()
    }

    /// 生成 Merkle proof (证明某个 leaf 在 tree 中)
    pub fn generate_proof(&self, leaf_index: usize) -> Option<(Vec<[u8; 32]>, [u8; 32])> {
        if leaf_index >= self.leaves.len() {
            return None;
        }

        let mut proof = Vec::new();
        let mut index = leaf_index;

        for layer in &self.layers {
            if layer.len() <= 1 {
                break;
            }

            let sibling_index = if index % 2 == 0 { index + 1 } else { index - 1 };
            if sibling_index < layer.len() {
                proof.push(layer[sibling_index]);
            }

            index /= 2;
        }

        Some((proof, self.leaves[leaf_index]))
    }
}
```

### 4.2 锚定服务

```rust
use alloy::primitives::{Address, Bytes, B256, U256};
use alloy::providers::{Provider, ProviderBuilder};
use alloy::sol;

// 自动生成合约绑定 (通过 alloy sol! macro)
sol! {
    #[allow(missing_docs)]
    #[sol(rpc)]
    contract AuditAnchor {
        function anchor(
            bytes32 root,
            bytes32 prevRoot,
            uint256 logStartIndex,
            uint256 logEndIndex,
            uint256 leafCount
        ) external;
    }
}

pub struct AnchorService {
    rpc_url: String,
    contract_address: Address,
    signer: LocalSigner<SigningKey>, // 提交用私钥
    batch_size: usize,
    last_anchored_root: B256,
}

impl AnchorService {
    pub async fn submit_anchor(
        &mut self,
        root: [u8; 32],
        log_start: u64,
        log_end: u64,
        leaf_count: usize,
    ) -> Result<AnchorReceipt> {
        let provider = ProviderBuilder::new()
            .on_builtin(&self.rpc_url)
            .await?;

        let contract = AuditAnchor::new(self.contract_address, provider);

        let tx = contract
            .anchor(
                B256::from_slice(&root),
                self.last_anchored_root,
                U256::from(log_start),
                U256::from(log_end),
                U256::from(leaf_count as u64),
            )
            .send()
            .await?
            .get_receipt()
            .await?;

        self.last_anchored_root = B256::from_slice(&root);

        Ok(AnchorReceipt {
            tx_hash: tx.transaction_hash,
            block_number: tx.block_number.unwrap_or(0),
            root,
        })
    }
}
```

### 4.3 周期性锚定调度

```rust
// 后台定时任务 (tokio)
pub async fn run_anchor_scheduler(
    mut service: AnchorService,
    db: &Database,
    anchor_interval: Duration, // 默认 1 小时
) -> Result<()> {
    let mut interval = tokio::time::interval(anchor_interval);

    loop {
        interval.tick().await;

        // 获取未锚定的日志批次
        let batch = db.get_unanchored_logs(service.batch_size).await?;
        if batch.is_empty() {
            continue;
        }

        // 构建 Merkle tree
        let leaves: Vec<[u8; 32]> = batch.iter()
            .map(compute_audit_leaf)
            .collect();

        let tree = MerkleTree::new(leaves);

        // 提交到 HashKey Chain
        let receipt = service.submit_anchor(
            tree.root,
            batch.first().unwrap().id,
            batch.last().unwrap().id,
            batch.len(),
        ).await?;

        // 记录锚定结果
        for log in &batch {
            db.mark_anchored(log.id, &receipt.tx_hash).await?;
        }

        tracing::info!(
            root = ?hex::encode(tree.root),
            tx_hash = ?hex::encode(receipt.tx_hash),
            log_count = batch.len(),
            "Audit log batch anchored to HashKey Chain"
        );
    }
}
```

---

## 5. 验证流程

### 5.1 审计员验证日志完整性

```
审计员收到一条审计日志:
  {id: 42, action: "sign_tx", decision: "allowed", tx_hash: "0xabc...", created_at: ...}

验证步骤:
  1. 计算 leaf = keccak256(42 + wallet_id + "sign_tx" + "allowed" + "0xabc..." + timestamp)
  2. 从 Gradience API 请求 Merkle proof (叶子 42 在当前批次 tree 中的路径)
  3. 在本地验证: verifyProof(root, leaf, proof) == true
  4. 确认 root 已锚定: isAnchored(root) == true (链上查询)
  5. 确认根在 HashKey Chain 上有时间戳 → 日志存在性得证
```

### 5.2 CLI 验证命令

```bash
# 验证某条日志是否被锚定
gradience audit verify --log-id 42

# 输出:
# Leaf 计算: 0x7f8a... (匹配)
# Merkle Proof: 12 层, 验证通过 ✓
# Root 锚定: 0x3b2c... (HashKey Chain block 1234567)
# 锚定时间: 2026-04-07T12:00:00Z
# 链式完整性: 从 Root_1 到 Root_45 全部链接 ✓
```

---

## 6. 架构集成

### 6.1 模块依赖

```
gradience-core/audit/
├── mod.rs
├── logger.rs         # 日志写入
├── exporter.rs       # CSV/JSON 导出
└── anchor/           # ← 新增
    ├── mod.rs
    ├── merkle.rs     # Merkle tree 实现
    ├── service.rs    # 锚定调度 + HashKey Chain 提交
    └── verifier.rs   # 链上验证 (cli/web 可调用)
```

### 6.2 数据库扩展

```sql
-- audit_logs 表新增锚定字段
ALTER TABLE audit_logs ADD COLUMN anchor_tx_hash TEXT;      -- 锚定交易哈希
ALTER TABLE audit_logs ADD COLUMN anchor_root TEXT;          -- Merkle root
ALTER TABLE audit_logs ADD COLUMN anchor_leaf_index INTEGER; -- 在批次中的索引

-- 锚定批次记录
CREATE TABLE anchor_batches (
    id              BIGSERIAL PRIMARY KEY,
    root            TEXT NOT NULL,
    prev_root       TEXT,
    log_start_index INTEGER NOT NULL,
    log_end_index   INTEGER NOT NULL,
    leaf_count      INTEGER NOT NULL,
    tx_hash         TEXT NOT NULL,
    block_number    BIGINT,
    anchored_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

---

## 7. 配置

```toml
# config.toml
[merkle_anchor]
enabled = false                          # v2.0 功能，默认关闭
hashkey_rpc = "https://mainnet.hsk.xyz"  # HashKey Chain RPC
contract_address = "0x..."               # AuditAnchor 合约地址
batch_size = 1000                        # 每批次日志数量
anchor_interval_secs = 3600              # 锚定间隔 (1小时)
signer_key_ref = "env:HSK_PRIVATE_KEY"   # 提交用私钥 (从环境变量读取)
```

---

## 8. Hackathon Demo 路径

```
Demo 场景: "Agent 支付审计不可篡改证明"

1. Agent 通过 MCP 发起支付 (x402 或 HSP)
2. 策略引擎评估 (allow/deny/warn)
3. 审计日志记录 (包含 intent + decision)
4. 后台锚定服务批量提交 Merkle root 到 HashKey Chain
5. 在 Web Dashboard 展示:
   - 审计日志时间线
   - 每条日志的 Merkle 锚定状态 (已锚定 ✓)
   - 点击验证 → 展示完整 Merkle proof 验证过程
   - 链接到 HashKey Explorer 查看锚定交易

Pitch 亮点: "Gradience 不仅是 Agent 钱包，更是可证明合规的 Agent 支付治理平台。
每条审计日志都在 HashKey Chain 上有不可篡改的密码学证明。"
```

---

## 9. 安全考虑

| 风险 | 缓解 |
|---|---|
| 伪造 Merkle root | 链上合约只接受已锚定 root 的验证，root 必须通过 anchor() 提交 |
| 锚定私钥泄露 | 使用专用提交 key (非用户主 key)，可轮换 |
| 链重组 (reorg) | HashKey Chain 基于 OP Stack，最终确认性快；可等待 3-5 block 确认 |
| 大批次 Gas 过高 | 限制 batch_size (默认 1000)，单次 anchor gas < 100k |
| 锚定服务中断 | 恢复后自动从上次未锚定的日志继续 |

---

*本文为 Pre-Tech Spec 设计草案，待 Phase 3 阶段精确到字节级别实现。*
