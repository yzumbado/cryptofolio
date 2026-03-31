# Cryptofolio v0.5.0 — Wallet Integration Architecture

**Version:** v0.5.0 (planned)
**Status:** Design document — authoritative reference for all blockchain wallet work
**Last Updated:** March 2026

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Current State & Gap Analysis](#2-current-state--gap-analysis)
3. [Target Architecture Overview](#3-target-architecture-overview)
4. [BlockchainClient Trait](#4-blockchainclient-trait)
5. [Unified Type System](#5-unified-type-system)
6. [Provider Registry & Health-Check Routing](#6-provider-registry--health-check-routing)
7. [Rate Limiting](#7-rate-limiting)
8. [Security Model](#8-security-model)
9. [Chain-Specific Implementations](#9-chain-specific-implementations)
10. [Solana Integration Design](#10-solana-integration-design)
11. [Sync Engine Redesign](#11-sync-engine-redesign)
12. [New Database Tables](#12-new-database-tables)
13. [Implementation Checklist](#13-implementation-checklist)

---

## 1. Executive Summary

v0.5.0 transitions blockchain wallet tracking from ad-hoc per-chain implementations to a
**unified, trait-based integration layer**. The goals are:

| Goal | Rationale |
|------|-----------|
| Single `BlockchainClient` trait | Consistent interface enables provider swapping, testing, future chains |
| Health-check–driven provider selection | Prefer local nodes → custom RPC → public API; degrades gracefully |
| Per-provider rate limiting | Respect API quotas, avoid bans |
| Comprehensive security model | Watch-only invariant, xpub privacy, API key isolation, audit log |
| Solana integration | SPL tokens, stake accounts, no default public RPC |
| Parallel sync engine | `JoinSet`-based concurrency, per-chain error isolation, progress bars |

This document is the authoritative design. Any implementation that deviates must update
this document first.

---

## 2. Current State & Gap Analysis

### 2.1 Existing Clients

| Chain | Client | Provider | Auth | Status |
|-------|--------|----------|------|--------|
| Bitcoin | `BlockstreamClient` | Blockstream.info | None (public) | ✅ functional |
| Ethereum | `EtherscanClient` | Etherscan V2 | Optional API key | ✅ functional |
| Cardano | `BlockfrostClient` | Blockfrost | API key | ✅ functional |
| Solana | — | — | — | ❌ not implemented |

### 2.2 Gaps vs Target Architecture

| Area | Current | Required |
|------|---------|----------|
| Common trait | None — each client has its own method signatures | `BlockchainClient: Send + Sync` |
| Provider fallback | None — single hardcoded URL per chain | `ProviderRegistry` with health-check ordering |
| Rate limiting | None | Token-bucket `RateLimitedClient<T>` wrapper |
| Unified return types | Chain-specific structs per client | `AddressSummary`, `WalletTransaction`, `WalletBalance` |
| Solana support | Missing entirely | `SolanaRpcClient` with SPL + stake |
| xpub privacy | xpub stored in DB `config` JSON column (plaintext) | Encrypted in keychain, never logged |
| Audit log | None | `sync_audit_log` table (new) |
| Transaction direction | `is_incoming: bool` (Bitcoin) — inconsistent | `TransactionDirection` enum across all chains |
| Parallel sync | Sequential per-address loop | `JoinSet` with progress bars |

---

## 3. Target Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│  CLI Layer                                                       │
│  wallet sync / wallet add / wallet show                         │
└──────────────────────┬──────────────────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────────────────┐
│  SyncEngine  (src/blockchain/sync.rs)                           │
│  • JoinSet parallelism — one task per address                   │
│  • Per-chain error isolation — one failure doesn't abort others │
│  • indicatif multi-progress bars                                │
│  • Writes to DB via repositories                                │
└──────────┬────────────────────────┬────────────────────────────┘
           │                        │
┌──────────▼──────────┐  ┌──────────▼──────────┐  ┌─────────────┐
│  ProviderRegistry   │  │  ProviderRegistry   │  │  ...        │
│  (Bitcoin)          │  │  (Ethereum)         │  │             │
│  • local node       │  │  • local node       │  │             │
│  • custom RPC       │  │  • custom RPC       │  │             │
│  • Blockstream      │  │  • Etherscan        │  │             │
└──────────┬──────────┘  └──────────┬──────────┘  └─────────────┘
           │                        │
┌──────────▼──────────┐  ┌──────────▼──────────┐
│ RateLimitedClient   │  │ RateLimitedClient   │
│ <BlockstreamClient> │  │ <EtherscanClient>   │
└──────────┬──────────┘  └──────────┬──────────┘
           │                        │
┌──────────▼──────────┐  ┌──────────▼──────────┐
│  BlockstreamClient  │  │  EtherscanClient    │
│  impl BlockchainClient  │  impl BlockchainClient │
└─────────────────────┘  └─────────────────────┘
```

All clients implement `BlockchainClient`. The `ProviderRegistry` wraps the selected
client in `RateLimitedClient` before returning it. The `SyncEngine` calls only the trait.

---

## 4. BlockchainClient Trait

File: `src/blockchain/trait.rs` (new)

```rust
use async_trait::async_trait;
use crate::error::Result;
use super::types::{AddressSummary, WalletTransaction, HealthStatus};

/// Common interface for all blockchain providers.
///
/// SECURITY: Implementations MUST be watch-only. No private key material
/// must ever pass through this interface.
#[async_trait]
pub trait BlockchainClient: Send + Sync {
    /// Human-readable provider name, used in logs and error messages.
    fn provider_name(&self) -> &str;

    /// Check provider reachability and current block height.
    /// Called by ProviderRegistry to select the best available provider.
    async fn health_check(&self) -> Result<HealthStatus>;

    /// Return the full balance and token/asset summary for an address.
    async fn get_address_summary(&self, address: &str) -> Result<AddressSummary>;

    /// Return the transaction history for an address.
    ///
    /// `since_block` — if Some, return only transactions in blocks >= this height.
    /// Clients that cannot honour `since_block` must return all transactions.
    async fn get_transactions(
        &self,
        address: &str,
        since_block: Option<u64>,
    ) -> Result<Vec<WalletTransaction>>;

    /// Chain-specific extras (stake info, pool delegation, etc.).
    /// Default implementation returns None.
    async fn get_chain_extras(&self, address: &str) -> Result<Option<ChainExtras>> {
        let _ = address;
        Ok(None)
    }
}

/// Result of a health check.
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub provider: String,
    pub reachable: bool,
    pub block_height: Option<u64>,
    pub latency_ms: u64,
}
```

### 4.1 Object Safety

`Box<dyn BlockchainClient>` is the unit of composition throughout the codebase.
The trait is object-safe because:
- No generic methods (except `get_chain_extras` which has a provided default, not a generic).
- All methods take `&self` or `&str`.
- `Send + Sync` bounds allow use across `tokio::task::spawn`.

---

## 5. Unified Type System

File: `src/blockchain/types.rs` (new)

```rust
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Supported blockchain networks.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Chain {
    Bitcoin,
    Ethereum,
    Cardano,
    Solana,
}

impl Chain {
    pub fn native_asset(&self) -> &str {
        match self {
            Chain::Bitcoin  => "BTC",
            Chain::Ethereum => "ETH",
            Chain::Cardano  => "ADA",
            Chain::Solana   => "SOL",
        }
    }
    pub fn decimals(&self) -> u8 {
        match self {
            Chain::Bitcoin  => 8,
            Chain::Ethereum => 18,
            Chain::Cardano  => 6,
            Chain::Solana   => 9,
        }
    }
}

/// Balance of a single asset at an address.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBalance {
    /// Ticker symbol, e.g. "BTC", "ETH", "USDC", "ADA"
    pub asset: String,
    /// Canonical asset identifier (contract address, policy ID+name, mint pubkey)
    pub asset_id: Option<String>,
    /// Human-readable balance (already divided by `decimals`)
    pub quantity: Decimal,
    /// Number of decimal places for this asset
    pub decimals: u8,
}

/// A single transaction as seen by a wallet address.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletTransaction {
    /// Chain-specific transaction identifier (txid, hash, signature)
    pub external_id: String,
    pub direction: TransactionDirection,
    /// Net amount transferred in the native asset (positive = received)
    pub amount: Decimal,
    pub asset: String,
    pub fee: Option<Decimal>,
    pub fee_asset: Option<String>,
    pub block_height: Option<u64>,
    pub timestamp: DateTime<Utc>,
    pub counterparty: Option<String>, // from/to address
    pub memo: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionDirection {
    Incoming,
    Outgoing,
    Internal, // self-transfer, contract interaction, etc.
}

/// Full address summary returned by get_address_summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressSummary {
    pub address: String,
    pub chain: Chain,
    /// Native coin balance (first entry is always the native asset)
    pub balances: Vec<WalletBalance>,
    pub transaction_count: u64,
    /// Chain-specific extras (stake info, DeFi positions, etc.)
    pub extras: Option<ChainExtras>,
}

/// Chain-specific information that doesn't fit the common model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "chain")]
pub enum ChainExtras {
    Cardano {
        stake_address: Option<String>,
        pool_id: Option<String>,
        pool_ticker: Option<String>,
        pool_name: Option<String>,
        active_stake: Option<Decimal>,
        margin_cost: Option<f64>,
    },
    Solana {
        stake_accounts: Vec<SolanaStakeAccount>,
    },
    Bitcoin {},
    Ethereum {},
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaStakeAccount {
    pub pubkey: String,
    pub lamports: u64,
    pub validator_vote_account: Option<String>,
    pub activation_epoch: Option<u64>,
}
```

---

## 6. Provider Registry & Health-Check Routing

File: `src/blockchain/provider.rs` (new)

### 6.1 Design

The registry maintains an **ordered list** of providers for each chain, sorted by
preference (local first). On each sync, it calls `health_check()` in order and returns
the first healthy provider. Results are cached for 60 seconds to avoid unnecessary
health-check round-trips.

```rust
pub struct ProviderRegistry {
    /// Ordered provider list per chain
    providers: HashMap<Chain, Vec<ProviderEntry>>,
    /// Privacy mode affects which providers are eligible
    privacy_mode: PrivacyMode,
    /// Cache: (chain, provider_name) → (HealthStatus, Instant)
    health_cache: Mutex<HashMap<(Chain, String), (HealthStatus, Instant)>>,
}

struct ProviderEntry {
    name: String,
    client: Arc<dyn BlockchainClient>,
    privacy_level: PrivacyLevel, // Local | Custom | Public
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivacyMode {
    /// Only local nodes (Bitcoin Core, Geth, etc.) — never call public APIs
    Strict,
    /// Local if available, custom RPC second, public API last
    Balanced,
    /// Always use the most reliable provider (usually public API)
    Convenience,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PrivacyLevel {
    Local   = 0, // localhost — no address data leaves machine
    Custom  = 1, // user-supplied RPC — address data goes to their server
    Public  = 2, // third-party API — address data goes to provider
}
```

### 6.2 Provider Selection Algorithm

```
For each provider in chain's ordered list (Local → Custom → Public):
  1. Skip if privacy_mode == Strict and privacy_level > Local
  2. Check health_cache — use cached result if < 60 seconds old
  3. Call health_check() — if reachable and block_height is recent: use it
  4. Log skip reason at DEBUG level

If no provider is healthy: return Err(CryptofolioError::NoProvider(chain))
```

### 6.3 Default Provider Order

| Chain | #1 (Local) | #2 (Custom) | #3 (Public) |
|-------|-----------|-------------|-------------|
| Bitcoin | Bitcoin Core RPC | Custom URL | Blockstream.info |
| Ethereum | Geth/Erigon RPC | Custom URL | Etherscan V2 |
| Cardano | Cardano Node / Ogmios | Custom URL | Blockfrost |
| Solana | Solana validator RPC | Custom URL | Helius (user key) |

**Solana has no public-API default.** Users must configure either a custom RPC or a
Helius API key. The registry returns an error with a helpful message if none is set.

### 6.4 Configuration (`config.toml`)

```toml
[blockchain.privacy_mode]
mode = "balanced"   # "strict" | "balanced" | "convenience"

[blockchain.bitcoin]
local_rpc = "http://127.0.0.1:8332"
local_rpc_user = "rpcuser"    # stored in keychain
local_rpc_pass = "rpcpass"    # stored in keychain

[blockchain.ethereum]
local_rpc = "http://127.0.0.1:8545"

[blockchain.cardano]
local_rpc = "http://127.0.0.1:1337"   # Ogmios

[blockchain.solana]
rpc_url = "https://mainnet.helius-rpc.com"   # user must provide
```

---

## 7. Rate Limiting

File: `src/blockchain/rate_limit.rs` (new)

### 7.1 Token Bucket Wrapper

```rust
pub struct RateLimitedClient<T: BlockchainClient> {
    inner: T,
    limiter: Arc<TokenBucket>,
}

struct TokenBucket {
    tokens: Mutex<f64>,
    max_tokens: f64,
    refill_rate: f64,      // tokens per second
    last_refill: Mutex<Instant>,
}

impl TokenBucket {
    /// Block until a token is available, then consume one.
    pub async fn acquire(&self) { /* tokio::time::sleep in loop */ }
}

#[async_trait]
impl<T: BlockchainClient + Send + Sync> BlockchainClient for RateLimitedClient<T> {
    fn provider_name(&self) -> &str { self.inner.provider_name() }
    async fn health_check(&self) -> Result<HealthStatus> { self.inner.health_check().await }
    async fn get_address_summary(&self, address: &str) -> Result<AddressSummary> {
        self.limiter.acquire().await;
        self.inner.get_address_summary(address).await
    }
    async fn get_transactions(&self, address: &str, since_block: Option<u64>) -> Result<Vec<WalletTransaction>> {
        self.limiter.acquire().await;
        self.inner.get_transactions(address, since_block).await
    }
}
```

### 7.2 Default Limits

| Provider | Rate Limit | Burst |
|----------|-----------|-------|
| Blockstream (no key) | 4 req/s | 10 |
| Etherscan (free tier) | 4 req/s | 10 |
| Etherscan (paid) | 10 req/s | 20 |
| Blockfrost (free) | 8 req/s | 16 |
| Helius (free) | 5 req/s | 10 |
| Local node | unlimited | — |

Limits are configurable via `config.toml` under `[blockchain.<chain>.rate_limit]`.

---

## 8. Security Model

### 8.1 Watch-Only Invariant

**No private key material must ever enter the Cryptofolio codebase.**

Enforcement:
- The `BlockchainClient` trait accepts only addresses and public keys (xpub).
- `validate_address_for_blockchain()` rejects inputs that look like WIF private keys
  (`5...`, `K...`, `L...` for Bitcoin; `0x` + 64-hex-chars for Ethereum).
- A regex pattern check runs on every `wallet add` input:
  ```rust
  // Detect common private key patterns
  const PRIVATE_KEY_PATTERNS: &[&str] = &[
      r"^5[HJK][1-9A-HJ-NP-Za-km-z]{49}$",   // Bitcoin WIF uncompressed
      r"^[KL][1-9A-HJ-NP-Za-km-z]{51}$",      // Bitcoin WIF compressed
      r"^0x[0-9a-fA-F]{64}$",                  // Ethereum raw private key
      r"^\[([0-9a-fA-F]{8}/.*)\][xyztuv]pub", // Descriptor with fingerprint — OK (public)
  ];
  ```

### 8.2 xpub Privacy

An xpub exposes the entire account's address space to the API provider. Mitigations:

1. **Keychain storage**: xpub is stored in the macOS Keychain, not in the SQLite DB.
   The DB stores only the derived addresses (up to `derive_count`).
2. **Address-level querying**: the sync engine queries individual derived addresses,
   not the xpub itself. This matches the behaviour already implemented in `xpub.rs`.
3. **Strict mode**: in `PrivacyMode::Strict`, derived addresses are queried against a
   local Bitcoin Core node, so no address data leaves the machine.

### 8.3 API Key Storage

All provider API keys are stored in the macOS Keychain using the existing
`security::keychain` module. Config file holds only non-secret metadata (URLs, network).

| Secret | Keychain Key |
|--------|-------------|
| Blockfrost API key | `cryptofolio.blockfrost.api_key` |
| Etherscan API key | `cryptofolio.etherscan.api_key` |
| Helius API key | `cryptofolio.helius.api_key` |
| Bitcoin Core RPC user | `cryptofolio.bitcoin_rpc.user` |
| Bitcoin Core RPC pass | `cryptofolio.bitcoin_rpc.pass` |

### 8.4 Audit Log

Every sync operation is recorded for tamper-evident provenance:

```sql
CREATE TABLE sync_audit_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    account_id  TEXT NOT NULL REFERENCES accounts(id),
    address     TEXT NOT NULL,      -- address queried
    chain       TEXT NOT NULL,      -- "bitcoin" | "ethereum" | "cardano" | "solana"
    provider    TEXT NOT NULL,      -- provider_name() used
    action      TEXT NOT NULL,      -- "balance_sync" | "tx_sync" | "health_check"
    records_in  INTEGER,            -- tx count returned by provider
    records_new INTEGER,            -- new records inserted
    error       TEXT,               -- NULL on success
    duration_ms INTEGER
);

CREATE INDEX sync_audit_log_account ON sync_audit_log(account_id, timestamp);
```

This log answers "what data came from where and when" — critical for debugging
discrepancies and demonstrating data provenance.

### 8.5 Address Privacy Threat Model

| Threat | Mitigation |
|--------|-----------|
| API provider learns address | `PrivacyMode::Strict` (local node only) |
| xpub derivation leaks all addresses | xpub in keychain, not DB |
| DB contains sensitive addresses | SQLite file at `0600` — local only |
| Man-in-the-middle API call | TLS enforced, no HTTP fallback |
| Log files contain addresses | Addresses logged at TRACE only, redacted at INFO |

---

## 9. Chain-Specific Implementations

### 9.1 Bitcoin (`BlockstreamClient`)

**Current:** functional
**Changes required:**
- Implement `BlockchainClient` trait (rename methods, add `since_block` support)
- Return `AddressSummary` / `WalletTransaction` instead of chain-specific structs
- Add `health_check()` using `/blocks/tip/height`
- Wrap in `RateLimitedClient` (4 req/s default)
- Support Bitcoin Core JSON-RPC as `PrivacyLevel::Local` alternative

### 9.2 Ethereum (`EtherscanClient`)

**Current:** functional
**Changes required:**
- Implement `BlockchainClient` trait
- Map `EthereumTransaction` → `WalletTransaction` with `TransactionDirection`
- Add `since_block` to `get_transactions` (Etherscan `startblock` param)
- Add `health_check()` using `eth_blockNumber` JSON-RPC module
- Support local Geth/Erigon via `BlockchainClient` local provider

### 9.3 Cardano (`BlockfrostClient`)

**Current:** functional
**Changes required:**
- Implement `BlockchainClient` trait
- Move `stake_address` / `stake_pool` to `ChainExtras::Cardano` in `AddressSummary`
- Add `since_block` filtering (Blockfrost `/addresses/{addr}/transactions?from={hash}`)
- Add `health_check()` using `/health` endpoint
- Wrap in `RateLimitedClient` (8 req/s default)

### 9.4 Solana (new — `SolanaRpcClient`)

See Section 10.

---

## 10. Solana Integration Design

File: `src/blockchain/solana/client.rs` (new)

### 10.1 Rationale for No Public Default

Solana's public RPC (`api.mainnet-beta.solana.com`) aggressively rate-limits unauthenticated
requests. Using it in production-grade tooling creates a poor user experience and risks
the endpoint blocking Cryptofolio's IP. Helius provides a generous free tier (10 req/s,
100k credits/day) and is the recommended provider. Users must opt-in.

### 10.2 SOL Balance

```
POST <rpc_url>
{"jsonrpc":"2.0","id":1,"method":"getBalance","params":["<pubkey>",{"commitment":"finalized"}]}
```
Response: `result.value` in lamports → divide by 10^9 for SOL.

### 10.3 SPL Token Balances

```
POST <rpc_url>
{
  "jsonrpc":"2.0","id":1,
  "method":"getTokenAccountsByOwner",
  "params":[
    "<pubkey>",
    {"programId":"TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"},
    {"encoding":"jsonParsed"}
  ]
}
```
Parse each account's `account.data.parsed.info`:
- `mint` → token mint address
- `tokenAmount.uiAmount` → human-readable balance
- `tokenAmount.decimals`

Resolve mint → ticker via **Jupiter token list cache** (fetched once, cached in memory
for the process lifetime, invalidated on `--refresh-tokens` flag).

### 10.4 Stake Accounts

```
POST <rpc_url>
{
  "jsonrpc":"2.0","id":1,
  "method":"getProgramAccounts",
  "params":[
    "Stake11111111111111111111111111111111111111111",
    {
      "filters":[{"memcmp":{"offset":44,"bytes":"<base58-pubkey>"}}],
      "encoding":"jsonParsed"
    }
  ]
}
```
Returns stake accounts whose `withdrawer` is the queried pubkey. For each:
- `lamports` → SOL amount
- `account.data.parsed.info.stake.delegation.voter` → validator vote account

### 10.5 Transaction History

```
POST <rpc_url>
{"jsonrpc":"2.0","id":1,"method":"getSignaturesForAddress",
 "params":["<pubkey>",{"limit":1000,"before":"<last_sig_if_paginating>"}]}
```
Paginate until empty. For each signature, fetch with `getTransaction` to determine
direction (debit vs credit).

### 10.6 Address Validation

Solana addresses are base58-encoded 32-byte Ed25519 public keys. Validation:
```rust
// src/blockchain/solana/address.rs
pub fn validate_address(address: &str) -> Result<()> {
    let decoded = bitcoin::base58::decode(address)
        .map_err(|_| CryptofolioError::InvalidAddress("Invalid base58".into()))?;
    if decoded.len() != 32 {
        return Err(CryptofolioError::InvalidAddress(
            format!("Invalid Solana address: expected 32 bytes, got {}", decoded.len())
        ));
    }
    Ok(())
}
```

### 10.7 `SolanaRpcClient` Struct

```rust
pub struct SolanaRpcClient {
    rpc_url: String,
    /// Resolved Jupiter token list: mint → (symbol, name, decimals)
    token_list: Arc<RwLock<HashMap<String, TokenInfo>>>,
}

impl SolanaRpcClient {
    pub fn new(rpc_url: String) -> Self { ... }
    async fn post_rpc<T: DeserializeOwned>(&self, method: &str, params: Value) -> Result<T> { ... }
    async fn ensure_token_list(&self) -> Result<()> { ... }
}

#[async_trait]
impl BlockchainClient for SolanaRpcClient {
    fn provider_name(&self) -> &str { "Solana RPC" }
    async fn health_check(&self) -> Result<HealthStatus> { ... }  // getSlot
    async fn get_address_summary(&self, address: &str) -> Result<AddressSummary> { ... }
    async fn get_transactions(&self, address: &str, since_block: Option<u64>) -> Result<Vec<WalletTransaction>> { ... }
    async fn get_chain_extras(&self, address: &str) -> Result<Option<ChainExtras>> {
        // Returns ChainExtras::Solana { stake_accounts: ... }
    }
}
```

---

## 11. Sync Engine Redesign

File: `src/blockchain/sync.rs` (new, replaces inline sync in `wallet.rs`)

### 11.1 Architecture

```rust
pub struct SyncEngine {
    registry: Arc<ProviderRegistry>,
    pool: SqlitePool,
}

pub struct SyncOptions {
    pub since_block: Option<u64>,
    pub dry_run: bool,
    pub full_history: bool,     // clear watermarks and re-fetch all
}

pub struct SyncReport {
    pub account_id: String,
    pub addresses_synced: usize,
    pub balances_updated: usize,
    pub transactions_new: usize,
    pub errors: Vec<SyncError>,
    pub duration: Duration,
}
```

### 11.2 Parallel Execution

```rust
impl SyncEngine {
    pub async fn sync_wallet(
        &self,
        account: &Account,
        addresses: Vec<String>,
        opts: SyncOptions,
    ) -> SyncReport {
        let mp = MultiProgress::new();
        let mut set = JoinSet::new();

        for address in addresses {
            let registry = Arc::clone(&self.registry);
            let pool = self.pool.clone();
            let pb = mp.add(ProgressBar::new_spinner());
            let opts = opts.clone();

            set.spawn(async move {
                sync_single_address(registry, pool, address, opts, pb).await
            });
        }

        let mut report = SyncReport::default();
        while let Some(result) = set.join_next().await {
            match result {
                Ok(Ok(r))  => report.merge(r),
                Ok(Err(e)) => report.errors.push(SyncError::from(e)),
                Err(e)     => report.errors.push(SyncError::JoinError(e.to_string())),
            }
        }
        report
    }
}
```

One `tokio::task` per address. A single address error doesn't abort the others.
The multi-progress bar shows per-address status (spinner + "Fetching BTC transactions…").

### 11.3 Watermarks

Block-height watermarks are stored in `binance_sync_state` (for exchanges) and
the new `wallet_sync_state` table for blockchain wallets:

```sql
CREATE TABLE wallet_sync_state (
    address         TEXT PRIMARY KEY,
    chain           TEXT NOT NULL,
    last_block      INTEGER,        -- last block height synced
    last_sync_at    DATETIME,
    updated_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

On incremental sync, `get_transactions(address, Some(last_block + 1))` is called.
On `--full-history`, the watermark row is deleted before syncing.

---

## 12. New Database Tables

Two new tables are required for v0.5.0 features. These are additive — no existing data
is touched. Add them to `src/db/migrations.rs` using the existing inline-SQL-constant
pattern.

| Table | Purpose |
|-------|---------|
| `wallet_sync_state` | Block-height watermarks for incremental sync |
| `sync_audit_log` | Provider, timing, and error audit trail |

The full `CREATE TABLE` statements are defined inline in Sections 11.3 and 8.4.

---

## 13. Implementation Checklist

### New Files

- [ ] `src/blockchain/types.rs` — unified type system
- [ ] `src/blockchain/trait.rs` — `BlockchainClient` trait
- [ ] `src/blockchain/rate_limit.rs` — `TokenBucket` + `RateLimitedClient`
- [ ] `src/blockchain/provider.rs` — `ProviderRegistry`
- [ ] `src/blockchain/sync.rs` — `SyncEngine` with `JoinSet`
- [ ] `src/blockchain/solana/mod.rs`
- [ ] `src/blockchain/solana/address.rs`
- [ ] `src/blockchain/solana/client.rs` — `SolanaRpcClient`

### Modified Files

- [ ] `src/blockchain/bitcoin/client.rs` — impl `BlockchainClient`
- [ ] `src/blockchain/ethereum/client.rs` — impl `BlockchainClient`
- [ ] `src/blockchain/cardano/client.rs` — impl `BlockchainClient`
- [ ] `src/blockchain/mod.rs` — re-export `types`, `trait`, `provider`, `sync`
- [ ] `src/db/migrations.rs` — add `wallet_sync_state` and `sync_audit_log` tables
- [ ] `src/cli/commands/wallet.rs` — use `SyncEngine` instead of inline sync
- [ ] `config.toml` schema — add `[blockchain.*]` sections

### Tests Required

- [ ] `BlockstreamClient` implements `BlockchainClient` — unit tests with `wiremock`
- [ ] `EtherscanClient` implements `BlockchainClient` — unit tests with `wiremock`
- [ ] `BlockfrostClient` implements `BlockchainClient` — unit tests with `wiremock`
- [ ] `SolanaRpcClient` — unit tests with `wiremock`
- [ ] `ProviderRegistry` — health-check selection, fallback, privacy mode filtering
- [ ] `RateLimitedClient` — rate limiting enforced, does not drop requests
- [ ] `SyncEngine` — parallel sync, error isolation, watermark update
- [ ] Address validation — all 4 chains with known-good and known-bad addresses

---

**Next action:** Begin Phase 1 — add `types.rs`, `trait.rs`, and the two new DB tables.
This document is the source of truth; update it if requirements change before implementing.

---

*Generated by Claude Code | cryptofolio v0.5.0 architecture | March 2026*
