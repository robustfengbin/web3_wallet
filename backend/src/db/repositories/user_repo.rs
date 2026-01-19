#![allow(dead_code)]

use crate::db::models::User;
use crate::error::AppResult;
use sqlx::MySqlPool;

pub struct UserRepository {
    pool: MySqlPool,
}

impl UserRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_username(&self, username: &str) -> AppResult<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            "SELECT id, username, password_hash, role, created_at, updated_at FROM users WHERE username = ?"
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn find_by_id(&self, id: i32) -> AppResult<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            "SELECT id, username, password_hash, role, created_at, updated_at FROM users WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn create(&self, username: &str, password_hash: &str, role: &str) -> AppResult<i32> {
        let result = sqlx::query(
            "INSERT INTO users (username, password_hash, role) VALUES (?, ?, ?)"
        )
        .bind(username)
        .bind(password_hash)
        .bind(role)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_id() as i32)
    }

    pub async fn update_password(&self, user_id: i32, new_password_hash: &str) -> AppResult<()> {
        sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
            .bind(new_password_hash)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn list_all(&self) -> AppResult<Vec<User>> {
        let users = sqlx::query_as::<_, User>(
            "SELECT id, username, password_hash, role, created_at, updated_at FROM users ORDER BY id"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(users)
    }

    pub async fn create_default_admin_if_not_exists(&self, password_hash: &str) -> AppResult<()> {
        // Check if any user exists
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;

        if count.0 == 0 {
            self.create("admin", password_hash, "admin").await?;
            tracing::info!("Default admin user created");
        }

        Ok(())
    }
}
