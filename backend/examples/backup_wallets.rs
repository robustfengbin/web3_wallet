//! Wallet Backup/Restore Example
//!
//! Usage:
//!   cargo run --example backup_wallets -- backup    # Backup to wallets_backup.json
//!   cargo run --example backup_wallets -- restore   # Restore from wallets_backup.json

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::FromRow;
use std::env;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
struct Wallet {
    pub id: i32,
    pub name: String,
    pub address: String,
    pub encrypted_private_key: String,
    pub chain: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
struct WalletBackup {
    pub backup_time: DateTime<Utc>,
    pub wallets: Vec<Wallet>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("backup");

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

    println!("Connecting to database...");
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    match command {
        "backup" => {
            println!("Backing up wallets...");

            let wallets: Vec<Wallet> = sqlx::query_as(
                "SELECT id, name, address, encrypted_private_key, chain, is_active, created_at FROM wallets"
            )
            .fetch_all(&pool)
            .await?;

            let backup = WalletBackup {
                backup_time: Utc::now(),
                wallets,
            };

            let json = serde_json::to_string_pretty(&backup)?;
            fs::write("wallets_backup.json", &json)?;

            println!("Backup completed! {} wallets saved to wallets_backup.json", backup.wallets.len());

            // Print summary
            for wallet in &backup.wallets {
                println!("  - {} ({}) [{}] {}",
                    wallet.name,
                    wallet.chain,
                    if wallet.is_active { "active" } else { "inactive" },
                    wallet.address
                );
            }
        }
        "restore" => {
            println!("Restoring wallets from backup...");

            let json = fs::read_to_string("wallets_backup.json")?;
            let backup: WalletBackup = serde_json::from_str(&json)?;

            println!("Found {} wallets in backup (from {})",
                backup.wallets.len(),
                backup.backup_time
            );

            for wallet in &backup.wallets {
                // Check if wallet already exists
                let existing: Option<(i32,)> = sqlx::query_as(
                    "SELECT id FROM wallets WHERE address = ? AND chain = ?"
                )
                .bind(&wallet.address)
                .bind(&wallet.chain)
                .fetch_optional(&pool)
                .await?;

                if existing.is_some() {
                    println!("  - Skipping {} (already exists)", wallet.name);
                    continue;
                }

                sqlx::query(
                    "INSERT INTO wallets (name, address, encrypted_private_key, chain, is_active, created_at)
                     VALUES (?, ?, ?, ?, ?, ?)"
                )
                .bind(&wallet.name)
                .bind(&wallet.address)
                .bind(&wallet.encrypted_private_key)
                .bind(&wallet.chain)
                .bind(wallet.is_active)
                .bind(wallet.created_at)
                .execute(&pool)
                .await?;

                println!("  - Restored {} ({}) {}", wallet.name, wallet.chain, wallet.address);
            }

            println!("Restore completed!");
        }
        "list" => {
            println!("Current wallets in database:");

            let wallets: Vec<Wallet> = sqlx::query_as(
                "SELECT id, name, address, encrypted_private_key, chain, is_active, created_at FROM wallets"
            )
            .fetch_all(&pool)
            .await?;

            for wallet in &wallets {
                println!("  [{}] {} ({}) [{}] {}",
                    wallet.id,
                    wallet.name,
                    wallet.chain,
                    if wallet.is_active { "active" } else { "inactive" },
                    wallet.address
                );
            }
            println!("Total: {} wallets", wallets.len());
        }
        _ => {
            println!("Usage:");
            println!("  cargo run --example backup_wallets -- backup   # Backup wallets");
            println!("  cargo run --example backup_wallets -- restore  # Restore wallets");
            println!("  cargo run --example backup_wallets -- list     # List wallets");
        }
    }

    Ok(())
}
