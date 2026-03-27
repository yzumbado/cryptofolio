use cucumber::World;
use sqlx::SqlitePool;
use std::path::PathBuf;
use anyhow::{Context, Result};
use crate::support::blockchain_mock::BlockstreamMock;
use crate::support::ethereum_mock::EthereumMock;
use crate::support::cardano_mock::CardanoMock;

/// Shared state for BDD tests
#[derive(World)]
#[world(init = Self::new)]
pub struct CryptofolioWorld {
    /// In-memory test database
    pub pool: Option<SqlitePool>,

    /// Last command output
    pub last_output: String,

    /// Last command exit code
    pub last_exit_code: i32,

    /// Test data directory
    pub test_dir: Option<PathBuf>,

    /// Captured wallet ID from commands
    pub last_wallet_id: Option<i64>,

    /// Captured account ID from commands
    pub last_account_id: Option<String>,

    /// Mock Blockstream API server
    pub blockchain_mock: Option<BlockstreamMock>,

    /// Mock Ethereum API server
    pub ethereum_mock: Option<EthereumMock>,

    /// Accumulated ERC-20 tokens for mocking (symbol, name, balance, decimals)
    pub accumulated_tokens: Vec<(String, String, f64, u8)>,

    /// Mock Cardano API server
    pub cardano_mock: Option<CardanoMock>,

    /// Accumulated Cardano native tokens for mocking (symbol, quantity, decimals)
    pub accumulated_cardano_tokens: Vec<(String, String, u8)>,
}

impl std::fmt::Debug for CryptofolioWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CryptofolioWorld")
            .field("pool", &self.pool.is_some())
            .field("last_output", &self.last_output)
            .field("last_exit_code", &self.last_exit_code)
            .field("blockchain_mock", &self.blockchain_mock.is_some())
            .field("ethereum_mock", &self.ethereum_mock.is_some())
            .finish()
    }
}

impl CryptofolioWorld {
    async fn new() -> Result<Self> {
        Ok(Self {
            pool: None,
            last_output: String::new(),
            last_exit_code: 0,
            test_dir: None,
            last_wallet_id: None,
            last_account_id: None,
            blockchain_mock: None,
            ethereum_mock: None,
            accumulated_tokens: Vec::new(),
            cardano_mock: None,
            accumulated_cardano_tokens: Vec::new(),
        })
    }

    /// Setup mock blockchain server
    pub async fn setup_blockchain_mock(&mut self) -> Result<()> {
        let mock = BlockstreamMock::new().await;
        self.blockchain_mock = Some(mock);
        Ok(())
    }

    /// Get blockchain mock server URL
    pub fn blockchain_mock_url(&self) -> Option<String> {
        self.blockchain_mock.as_ref().map(|m| m.url())
    }

    /// Setup mock Ethereum server
    pub async fn setup_ethereum_mock(&mut self) -> Result<()> {
        let mock = EthereumMock::new().await;
        self.ethereum_mock = Some(mock);
        Ok(())
    }

    /// Get Ethereum mock server URL
    pub fn ethereum_mock_url(&self) -> Option<String> {
        self.ethereum_mock.as_ref().map(|m| m.url())
    }

    /// Setup mock Cardano server
    pub async fn setup_cardano_mock(&mut self) -> Result<()> {
        let mock = CardanoMock::new().await;
        self.cardano_mock = Some(mock);
        Ok(())
    }

    /// Get Cardano mock server URL
    pub fn cardano_mock_url(&self) -> Option<String> {
        self.cardano_mock.as_ref().map(|m| m.url())
    }

    /// Setup test database with all migrations
    pub async fn setup_db(&mut self) -> Result<()> {
        use cryptofolio::db::migrations;

        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .context("Failed to create test database")?;

        // Apply all migrations
        migrations::run(&pool)
            .await
            .context("Failed to run migrations")?;

        self.pool = Some(pool);
        Ok(())
    }

    /// Get database pool (panics if not set up)
    pub fn pool(&self) -> &SqlitePool {
        self.pool.as_ref().expect("Database not initialized - call setup_db() first")
    }

    /// Reset last command state
    pub fn reset_command_state(&mut self) {
        self.last_output.clear();
        self.last_exit_code = 0;
    }
}

impl Drop for CryptofolioWorld {
    fn drop(&mut self) {
        // Cleanup test directory if it exists
        if let Some(dir) = &self.test_dir {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}
