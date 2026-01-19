#![allow(dead_code)]

use sqlx::MySqlPool;

use crate::error::AppResult;

pub struct SettingsRepository {
    pool: MySqlPool,
}

impl SettingsRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Get a setting value by key
    pub async fn get(&self, key: &str) -> AppResult<Option<String>> {
        let result: Option<(String,)> = sqlx::query_as(
            "SELECT `value` FROM settings WHERE `key` = ?"
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|(v,)| v))
    }

    /// Set a setting value (insert or update)
    pub async fn set(&self, key: &str, value: &str) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO settings (`key`, `value`) VALUES (?, ?)
            ON DUPLICATE KEY UPDATE `value` = VALUES(`value`)
            "#
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete a setting
    pub async fn delete(&self, key: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM settings WHERE `key` = ?")
            .bind(key)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
