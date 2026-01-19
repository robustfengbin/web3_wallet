pub mod models;
pub mod repositories;

use crate::config::DatabaseConfig;
use crate::error::AppResult;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::MySqlPool;

pub async fn create_pool(config: &DatabaseConfig) -> AppResult<MySqlPool> {
    use std::time::Duration;

    let url = config.url();
    tracing::info!("Connecting to database at {}:{}/{}", config.host, config.port, config.name);

    let pool = MySqlPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(2)
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .connect(&url)
        .await
        .map_err(|e| crate::error::AppError::DatabaseError(format!("Failed to connect to database: {}", e)))?;

    tracing::info!("Database connection pool created successfully (max: {}, min: 2)", config.max_connections);
    Ok(pool)
}

pub async fn run_migrations(pool: &MySqlPool) -> AppResult<()> {
    // Create tables if they don't exist
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id INT PRIMARY KEY AUTO_INCREMENT,
            username VARCHAR(50) UNIQUE NOT NULL,
            password_hash VARCHAR(255) NOT NULL,
            role VARCHAR(20) DEFAULT 'operator',
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS wallets (
            id INT PRIMARY KEY AUTO_INCREMENT,
            name VARCHAR(100) NOT NULL,
            address VARCHAR(42) NOT NULL,
            encrypted_private_key TEXT NOT NULL,
            chain VARCHAR(20) DEFAULT 'ethereum',
            is_active BOOLEAN DEFAULT FALSE,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            UNIQUE KEY unique_address_chain (address, chain)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS transfers (
            id INT PRIMARY KEY AUTO_INCREMENT,
            wallet_id INT NOT NULL,
            chain VARCHAR(20) NOT NULL,
            from_address VARCHAR(42) NOT NULL,
            to_address VARCHAR(42) NOT NULL,
            token VARCHAR(20) NOT NULL,
            amount DECIMAL(36, 18) NOT NULL,
            gas_price DECIMAL(20, 9),
            gas_limit BIGINT,
            gas_used BIGINT,
            status VARCHAR(20) DEFAULT 'pending',
            tx_hash VARCHAR(66),
            block_number BIGINT,
            error_message TEXT,
            initiated_by INT NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
            FOREIGN KEY (wallet_id) REFERENCES wallets(id),
            FOREIGN KEY (initiated_by) REFERENCES users(id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS audit_logs (
            id INT PRIMARY KEY AUTO_INCREMENT,
            user_id INT,
            action VARCHAR(100) NOT NULL,
            resource VARCHAR(100),
            details JSON,
            ip_address VARCHAR(45),
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES users(id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Settings table for storing configuration like RPC endpoints
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            `key` VARCHAR(100) PRIMARY KEY,
            `value` TEXT NOT NULL,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    tracing::info!("Database migrations completed successfully");
    Ok(())
}
