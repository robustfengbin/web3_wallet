use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "VARCHAR")]
#[sqlx(rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    Operator,
}

impl Default for UserRole {
    fn default() -> Self {
        UserRole::Operator
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i32,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: i32,
    pub username: String,
    pub role: String,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        UserResponse {
            id: user.id,
            username: user.username,
            role: user.role,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Wallet {
    pub id: i32,
    pub name: String,
    pub address: String,
    #[serde(skip_serializing)]
    pub encrypted_private_key: String,
    pub chain: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletResponse {
    pub id: i32,
    pub name: String,
    pub address: String,
    pub chain: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

impl From<Wallet> for WalletResponse {
    fn from(wallet: Wallet) -> Self {
        WalletResponse {
            id: wallet.id,
            name: wallet.name,
            address: wallet.address,
            chain: wallet.chain,
            is_active: wallet.is_active,
            created_at: wallet.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "VARCHAR")]
#[sqlx(rename_all = "lowercase")]
pub enum TransferStatus {
    Pending,
    Submitted,
    Confirmed,
    Failed,
}

impl Default for TransferStatus {
    fn default() -> Self {
        TransferStatus::Pending
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Transfer {
    pub id: i32,
    pub wallet_id: i32,
    pub chain: String,
    pub from_address: String,
    pub to_address: String,
    pub token: String,
    pub amount: Decimal,
    pub gas_price: Option<Decimal>,
    pub gas_limit: Option<i64>,
    pub gas_used: Option<i64>,
    pub status: String,
    pub tx_hash: Option<String>,
    pub block_number: Option<i64>,
    pub error_message: Option<String>,
    pub initiated_by: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditLog {
    pub id: i32,
    pub user_id: Option<i32>,
    pub action: String,
    pub resource: Option<String>,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
}

// Request/Response DTOs
#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserResponse,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateWalletRequest {
    pub name: String,
    #[serde(default = "default_chain")]
    pub chain: String,
}

fn default_chain() -> String {
    "ethereum".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct ImportWalletRequest {
    pub name: String,
    pub private_key: String,
    #[serde(default = "default_chain")]
    pub chain: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExportPrivateKeyRequest {
    pub password: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportPrivateKeyResponse {
    pub private_key: String,
    pub warning: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TransferRequest {
    pub chain: String,
    pub to_address: String,
    pub token: String,
    pub amount: String,
    pub gas_price_gwei: Option<String>,
    pub gas_limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BalanceResponse {
    pub address: String,
    pub chain: String,
    pub native_balance: String,
    pub tokens: Vec<TokenBalance>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TokenBalance {
    pub symbol: String,
    pub balance: String,
    pub contract_address: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}
