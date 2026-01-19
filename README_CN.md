[English](README.md) | [中文](README_CN.md)

# Web3 钱包服务

一个模块化的 Web3 钱包管理服务，支持多链架构，采用 Rust 后端 + React 前端技术栈。

## 功能特性

- **钱包管理** - 创建、导入和管理多个钱包，私钥加密存储
- **多链支持** - 可扩展的多链架构（当前支持以太坊）
- **代币支持** - 原生代币和 ERC20 代币（USDT、USDC、DAI、WETH）
- **转账管理** - 发起、执行和追踪交易，实时状态更新
- **Gas 估算** - 兼容 EIP-1559 的 Gas 费用估算
- **RPC 管理** - 动态 RPC 节点配置，支持备用节点
- **权限控制** - 管理员和操作员角色权限管理
- **国际化** - 前端多语言支持

## 界面预览

### 仪表盘
![仪表盘](docs/images/dashboard.png)

### 钱包管理
![钱包管理](docs/images/wallet.jpg)

### 转账
![转账](docs/images/transfer.jpg)

### RPC 节点设置
![RPC设置](docs/images/node_rpc.jpg)

## 技术栈

### 后端
- **Rust** + Actix-web 4
- **MySQL 5.7+** + SQLx
- **Ethers-rs** 区块链集成
- **AES-256-GCM** 私钥加密
- **JWT** 身份认证

### 前端
- **React 19** + TypeScript
- **Vite** 构建工具
- **Tailwind CSS**
- **i18next** 国际化

## 项目结构

```
github_web3_wallet_service/
├── backend/                    # Rust 后端服务
│   ├── src/
│   │   ├── api/               # REST API 端点和中间件
│   │   ├── blockchain/        # 链客户端和代币定义
│   │   ├── services/          # 业务逻辑层
│   │   ├── db/                # 数据库模型和仓储
│   │   ├── crypto/            # 加密和密码哈希
│   │   └── config/            # 配置管理
│   └── Cargo.toml
│
└── frontend/                   # React TypeScript 前端
    ├── src/
    │   ├── pages/             # 页面组件
    │   ├── components/        # 可复用 UI 组件
    │   ├── services/          # API 客户端模块
    │   └── hooks/             # 自定义 React Hooks
    └── package.json
```

## 快速开始

### 环境要求

- Rust（最新稳定版）
- Node.js 18+
- MySQL 5.7+

### 后端配置

1. 进入后端目录：
```bash
cd backend
```

2. 复制环境配置文件：
```bash
cp .env.example .env
```

3. 编辑 `.env` 文件：
```env
# 服务器配置
WEB3_SERVER__HOST=127.0.0.1
WEB3_SERVER__PORT=8080

# 数据库配置
WEB3_DATABASE__HOST=localhost
WEB3_DATABASE__PORT=3306
WEB3_DATABASE__USER=root
WEB3_DATABASE__PASSWORD=your_password
WEB3_DATABASE__NAME=web3_wallet

# JWT 配置
WEB3_JWT__SECRET=your-secure-jwt-secret-key
WEB3_JWT__EXPIRE_HOURS=24

# 安全配置（必须是 32 个字符）
WEB3_SECURITY__ENCRYPTION_KEY=uK7m2VxQ9nL3aT1aR8c26yH0uJ4bZ5wE

# 以太坊配置
WEB3_ETHEREUM__RPC_URL=https://eth.llamarpc.com
WEB3_ETHEREUM__CHAIN_ID=1
```

4. 启动后端服务：
```bash
# 开发模式
./start.sh run

# 生产模式
./start.sh run-release

# 使用 PM2
./start.sh pm2
```

### 前端配置

1. 进入前端目录：
```bash
cd frontend
```

2. 安装依赖：
```bash
npm install
```

3. 启动开发服务器：
```bash
npm run dev
```

4. 构建生产版本：
```bash
npm run build
```

### 默认账号

- **用户名：** admin
- **密码：** admin123

> **重要提示：** 生产环境请立即修改默认密码。

## API 接口

### 认证接口
| 方法 | 端点 | 描述 |
|------|------|------|
| POST | `/api/v1/auth/login` | 用户登录 |
| POST | `/api/v1/auth/logout` | 用户登出 |
| PUT | `/api/v1/auth/password` | 修改密码 |
| GET | `/api/v1/auth/me` | 获取当前用户信息 |

### 钱包接口
| 方法 | 端点 | 描述 |
|------|------|------|
| GET | `/api/v1/wallets` | 获取钱包列表 |
| POST | `/api/v1/wallets` | 创建新钱包 |
| POST | `/api/v1/wallets/import` | 导入钱包（通过私钥） |
| GET | `/api/v1/wallets/{id}` | 获取钱包详情 |
| DELETE | `/api/v1/wallets/{id}` | 删除钱包 |
| PUT | `/api/v1/wallets/{id}/activate` | 设为活跃钱包 |
| POST | `/api/v1/wallets/{id}/export-key` | 导出私钥 |
| GET | `/api/v1/wallets/balance` | 获取钱包余额 |

