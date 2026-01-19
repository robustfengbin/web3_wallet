#![allow(dead_code)]

use async_trait::async_trait;
use rust_decimal::Decimal;

use crate::error::AppResult;

/// Represents a transfer request
#[derive(Debug, Clone)]
pub struct TransferParams {
    pub from_address: String,
    pub to_address: String,
    pub private_key: String,
    pub token: String,
    pub amount: Decimal,
    pub gas_price_gwei: Option<Decimal>,
    pub gas_limit: Option<u64>,
}

/// Represents gas estimation result with EIP-1559 parameters
#[derive(Debug, Clone)]
pub struct GasEstimate {
    pub gas_limit: u64,
    pub gas_price_gwei: Decimal,       // Legacy gas price for display
    pub estimated_fee_eth: Decimal,    // Estimated fee using max_fee
    // EIP-1559 specific fields
    pub base_fee_gwei: Option<Decimal>,
    pub priority_fee_gwei: Option<Decimal>,
    pub max_fee_gwei: Option<Decimal>,
}

/// Represents transaction status
#[derive(Debug, Clone, PartialEq)]
pub enum TxStatus {
    Pending,
    Confirmed { block_number: u64, gas_used: u64 },
    Failed { reason: String },
    NotFound,
}

/// Token balance information
#[derive(Debug, Clone)]
pub struct TokenBalance {
    pub symbol: String,
    pub balance: Decimal,
    pub contract_address: Option<String>,
}

/// Abstract trait for blockchain clients
/// Implement this trait to add support for new chains
#[async_trait]
pub trait ChainClient: Send + Sync {
    /// Get the chain identifier (e.g., "ethereum", "bsc", "polygon")
    fn chain_id(&self) -> &str;

    /// Get the chain's display name
    fn chain_name(&self) -> &str;

    /// Get the native token symbol (e.g., "ETH", "BNB")
    fn native_token_symbol(&self) -> &str;

    /// Get native token balance for an address
    async fn get_native_balance(&self, address: &str) -> AppResult<Decimal>;

    /// Get ERC20/BEP20 token balance
    async fn get_token_balance(&self, address: &str, token_symbol: &str) -> AppResult<Decimal>;

    /// Get all supported token balances for an address
    async fn get_all_balances(&self, address: &str) -> AppResult<(Decimal, Vec<TokenBalance>)>;

    /// Estimate gas for a transfer
    async fn estimate_gas(&self, params: &TransferParams) -> AppResult<GasEstimate>;

    /// Execute a native token transfer
    async fn transfer_native(&self, params: &TransferParams) -> AppResult<String>;

    /// Execute an ERC20/BEP20 token transfer
    async fn transfer_token(&self, params: &TransferParams) -> AppResult<String>;

    /// Get transaction status
    async fn get_tx_status(&self, tx_hash: &str) -> AppResult<TxStatus>;

    /// Validate an address format
    fn validate_address(&self, address: &str) -> bool;

    /// Get current gas price in Gwei
    async fn get_gas_price(&self) -> AppResult<Decimal>;
}
