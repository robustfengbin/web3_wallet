//! Orchard notes and sync state repository
//!
//! Provides persistence for Orchard scan state and discovered notes.
//! Some methods are reserved for future use (e.g., mark_note_spent).

#![allow(dead_code)]

use crate::error::AppResult;
use sqlx::MySqlPool;

/// Stored Orchard note from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StoredOrchardNote {
    pub id: i32,
    pub wallet_id: i32,
    pub nullifier: String,
    pub value_zatoshis: u64,
    pub block_height: u64,
    pub tx_hash: String,
    pub position_in_block: u32,
    pub is_spent: bool,
    pub spent_in_tx: Option<String>,
    pub memo: Option<String>,
    // Spending data (for shielded-to-shielded transfers)
    pub recipient: Option<String>,  // Hex-encoded 43 bytes
    pub rho: Option<String>,        // Hex-encoded 32 bytes
    pub rseed: Option<String>,      // Hex-encoded 32 bytes
    // Witness data for spending (JSON-serialized)
    pub witness_position: Option<u64>,
    pub witness_auth_path: Option<String>, // JSON array of hex-encoded 32-byte hashes
    pub witness_root: Option<String>,      // Hex-encoded 32-byte root
}

/// Sync state for a wallet
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OrchardSyncState {
    pub wallet_id: i32,
    pub last_scanned_height: u64,
    pub notes_found: u32,
}

pub struct OrchardRepository {
    pool: MySqlPool,
}

