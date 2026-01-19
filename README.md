[English](README.md) | [中文](README_CN.md)

# Web3 Wallet Service

A modular Web3 wallet management service with multi-chain support, featuring a Rust backend and React frontend.

## Features

- **Wallet Management** - Create, import, and manage multiple wallets with encrypted private key storage
- **Multi-Chain Support** - Extensible architecture for multiple blockchain networks (Ethereum supported)
- **Token Support** - Native and ERC20 tokens (USDT, USDC, DAI, WETH)
- **Transfer Management** - Initiate, execute, and track transactions with real-time status updates
- **Gas Estimation** - EIP-1559 compatible gas fee estimation
- **RPC Management** - Dynamic RPC endpoint configuration with fallback support
- **Role-Based Access** - Admin and Operator roles with permission controls
- **Internationalization** - Multi-language frontend support

## Screenshots

### Dashboard
![Dashboard](docs/images/dashboard.png)

### Wallet Management
![Wallet](docs/images/wallet.jpg)

### Transfer
![Transfer](docs/images/transfer.jpg)

### RPC Node Settings
![RPC Settings](docs/images/node_rpc.jpg)

## Tech Stack

### Backend
- **Rust** with Actix-web 4
- **MySQL 5.7+** with SQLx
- **Ethers-rs** for blockchain integration
- **AES-256-GCM** encryption for private keys
- **JWT** authentication

### Frontend
- **React 19** with TypeScript
- **Vite** build tool
- **Tailwind CSS**
- **i18next** for internationalization

## Project Structure

```
github_web3_wallet_service/
├── backend/                    # Rust backend service
│   ├── src/
│   │   ├── api/               # REST API endpoints and middleware
│   │   ├── blockchain/        # Chain clients and token definitions
│   │   ├── services/          # Business logic layer
│   │   ├── db/                # Database models and repositories
│   │   ├── crypto/            # Encryption and password hashing
│   │   └── config/            # Configuration management
│   └── Cargo.toml
│
└── frontend/                   # React TypeScript frontend
    ├── src/
    │   ├── pages/             # Page components
    │   ├── components/        # Reusable UI components
    │   ├── services/          # API client modules
    │   └── hooks/             # Custom React hooks
    └── package.json
```

## Quick Start

### Prerequisites

- Rust (latest stable)
- Node.js 18+
- MySQL 5.7+

### Backend Setup

1. Navigate to the backend directory:
```bash
cd backend
```

2. Copy the environment file and configure:
```bash
cp .env.example .env
```

3. Configure your `.env` file:
```env
# Server
WEB3_SERVER__HOST=127.0.0.1
WEB3_SERVER__PORT=8080

# Database
WEB3_DATABASE__HOST=localhost
WEB3_DATABASE__PORT=3306
WEB3_DATABASE__USER=root
WEB3_DATABASE__PASSWORD=your_password
WEB3_DATABASE__NAME=web3_wallet

# JWT
WEB3_JWT__SECRET=your-secure-jwt-secret-key
WEB3_JWT__EXPIRE_HOURS=24

# Security (must be exactly 32 characters)
WEB3_SECURITY__ENCRYPTION_KEY=uK7m2VxQ9nL3aT1aR8c26yH0uJ4bZ5wE

# Ethereum
WEB3_ETHEREUM__RPC_URL=https://eth.llamarpc.com
WEB3_ETHEREUM__CHAIN_ID=1
```

4. Start the backend:
```bash
# Development mode
./start.sh run

# Production mode
./start.sh run-release

# Using PM2
./start.sh pm2
```

### Frontend Setup

1. Navigate to the frontend directory:
```bash
cd frontend
```

2. Install dependencies:
```bash
npm install
```

3. Start the development server:
```bash
npm run dev
```

4. Build for production:
```bash
npm run build
```

### Default Credentials

- **Username:** admin
- **Password:** admin123

> **Important:** Change the default password immediately in production.

## API Reference

### Authentication
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/auth/login` | User login |
| POST | `/api/v1/auth/logout` | User logout |
| PUT | `/api/v1/auth/password` | Change password |
| GET | `/api/v1/auth/me` | Get current user info |

### Wallets
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/wallets` | List all wallets |
| POST | `/api/v1/wallets` | Create new wallet |
| POST | `/api/v1/wallets/import` | Import wallet from private key |
| GET | `/api/v1/wallets/{id}` | Get wallet details |
| DELETE | `/api/v1/wallets/{id}` | Delete wallet |
| PUT | `/api/v1/wallets/{id}/activate` | Set as active wallet |
| POST | `/api/v1/wallets/{id}/export-key` | Export private key |
| GET | `/api/v1/wallets/balance` | Get wallet balance |

