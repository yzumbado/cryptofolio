pub mod bitcoin;
pub mod cardano;
pub mod ethereum;
pub mod provider;
pub mod security;
pub mod solana;
pub mod sync;
pub mod trait_def;
pub mod types;

// Re-export commonly used items
pub use bitcoin::validate_address as validate_bitcoin_address;
pub use cardano::validate_address as validate_cardano_address;
pub use ethereum::validate_address as validate_ethereum_address;
pub use solana::validate_address as validate_solana_address;

// Re-export the unified trait and type system
pub use trait_def::BlockchainClient;
pub use types::{
    AddressSummary, Chain, ChainExtras, HealthStatus, SolanaStakeAccount, TransactionDirection,
    WalletBalance, WalletTransaction,
};

// Re-export the provider registry
pub use provider::{PrivacyLevel, PrivacyMode, ProviderRegistry};

// Re-export the sync engine
pub use sync::{SyncEngine, SyncOptions, SyncReport};

// Re-export security guards
pub use security::{detect_private_key, reject_if_private_key};
