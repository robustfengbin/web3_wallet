use async_trait::async_trait;
use ethers::prelude::*;
use ethers::providers::{Http, Provider};
use ethers::utils::{format_units, parse_units};
use reqwest::Proxy;
use rust_decimal::Decimal;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use url::Url;

use crate::blockchain::traits::{ChainClient, GasEstimate, TokenBalance, TransferParams, TxStatus};
use crate::config::EthereumConfig;
use crate::error::{AppError, AppResult};

use super::tokens::{get_token_info, SUPPORTED_TOKENS};

// ERC20 ABI for balanceOf and transfer
abigen!(
    ERC20,
    r#"[
        function balanceOf(address account) external view returns (uint256)
        function transfer(address to, uint256 amount) external returns (bool)
        function decimals() external view returns (uint8)
    ]"#
);

/// Dynamic RPC configuration that can be updated at runtime
pub struct RpcSettings {
    pub primary_rpc: String,
    pub fallback_rpcs: Vec<String>,
    pub rpc_proxy: Option<String>,
}

pub struct EthereumClient {
    rpc_settings: RwLock<RpcSettings>,
    chain_id: u64,
}

impl EthereumClient {
    /// Create a reqwest client with optional proxy support
    fn create_http_client(proxy_url: &Option<String>) -> AppResult<reqwest::Client> {
        let mut client_builder = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30));

        if let Some(proxy) = proxy_url {
            if !proxy.is_empty() {
                let proxy = Proxy::all(proxy)
                    .map_err(|e| AppError::BlockchainError(format!("Invalid proxy URL: {}", e)))?;
                client_builder = client_builder.proxy(proxy);
                tracing::debug!("RPC proxy configured: {}", proxy_url.as_ref().unwrap());
            }
        }

        client_builder
            .build()
            .map_err(|e| AppError::BlockchainError(format!("Failed to create HTTP client: {}", e)))
    }

    /// Create a provider with the given RPC URL and optional proxy
    fn create_provider_with_proxy(rpc_url: &str, proxy_url: &Option<String>) -> AppResult<Provider<Http>> {
        let client = Self::create_http_client(proxy_url)?;
        let url = Url::parse(rpc_url)
            .map_err(|e| AppError::BlockchainError(format!("Invalid RPC URL: {}", e)))?;
        let http = Http::new_with_client(url, client);
        Ok(Provider::new(http))
    }
}

impl EthereumClient {
    pub fn new(config: &EthereumConfig) -> AppResult<Self> {
        // Validate the initial RPC URL with proxy
        Self::create_provider_with_proxy(&config.rpc_url, &config.rpc_proxy)?;

        if config.rpc_proxy.is_some() {
            tracing::info!("RPC proxy enabled: {}", config.rpc_proxy.as_ref().unwrap());
        }

        Ok(Self {
            rpc_settings: RwLock::new(RpcSettings {
                primary_rpc: config.rpc_url.clone(),
                fallback_rpcs: config.fallback_rpcs.clone(),
                rpc_proxy: config.rpc_proxy.clone(),
            }),
            chain_id: config.chain_id,
        })
    }

    /// Update RPC configuration dynamically (no restart required)
    pub async fn update_rpc(&self, primary_rpc: String, fallback_rpcs: Option<Vec<String>>) -> AppResult<()> {
        // Get current proxy setting
        let proxy = self.rpc_settings.read().await.rpc_proxy.clone();

        // Validate new RPC URL with proxy
        let provider = Self::create_provider_with_proxy(&primary_rpc, &proxy)?;

        // Test connection
        provider
            .get_block_number()
            .await
            .map_err(|e| AppError::BlockchainError(format!("Failed to connect to new RPC: {}", e)))?;

        // Update settings
        let mut settings = self.rpc_settings.write().await;
        settings.primary_rpc = primary_rpc.clone();
        if let Some(fallbacks) = fallback_rpcs {
            settings.fallback_rpcs = fallbacks;
        }

        tracing::info!("RPC updated dynamically to: {}", primary_rpc);
        Ok(())
    }

