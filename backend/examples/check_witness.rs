//! Witness Diagnostic Tool
//!
//! This example checks witness data stored in the database and validates
//! if the anchor is still valid on the chain.
//!
//! Usage:
//!   cargo run --example check_witness
//!   cargo run --example check_witness -- --wallet 7
//!   cargo run --example check_witness -- --find-anchor d11cba04ae71e23872e0dd462eb4b429121ac143875f092f9e523811c80fef28

use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::FromRow;
use std::env;

/// Maximum anchor age in blocks - Zcash nodes accept anchors up to ~100 blocks old
const MAX_ANCHOR_AGE_BLOCKS: u64 = 100;

#[derive(Debug, FromRow)]
struct OrchardSyncState {
    wallet_id: i32,
    last_scanned_height: u64,
    notes_found: u32,
    last_witness_height: u64,
}

#[derive(Debug, FromRow)]
struct OrchardNote {
    id: i32,
    wallet_id: i32,
    nullifier: String,
    value_zatoshis: u64,
    block_height: u64,
    is_spent: bool,
    witness_position: Option<u64>,
    witness_root: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RpcResponse<T> {
    result: Option<T>,
    error: Option<RpcError>,
}

#[derive(Debug, Deserialize)]
struct RpcError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct TreeStateResponse {
    height: u64,
    orchard: OrchardTreeState,
}

#[derive(Debug, Deserialize)]
struct OrchardTreeState {
    commitments: OrchardCommitments,
}

#[derive(Debug, Deserialize)]
struct OrchardCommitments {
    #[serde(rename = "finalRoot")]
    final_root: String,
}

#[derive(Debug, Serialize)]
struct RpcRequest {
    jsonrpc: &'static str,
    id: &'static str,
    method: String,
    params: serde_json::Value,
}

async fn rpc_call<T: for<'de> Deserialize<'de>>(
    client: &Client,
    rpc_url: &str,
    rpc_user: Option<&str>,
    rpc_password: Option<&str>,
    method: &str,
    params: serde_json::Value,
) -> Result<T, Box<dyn std::error::Error>> {
    let request = RpcRequest {
        jsonrpc: "2.0",
        id: "1",
        method: method.to_string(),
        params,
    };

    let mut req = client.post(rpc_url).json(&request);
    if let (Some(user), Some(pass)) = (rpc_user, rpc_password) {
        req = req.basic_auth(user, Some(pass));
    }

    let response: RpcResponse<T> = req.send().await?.json().await?;
    response.result.ok_or_else(|| {
        response
            .error
            .map(|e| e.message)
            .unwrap_or_else(|| "Unknown RPC error".to_string())
            .into()
    })
}

async fn get_chain_height(
    client: &Client,
    rpc_url: &str,
    rpc_user: Option<&str>,
    rpc_password: Option<&str>,
) -> Result<u64, Box<dyn std::error::Error>> {
    rpc_call(client, rpc_url, rpc_user, rpc_password, "getblockcount", serde_json::json!([])).await
}

async fn get_tree_state(
    client: &Client,
    rpc_url: &str,
    rpc_user: Option<&str>,
    rpc_password: Option<&str>,
    height: u64,
) -> Result<TreeStateResponse, Box<dyn std::error::Error>> {
    rpc_call(
        client,
        rpc_url,
        rpc_user,
        rpc_password,
        "z_gettreestate",
        serde_json::json!([height.to_string()]),
    )
    .await
}

/// Find at which block height a given anchor was valid
async fn find_anchor_height(
    client: &Client,
    rpc_url: &str,
    rpc_user: Option<&str>,
    rpc_password: Option<&str>,
    anchor: &str,
    chain_tip: u64,
    search_depth: u64,
) -> Option<u64> {
    let start = chain_tip.saturating_sub(search_depth);
    println!("Searching for anchor {} in blocks {} to {}", anchor, start, chain_tip);

    for height in (start..=chain_tip).rev() {
        if let Ok(state) = get_tree_state(client, rpc_url, rpc_user, rpc_password, height).await {
            if state.orchard.commitments.final_root == anchor {
                return Some(height);
            }
        }
        if (chain_tip - height) % 100 == 0 {
            println!("  Checked down to height {} ...", height);
        }
    }
    None
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let args: Vec<String> = env::args().collect();
    let mut wallet_filter: Option<i32> = None;
    let mut find_anchor: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--wallet" => {
                if i + 1 < args.len() {
                    wallet_filter = Some(args[i + 1].parse()?);
                    i += 1;
                }
            }
            "--find-anchor" => {
                if i + 1 < args.len() {
                    find_anchor = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    // Build database URL from env
    let db_host = env::var("WEB3_DATABASE__HOST").unwrap_or_else(|_| "localhost".to_string());
    let db_port = env::var("WEB3_DATABASE__PORT").unwrap_or_else(|_| "3306".to_string());
    let db_user = env::var("WEB3_DATABASE__USER").unwrap_or_else(|_| "root".to_string());
    let db_pass = env::var("WEB3_DATABASE__PASSWORD").unwrap_or_else(|_| "".to_string());
    let db_name = env::var("WEB3_DATABASE__NAME").unwrap_or_else(|_| "web3_wallet".to_string());

    let database_url = format!(
        "mysql://{}:{}@{}:{}/{}",
        db_user, db_pass, db_host, db_port, db_name
    );

    // Zcash RPC config
    let rpc_url = env::var("WEB3_ZCASH__RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:8232".to_string());
    let rpc_user = env::var("WEB3_ZCASH__RPC_USER").ok();
    let rpc_password = env::var("WEB3_ZCASH__RPC_PASSWORD").ok();

    println!("========================================");
    println!("       Orchard Witness Diagnostic");
    println!("========================================\n");

    println!("Connecting to database...");
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("Connecting to Zcash RPC: {}", rpc_url);
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    // Get current chain height
    let chain_tip = get_chain_height(&client, &rpc_url, rpc_user.as_deref(), rpc_password.as_deref()).await?;
    println!("Current chain height: {}\n", chain_tip);

    // Get current anchor at chain tip
    let current_state = get_tree_state(&client, &rpc_url, rpc_user.as_deref(), rpc_password.as_deref(), chain_tip).await?;
    println!("Current Orchard anchor (at height {}): {}\n", chain_tip, current_state.orchard.commitments.final_root);

    // If --find-anchor was specified, search for it
    if let Some(anchor) = find_anchor {
        println!("========================================");
        println!("Searching for anchor in chain history...");
        println!("========================================\n");

        if let Some(height) = find_anchor_height(
            &client,
            &rpc_url,
            rpc_user.as_deref(),
            rpc_password.as_deref(),
            &anchor,
            chain_tip,
            500, // Search last 500 blocks
        ).await {
            let age = chain_tip - height;
            println!("\n✅ Found anchor at height {}", height);
            println!("   Age: {} blocks behind chain tip", age);
            if age > MAX_ANCHOR_AGE_BLOCKS {
                println!("   ⚠️  EXPIRED! Anchor is {} blocks old (max allowed: {})", age, MAX_ANCHOR_AGE_BLOCKS);
            } else {
                println!("   ✅ VALID! Anchor is within acceptable range");
            }
        } else {
            println!("\n❌ Anchor NOT FOUND in last 500 blocks!");
            println!("   The anchor may be too old or invalid.");
        }
        return Ok(());
    }

    // Query sync state
    println!("========================================");
    println!("Orchard Sync State (orchard_sync_state)");
    println!("========================================\n");

    let sync_states: Vec<OrchardSyncState> = if let Some(wid) = wallet_filter {
        sqlx::query_as(
            "SELECT wallet_id, last_scanned_height, notes_found, last_witness_height FROM orchard_sync_state WHERE wallet_id = ?"
        )
        .bind(wid)
        .fetch_all(&pool)
        .await?
    } else {
        sqlx::query_as(
            "SELECT wallet_id, last_scanned_height, notes_found, last_witness_height FROM orchard_sync_state ORDER BY wallet_id"
        )
        .fetch_all(&pool)
        .await?
    };

    if sync_states.is_empty() {
        println!("No sync state records found.\n");
    } else {
        println!("{:<10} {:>15} {:>12} {:>18} {:>15}", "Wallet ID", "Scanned Height", "Notes Found", "Witness Height", "Behind Chain");
        println!("{}", "-".repeat(75));

        for state in &sync_states {
            let behind = chain_tip.saturating_sub(state.last_witness_height);
            let status = if behind > MAX_ANCHOR_AGE_BLOCKS {
                format!("⚠️  {} (STALE!)", behind)
            } else if behind > 50 {
                format!("⚡ {} (needs sync)", behind)
            } else {
                format!("✅ {}", behind)
            };
            println!(
                "{:<10} {:>15} {:>12} {:>18} {:>15}",
                state.wallet_id,
                state.last_scanned_height,
                state.notes_found,
                state.last_witness_height,
                status
            );
        }
        println!();
    }

    // Query notes with witness data
    println!("========================================");
    println!("Orchard Notes with Witness Data");
    println!("========================================\n");

    let notes: Vec<OrchardNote> = if let Some(wid) = wallet_filter {
        sqlx::query_as(
            "SELECT id, wallet_id, nullifier, value_zatoshis, block_height, is_spent, witness_position, witness_root
             FROM orchard_notes WHERE wallet_id = ? AND is_spent = FALSE ORDER BY block_height"
        )
        .bind(wid)
        .fetch_all(&pool)
        .await?
    } else {
        sqlx::query_as(
            "SELECT id, wallet_id, nullifier, value_zatoshis, block_height, is_spent, witness_position, witness_root
             FROM orchard_notes WHERE is_spent = FALSE ORDER BY wallet_id, block_height"
        )
        .fetch_all(&pool)
        .await?
    };

    if notes.is_empty() {
        println!("No unspent notes found.\n");
    } else {
        println!("{:<6} {:>8} {:>15} {:>12} {:>10} {:>66}", "ID", "Wallet", "Value (ZAT)", "Note Height", "Position", "Witness Root");
        println!("{}", "-".repeat(130));

        for note in &notes {
            let position_str = note.witness_position.map(|p| p.to_string()).unwrap_or_else(|| "N/A".to_string());
            let root_str = note.witness_root.clone().unwrap_or_else(|| "N/A".to_string());

            println!(
                "{:<6} {:>8} {:>15} {:>12} {:>10} {:>66}",
                note.id,
                note.wallet_id,
                note.value_zatoshis,
                note.block_height,
                position_str,
                &root_str[..std::cmp::min(64, root_str.len())]
            );
        }
        println!();
    }

    // Check anchor validity for each unique witness_root
    println!("========================================");
    println!("Anchor Validity Check");
    println!("========================================\n");

    let unique_roots: Vec<String> = notes
        .iter()
        .filter_map(|n| n.witness_root.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    if unique_roots.is_empty() {
        println!("No witness roots to check.\n");
    } else {
        println!("Checking {} unique anchor(s) against chain...\n", unique_roots.len());

        for root in &unique_roots {
            print!("Anchor: {}... ", &root[..16]);

            // Search for this anchor in recent blocks
            if let Some(height) = find_anchor_height(
                &client,
                &rpc_url,
                rpc_user.as_deref(),
                rpc_password.as_deref(),
                root,
                chain_tip,
                200, // Search last 200 blocks
            ).await {
                let age = chain_tip - height;
                if age > MAX_ANCHOR_AGE_BLOCKS {
                    println!("❌ EXPIRED at height {} ({} blocks old, max={})", height, age, MAX_ANCHOR_AGE_BLOCKS);
                } else {
                    println!("✅ Valid at height {} ({} blocks old)", height, age);
                }
            } else {
                println!("❌ NOT FOUND in last 200 blocks - possibly too old!");
            }
        }
    }

    println!("\n========================================");
    println!("Summary");
    println!("========================================\n");

    let min_witness_height = sync_states.iter().map(|s| s.last_witness_height).min().unwrap_or(0);
    let blocks_behind = chain_tip.saturating_sub(min_witness_height);

    println!("Chain tip:           {}", chain_tip);
    println!("Min witness height:  {}", min_witness_height);
    println!("Blocks behind:       {}", blocks_behind);
    println!();

    if blocks_behind > MAX_ANCHOR_AGE_BLOCKS {
        println!("⚠️  WARNING: Witnesses are {} blocks behind chain tip!", blocks_behind);
        println!("   The witness data is STALE and anchors will be rejected by the network.");
        println!();
        println!("   Possible causes:");
        println!("   1. The sync service hasn't been running for a while");
        println!("   2. The WITNESS_SYNC_THRESHOLD (50) delayed persistence");
        println!("   3. refresh_witnesses_for_spending() didn't refresh properly before transfer");
        println!();
        println!("   To fix:");
        println!("   1. Ensure the sync service is running");
        println!("   2. Call refresh_witnesses_for_spending() before any transfer");
        println!("   3. Check if the tree state in memory is valid");
    } else if blocks_behind > 50 {
        println!("⚡ NOTICE: Witnesses are {} blocks behind.", blocks_behind);
        println!("   This is within limits but refresh_witnesses_for_spending() should update them.");
    } else {
        println!("✅ Witnesses are up to date ({} blocks behind)", blocks_behind);
    }

    Ok(())
}
