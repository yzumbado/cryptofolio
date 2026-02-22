//! Keychain metadata repository
//!
//! Tracks which secrets are stored in which location (keychain/TOML/env)
//! and their security levels.

use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

use crate::config::keychain::KeychainSecurityLevel;
use crate::error::Result;

/// Storage type for a secret
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StorageType {
    Keychain,
    Toml,
    Env,
}

impl StorageType {
    pub fn as_str(&self) -> &'static str {
        match self {
            StorageType::Keychain => "keychain",
            StorageType::Toml => "toml",
            StorageType::Env => "env",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "keychain" => Some(StorageType::Keychain),
            "toml" => Some(StorageType::Toml),
            "env" => Some(StorageType::Env),
            _ => None,
        }
    }
}

/// Keychain key metadata
#[derive(Debug, Clone)]
pub struct KeychainKey {
    pub id: i64,
    pub key_name: String,
    pub storage_type: StorageType,
    pub security_level: Option<KeychainSecurityLevel>,
    pub last_accessed: Option<DateTime<Utc>>,
    pub migrated_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Repository for keychain key metadata
pub struct KeychainKeyRepository {
    pool: SqlitePool,
}

impl KeychainKeyRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create or update a keychain key metadata entry
    pub async fn upsert(
        &self,
        key_name: &str,
        storage_type: StorageType,
        security_level: Option<KeychainSecurityLevel>,
    ) -> Result<()> {
        let security_level_str = security_level.map(|l| l.as_db_str());

        sqlx::query(
            r#"
            INSERT INTO keychain_keys (key_name, storage_type, security_level)
            VALUES (?, ?, ?)
            ON CONFLICT(key_name) DO UPDATE SET
                storage_type = excluded.storage_type,
                security_level = excluded.security_level
            "#,
        )
        .bind(key_name)
        .bind(storage_type.as_str())
        .bind(security_level_str)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get metadata for a specific key
    pub async fn get(&self, key_name: &str) -> Result<Option<KeychainKey>> {
        let row = sqlx::query(
            r#"
            SELECT id, key_name, storage_type, security_level, last_accessed, migrated_at, created_at
            FROM keychain_keys
            WHERE key_name = ?
            "#,
        )
        .bind(key_name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| {
            let storage_type_str: String = r.get("storage_type");
            let security_level_str: Option<String> = r.get("security_level");

            // Parse DateTime from SQLite string format
            let last_accessed: Option<String> = r.get("last_accessed");
            let migrated_at: Option<String> = r.get("migrated_at");
            let created_at: String = r.get("created_at");

            KeychainKey {
                id: r.get("id"),
                key_name: r.get("key_name"),
                storage_type: StorageType::from_str(&storage_type_str)
                    .unwrap_or(StorageType::Toml),
                security_level: security_level_str
                    .and_then(|s| KeychainSecurityLevel::from_str(&s)),
                last_accessed: last_accessed.and_then(|s| s.parse().ok()),
                migrated_at: migrated_at.and_then(|s| s.parse().ok()),
                created_at: created_at.parse().unwrap_or_else(|_| Utc::now()),
            }
        }))
    }

    /// List all tracked keys
    pub async fn list(&self) -> Result<Vec<KeychainKey>> {
        let rows = sqlx::query(
            r#"
            SELECT id, key_name, storage_type, security_level, last_accessed, migrated_at, created_at
            FROM keychain_keys
            ORDER BY key_name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let keys = rows
            .into_iter()
            .map(|r| {
                let storage_type_str: String = r.get("storage_type");
                let security_level_str: Option<String> = r.get("security_level");

                // Parse DateTime from SQLite string format
                let last_accessed: Option<String> = r.get("last_accessed");
                let migrated_at: Option<String> = r.get("migrated_at");
                let created_at: String = r.get("created_at");

                KeychainKey {
                    id: r.get("id"),
                    key_name: r.get("key_name"),
                    storage_type: StorageType::from_str(&storage_type_str)
                        .unwrap_or(StorageType::Toml),
                    security_level: security_level_str
                        .and_then(|s| KeychainSecurityLevel::from_str(&s)),
                    last_accessed: last_accessed.and_then(|s| s.parse().ok()),
                    migrated_at: migrated_at.and_then(|s| s.parse().ok()),
                    created_at: created_at.parse().unwrap_or_else(|_| Utc::now()),
                }
            })
            .collect();

        Ok(keys)
    }

    /// Update last accessed timestamp
    pub async fn update_last_accessed(&self, key_name: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE keychain_keys
            SET last_accessed = CURRENT_TIMESTAMP
            WHERE key_name = ?
            "#,
        )
        .bind(key_name)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark a key as migrated to keychain
    pub async fn mark_migrated(&self, key_name: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE keychain_keys
            SET migrated_at = CURRENT_TIMESTAMP
            WHERE key_name = ?
            "#,
        )
        .bind(key_name)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update security level
    pub async fn update_security_level(
        &self,
        key_name: &str,
        security_level: KeychainSecurityLevel,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE keychain_keys
            SET security_level = ?
            WHERE key_name = ?
            "#,
        )
        .bind(security_level.as_db_str())
        .bind(key_name)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete a key metadata entry
    pub async fn delete(&self, key_name: &str) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM keychain_keys
            WHERE key_name = ?
            "#,
        )
        .bind(key_name)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get all keys stored in keychain
    pub async fn list_keychain_keys(&self) -> Result<Vec<KeychainKey>> {
        let rows = sqlx::query(
            r#"
            SELECT id, key_name, storage_type, security_level, last_accessed, migrated_at, created_at
            FROM keychain_keys
            WHERE storage_type = 'keychain'
            ORDER BY key_name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let keys = rows
            .into_iter()
            .map(|r| {
                let security_level_str: Option<String> = r.get("security_level");

                // Parse DateTime from SQLite string format
                let last_accessed: Option<String> = r.get("last_accessed");
                let migrated_at: Option<String> = r.get("migrated_at");
                let created_at: String = r.get("created_at");

                KeychainKey {
                    id: r.get("id"),
                    key_name: r.get("key_name"),
                    storage_type: StorageType::Keychain,
                    security_level: security_level_str
                        .and_then(|s| KeychainSecurityLevel::from_str(&s)),
                    last_accessed: last_accessed.and_then(|s| s.parse().ok()),
                    migrated_at: migrated_at.and_then(|s| s.parse().ok()),
                    created_at: created_at.parse().unwrap_or_else(|_| Utc::now()),
                }
            })
            .collect();

        Ok(keys)
    }
}