    /// Get current RPC URL
    pub async fn get_current_rpc(&self) -> String {
        self.rpc_settings.read().await.primary_rpc.clone()
    }

    /// Try to get a working provider, falling back to alternative RPCs if needed
    async fn get_provider(&self) -> AppResult<Arc<Provider<Http>>> {
        let settings = self.rpc_settings.read().await;
        let start = std::time::Instant::now();
        let proxy = &settings.rpc_proxy;

        tracing::debug!("Attempting to connect to primary RPC: {} (proxy: {:?})", settings.primary_rpc, proxy);

        // First try the primary RPC
        if let Ok(provider) = Self::create_provider_with_proxy(&settings.primary_rpc, proxy) {
            match provider.get_block_number().await {
                Ok(block) => {
                    let elapsed = start.elapsed().as_millis();
                    tracing::debug!(
                        "Connected to primary RPC {} (block: {}, latency: {}ms)",
                        settings.primary_rpc,
                        block,
                        elapsed
                    );
                    return Ok(Arc::new(provider));
                }
                Err(e) => {
                    tracing::warn!(
                        "Primary RPC {} unavailable: {}",
                        settings.primary_rpc,
                        e
                    );
                }
            }
        }

        // Try fallback RPCs
        for rpc_url in &settings.fallback_rpcs {
            tracing::debug!("Trying fallback RPC: {}", rpc_url);
            if let Ok(provider) = Self::create_provider_with_proxy(rpc_url, proxy) {
                match provider.get_block_number().await {
                    Ok(block) => {
                        let elapsed = start.elapsed().as_millis();
                        tracing::info!(
                            "Using fallback RPC {} (block: {}, latency: {}ms)",
                            rpc_url,
                            block,
                            elapsed
                        );
                        return Ok(Arc::new(provider));
                    }
                    Err(e) => {
                        tracing::warn!("Fallback RPC {} failed: {}", rpc_url, e);
                    }
                }
            }
        }

        tracing::error!("All RPC endpoints are unavailable");
        Err(AppError::BlockchainError(
            "All RPC endpoints are unavailable".to_string(),
        ))
    }

    fn parse_address(&self, address: &str) -> AppResult<Address> {
        address
            .parse::<Address>()
            .map_err(|e| AppError::ValidationError(format!("Invalid address: {}", e)))
    }

    fn parse_private_key(&self, key: &str) -> AppResult<LocalWallet> {
        let key = key.strip_prefix("0x").unwrap_or(key);
        key.parse::<LocalWallet>()
            .map_err(|e| AppError::ValidationError(format!("Invalid private key: {}", e)))
            .map(|w| w.with_chain_id(self.chain_id))
    }

    async fn get_erc20_balance(&self, token_address: &str, holder: &str) -> AppResult<(U256, u8)> {
        let provider = self.get_provider().await?;
        let token_addr = self.parse_address(token_address)?;
        let holder_addr = self.parse_address(holder)?;

        let contract = ERC20::new(token_addr, provider);

        let balance = contract
            .balance_of(holder_addr)
            .call()
            .await
            .map_err(|e| AppError::BlockchainError(format!("Failed to get token balance: {}", e)))?;

        let decimals = contract
            .decimals()
            .call()
            .await
            .unwrap_or(18);

        Ok((balance, decimals))
    }

