use std::sync::Arc;

use crate::blockchain::ChainRegistry;
use crate::config::SecurityConfig;
use crate::crypto::{
    decrypt, encrypt,
    generate_ethereum_wallet, import_ethereum_wallet,
    generate_zcash_wallet, import_zcash_wallet,
};
use crate::db::models::{BalanceResponse, TokenBalance, Wallet, WalletResponse};
use crate::db::repositories::WalletRepository;
use crate::error::{AppError, AppResult};

pub struct WalletService {
    wallet_repo: WalletRepository,
    chain_registry: Arc<ChainRegistry>,
    security_config: SecurityConfig,
}

impl WalletService {
    pub fn new(
        wallet_repo: WalletRepository,
        chain_registry: Arc<ChainRegistry>,
        security_config: SecurityConfig,
    ) -> Self {
        Self {
            wallet_repo,
            chain_registry,
            security_config,
        }
    }

    /// Create a new wallet with generated private key
    pub async fn create_wallet(&self, name: &str, chain: &str) -> AppResult<WalletResponse> {
        // Verify chain is supported
        self.chain_registry.get(chain)?;

        // Generate wallet based on chain type
        let (address, private_key) = match chain {
            "zcash" => generate_zcash_wallet()?,
            "ethereum" | _ => generate_ethereum_wallet()?,
        };

        // Check if address already exists
        if self.wallet_repo.find_by_address(&address, chain).await?.is_some() {
            return Err(AppError::AlreadyExists(format!(
                "Wallet with address {} already exists",
                address
            )));
        }

        // Encrypt private key
        let encrypted_key = encrypt(&private_key, &self.security_config.encryption_key)?;

        // Store wallet
        let id = self
            .wallet_repo
            .create(name, &address, &encrypted_key, chain)
            .await?;

        let wallet = self
            .wallet_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::InternalError("Failed to retrieve created wallet".to_string()))?;

        Ok(WalletResponse::from(wallet))
    }

    /// Import an existing wallet from private key
    pub async fn import_wallet(
        &self,
        name: &str,
        private_key: &str,
        chain: &str,
    ) -> AppResult<WalletResponse> {
        // Verify chain is supported
        self.chain_registry.get(chain)?;

        // Parse and validate private key based on chain type
        let key = private_key.strip_prefix("0x").unwrap_or(private_key);

        let address = match chain {
            "zcash" => import_zcash_wallet(key)?,
            "ethereum" | _ => import_ethereum_wallet(key)?,
        };

        // Check if address already exists
        if self.wallet_repo.find_by_address(&address, chain).await?.is_some() {
            return Err(AppError::AlreadyExists(format!(
                "Wallet with address {} already exists",
                address
            )));
        }

        // Encrypt private key
        let encrypted_key = encrypt(key, &self.security_config.encryption_key)?;

        // Store wallet
        let id = self
            .wallet_repo
            .create(name, &address, &encrypted_key, chain)
            .await?;

        let wallet = self
            .wallet_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::InternalError("Failed to retrieve imported wallet".to_string()))?;

        Ok(WalletResponse::from(wallet))
    }

    /// List all wallets
    pub async fn list_wallets(&self) -> AppResult<Vec<WalletResponse>> {
        let wallets = self.wallet_repo.list_all().await?;
        Ok(wallets.into_iter().map(WalletResponse::from).collect())
    }

    /// List wallets by chain
    pub async fn list_wallets_by_chain(&self, chain: &str) -> AppResult<Vec<WalletResponse>> {
        let wallets = self.wallet_repo.list_by_chain(chain).await?;
        Ok(wallets.into_iter().map(WalletResponse::from).collect())
    }

    /// Get wallet by ID
    pub async fn get_wallet(&self, id: i32) -> AppResult<WalletResponse> {
        let wallet = self
            .wallet_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound("Wallet not found".to_string()))?;

        Ok(WalletResponse::from(wallet))
    }

    /// Get active wallet for a chain
    pub async fn get_active_wallet(&self, chain: &str) -> AppResult<Wallet> {
        self.wallet_repo
            .get_active_wallet(chain)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("No active wallet for chain {}", chain)))
    }

    /// Set a wallet as active
    pub async fn set_active_wallet(&self, id: i32) -> AppResult<()> {
        let wallet = self
            .wallet_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound("Wallet not found".to_string()))?;

        self.wallet_repo.set_active(id, &wallet.chain).await
    }

    /// Get wallet balance
    pub async fn get_balance(&self, address: &str, chain: &str) -> AppResult<BalanceResponse> {
        let chain_client = self.chain_registry.get(chain)?;

        let (native_balance, token_balances) = chain_client.get_all_balances(address).await?;

        Ok(BalanceResponse {
            address: address.to_string(),
            chain: chain.to_string(),
            native_balance: native_balance.to_string(),
            tokens: token_balances
                .into_iter()
                .map(|t| TokenBalance {
                    symbol: t.symbol,
                    balance: t.balance.to_string(),
                    contract_address: t.contract_address,
                })
                .collect(),
        })
    }

    /// Export private key (requires password verification)
    pub async fn export_private_key(&self, wallet_id: i32) -> AppResult<String> {
        let wallet = self
            .wallet_repo
            .find_by_id(wallet_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Wallet not found".to_string()))?;

        let private_key = decrypt(
            &wallet.encrypted_private_key,
            &self.security_config.encryption_key,
        )?;

        Ok(format!("0x{}", private_key))
    }

    /// Get decrypted private key for internal use
    pub async fn get_private_key(&self, wallet_id: i32) -> AppResult<String> {
        let wallet = self
            .wallet_repo
            .find_by_id(wallet_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Wallet not found".to_string()))?;

        decrypt(
            &wallet.encrypted_private_key,
            &self.security_config.encryption_key,
        )
    }

    /// Delete a wallet
    pub async fn delete_wallet(&self, id: i32) -> AppResult<()> {
        // Verify wallet exists
        self.wallet_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound("Wallet not found".to_string()))?;

        self.wallet_repo.delete(id).await
    }
}
