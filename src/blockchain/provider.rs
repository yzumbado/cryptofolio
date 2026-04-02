/// Provider registry with health-check–driven provider selection.
///
/// Maintains an ordered list of `BlockchainClient` implementations per chain.
/// On each sync the registry picks the first healthy provider that satisfies
/// the configured `PrivacyMode`. Results are cached for 60 seconds to avoid
/// unnecessary round-trips.
///
/// # Provider priority (default)
///
/// Local (0) → Custom (1) → Public (2)
///
/// `PrivacyMode::Strict`      — Local only
/// `PrivacyMode::Balanced`    — Local → Custom (no public API)
/// `PrivacyMode::Convenience` — Local → Custom → Public

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;

use crate::blockchain::trait_def::BlockchainClient;
use crate::blockchain::types::{Chain, HealthStatus};
use crate::error::{CryptofolioError, Result};

// ---------------------------------------------------------------------------
// Privacy types
// ---------------------------------------------------------------------------

/// Controls which provider tiers are eligible for use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivacyMode {
    /// Only local nodes — address data never leaves the machine.
    Strict,
    /// Local if available, custom RPC second — no public third-party APIs.
    Balanced,
    /// Use any provider; prefer the most reliable (usually public API).
    Convenience,
}

impl Default for PrivacyMode {
    fn default() -> Self {
        PrivacyMode::Balanced
    }
}

/// Privacy level of a specific provider instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PrivacyLevel {
    /// localhost — no address data leaves the machine.
    Local = 0,
    /// User-supplied RPC endpoint — address data goes to their server.
    Custom = 1,
    /// Third-party public API — address data goes to the provider.
    Public = 2,
}

// ---------------------------------------------------------------------------
// Registry internals
// ---------------------------------------------------------------------------

struct ProviderEntry {
    client: Arc<dyn BlockchainClient>,
    privacy_level: PrivacyLevel,
}

/// Cache key: (chain, provider_name).
type CacheKey = (String, String);

struct CachedHealth {
    status: HealthStatus,
    cached_at: Instant,
}

const HEALTH_CACHE_TTL: Duration = Duration::from_secs(60);

// ---------------------------------------------------------------------------
// ProviderRegistry
// ---------------------------------------------------------------------------

/// Registry of `BlockchainClient` implementations, one list per chain.
///
/// Call [`ProviderRegistry::get`] to obtain the best healthy provider for a
/// given chain under the current `PrivacyMode`.
pub struct ProviderRegistry {
    /// Ordered provider list per chain (index 0 = highest priority).
    providers: HashMap<String, Vec<ProviderEntry>>,
    privacy_mode: PrivacyMode,
    /// (chain_id, provider_name) → (HealthStatus, cached_at)
    health_cache: Mutex<HashMap<CacheKey, CachedHealth>>,
}

impl ProviderRegistry {
    pub fn new(privacy_mode: PrivacyMode) -> Self {
        Self {
            providers: HashMap::new(),
            privacy_mode,
            health_cache: Mutex::new(HashMap::new()),
        }
    }

    /// Register a provider for `chain`.
    ///
    /// Providers are selected in registration order within each `PrivacyLevel`.
    /// Register Local providers first, then Custom, then Public.
    pub fn register(
        &mut self,
        chain: &Chain,
        client: Arc<dyn BlockchainClient>,
        privacy_level: PrivacyLevel,
    ) {
        self.providers
            .entry(chain.as_str().to_string())
            .or_default()
            .push(ProviderEntry {
                client,
                privacy_level,
            });
    }

