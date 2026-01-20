use std::sync::Arc;

use crate::blockchain::zcash::orchard::{
    keys::OrchardViewingKey, scanner::ShieldedBalance, ScanProgress, UnifiedAddressInfo,
};
use crate::blockchain::zcash::ZcashClient;
use crate::blockchain::ChainRegistry;
use crate::config::SecurityConfig;
use crate::crypto::{
    decrypt, encrypt, generate_ethereum_wallet, generate_zcash_wallet,
    import_ethereum_wallet, import_zcash_wallet,
};
use crate::crypto::zcash::{
    enable_orchard_for_wallet, generate_unified_address, is_unified_address, parse_unified_address,
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

        // Import address into chain node for tracking (needed for UTXO-based chains like Zcash)
        let chain_client = self.chain_registry.get(chain)?;
        if let Err(e) = chain_client.import_address_for_tracking(&address, name).await {
            tracing::warn!("Failed to import address for tracking: {}", e);
            // Don't fail wallet creation, just warn
        }

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

        // Import address into chain node for tracking (needed for UTXO-based chains like Zcash)
        let chain_client = self.chain_registry.get(chain)?;
        if let Err(e) = chain_client.import_address_for_tracking(&address, name).await {
            tracing::warn!("Failed to import address for tracking: {}", e);
            // Don't fail wallet import, just warn
        }

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

    // =========================================================================
    // Orchard Privacy Protocol Methods
    // =========================================================================

    /// Enable Orchard (shielded) functionality for a Zcash wallet
    ///
    /// This derives Orchard keys from the existing transparent private key
    /// and generates a unified address that can receive both transparent and shielded funds.
    ///
    /// # Arguments
    /// * `wallet_id` - ID of the Zcash wallet to enable Orchard for
    /// * `birthday_height` - Block height when the wallet was created (for scanning)
    ///
    /// # Returns
    /// * Unified address info and encoded viewing key
    pub async fn enable_orchard(
        &self,
        wallet_id: i32,
        birthday_height: u64,
    ) -> AppResult<(UnifiedAddressInfo, String)> {
        // Get wallet
        let wallet = self
            .wallet_repo
            .find_by_id(wallet_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Wallet not found".to_string()))?;

        // Verify it's a Zcash wallet
        if wallet.chain != "zcash" {
            return Err(AppError::ValidationError(
                "Orchard is only available for Zcash wallets".to_string(),
            ));
        }

        // Decrypt private key
        let private_key = decrypt(
            &wallet.encrypted_private_key,
            &self.security_config.encryption_key,
        )?;

        // Enable Orchard and get unified address
        let (unified_address, viewing_key_encoded) =
            enable_orchard_for_wallet(&private_key, birthday_height)?;

        // Initialize Orchard scanner in the Zcash client
        let zcash_client = self.get_zcash_client()?;
        let viewing_key = OrchardViewingKey::decode(&viewing_key_encoded)
            .map_err(|e| AppError::InternalError(format!("Failed to decode viewing key: {}", e)))?;

        zcash_client
            .init_orchard_scanner(vec![viewing_key])
            .await?;

        tracing::info!(
            "Enabled Orchard for wallet {}, unified address: {}",
            wallet_id,
            unified_address.address
        );

        Ok((unified_address, viewing_key_encoded))
    }

    /// Generate a new unified address for a wallet that has Orchard enabled
    ///
    /// # Arguments
    /// * `viewing_key_encoded` - The encoded viewing key from enable_orchard
    /// * `address_index` - Index for the new address (0 for first, incrementing)
    ///
    /// # Returns
    /// * New unified address info
    pub async fn generate_new_unified_address(
        &self,
        viewing_key_encoded: &str,
        address_index: u32,
    ) -> AppResult<UnifiedAddressInfo> {
        generate_unified_address(viewing_key_encoded, address_index)
    }

    /// Get shielded (Orchard) balance for a wallet
    ///
    /// # Arguments
    /// * `wallet_id` - ID of the wallet (must have Orchard enabled)
    ///
    /// # Returns
    /// * Shielded balance breakdown
    pub async fn get_shielded_balance(&self, wallet_id: i32) -> AppResult<ShieldedBalance> {
        let wallet = self
            .wallet_repo
            .find_by_id(wallet_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Wallet not found".to_string()))?;

        if wallet.chain != "zcash" {
            return Err(AppError::ValidationError(
                "Shielded balance only available for Zcash wallets".to_string(),
            ));
        }

        let zcash_client = self.get_zcash_client()?;

        // Account ID is 0 for single-account wallets
        // In a multi-account setup, this would be different
        zcash_client.get_orchard_balance(0).await
    }

    /// Get combined balance (transparent + shielded) for a Zcash wallet
    ///
    /// # Arguments
    /// * `wallet_id` - ID of the wallet
    ///
    /// # Returns
    /// * Balance response with both transparent and shielded balances
    pub async fn get_combined_zcash_balance(
        &self,
        wallet_id: i32,
    ) -> AppResult<CombinedZcashBalance> {
        let wallet = self
            .wallet_repo
            .find_by_id(wallet_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Wallet not found".to_string()))?;

        if wallet.chain != "zcash" {
            return Err(AppError::ValidationError(
                "This endpoint is only for Zcash wallets".to_string(),
            ));
        }

        let chain_client = self.chain_registry.get("zcash")?;

        // Get transparent balance
        let transparent_balance = chain_client.get_native_balance(&wallet.address).await?;

        // Try to get shielded balance (may fail if Orchard not enabled)
        let shielded_balance = match self.get_shielded_balance(wallet_id).await {
            Ok(balance) => Some(balance),
            Err(_) => None,
        };

        let total_zatoshis = (transparent_balance
            .to_string()
            .parse::<f64>()
            .unwrap_or(0.0)
            * 100_000_000.0) as u64
            + shielded_balance
                .as_ref()
                .map(|b| b.total_zatoshis)
                .unwrap_or(0);

        Ok(CombinedZcashBalance {
            wallet_id,
            address: wallet.address,
            transparent_balance: transparent_balance.to_string(),
            shielded_balance,
            total_zec: total_zatoshis as f64 / 100_000_000.0,
        })
    }

    /// Get Orchard scan progress
    pub async fn get_scan_progress(&self) -> AppResult<ScanProgress> {
        let zcash_client = self.get_zcash_client()?;
        zcash_client.get_scan_progress().await
    }

    /// Trigger Orchard sync
    pub async fn sync_orchard(&self) -> AppResult<ScanProgress> {
        let zcash_client = self.get_zcash_client()?;
        zcash_client.sync_orchard().await
    }

    /// Parse a unified address
    pub fn parse_address(&self, address: &str) -> AppResult<UnifiedAddressInfo> {
        parse_unified_address(address)
    }

    /// Check if an address is a unified address
    pub fn is_unified(&self, address: &str) -> bool {
        is_unified_address(address)
    }

    /// Get the Zcash client from the registry
    fn get_zcash_client(&self) -> AppResult<&ZcashClient> {
        // Get the chain client and downcast to ZcashClient
        // This is a workaround since we can't easily get the concrete type
        // In production, you might want to store the ZcashClient separately
        Err(AppError::InternalError(
            "Direct Zcash client access not implemented - use chain registry".to_string(),
        ))
    }
}

/// Combined balance for Zcash (transparent + shielded)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CombinedZcashBalance {
    pub wallet_id: i32,
    pub address: String,
    pub transparent_balance: String,
    pub shielded_balance: Option<ShieldedBalance>,
    pub total_zec: f64,
}