### Transfers
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/transfers` | List transfers with pagination |
| POST | `/api/v1/transfers` | Initiate new transfer |
| GET | `/api/v1/transfers/{id}` | Get transfer details |
| POST | `/api/v1/transfers/{id}/execute` | Execute pending transfer |
| POST | `/api/v1/transfers/estimate-gas` | Estimate gas fees |

### Settings
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/settings/rpc` | Get current RPC config |
| PUT | `/api/v1/settings/rpc` | Update RPC config |
| POST | `/api/v1/settings/rpc/test` | Test RPC endpoint |
| GET | `/api/v1/settings/rpc/presets` | Get RPC presets |

### Health
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/health` | Health check |

## Security

- **Private Key Encryption:** AES-256-GCM encryption for all private keys at rest
- **Password Hashing:** Argon2 algorithm for password security
- **JWT Authentication:** Stateless authentication with configurable expiration
- **Role-Based Access Control:** Admin and Operator roles with different permissions
- **Sensitive Operation Protection:** Password verification required for private key export

### Security Best Practices

1. Change the default admin password immediately
2. Use a strong, random 32-byte encryption key
3. Use a cryptographically secure JWT secret
4. Enable HTTPS in production
5. Configure proper database access controls

## Database

The service automatically creates the required tables on startup:

- `users` - User accounts and roles
- `wallets` - Wallet information with encrypted private keys
- `transfers` - Transaction history and status
- `audit_logs` - Security audit trail
- `settings` - Application configuration

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `WEB3_SERVER__HOST` | Server bind address | 127.0.0.1 |
| `WEB3_SERVER__PORT` | Server port | 8080 |
| `WEB3_DATABASE__HOST` | MySQL host | localhost |
| `WEB3_DATABASE__PORT` | MySQL port | 3306 |
| `WEB3_DATABASE__USER` | MySQL user | root |
| `WEB3_DATABASE__PASSWORD` | MySQL password | - |
| `WEB3_DATABASE__NAME` | Database name | web3_wallet |
| `WEB3_JWT__SECRET` | JWT signing secret | - |
| `WEB3_JWT__EXPIRE_HOURS` | Token expiration | 24 |
| `WEB3_SECURITY__ENCRYPTION_KEY` | 32-byte encryption key | - |
| `WEB3_ETHEREUM__RPC_URL` | Ethereum RPC endpoint | - |
| `WEB3_ETHEREUM__CHAIN_ID` | Ethereum chain ID | 1 |
| `WEB3_ETHEREUM__RPC_PROXY` | Optional RPC proxy | - |

### Frontend Configuration

| Variable | Description | Default |
|----------|-------------|---------|
| `VITE_API_BASE_URL` | Backend API URL | http://localhost:8080/api/v1 |

## PM2 Deployment

The backend includes PM2 configuration for production deployment:

```bash
# Start with PM2
./start.sh pm2

# View status
./start.sh status

# Stop
./start.sh pm2-stop

# Restart
./start.sh pm2-restart
```

## Extending

### Adding New Chains

Implement the `ChainClient` trait in `backend/src/blockchain/traits.rs`:

```rust
#[async_trait]
pub trait ChainClient: Send + Sync {
    async fn get_balance(&self, address: &str) -> Result<String>;
    async fn get_token_balance(&self, address: &str, token: &str) -> Result<String>;
    async fn send_transaction(&self, tx: TransactionRequest) -> Result<String>;
    // ... other methods
}
```

### Adding New Tokens

Add token definitions in `backend/src/blockchain/ethereum/tokens.rs`:

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

## Logging

Backend logs are written to `backend/logs/web3-wallet.log` with:
- 500MB file size limit
- 10 backup files rotation
- Configurable log level via `RUST_LOG`

```bash
# Example log configuration
RUST_LOG=info,sqlx=warn
```

## Support

If you find this project useful, consider supporting the development:

**ETH / USDT / USDC (ERC20):** `0xD76f061DaEcfC3ddaD7902A8Ff7c47FC68b3Dc49`

## License

MIT License

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request