    /// Return the best healthy provider for `chain` under the current privacy mode.
    ///
    /// Iterates the registered providers in order. For each:
    /// 1. Skip if the privacy level exceeds what the mode allows.
    /// 2. Return cached health if < 60 s old.
    /// 3. Call `health_check()` and cache the result.
    /// 4. Return the client if reachable.
    ///
    /// Returns `Err(CryptofolioError::NoProvider)` if no healthy provider found.
    pub async fn get(&self, chain: &Chain) -> Result<Arc<dyn BlockchainClient>> {
        let chain_key = chain.as_str().to_string();

        let entries = self.providers.get(&chain_key).ok_or_else(|| {
            CryptofolioError::Other(format!(
                "No providers registered for chain: {}",
                chain.as_str()
            ))
        })?;

        let max_level = match self.privacy_mode {
            PrivacyMode::Strict => PrivacyLevel::Local,
            PrivacyMode::Balanced => PrivacyLevel::Custom,
            PrivacyMode::Convenience => PrivacyLevel::Public,
        };

        for entry in entries {
            // Enforce privacy mode
            if entry.privacy_level > max_level {
                continue;
            }

            let provider_name = entry.client.provider_name().to_string();
            let cache_key = (chain_key.clone(), provider_name.clone());

            // Check cache
            {
                let cache = self.health_cache.lock().await;
                if let Some(cached) = cache.get(&cache_key) {
                    if cached.cached_at.elapsed() < HEALTH_CACHE_TTL {
                        if cached.status.reachable {
                            return Ok(Arc::clone(&entry.client));
                        } else {
                            continue;
                        }
                    }
                }
            }

            // Cache miss or expired — call health_check
            let status = entry.client.health_check().await.unwrap_or(HealthStatus {
                provider: provider_name.clone(),
                reachable: false,
                block_height: None,
                latency_ms: 0,
            });

            let reachable = status.reachable;

            {
                let mut cache = self.health_cache.lock().await;
                cache.insert(
                    cache_key,
                    CachedHealth {
                        status,
                        cached_at: Instant::now(),
                    },
                );
            }

            if reachable {
                return Ok(Arc::clone(&entry.client));
            }
        }

        Err(CryptofolioError::Other(format!(
            "No healthy {} provider available (privacy_mode: {:?})",
            chain.as_str(),
            self.privacy_mode,
        )))
    }

    /// Invalidate the health cache for a specific chain + provider.
    /// Call this after a provider returns unexpected errors mid-sync.
    pub async fn invalidate(&self, chain: &Chain, provider_name: &str) {
        let mut cache = self.health_cache.lock().await;
        cache.remove(&(chain.as_str().to_string(), provider_name.to_string()));
    }