### 转账接口
| 方法 | 端点 | 描述 |
|------|------|------|
| GET | `/api/v1/transfers` | 获取转账记录（分页） |
| POST | `/api/v1/transfers` | 发起转账 |
| GET | `/api/v1/transfers/{id}` | 获取转账详情 |
| POST | `/api/v1/transfers/{id}/execute` | 执行待处理转账 |
| POST | `/api/v1/transfers/estimate-gas` | 估算 Gas 费用 |

### 设置接口
| 方法 | 端点 | 描述 |
|------|------|------|
| GET | `/api/v1/settings/rpc` | 获取当前 RPC 配置 |
| PUT | `/api/v1/settings/rpc` | 更新 RPC 配置 |
| POST | `/api/v1/settings/rpc/test` | 测试 RPC 连接 |
| GET | `/api/v1/settings/rpc/presets` | 获取 RPC 预设列表 |

### 健康检查
| 方法 | 端点 | 描述 |
|------|------|------|
| GET | `/api/v1/health` | 健康检查 |

## 安全性

- **私钥加密：** 使用 AES-256-GCM 加密存储所有私钥
- **密码哈希：** 使用 Argon2 算法保护密码安全
- **JWT 认证：** 无状态认证，可配置过期时间
- **角色权限控制：** 管理员和操作员角色拥有不同权限
- **敏感操作保护：** 导出私钥需要密码验证

### 安全最佳实践

1. 立即修改默认管理员密码
2. 使用强随机的 32 字节加密密钥
3. 使用加密安全的 JWT 密钥
4. 生产环境启用 HTTPS
5. 配置适当的数据库访问控制

## 数据库

服务启动时自动创建所需数据表：

- `users` - 用户账号和角色
- `wallets` - 钱包信息（含加密私钥）
- `transfers` - 交易历史和状态
- `audit_logs` - 安全审计日志
- `settings` - 应用配置

## 配置说明

### 环境变量

| 变量 | 描述 | 默认值 |
|------|------|--------|
| `WEB3_SERVER__HOST` | 服务器绑定地址 | 127.0.0.1 |
| `WEB3_SERVER__PORT` | 服务器端口 | 8080 |
| `WEB3_DATABASE__HOST` | MySQL 主机 | localhost |
| `WEB3_DATABASE__PORT` | MySQL 端口 | 3306 |
| `WEB3_DATABASE__USER` | MySQL 用户名 | root |
| `WEB3_DATABASE__PASSWORD` | MySQL 密码 | - |
| `WEB3_DATABASE__NAME` | 数据库名称 | web3_wallet |
| `WEB3_JWT__SECRET` | JWT 签名密钥 | - |
| `WEB3_JWT__EXPIRE_HOURS` | Token 过期时间（小时） | 24 |
| `WEB3_SECURITY__ENCRYPTION_KEY` | 32 字节加密密钥 | - |
| `WEB3_ETHEREUM__RPC_URL` | 以太坊 RPC 节点 | - |
| `WEB3_ETHEREUM__CHAIN_ID` | 以太坊链 ID | 1 |
| `WEB3_ETHEREUM__RPC_PROXY` | RPC 代理（可选） | - |

### 前端配置

| 变量 | 描述 | 默认值 |
|------|------|--------|
| `VITE_API_BASE_URL` | 后端 API 地址 | http://localhost:8080/api/v1 |

## PM2 部署

后端包含 PM2 配置用于生产环境部署：

```bash
# 使用 PM2 启动
./start.sh pm2

# 查看状态
./start.sh status

# 停止服务
./start.sh pm2-stop

# 重启服务
./start.sh pm2-restart
```

## 扩展开发

### 添加新链

在 `backend/src/blockchain/traits.rs` 中实现 `ChainClient` trait：

```rust
#[async_trait]
pub trait ChainClient: Send + Sync {
    async fn get_balance(&self, address: &str) -> Result<String>;
    async fn get_token_balance(&self, address: &str, token: &str) -> Result<String>;
    async fn send_transaction(&self, tx: TransactionRequest) -> Result<String>;
    // ... 其他方法
}
```

### 添加新代币

在 `backend/src/blockchain/ethereum/tokens.rs` 中添加代币定义：

```rust
pub static SUPPORTED_TOKENS: Lazy<HashMap<&'static str, TokenInfo>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("NEW_TOKEN", TokenInfo {
        address: "0x...",
        decimals: 18,
        symbol: "NEW",
    });
    m
});
```

## 日志

后端日志写入 `backend/logs/web3-wallet.log`，配置如下：
- 文件大小限制 500MB
- 保留 10 个备份文件
- 通过 `RUST_LOG` 配置日志级别

```bash
# 日志级别配置示例
RUST_LOG=info,sqlx=warn
```

## 开源协议

MIT License

## 参与贡献

1. Fork 本仓库
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m '添加某个新功能'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 提交 Pull Request
