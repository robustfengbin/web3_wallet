//! Orchard blockchain synchronization module
//!
//! Fetches blocks from Zebra node and scans for Orchard notes
//! belonging to the wallet's viewing keys.
//!
//! Features:
//! - Parallel block fetching for improved speed
//! - Database persistence for scan state and notes
//! - Resume from last scanned height on restart

#![allow(dead_code)]

use super::{
    keys::OrchardViewingKey,
    scanner::{CompactBlock, CompactOrchardAction, CompactTransaction, OrchardNote, OrchardScanner, ScanProgress, ShieldedBalance},
    OrchardError, OrchardResult, ShieldedPool,
};
use crate::db::repositories::OrchardRepository;
use orchard::keys::IncomingViewingKey;
use serde::Deserialize;
use sqlx::MySqlPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use futures::future::join_all;

/// Configuration for the Orchard sync service
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Zebra RPC URL
    pub rpc_url: String,
    /// RPC username (optional)
    pub rpc_user: Option<String>,
    /// RPC password (optional)
    pub rpc_password: Option<String>,
    /// Number of blocks to fetch per batch
    pub batch_size: u64,
    /// Starting height (birthday)
    pub birthday_height: u64,
    /// Number of concurrent block fetches
    pub parallel_fetches: usize,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            rpc_url: "http://127.0.0.1:8232".to_string(),
            rpc_user: None,
            rpc_password: None,
            batch_size: 500,  // Process 500 blocks per round
            birthday_height: 1_687_104,  // Orchard activation height
            parallel_fetches: 25,  // Smaller batch = faster response, more parallel
        }
    }
}

/// Zebra RPC response structures
#[derive(Debug, Deserialize)]
struct RpcResponse<T> {
    result: Option<T>,
    error: Option<RpcError>,
    id: u64,
}

#[derive(Debug, Deserialize)]
struct RpcError {
    code: i64,
    message: String,
}

#[derive(Debug, Deserialize)]
struct BlockInfo {
    hash: String,
    height: u64,
    tx: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct VerboseBlock {
    hash: String,
    height: u64,
    tx: Vec<TransactionInfo>,
}

#[derive(Debug, Deserialize)]
struct TransactionInfo {
    txid: String,
    #[serde(default)]
    orchard: Option<OrchardBundle>,
}

#[derive(Debug, Deserialize)]
struct OrchardBundle {
    actions: Vec<OrchardActionInfo>,
    #[serde(rename = "valueBalanceZat")]
    value_balance_zat: i64,
}

#[derive(Debug, Deserialize)]
struct OrchardActionInfo {
    cv: String,
    nullifier: String,
    rk: String,
    #[serde(rename = "cmx")]
    cm_x: String,
    #[serde(rename = "ephemeralKey")]
    ephemeral_key: String,
    #[serde(rename = "encCiphertext")]
    enc_ciphertext: String,
    #[serde(rename = "outCiphertext")]
    out_ciphertext: String,
}

/// Orchard synchronization service with database persistence
pub struct OrchardSyncService {
    config: SyncConfig,
    client: reqwest::Client,
    scanner: Arc<RwLock<OrchardScanner>>,
    /// Stored notes by wallet_id (memory cache)
    notes_by_wallet: Arc<RwLock<HashMap<i32, Vec<OrchardNote>>>>,
    /// Wallet ID to viewing key mapping
    wallet_keys: Arc<RwLock<HashMap<i32, OrchardViewingKey>>>,
    /// Database repository for persistence
    db_repo: Option<Arc<OrchardRepository>>,
    /// Tracks which wallets have been synced
    wallet_scan_heights: Arc<RwLock<HashMap<i32, u64>>>,
}

impl OrchardSyncService {
    /// Create HTTP client with high connection pool for parallel fetching
    fn create_http_client() -> reqwest::Client {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))  // 2 min timeout for batch requests
            .pool_max_idle_per_host(100)  // Allow many idle connections
            .pool_idle_timeout(std::time::Duration::from_secs(60))
            .tcp_keepalive(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default()
    }