    /// Invalidate the entire health cache.
    pub async fn invalidate_all(&self) {
        let mut cache = self.health_cache.lock().await;
        cache.clear();
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::types::{AddressSummary, WalletTransaction};
    use async_trait::async_trait;

    /// Minimal stub that always reports healthy with a fixed block height.
    struct StubClient {
        name: &'static str,
        reachable: bool,
    }

    #[async_trait]
    impl BlockchainClient for StubClient {
        fn provider_name(&self) -> &str {
            self.name
        }

        async fn health_check(&self) -> Result<HealthStatus> {
            Ok(HealthStatus {
                provider: self.name.to_string(),
                reachable: self.reachable,
                block_height: if self.reachable { Some(800_000) } else { None },
                latency_ms: 10,
            })
        }

        async fn get_address_summary(&self, _address: &str) -> Result<AddressSummary> {
            unimplemented!()
        }

        async fn get_transactions(
            &self,
            _address: &str,
            _since_block: Option<u64>,
        ) -> Result<Vec<WalletTransaction>> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn test_returns_first_healthy_provider() {
        let mut registry = ProviderRegistry::new(PrivacyMode::Convenience);
        registry.register(
            &Chain::Bitcoin,
            Arc::new(StubClient { name: "local", reachable: true }),
            PrivacyLevel::Local,
        );
        registry.register(
            &Chain::Bitcoin,
            Arc::new(StubClient { name: "public", reachable: true }),
            PrivacyLevel::Public,
        );

        let client = registry.get(&Chain::Bitcoin).await.unwrap();
        assert_eq!(client.provider_name(), "local");
    }

    #[tokio::test]
    async fn test_falls_back_when_first_unhealthy() {
        let mut registry = ProviderRegistry::new(PrivacyMode::Convenience);
        registry.register(
            &Chain::Bitcoin,
            Arc::new(StubClient { name: "local", reachable: false }),
            PrivacyLevel::Local,
        );
        registry.register(
            &Chain::Bitcoin,
            Arc::new(StubClient { name: "public", reachable: true }),
            PrivacyLevel::Public,
        );

        let client = registry.get(&Chain::Bitcoin).await.unwrap();
        assert_eq!(client.provider_name(), "public");
    }

    #[tokio::test]
    async fn test_strict_mode_skips_public_providers() {
        let mut registry = ProviderRegistry::new(PrivacyMode::Strict);
        registry.register(
            &Chain::Bitcoin,
            Arc::new(StubClient { name: "local", reachable: false }),
            PrivacyLevel::Local,
        );
        registry.register(
            &Chain::Bitcoin,
            Arc::new(StubClient { name: "public", reachable: true }),
            PrivacyLevel::Public,
        );

        // Public provider should be skipped — should return error
        assert!(registry.get(&Chain::Bitcoin).await.is_err());
    }

    #[tokio::test]
    async fn test_balanced_mode_skips_public_providers() {
        let mut registry = ProviderRegistry::new(PrivacyMode::Balanced);
        registry.register(
            &Chain::Bitcoin,
            Arc::new(StubClient { name: "custom", reachable: false }),
            PrivacyLevel::Custom,
        );
        registry.register(
            &Chain::Bitcoin,
            Arc::new(StubClient { name: "public", reachable: true }),
            PrivacyLevel::Public,
        );

        assert!(registry.get(&Chain::Bitcoin).await.is_err());
    }

    #[tokio::test]
    async fn test_no_providers_returns_error() {
        let registry = ProviderRegistry::new(PrivacyMode::Convenience);
        assert!(registry.get(&Chain::Bitcoin).await.is_err());
    }

    #[tokio::test]
    async fn test_health_cache_is_used() {
        use std::sync::atomic::{AtomicU32, Ordering};

        struct CountingClient {
            call_count: Arc<AtomicU32>,
        }

        #[async_trait]
        impl BlockchainClient for CountingClient {
            fn provider_name(&self) -> &str { "counting" }
            async fn health_check(&self) -> Result<HealthStatus> {
                self.call_count.fetch_add(1, Ordering::SeqCst);
                Ok(HealthStatus {
                    provider: "counting".to_string(),
                    reachable: true,
                    block_height: Some(1),
                    latency_ms: 1,
                })
            }
            async fn get_address_summary(&self, _: &str) -> Result<AddressSummary> { unimplemented!() }
            async fn get_transactions(&self, _: &str, _: Option<u64>) -> Result<Vec<WalletTransaction>> { unimplemented!() }
        }

        let counter = Arc::new(AtomicU32::new(0));
        let mut registry = ProviderRegistry::new(PrivacyMode::Convenience);
        registry.register(
            &Chain::Bitcoin,
            Arc::new(CountingClient { call_count: Arc::clone(&counter) }),
            PrivacyLevel::Public,
        );

        // First call — should hit health_check
        registry.get(&Chain::Bitcoin).await.unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        // Second call — should use cache, not call health_check again
        registry.get(&Chain::Bitcoin).await.unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_invalidate_clears_cache() {
        use std::sync::atomic::{AtomicU32, Ordering};

        struct CountingClient {
            call_count: Arc<AtomicU32>,
        }

        #[async_trait]
        impl BlockchainClient for CountingClient {
            fn provider_name(&self) -> &str { "counting" }
            async fn health_check(&self) -> Result<HealthStatus> {
                self.call_count.fetch_add(1, Ordering::SeqCst);
                Ok(HealthStatus {
                    provider: "counting".to_string(),
                    reachable: true,
                    block_height: Some(1),
                    latency_ms: 1,
                })
            }
            async fn get_address_summary(&self, _: &str) -> Result<AddressSummary> { unimplemented!() }
            async fn get_transactions(&self, _: &str, _: Option<u64>) -> Result<Vec<WalletTransaction>> { unimplemented!() }
        }

        let counter = Arc::new(AtomicU32::new(0));
        let mut registry = ProviderRegistry::new(PrivacyMode::Convenience);
        registry.register(
            &Chain::Bitcoin,
            Arc::new(CountingClient { call_count: Arc::clone(&counter) }),
            PrivacyLevel::Public,
        );

        registry.get(&Chain::Bitcoin).await.unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        registry.invalidate(&Chain::Bitcoin, "counting").await;

        registry.get(&Chain::Bitcoin).await.unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }
}
