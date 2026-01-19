use rust_decimal::Decimal;
use std::str::FromStr;
use std::sync::Arc;

use crate::blockchain::{ChainRegistry, TransferParams, TxStatus};
use crate::db::models::{Transfer, TransferRequest};
use crate::db::repositories::TransferRepository;
use crate::error::{AppError, AppResult};
use crate::services::WalletService;

pub struct TransferService {
    transfer_repo: TransferRepository,
    wallet_service: Arc<WalletService>,
    chain_registry: Arc<ChainRegistry>,
}

impl TransferService {
    pub fn new(
        transfer_repo: TransferRepository,
        wallet_service: Arc<WalletService>,
        chain_registry: Arc<ChainRegistry>,
    ) -> Self {
        Self {
            transfer_repo,
            wallet_service,
            chain_registry,
        }
    }

    /// Initiate a transfer (creates pending record)
    pub async fn initiate_transfer(
        &self,
        request: TransferRequest,
        user_id: i32,
    ) -> AppResult<Transfer> {
        let chain_client = self.chain_registry.get(&request.chain)?;

        // Validate address
        if !chain_client.validate_address(&request.to_address) {
            return Err(AppError::ValidationError("Invalid destination address".to_string()));
        }

        // Get active wallet
        let wallet = self.wallet_service.get_active_wallet(&request.chain).await?;

        // Parse amount
        let amount = Decimal::from_str(&request.amount)
            .map_err(|e| AppError::ValidationError(format!("Invalid amount: {}", e)))?;

        if amount <= Decimal::ZERO {
            return Err(AppError::ValidationError("Amount must be positive".to_string()));
        }

        // Check balance
        let (native_balance, token_balances) = chain_client.get_all_balances(&wallet.address).await?;

        let token_upper = request.token.to_uppercase();
        let is_native = token_upper == chain_client.native_token_symbol();

        if is_native {
            if native_balance < amount {
                return Err(AppError::InsufficientBalance(format!(
                    "Insufficient {} balance. Required: {}, Available: {}",
                    chain_client.native_token_symbol(),
                    amount,
                    native_balance
                )));
            }
        } else {
            let token_balance = token_balances
                .iter()
                .find(|t| t.symbol.to_uppercase() == token_upper)
                .map(|t| t.balance)
                .unwrap_or(Decimal::ZERO);

            if token_balance < amount {
                return Err(AppError::InsufficientBalance(format!(
                    "Insufficient {} balance. Required: {}, Available: {}",
                    request.token, amount, token_balance
                )));
            }
        }

        // Parse optional gas parameters
        let gas_price = request
            .gas_price_gwei
            .as_ref()
            .map(|p| Decimal::from_str(p))
            .transpose()
            .map_err(|e| AppError::ValidationError(format!("Invalid gas price: {}", e)))?;

        // Create transfer record
        let transfer_id = self
            .transfer_repo
            .create(
                wallet.id,
                &request.chain,
                &wallet.address,
                &request.to_address,
                &request.token,
                amount,
                gas_price,
                request.gas_limit,
                user_id,
            )
            .await?;

        self.transfer_repo
            .find_by_id(transfer_id)
            .await?
            .ok_or_else(|| AppError::InternalError("Failed to retrieve transfer".to_string()))
    }

    /// Execute a pending transfer
    pub async fn execute_transfer(&self, transfer_id: i32) -> AppResult<Transfer> {
        let transfer = self
            .transfer_repo
            .find_by_id(transfer_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Transfer not found".to_string()))?;

        if transfer.status != "pending" {
            return Err(AppError::ValidationError(format!(
                "Transfer is not pending. Current status: {}",
                transfer.status
            )));
        }

        let chain_client = self.chain_registry.get(&transfer.chain)?;

        // Get private key
        let private_key = self.wallet_service.get_private_key(transfer.wallet_id).await?;

        let params = TransferParams {
            from_address: transfer.from_address.clone(),
            to_address: transfer.to_address.clone(),
            private_key,
            token: transfer.token.clone(),
            amount: transfer.amount,
            gas_price_gwei: transfer.gas_price,
            gas_limit: transfer.gas_limit.map(|g| g as u64),
        };

        // Execute transfer
        let is_native = transfer.token.to_uppercase() == chain_client.native_token_symbol();
        let result = if is_native {
            chain_client.transfer_native(&params).await
        } else {
            chain_client.transfer_token(&params).await
        };

        match result {
            Ok(tx_hash) => {
                self.transfer_repo
                    .update_status(transfer_id, "submitted", Some(&tx_hash), None)
                    .await?;
            }
            Err(e) => {
                self.transfer_repo
                    .update_status(transfer_id, "failed", None, Some(&e.to_string()))
                    .await?;
                return Err(e);
            }
        }

        self.transfer_repo
            .find_by_id(transfer_id)
            .await?
            .ok_or_else(|| AppError::InternalError("Failed to retrieve transfer".to_string()))
    }

    /// Check and update status of submitted transfers
    pub async fn check_pending_transfers(&self) -> AppResult<()> {
        let pending = self.transfer_repo.list_pending().await?;

        for transfer in pending {
            if let Some(tx_hash) = &transfer.tx_hash {
                let chain_client = match self.chain_registry.get(&transfer.chain) {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                match chain_client.get_tx_status(tx_hash).await {
                    Ok(TxStatus::Confirmed { block_number, gas_used }) => {
                        self.transfer_repo
                            .update_confirmed(transfer.id, block_number as i64, gas_used as i64)
                            .await?;
                        tracing::info!("Transfer {} confirmed at block {}", transfer.id, block_number);
                    }
                    Ok(TxStatus::Failed { reason }) => {
                        self.transfer_repo
                            .update_status(transfer.id, "failed", None, Some(&reason))
                            .await?;
                        tracing::warn!("Transfer {} failed: {}", transfer.id, reason);
                    }
                    Ok(TxStatus::Pending) | Ok(TxStatus::NotFound) => {
                        // Still pending, do nothing
                    }
                    Err(e) => {
                        tracing::warn!("Failed to check transfer {}: {}", transfer.id, e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Get transfer by ID
    pub async fn get_transfer(&self, id: i32) -> AppResult<Transfer> {
        self.transfer_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound("Transfer not found".to_string()))
    }

    /// List transfers with pagination
    pub async fn list_transfers(&self, limit: i32, offset: i32) -> AppResult<(Vec<Transfer>, i64)> {
        let transfers = self.transfer_repo.list_all(limit, offset).await?;
        let total = self.transfer_repo.count_all().await?;
        Ok((transfers, total))
    }

    /// List transfers for a specific wallet
    pub async fn list_wallet_transfers(
        &self,
        wallet_id: i32,
        limit: i32,
        offset: i32,
    ) -> AppResult<(Vec<Transfer>, i64)> {
        let transfers = self.transfer_repo.list_by_wallet(wallet_id, limit, offset).await?;
        let total = self.transfer_repo.count_by_wallet(wallet_id).await?;
        Ok((transfers, total))
    }
}