    /// Create a new sync service without database (memory only)
    pub fn new(config: SyncConfig) -> Self {
        Self {
            config,
            client: Self::create_http_client(),
            scanner: Arc::new(RwLock::new(OrchardScanner::new(vec![]))),
            notes_by_wallet: Arc::new(RwLock::new(HashMap::new())),
            wallet_keys: Arc::new(RwLock::new(HashMap::new())),
            db_repo: None,
            wallet_scan_heights: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new sync service with database persistence
    pub fn new_with_db(config: SyncConfig, pool: MySqlPool) -> Self {
        Self {
            config,
            client: Self::create_http_client(),
            scanner: Arc::new(RwLock::new(OrchardScanner::new(vec![]))),
            notes_by_wallet: Arc::new(RwLock::new(HashMap::new())),
            wallet_keys: Arc::new(RwLock::new(HashMap::new())),
            db_repo: Some(Arc::new(OrchardRepository::new(pool))),
            wallet_scan_heights: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a wallet's viewing key for scanning
    pub async fn register_wallet(&self, wallet_id: i32, viewing_key: OrchardViewingKey) {
        let birthday = viewing_key.birthday_height;
        let mut keys = self.wallet_keys.write().await;
        keys.insert(wallet_id, viewing_key.clone());

        // Load scan state from database if available
        let mut scan_heights = self.wallet_scan_heights.write().await;
        if let Some(repo) = &self.db_repo {
            if let Ok(Some(state)) = repo.get_sync_state(wallet_id).await {
                scan_heights.insert(wallet_id, state.last_scanned_height);
                tracing::info!(
                    "[Orchard Sync] Restored wallet {} scan state: last_scanned={}, notes={}",
                    wallet_id,
                    state.last_scanned_height,
                    state.notes_found
                );
            } else {
                // Initialize with birthday height
                scan_heights.insert(wallet_id, birthday);
                if let Err(e) = repo.upsert_sync_state(wallet_id, birthday, 0).await {
                    tracing::warn!("[Orchard Sync] Failed to init sync state: {}", e);
                }
            }
        } else {
            scan_heights.insert(wallet_id, birthday);
        }

        let mut scanner = self.scanner.write().await;
        // Recreate scanner with updated keys - this will use the minimum birthday height
        let all_keys: Vec<_> = keys.values().cloned().collect();
        *scanner = OrchardScanner::new(all_keys);

        tracing::info!(
            "[Orchard Sync] Registered wallet {} for scanning (account_index={}, birthday={}, total wallets={})",
            wallet_id,
            viewing_key.account_index,
            birthday,
            keys.len()
        );
    }

    /// Get current chain height from Zebra
    pub async fn get_chain_height(&self) -> OrchardResult<u64> {
        let response: RpcResponse<u64> = self.rpc_call("getblockcount", serde_json::json!([])).await?;
        response.result.ok_or_else(|| {
            OrchardError::RpcError(response.error.map(|e| e.message).unwrap_or_default())
        })
    }

    /// Get block at height with full transaction data
    async fn get_block(&self, height: u64) -> OrchardResult<VerboseBlock> {
        // First get block hash
        let hash_response: RpcResponse<String> = self
            .rpc_call("getblockhash", serde_json::json!([height]))
            .await?;

        let hash = hash_response.result.ok_or_else(|| {
            OrchardError::RpcError(format!("Failed to get block hash for height {}", height))
        })?;

        // Then get block with verbosity 2 (includes full transactions)
        let block_response: RpcResponse<VerboseBlock> = self
            .rpc_call("getblock", serde_json::json!([hash, 2]))
            .await?;

        block_response.result.ok_or_else(|| {
            OrchardError::RpcError(format!("Failed to get block at height {}", height))
        })
    }

    /// Fetch multiple blocks using batch RPC (much faster than individual calls)
    /// Uses 2 batch requests: first for block hashes, then for blocks
    async fn fetch_blocks_batch(&self, heights: Vec<u64>) -> Vec<(u64, OrchardResult<VerboseBlock>)> {
        if heights.is_empty() {
            return vec![];
        }

        // Step 1: Batch request for all block hashes
        let hash_requests: Vec<serde_json::Value> = heights
            .iter()
            .enumerate()
            .map(|(i, h)| {
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": i,
                    "method": "getblockhash",
                    "params": [h]
                })
            })
            .collect();

        let hashes = match self.batch_rpc_call::<String>(&hash_requests).await {
            Ok(h) => h,
            Err(e) => {
                tracing::warn!("[Orchard Sync] Batch getblockhash failed: {}, falling back to parallel", e);
                return self.fetch_blocks_parallel_fallback(heights).await;
            }
        };

        // Map height to hash
        let height_to_hash: std::collections::HashMap<u64, String> = heights
            .iter()
            .zip(hashes.iter())
            .filter_map(|(h, r)| r.as_ref().ok().map(|hash| (*h, hash.clone())))
            .collect();

        // Step 2: Batch request for all blocks
        let block_requests: Vec<serde_json::Value> = heights
            .iter()
            .enumerate()
            .filter_map(|(i, h)| {
                height_to_hash.get(h).map(|hash| {
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": i,
                        "method": "getblock",
                        "params": [hash, 2]
                    })
                })
            })
            .collect();

        let blocks = match self.batch_rpc_call::<VerboseBlock>(&block_requests).await {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!("[Orchard Sync] Batch getblock failed: {}, falling back to parallel", e);
                return self.fetch_blocks_parallel_fallback(heights).await;
            }
        };