    /// Calculate optimal EIP-1559 gas parameters
    /// Returns (max_fee_per_gas, max_priority_fee_per_gas) in Wei
    async fn calculate_eip1559_fees(&self, provider: &Provider<Http>) -> AppResult<(U256, U256)> {
        // Get the latest block to read base fee
        let block = provider
            .get_block(BlockNumber::Latest)
            .await
            .map_err(|e| AppError::BlockchainError(format!("Failed to get latest block: {}", e)))?
            .ok_or_else(|| AppError::BlockchainError("Latest block not found".to_string()))?;

        let base_fee = block.base_fee_per_gas
            .ok_or_else(|| AppError::BlockchainError("Base fee not available (pre-EIP1559?)".to_string()))?;

        // Get network-suggested priority fee via eth_maxPriorityFeePerGas
        // This dynamically adjusts based on network conditions
        let suggested_priority_fee = provider
            .request::<_, U256>("eth_maxPriorityFeePerGas", ())
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to get suggested priority fee: {}, using fallback", e);
                // Fallback: use 10% of base fee, minimum 0.01 Gwei, maximum 2 Gwei
                let fallback = base_fee / 10;
                let min_priority = U256::from(10_000_000u64);   // 0.01 Gwei
                let max_priority = U256::from(2_000_000_000u64); // 2 Gwei
                fallback.max(min_priority).min(max_priority)
            });

        // Apply reasonable bounds to the suggested priority fee
        // Min: 0.01 Gwei (to ensure transaction gets picked up)
        // Max: 3 Gwei (to prevent overpaying in edge cases)
        let min_priority = U256::from(10_000_000u64);    // 0.01 Gwei
        let max_priority = U256::from(3_000_000_000u64); // 3 Gwei
        let priority_fee = suggested_priority_fee.max(min_priority).min(max_priority);

        // Max fee = base_fee * 2 + priority_fee
        // This allows for base fee increases while staying economical
        let max_fee = base_fee * 2 + priority_fee;

        tracing::info!(
            "EIP-1559 fees calculated - base_fee: {} Gwei, priority_fee: {} Gwei (suggested: {} Gwei), max_fee: {} Gwei",
            format_units(base_fee, "gwei").unwrap_or_default(),
            format_units(priority_fee, "gwei").unwrap_or_default(),
            format_units(suggested_priority_fee, "gwei").unwrap_or_default(),
            format_units(max_fee, "gwei").unwrap_or_default()
        );

        Ok((max_fee, priority_fee))
    }
}

#[async_trait]
impl ChainClient for EthereumClient {
    fn chain_id(&self) -> &str {
        "ethereum"
    }

    fn chain_name(&self) -> &str {
        "Ethereum Mainnet"
    }

    fn native_token_symbol(&self) -> &str {
        "ETH"
    }

    async fn get_native_balance(&self, address: &str) -> AppResult<Decimal> {
        let start = std::time::Instant::now();
        tracing::debug!("Getting ETH balance for {}", address);

        let provider = self.get_provider().await?;
        let addr = self.parse_address(address)?;

        let balance = provider
            .get_balance(addr, None)
            .await
            .map_err(|e| AppError::BlockchainError(format!("Failed to get balance: {}", e)))?;

        let balance_str = format_units(balance, "ether")
            .map_err(|e| AppError::BlockchainError(format!("Failed to format balance: {}", e)))?;

        let result = Decimal::from_str(&balance_str)
            .map_err(|e| AppError::BlockchainError(format!("Failed to parse balance: {}", e)))?;

        tracing::debug!(
            "ETH balance for {}: {} (took {}ms)",
            address,
            result,
            start.elapsed().as_millis()
        );

        Ok(result)
    }

    async fn get_token_balance(&self, address: &str, token_symbol: &str) -> AppResult<Decimal> {
        let token_info = get_token_info(token_symbol)
            .ok_or_else(|| AppError::NotFound(format!("Token {} not supported", token_symbol)))?;

        let (balance, decimals) = self
            .get_erc20_balance(&token_info.contract_address, address)
            .await?;

        let balance_str = format_units(balance, decimals as u32)
            .map_err(|e| AppError::BlockchainError(format!("Failed to format balance: {}", e)))?;

        Decimal::from_str(&balance_str)
            .map_err(|e| AppError::BlockchainError(format!("Failed to parse balance: {}", e)))
    }

