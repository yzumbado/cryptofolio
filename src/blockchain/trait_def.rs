use async_trait::async_trait;

use super::types::{AddressSummary, ChainExtras, HealthStatus, WalletTransaction};
use crate::error::Result;

/// Common interface for all blockchain providers.
///
/// # Security
///
/// Implementations **MUST** be watch-only. No private key material must ever
/// pass through this interface. The only inputs are public addresses and,
/// optionally, extended public keys (xpub) derived from elsewhere in the
/// security layer.
///
/// # Object safety
///
/// The trait is object-safe (`Box<dyn BlockchainClient>` is valid) because:
/// - No generic methods.
/// - All methods take `&self` or primitive/borrowed parameters.
/// - `Send + Sync` bounds allow use across `tokio::task::spawn`.
#[async_trait]
pub trait BlockchainClient: Send + Sync {
    /// Human-readable provider name, used in logs and error messages.
    ///
    /// Examples: `"Blockstream"`, `"Etherscan"`, `"Blockfrost"`, `"Solana RPC"`.
    fn provider_name(&self) -> &str;

    /// Check provider reachability and current block/slot height.
    ///
    /// Called by `ProviderRegistry` to select the best available provider.
    /// Should be fast (< 2 s timeout recommended). Results are cached for 60 s.
    async fn health_check(&self) -> Result<HealthStatus>;

    /// Return the full balance and token/asset summary for `address`.
    async fn get_address_summary(&self, address: &str) -> Result<AddressSummary>;

    /// Return the transaction history for `address`.
    ///
    /// `since_block` — if `Some(h)`, return only transactions in blocks >= `h`.
    /// Clients that cannot honour `since_block` must return all transactions;
    /// the caller is responsible for deduplication.
    async fn get_transactions(
        &self,
        address: &str,
        since_block: Option<u64>,
    ) -> Result<Vec<WalletTransaction>>;

    /// Chain-specific extras (stake info, pool delegation, SPL tokens, etc.).
    ///
    /// The default implementation returns `Ok(None)` — override only when the
    /// chain has data that doesn't fit `AddressSummary::balances`.
    async fn get_chain_extras(&self, address: &str) -> Result<Option<ChainExtras>> {
        let _ = address;
        Ok(None)
    }
}