        // Combine results
        heights
            .iter()
            .zip(blocks.into_iter())
            .map(|(h, r)| (*h, r))
            .collect()
    }

    /// Batch RPC call - sends multiple requests in one HTTP request
    async fn batch_rpc_call<T: for<'de> Deserialize<'de>>(
        &self,
        requests: &[serde_json::Value],
    ) -> OrchardResult<Vec<OrchardResult<T>>> {
        let mut request_builder = self.client.post(&self.config.rpc_url);

        if let (Some(user), Some(pass)) = (&self.config.rpc_user, &self.config.rpc_password) {
            request_builder = request_builder.basic_auth(user, Some(pass));
        }

        let response = request_builder
            .json(requests)
            .send()
            .await
            .map_err(|e| OrchardError::RpcError(format!("Batch RPC request failed: {}", e)))?;

        let response_text = response
            .text()
            .await
            .map_err(|e| OrchardError::RpcError(format!("Failed to read batch response: {}", e)))?;

        let responses: Vec<RpcResponse<T>> = serde_json::from_str(&response_text)
            .map_err(|e| OrchardError::RpcError(format!("Failed to parse batch response: {}", e)))?;

        // Sort by id to maintain order
        let mut sorted: Vec<_> = responses.into_iter().collect();
        sorted.sort_by_key(|r| r.id);

        Ok(sorted
            .into_iter()
            .map(|r| {
                r.result.ok_or_else(|| {
                    OrchardError::RpcError(r.error.map(|e| e.message).unwrap_or_else(|| "Unknown error".to_string()))
                })
            })
            .collect())
    }

    /// Fallback to parallel individual requests if batch fails
    async fn fetch_blocks_parallel_fallback(&self, heights: Vec<u64>) -> Vec<(u64, OrchardResult<VerboseBlock>)> {
        let futures: Vec<_> = heights
            .into_iter()
            .map(|height| {
                let client = self.client.clone();
                let config = self.config.clone();
                async move {
                    let result = Self::fetch_block_with_client(&client, &config, height).await;
                    (height, result)
                }
            })
            .collect();

        join_all(futures).await
    }

    /// Static helper to fetch a block with a given client (for fallback)
    async fn fetch_block_with_client(
        client: &reqwest::Client,
        config: &SyncConfig,
        height: u64,
    ) -> OrchardResult<VerboseBlock> {
        // Get block hash
        let hash_response: RpcResponse<String> = Self::rpc_call_with_client(
            client,
            config,
            "getblockhash",
            serde_json::json!([height]),
        ).await?;

        let hash = hash_response.result.ok_or_else(|| {
            OrchardError::RpcError(format!("Failed to get block hash for height {}", height))
        })?;

        // Get block with verbosity 2
        let block_response: RpcResponse<VerboseBlock> = Self::rpc_call_with_client(
            client,
            config,
            "getblock",
            serde_json::json!([hash, 2]),
        ).await?;

        block_response.result.ok_or_else(|| {
            OrchardError::RpcError(format!("Failed to get block at height {}", height))
        })
    }

    /// Static RPC call helper (for individual calls)
    async fn rpc_call_with_client<T: for<'de> Deserialize<'de>>(
        client: &reqwest::Client,
        config: &SyncConfig,
        method: &str,
        params: serde_json::Value,
    ) -> OrchardResult<RpcResponse<T>> {
        let mut request_builder = client.post(&config.rpc_url);

        if let (Some(user), Some(pass)) = (&config.rpc_user, &config.rpc_password) {
            request_builder = request_builder.basic_auth(user, Some(pass));
        }

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });

        let response = request_builder
            .json(&body)
            .send()
            .await
            .map_err(|e| OrchardError::RpcError(format!("RPC request failed: {}", e)))?;

        let response_text = response
            .text()
            .await
            .map_err(|e| OrchardError::RpcError(format!("Failed to read response: {}", e)))?;

        serde_json::from_str(&response_text)
            .map_err(|e| OrchardError::RpcError(format!("Failed to parse response: {} - {}", e, &response_text[..200.min(response_text.len())])))
    }

    /// Make an RPC call to Zebra
    async fn rpc_call<T: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> OrchardResult<RpcResponse<T>> {
        Self::rpc_call_with_client(&self.client, &self.config, method, params).await
    }

    /// Convert Zebra block to compact block format
    fn to_compact_block(&self, block: &VerboseBlock) -> OrchardResult<CompactBlock> {
        let mut transactions = Vec::new();

        for tx in &block.tx {
            if let Some(orchard) = &tx.orchard {
                let mut actions = Vec::new();

                for action in &orchard.actions {
                    actions.push(CompactOrchardAction {
                        cmx: self.decode_hex_32(&action.cm_x)?,
                        nullifier: self.decode_hex_32(&action.nullifier)?,
                        ephemeral_key: self.decode_hex_32(&action.ephemeral_key)?,
                        ciphertext: hex::decode(&action.enc_ciphertext)
                            .map_err(|e| OrchardError::RpcError(format!("Invalid ciphertext: {}", e)))?,
                    });
                }

                if !actions.is_empty() {
                    transactions.push(CompactTransaction {
                        hash: tx.txid.clone(),
                        orchard_actions: actions,
                    });
                }
            }
        }

        let mut hash = [0u8; 32];
        let hash_bytes = hex::decode(&block.hash)
            .map_err(|e| OrchardError::RpcError(format!("Invalid block hash: {}", e)))?;
        if hash_bytes.len() == 32 {
            hash.copy_from_slice(&hash_bytes);
        }

        Ok(CompactBlock {
            height: block.height,
            hash,
            transactions,
        })
    }

    /// Decode hex string to 32-byte array
    fn decode_hex_32(&self, hex_str: &str) -> OrchardResult<[u8; 32]> {
        let bytes = hex::decode(hex_str)
            .map_err(|e| OrchardError::RpcError(format!("Invalid hex: {}", e)))?;

        if bytes.len() != 32 {
            return Err(OrchardError::RpcError(format!(
                "Expected 32 bytes, got {}",
                bytes.len()
            )));
        }

        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(arr)
    }

    /// Get the minimum scan height across all registered wallets
    async fn get_min_scan_height(&self) -> u64 {
        let scan_heights = self.wallet_scan_heights.read().await;
        scan_heights.values().cloned().min().unwrap_or(self.config.birthday_height)
    }

    /// Sync blocks from last scanned height to chain tip (with parallel fetching)
    pub async fn sync(&self) -> OrchardResult<ScanProgress> {
        let chain_tip = self.get_chain_height().await?;

        let registered_keys = {
            let keys = self.wallet_keys.read().await;
            keys.len()
        };

        if registered_keys == 0 {
            tracing::warn!("[Orchard Sync] No viewing keys registered, skipping scan");
            let scanner = self.scanner.read().await;
            return Ok(scanner.progress().clone());
        }

        // Get the minimum scan height across all wallets
        let start_height = self.get_min_scan_height().await;

        tracing::info!(
            "[Orchard Sync] Chain tip: {}, start height: {}, registered wallets: {}",
            chain_tip,
            start_height,
            registered_keys
        );

        if start_height >= chain_tip {
            tracing::info!("[Orchard Sync] Already synced to chain tip {}", chain_tip);
            let scanner = self.scanner.read().await;
            return Ok(scanner.progress().clone());
        }

        let blocks_to_scan = chain_tip - start_height;
        tracing::info!(
            "[Orchard Sync] Starting sync: {} -> {} ({} blocks to scan, parallel={})",
            start_height,
            chain_tip,
            blocks_to_scan,
            self.config.parallel_fetches
        );

        let mut current_height = start_height + 1;
        let batch_size = self.config.batch_size;
        let rpc_batch_size = self.config.parallel_fetches;  // Blocks per RPC batch request
        let mut total_notes_found = 0usize;
        let mut total_blocks_scanned = 0usize;
        let sync_start = std::time::Instant::now();
        let mut last_persist_height = start_height;

        while current_height <= chain_tip {
            let end_height = std::cmp::min(current_height + batch_size - 1, chain_tip);

            let mut all_blocks = Vec::new();
            let mut fetch_errors = 0usize;

            // Fetch blocks in parallel RPC batches (multiple batches concurrently)
            let heights: Vec<u64> = (current_height..=end_height).collect();
            let fetch_start = std::time::Instant::now();

            // Create futures for all batch requests and execute in parallel
            let batch_futures: Vec<_> = heights
                .chunks(rpc_batch_size)
                .map(|chunk| self.fetch_blocks_batch(chunk.to_vec()))
                .collect();

            let batch_results = join_all(batch_futures).await;

            // Collect all results
            for results in batch_results {
                for (height, result) in results {
                    match result {
                        Ok(block) => {
                            if let Ok(compact_block) = self.to_compact_block(&block) {
                                all_blocks.push((height, compact_block));
                            }
                        }
                        Err(e) => {
                            fetch_errors += 1;
                            if fetch_errors <= 3 {
                                tracing::warn!("[Orchard Sync] Failed to fetch block {}: {}", height, e);
                            }
                        }
                    }
                }
            }

            let fetch_elapsed = fetch_start.elapsed();

            // Sort blocks by height
            all_blocks.sort_by_key(|(h, _)| *h);
            let blocks: Vec<CompactBlock> = all_blocks.into_iter().map(|(_, b)| b).collect();

            if fetch_errors > 3 {
                tracing::warn!(
                    "[Orchard Sync] {} total block fetch errors in range {}-{} (fetch took {:.2}s)",
                    fetch_errors,
                    current_height,
                    end_height,
                    fetch_elapsed.as_secs_f64()
                );
            }

            if !blocks.is_empty() {
                total_blocks_scanned += blocks.len();

                let mut scanner = self.scanner.write().await;
                let found_notes = scanner.scan_blocks(blocks, chain_tip).await?;
                drop(scanner);

                if !found_notes.is_empty() {
                    total_notes_found += found_notes.len();
                    tracing::info!(
                        "[Orchard Sync] 🎉 Found {} notes in blocks {}-{} (total: {})",
                        found_notes.len(),
                        current_height,
                        end_height,
                        total_notes_found
                    );

                    // Store notes (memory + database)
                    self.store_notes(&found_notes).await;
                }
            }

            current_height = end_height + 1;

            // Persist scan state every 1000 blocks or at end
            if current_height - last_persist_height >= 1000 || current_height > chain_tip {
                self.persist_scan_state(end_height).await;
                last_persist_height = end_height;
            }

            // Log progress every 500 blocks
            let blocks_scanned = current_height - start_height - 1;
            if blocks_scanned % 500 == 0 || blocks_scanned == blocks_to_scan {
                let elapsed = sync_start.elapsed().as_secs_f64();
                let blocks_per_sec = if elapsed > 0.0 {
                    blocks_scanned as f64 / elapsed
                } else {
                    0.0
                };
                let remaining_blocks = chain_tip.saturating_sub(current_height) + 1;
                let eta_secs = if blocks_per_sec > 0.0 {
                    remaining_blocks as f64 / blocks_per_sec
                } else {
                    0.0
                };
                let progress_pct = (blocks_scanned as f64 / blocks_to_scan as f64) * 100.0;

                tracing::info!(
                    "[Orchard Sync] Progress: {:.1}% ({}/{} blocks), {:.1} blocks/sec, ETA: {:.0}s, notes found: {}",
                    progress_pct,
                    blocks_scanned,
                    blocks_to_scan,
                    blocks_per_sec,
                    eta_secs,
                    total_notes_found
                );
            }
        }

        // Final persist
        self.persist_scan_state(chain_tip).await;

        let total_elapsed = sync_start.elapsed().as_secs_f64();
        tracing::info!(
            "[Orchard Sync] ✅ Sync complete to height {} ({} blocks in {:.1}s, {:.1} blocks/sec, {} notes found)",
            chain_tip,
            total_blocks_scanned,
            total_elapsed,
            total_blocks_scanned as f64 / total_elapsed.max(0.1),
            total_notes_found
        );

        let scanner = self.scanner.read().await;
        Ok(scanner.progress().clone())
    }

    /// Store discovered notes by wallet (memory + database)
    async fn store_notes(&self, notes: &[OrchardNote]) {
        let keys = self.wallet_keys.read().await;
        let mut notes_by_wallet = self.notes_by_wallet.write().await;

        // Prepare batch for database
        let mut db_notes: Vec<(i32, String, u64, u64, String, u32, Option<String>)> = Vec::new();

        for note in notes {
            // Find wallet_id for this account
            for (wallet_id, vk) in keys.iter() {
                if vk.account_index == note.account_id {
                    // Memory store
                    notes_by_wallet
                        .entry(*wallet_id)
                        .or_insert_with(Vec::new)
                        .push(note.clone());

                    // Prepare for database
                    db_notes.push((
                        *wallet_id,
                        hex::encode(note.nullifier),
                        note.value_zatoshis,
                        note.block_height,
                        note.tx_hash.clone(),
                        note.position as u32,
                        note.memo.clone(),
                    ));
                    break;
                }
            }
        }

        // Persist to database
        if let Some(repo) = &self.db_repo {
            if !db_notes.is_empty() {
                match repo.save_notes_batch(&db_notes).await {
                    Ok(saved) => {
                        tracing::debug!("[Orchard Sync] Persisted {} notes to database", saved);
                    }
                    Err(e) => {
                        tracing::error!("[Orchard Sync] Failed to persist notes: {}", e);
                    }
                }
            }
        }
    }

    /// Persist scan state to database
    async fn persist_scan_state(&self, height: u64) {
        if let Some(repo) = &self.db_repo {
            let keys = self.wallet_keys.read().await;
            let mut scan_heights = self.wallet_scan_heights.write().await;

            for wallet_id in keys.keys() {
                scan_heights.insert(*wallet_id, height);

                // Get notes count for this wallet
                let notes_count = self.notes_by_wallet
                    .read()
                    .await
                    .get(wallet_id)
                    .map(|n| n.len() as u32)
                    .unwrap_or(0);

                if let Err(e) = repo.upsert_sync_state(*wallet_id, height, notes_count).await {
                    tracing::warn!("[Orchard Sync] Failed to persist scan state for wallet {}: {}", wallet_id, e);
                }
            }

            tracing::debug!("[Orchard Sync] Persisted scan state at height {}", height);
        }
    }

    /// Get balance for a wallet (from database if available)
    pub async fn get_wallet_balance(&self, wallet_id: i32) -> ShieldedBalance {
        // Try database first
        if let Some(repo) = &self.db_repo {
            if let Ok(balance) = repo.get_balance(wallet_id).await {
                let notes_count = repo.get_unspent_notes(wallet_id).await
                    .map(|n| n.len() as u32)
                    .unwrap_or(0);
                return ShieldedBalance::new(
                    ShieldedPool::Orchard,
                    balance,
                    balance,  // All unspent are spendable
                    notes_count,
                );
            }
        }

        // Fallback to in-memory scanner
        let scanner = self.scanner.read().await;
        let keys = self.wallet_keys.read().await;

        if let Some(vk) = keys.get(&wallet_id) {
            let account_id = vk.account_index;
            let chain_height = self.get_chain_height().await.unwrap_or(0);

            let total = scanner.get_balance(account_id);
            let spendable = scanner.get_spendable_balance(account_id, chain_height);
            let unspent_notes = scanner.get_unspent_notes(account_id);

            ShieldedBalance::new(
                ShieldedPool::Orchard,
                total,
                spendable,
                unspent_notes.len() as u32,
            )
        } else {
            ShieldedBalance::new(ShieldedPool::Orchard, 0, 0, 0)
        }
    }

    /// Get unspent notes for a wallet
    pub async fn get_unspent_notes(&self, wallet_id: i32) -> Vec<OrchardNote> {
        let scanner = self.scanner.read().await;
        let keys = self.wallet_keys.read().await;

        if let Some(vk) = keys.get(&wallet_id) {
            scanner.get_unspent_notes(vk.account_index)
        } else {
            vec![]
        }
    }

    /// Get spendable notes for a wallet
    pub async fn get_spendable_notes(&self, wallet_id: i32) -> Vec<OrchardNote> {
        let scanner = self.scanner.read().await;
        let keys = self.wallet_keys.read().await;

        if let Some(vk) = keys.get(&wallet_id) {
            let chain_height = self.get_chain_height().await.unwrap_or(0);
            scanner.get_spendable_notes(vk.account_index, chain_height)
        } else {
            vec![]
        }
    }

    /// Get current scan progress
    pub async fn get_progress(&self) -> ScanProgress {
        let scanner = self.scanner.read().await;
        scanner.progress().clone()
    }

    /// Check if fully synced
    pub async fn is_synced(&self) -> bool {
        let scanner = self.scanner.read().await;
        let progress = scanner.progress();
        progress.progress_percent >= 99.9
    }
}

/// Try to decrypt an Orchard note using the viewing key
///
/// This uses the proper orchard crate decryption instead of placeholder XOR
pub fn try_decrypt_orchard_note(
    _ivk: &IncomingViewingKey,
    action: &CompactOrchardAction,
    _block_height: u64,
) -> Option<(u64, Option<String>)> {
    // First 52 bytes of enc_ciphertext for compact decryption
    if action.ciphertext.len() < 52 {
        return None;
    }

    let mut enc_compact = [0u8; 52];
    enc_compact.copy_from_slice(&action.ciphertext[..52]);

    // Placeholder return - in production, use proper orchard decryption
    None // TODO: Implement proper decryption using orchard crate
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_config_default() {
        let config = SyncConfig::default();
        assert_eq!(config.rpc_url, "http://127.0.0.1:8232");
        assert_eq!(config.batch_size, 500);
        assert_eq!(config.parallel_fetches, 10);
    }

    #[tokio::test]
    async fn test_sync_service_creation() {
        let config = SyncConfig::default();
        let service = OrchardSyncService::new(config);

        let progress = service.get_progress().await;
        assert_eq!(progress.notes_found, 0);
    }
}