    async fn get_all_balances(&self, address: &str) -> AppResult<(Decimal, Vec<TokenBalance>)> {
        let total_start = std::time::Instant::now();
        let settings = self.rpc_settings.read().await;
        let current_rpc = settings.primary_rpc.clone();
        let proxy_info = match &settings.rpc_proxy {
            Some(p) if !p.is_empty() => format!("proxy: {}", p),
            _ => "proxy: disabled".to_string(),
        };
        drop(settings); // Release lock early

        tracing::info!(
            "Getting all balances for address: {} (RPC: {}, {})",
            address,
            current_rpc,
            proxy_info
        );

        let native_balance = self.get_native_balance(address).await?;
        tracing::info!("ETH balance: {}", native_balance);

        // Query all token balances in parallel
        let token_count = SUPPORTED_TOKENS.len();
        tracing::info!("Querying {} tokens in parallel...", token_count);

        let token_futures: Vec<_> = SUPPORTED_TOKENS
            .iter()
            .map(|(symbol, info)| {
                let symbol = symbol.clone();
                let contract_address = info.contract_address.clone();
                let address = address.to_string();
                async move {
                    let start = std::time::Instant::now();
                    let result = self.get_erc20_balance(&contract_address, &address).await;
                    let elapsed = start.elapsed().as_millis();
                    (symbol, contract_address, result, elapsed)
                }
            })
            .collect();

        let results = futures::future::join_all(token_futures).await;

        let mut token_balances = Vec::new();
        for (symbol, contract_address, result, elapsed) in results {
            match result {
                Ok((balance, decimals)) => {
                    if !balance.is_zero() {
                        let balance_str = format_units(balance, decimals as u32).unwrap_or_default();
                        if let Ok(decimal_balance) = Decimal::from_str(&balance_str) {
                            tracing::info!("{} balance: {} ({}ms)", symbol, decimal_balance, elapsed);
                            token_balances.push(TokenBalance {
                                symbol,
                                balance: decimal_balance,
                                contract_address: Some(contract_address),
                            });
                        }
                    } else {
                        tracing::debug!("{} balance: 0 ({}ms)", symbol, elapsed);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to get {} balance ({}ms): {}", symbol, elapsed, e);
                }
            }
        }

        tracing::info!(
            "Completed all balance queries for {} in {}ms (ETH: {}, tokens with balance: {})",
            address,
            total_start.elapsed().as_millis(),
            native_balance,
            token_balances.len()
        );

        Ok((native_balance, token_balances))
    }

    async fn estimate_gas(&self, params: &TransferParams) -> AppResult<GasEstimate> {
        let provider = self.get_provider().await?;
        let from = self.parse_address(&params.from_address)?;
        let to = self.parse_address(&params.to_address)?;

        // Get EIP-1559 parameters
        let block = provider
            .get_block(BlockNumber::Latest)
            .await
            .map_err(|e| AppError::BlockchainError(format!("Failed to get latest block: {}", e)))?
            .ok_or_else(|| AppError::BlockchainError("Latest block not found".to_string()))?;

        let base_fee = block.base_fee_per_gas
            .ok_or_else(|| AppError::BlockchainError("Base fee not available".to_string()))?;

        // Get network-suggested priority fee dynamically
        let suggested_priority_fee = provider
            .request::<_, U256>("eth_maxPriorityFeePerGas", ())
            .await
            .unwrap_or_else(|_| {
                // Fallback: use 10% of base fee with bounds
                let fallback = base_fee / 10;
                let min_priority = U256::from(10_000_000u64);
                let max_priority = U256::from(2_000_000_000u64);
                fallback.max(min_priority).min(max_priority)
            });

        // Apply reasonable bounds
        let min_priority = U256::from(10_000_000u64);    // 0.01 Gwei
        let max_priority = U256::from(3_000_000_000u64); // 3 Gwei
        let priority_fee = suggested_priority_fee.max(min_priority).min(max_priority);

        // Max fee = base_fee * 2 + priority_fee
        let max_fee = base_fee * 2 + priority_fee;

        let gas_limit = if params.token.to_uppercase() == "ETH" {
            // Standard ETH transfer
            U256::from(21000)
        } else {
            // ERC20 transfer - estimate or use default
            let token_info = get_token_info(&params.token)
                .ok_or_else(|| AppError::NotFound(format!("Token {} not supported", params.token)))?;

            let token_addr = self.parse_address(&token_info.contract_address)?;
            let contract = ERC20::new(token_addr, provider.clone());

            let amount = parse_units(&params.amount.to_string(), token_info.decimals as u32)
                .map_err(|e| AppError::ValidationError(format!("Invalid amount: {}", e)))?;

            contract
                .transfer(to, amount.into())
                .from(from)
                .estimate_gas()
                .await
                .unwrap_or(U256::from(100000))
        };

        // Format values
        let base_fee_gwei_str = format_units(base_fee, "gwei")
            .map_err(|e| AppError::BlockchainError(format!("Failed to format base fee: {}", e)))?;
        let priority_fee_gwei_str = format_units(priority_fee, "gwei")
            .map_err(|e| AppError::BlockchainError(format!("Failed to format priority fee: {}", e)))?;
        let max_fee_gwei_str = format_units(max_fee, "gwei")
            .map_err(|e| AppError::BlockchainError(format!("Failed to format max fee: {}", e)))?;

        // Estimated fee using max_fee (worst case scenario)
        let estimated_fee = gas_limit * max_fee;
        let fee_eth = format_units(estimated_fee, "ether")
            .map_err(|e| AppError::BlockchainError(format!("Failed to format fee: {}", e)))?;

        Ok(GasEstimate {
            gas_limit: gas_limit.as_u64(),
            gas_price_gwei: Decimal::from_str(&max_fee_gwei_str).unwrap_or_default(),
            estimated_fee_eth: Decimal::from_str(&fee_eth).unwrap_or_default(),
            base_fee_gwei: Some(Decimal::from_str(&base_fee_gwei_str).unwrap_or_default()),
            priority_fee_gwei: Some(Decimal::from_str(&priority_fee_gwei_str).unwrap_or_default()),
            max_fee_gwei: Some(Decimal::from_str(&max_fee_gwei_str).unwrap_or_default()),
        })
    }

    async fn transfer_native(&self, params: &TransferParams) -> AppResult<String> {
        let provider = self.get_provider().await?;
        let wallet = self.parse_private_key(&params.private_key)?;
        let client = SignerMiddleware::new(provider.clone(), wallet);

        let to = self.parse_address(&params.to_address)?;
        let value = parse_units(&params.amount.to_string(), "ether")
            .map_err(|e| AppError::ValidationError(format!("Invalid amount: {}", e)))?;

        // Use EIP-1559 transaction for better gas efficiency
        let (max_fee, priority_fee) = self.calculate_eip1559_fees(&provider).await?;

        let mut tx = Eip1559TransactionRequest::new()
            .to(to)
            .value(value)
            .max_fee_per_gas(max_fee)
            .max_priority_fee_per_gas(priority_fee);

        // If user specified gas_price, use it as max_fee
        if let Some(gas_price) = &params.gas_price_gwei {
            let gas_price_wei: U256 = parse_units(&gas_price.to_string(), "gwei")
                .map_err(|e| AppError::ValidationError(format!("Invalid gas price: {}", e)))?
                .into();
            // Override EIP-1559 params with user-specified max fee
            tx = tx.max_fee_per_gas(gas_price_wei).max_priority_fee_per_gas(priority_fee);
        }

        if let Some(gas_limit) = params.gas_limit {
            tx = tx.gas(gas_limit);
        }

        let pending_tx = client
            .send_transaction(tx, None)
            .await
            .map_err(|e| AppError::BlockchainError(format!("Failed to send transaction: {}", e)))?;

        let tx_hash = format!("{:?}", pending_tx.tx_hash());
        tracing::info!("ETH transfer submitted: {}", tx_hash);

        Ok(tx_hash)
    }

    async fn transfer_token(&self, params: &TransferParams) -> AppResult<String> {
        let token_info = get_token_info(&params.token)
            .ok_or_else(|| AppError::NotFound(format!("Token {} not supported", params.token)))?;

        let provider = self.get_provider().await?;
        let wallet = self.parse_private_key(&params.private_key)?;
        let client = Arc::new(SignerMiddleware::new(provider.clone(), wallet));

        let token_addr = self.parse_address(&token_info.contract_address)?;
        let to = self.parse_address(&params.to_address)?;

        let amount = parse_units(&params.amount.to_string(), token_info.decimals as u32)
            .map_err(|e| AppError::ValidationError(format!("Invalid amount: {}", e)))?;

        let contract = ERC20::new(token_addr, client);

        // Calculate optimal EIP-1559 gas parameters
        let (max_fee, priority_fee) = self.calculate_eip1559_fees(&provider).await?;

        let mut call = contract.transfer(to, amount.into());

        if let Some(gas_limit) = params.gas_limit {
            call = call.gas(gas_limit);
        }

        // Apply EIP-1559 gas settings
        if let Some(gas_price) = &params.gas_price_gwei {
            // User specified gas price - use as max_fee
            let gas_price_wei = parse_units(&gas_price.to_string(), "gwei")
                .map_err(|e| AppError::ValidationError(format!("Invalid gas price: {}", e)))?;
            call = call.gas_price(gas_price_wei);
        } else {
            // Use optimized EIP-1559 parameters
            // Note: For contract calls, we need to use legacy gas_price or build tx manually
            // Using effective gas price = max_fee for simplicity
            call = call.gas_price(max_fee);
            tracing::info!(
                "Token transfer using optimized gas - max_fee: {} Gwei, priority_fee: {} Gwei",
                format_units(max_fee, "gwei").unwrap_or_default(),
                format_units(priority_fee, "gwei").unwrap_or_default()
            );
        }

        let pending_tx = call
            .send()
            .await
            .map_err(|e| AppError::BlockchainError(format!("Failed to send token transfer: {}", e)))?;

        let tx_hash = format!("{:?}", pending_tx.tx_hash());
        tracing::info!("{} transfer submitted: {}", params.token, tx_hash);

        Ok(tx_hash)
    }

    async fn get_tx_status(&self, tx_hash: &str) -> AppResult<TxStatus> {
        let provider = self.get_provider().await?;

        let hash = tx_hash
            .parse::<H256>()
            .map_err(|e| AppError::ValidationError(format!("Invalid tx hash: {}", e)))?;

        let receipt = provider
            .get_transaction_receipt(hash)
            .await
            .map_err(|e| AppError::BlockchainError(format!("Failed to get receipt: {}", e)))?;

        match receipt {
            Some(r) => {
                if r.status == Some(U64::from(1)) {
                    Ok(TxStatus::Confirmed {
                        block_number: r.block_number.map(|b| b.as_u64()).unwrap_or(0),
                        gas_used: r.gas_used.map(|g| g.as_u64()).unwrap_or(0),
                    })
                } else {
                    Ok(TxStatus::Failed {
                        reason: "Transaction reverted".to_string(),
                    })
                }
            }
            None => {
                // Check if transaction exists but not yet mined
                let tx = provider.get_transaction(hash).await.ok().flatten();
                if tx.is_some() {
                    Ok(TxStatus::Pending)
                } else {
                    Ok(TxStatus::NotFound)
                }
            }
        }
    }

    fn validate_address(&self, address: &str) -> bool {
        address.parse::<Address>().is_ok()
    }

    async fn get_gas_price(&self) -> AppResult<Decimal> {
        let provider = self.get_provider().await?;

        let gas_price = provider
            .get_gas_price()
            .await
            .map_err(|e| AppError::BlockchainError(format!("Failed to get gas price: {}", e)))?;

        let gas_price_gwei = format_units(gas_price, "gwei")
            .map_err(|e| AppError::BlockchainError(format!("Failed to format gas price: {}", e)))?;

        Decimal::from_str(&gas_price_gwei)
            .map_err(|e| AppError::BlockchainError(format!("Failed to parse gas price: {}", e)))
    }
}