impl OrchardRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    // =========================================================================
    // Sync State Operations
    // =========================================================================

    /// Get sync state for a wallet
    pub async fn get_sync_state(&self, wallet_id: i32) -> AppResult<Option<OrchardSyncState>> {
        let state = sqlx::query_as::<_, OrchardSyncState>(
            "SELECT wallet_id, last_scanned_height, notes_found FROM orchard_sync_state WHERE wallet_id = ?"
        )
        .bind(wallet_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(state)
    }

    /// Get minimum last_scanned_height across all wallets (for global sync)
    pub async fn get_min_scanned_height(&self) -> AppResult<u64> {
        let result: Option<(u64,)> = sqlx::query_as(
            "SELECT MIN(last_scanned_height) FROM orchard_sync_state"
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(result.map(|(h,)| h).unwrap_or(0))
    }

    /// Update or insert sync state for a wallet
    pub async fn upsert_sync_state(&self, wallet_id: i32, last_scanned_height: u64, notes_found: u32) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO orchard_sync_state (wallet_id, last_scanned_height, notes_found)
            VALUES (?, ?, ?)
            ON DUPLICATE KEY UPDATE
                last_scanned_height = VALUES(last_scanned_height),
                notes_found = VALUES(notes_found)
            "#
        )
        .bind(wallet_id)
        .bind(last_scanned_height)
        .bind(notes_found)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Batch update sync state for multiple wallets
    pub async fn batch_update_sync_height(&self, wallet_ids: &[i32], height: u64) -> AppResult<()> {
        if wallet_ids.is_empty() {
            return Ok(());
        }

        // Build placeholders for IN clause
        let placeholders: Vec<String> = wallet_ids.iter().map(|_| "?".to_string()).collect();
        let query = format!(
            "UPDATE orchard_sync_state SET last_scanned_height = ? WHERE wallet_id IN ({})",
            placeholders.join(",")
        );

        let mut q = sqlx::query(&query).bind(height);
        for id in wallet_ids {
            q = q.bind(*id);
        }
        q.execute(&self.pool).await?;
        Ok(())
    }

    // =========================================================================
    // Notes Operations
    // =========================================================================

    /// Save a newly discovered note
    pub async fn save_note(
        &self,
        wallet_id: i32,
        nullifier: &str,
        value_zatoshis: u64,
        block_height: u64,
        tx_hash: &str,
        position_in_block: u32,
        memo: Option<&str>,
    ) -> AppResult<i32> {
        let result = sqlx::query(
            r#"
            INSERT INTO orchard_notes
                (wallet_id, nullifier, value_zatoshis, block_height, tx_hash, position_in_block, memo)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE id = id
            "#
        )
        .bind(wallet_id)
        .bind(nullifier)
        .bind(value_zatoshis)
        .bind(block_height)
        .bind(tx_hash)
        .bind(position_in_block)
        .bind(memo)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_id() as i32)
    }

    /// Save a newly discovered note with full spending data
    pub async fn save_note_full(
        &self,
        wallet_id: i32,
        nullifier: &str,
        value_zatoshis: u64,
        block_height: u64,
        tx_hash: &str,
        position_in_block: u32,
        memo: Option<&str>,
        recipient: &str,
        rho: &str,
        rseed: &str,
    ) -> AppResult<i32> {
        let result = sqlx::query(
            r#"
            INSERT INTO orchard_notes
                (wallet_id, nullifier, value_zatoshis, block_height, tx_hash, position_in_block, memo, recipient, rho, rseed)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
                recipient = VALUES(recipient),
                rho = VALUES(rho),
                rseed = VALUES(rseed)
            "#
        )
        .bind(wallet_id)
        .bind(nullifier)
        .bind(value_zatoshis)
        .bind(block_height)
        .bind(tx_hash)
        .bind(position_in_block)
        .bind(memo)
        .bind(recipient)
        .bind(rho)
        .bind(rseed)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_id() as i32)
    }

    /// Batch save multiple notes
    pub async fn save_notes_batch(&self, notes: &[(i32, String, u64, u64, String, u32, Option<String>)]) -> AppResult<usize> {
        if notes.is_empty() {
            return Ok(0);
        }

        let mut saved = 0;
        for (wallet_id, nullifier, value, height, tx_hash, position, memo) in notes {
            if self.save_note(*wallet_id, nullifier, *value, *height, tx_hash, *position, memo.as_deref()).await.is_ok() {
                saved += 1;
            }
        }
        Ok(saved)
    }

    /// Get all unspent notes for a wallet
    pub async fn get_unspent_notes(&self, wallet_id: i32) -> AppResult<Vec<StoredOrchardNote>> {
        let notes = sqlx::query_as::<_, StoredOrchardNote>(
            r#"
            SELECT id, wallet_id, nullifier, value_zatoshis, block_height, tx_hash,
                   position_in_block, is_spent, spent_in_tx, memo, recipient, rho, rseed,
                   witness_position, witness_auth_path, witness_root
            FROM orchard_notes
            WHERE wallet_id = ? AND is_spent = FALSE
            ORDER BY block_height ASC
            "#
        )
        .bind(wallet_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(notes)
    }

    /// Get unspent notes with spending data (for shielded transfers)
    pub async fn get_spendable_notes(&self, wallet_id: i32) -> AppResult<Vec<StoredOrchardNote>> {
        let notes = sqlx::query_as::<_, StoredOrchardNote>(
            r#"
            SELECT id, wallet_id, nullifier, value_zatoshis, block_height, tx_hash,
                   position_in_block, is_spent, spent_in_tx, memo, recipient, rho, rseed,
                   witness_position, witness_auth_path, witness_root
            FROM orchard_notes
            WHERE wallet_id = ? AND is_spent = FALSE
              AND recipient IS NOT NULL AND rho IS NOT NULL AND rseed IS NOT NULL
            ORDER BY value_zatoshis DESC
            "#
        )
        .bind(wallet_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(notes)
    }

    /// Get spendable notes with witness data
    pub async fn get_notes_with_witnesses(&self, wallet_id: i32) -> AppResult<Vec<StoredOrchardNote>> {
        let notes = sqlx::query_as::<_, StoredOrchardNote>(
            r#"
            SELECT id, wallet_id, nullifier, value_zatoshis, block_height, tx_hash,
                   position_in_block, is_spent, spent_in_tx, memo, recipient, rho, rseed,
                   witness_position, witness_auth_path, witness_root
            FROM orchard_notes
            WHERE wallet_id = ? AND is_spent = FALSE
              AND recipient IS NOT NULL AND rho IS NOT NULL AND rseed IS NOT NULL
              AND witness_auth_path IS NOT NULL AND witness_root IS NOT NULL
            ORDER BY value_zatoshis DESC
            "#
        )
        .bind(wallet_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(notes)
    }

    /// Update witness data for a note
    pub async fn update_witness_data(
        &self,
        note_id: i32,
        position: u64,
        auth_path: &str,  // JSON array of hex strings
        root: &str,       // Hex-encoded root
    ) -> AppResult<bool> {
        let result = sqlx::query(
            r#"
            UPDATE orchard_notes
            SET witness_position = ?, witness_auth_path = ?, witness_root = ?
            WHERE id = ?
            "#
        )
        .bind(position)
        .bind(auth_path)
        .bind(root)
        .bind(note_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Update witness data by nullifier
    pub async fn update_witness_by_nullifier(
        &self,
        nullifier: &str,
        position: u64,
        auth_path: &str,
        root: &str,
    ) -> AppResult<bool> {
        let result = sqlx::query(
            r#"
            UPDATE orchard_notes
            SET witness_position = ?, witness_auth_path = ?, witness_root = ?
            WHERE nullifier = ?
            "#
        )
        .bind(position)
        .bind(auth_path)
        .bind(root)
        .bind(nullifier)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Get total unspent balance for a wallet
    pub async fn get_balance(&self, wallet_id: i32) -> AppResult<u64> {
        // CAST to UNSIGNED because SUM returns DECIMAL which isn't compatible with i64
        let result: Option<(u64,)> = sqlx::query_as(
            "SELECT CAST(COALESCE(SUM(value_zatoshis), 0) AS UNSIGNED) FROM orchard_notes WHERE wallet_id = ? AND is_spent = FALSE"
        )
        .bind(wallet_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(result.map(|(b,)| b).unwrap_or(0))
    }

    /// Mark a note as spent
    pub async fn mark_note_spent(&self, nullifier: &str, spent_in_tx: &str) -> AppResult<bool> {
        let result = sqlx::query(
            "UPDATE orchard_notes SET is_spent = TRUE, spent_in_tx = ? WHERE nullifier = ? AND is_spent = FALSE"
        )
        .bind(spent_in_tx)
        .bind(nullifier)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Check if a nullifier exists (note was spent)
    pub async fn nullifier_exists(&self, nullifier: &str) -> AppResult<bool> {
        let result: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM orchard_notes WHERE nullifier = ? LIMIT 1"
        )
        .bind(nullifier)
        .fetch_optional(&self.pool)
        .await?;
        Ok(result.is_some())
    }

    /// Get notes count for a wallet
    pub async fn get_notes_count(&self, wallet_id: i32) -> AppResult<u32> {
        let result: Option<(i64,)> = sqlx::query_as(
            "SELECT COUNT(*) FROM orchard_notes WHERE wallet_id = ?"
        )
        .bind(wallet_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(result.map(|(c,)| c as u32).unwrap_or(0))
    }
}
