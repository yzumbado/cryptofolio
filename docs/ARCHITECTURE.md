# Cryptofolio Architecture

**Version:** 0.5.0  
**Last Updated:** May 2026

This document describes the technical architecture of Cryptofolio, a local-first cryptocurrency portfolio manager built with Rust.

---

## Table of Contents

- [Overview](#overview)
- [System Layers](#system-layers)
- [Blockchain Wallet Architecture](#blockchain-wallet-architecture)
- [Data Flow](#data-flow)
- [Database Schema](#database-schema)
- [MCP Server](#mcp-server)
- [AI Interface Layer](#ai-interface-layer)
- [Security Architecture](#security-architecture)
- [Module Structure](#module-structure)
- [Technology Stack](#technology-stack)
- [Design Decisions](#design-decisions)

---

## Overview

```
┌───────────────────────────────────────────────────────────────────────┐
│                        AI INTERFACE LAYER                             │
│   Claude Code skill (/portfolio)    Cowork session                    │
│   ─────────────────────────────────────────────────────────────────   │
│                    MCP Server (TypeScript, stdio)                     │
│                  cryptofolio_* tools (18 tools)                       │
└───────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌───────────────────────────────────────────────────────────────────────┐
│                         USER INTERFACE                                │
│              CLI (clap)  —  cryptofolio <command> [args]              │
└───────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌───────────────────────────────────────────────────────────────────────┐
│                       COMMAND HANDLERS                                │
│  account │ wallet │ tx │ portfolio │ pnl │ sync │ audit │ config ...  │
└───────────────────────────────────────────────────────────────────────┘
                                  │
              ┌───────────────────┼───────────────────┐
              ▼                   ▼                   ▼
┌─────────────────────┐ ┌─────────────────┐ ┌─────────────────────────┐
│    CORE DOMAIN      │ │  BLOCKCHAIN      │ │    EXCHANGE             │
│  Account / Holding  │ │  ProviderRegistry│ │  Binance (Spot+Alpha)   │
│  Transaction        │ │  SyncEngine      │ │                         │
│  P&L / Tax Lots     │ │  BlockchainClient│ │                         │
│  Portfolio          │ │  Bitcoin/ETH/ADA │ │                         │
└─────────────────────┘ │  Solana clients  │ └─────────────────────────┘
              │         └─────────────────┘           │
              └───────────────────┬───────────────────┘
                                  ▼
┌───────────────────────────────────────────────────────────────────────┐
│                         PERSISTENCE                                   │
│          SQLite  (~/.config/cryptofolio/database.sqlite)              │
│  accounts │ holdings │ transactions │ tax_lots │ realized_pnl         │
│  wallet_addresses │ sync_audit_log │ blockchain_sync_state            │
└───────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌───────────────────────────────────────────────────────────────────────┐
│                       CONFIGURATION                                   │
│  config.toml  │  macOS Keychain (API keys, xpub, secrets)             │
└───────────────────────────────────────────────────────────────────────┘
```

---

## System Layers

### 1. AI Interface Layer

Added in v0.5.x. Two components:

**MCP Server** (`mcp/`) — TypeScript process connected to Claude via stdio transport. Exposes 18 `cryptofolio_*` tools that wrap CLI commands. Claude calls tools; the MCP server shells out to the installed `cryptofolio` binary and returns structured JSON responses.

**Portfolio Skill** (`.claude/skills/portfolio/SKILL.md`) — A Claude Code / Cowork skill invoked with `/portfolio`. On invocation it bootstraps portfolio context via parallel MCP calls, then acts as an expert data agent. See [AI Interface Layer](#ai-interface-layer) section below.

### 2. CLI Layer

Built with `clap` v4 derive macros. Subcommands:

| Command | Description |
|---|---|
| `account` | Manage accounts (exchanges, wallets) |
| `wallet` | Manage blockchain wallet addresses, sync |
| `tx` | Record transactions (buy/sell/transfer/swap) |
| `portfolio` | Aggregated holdings view with P&L |
| `pnl` | Realized / unrealized P&L detail |
| `holdings` | Per-account holdings management |
| `sync` | Exchange sync (Binance) |
| `audit` | Sync audit log, coverage, errors |
| `price` / `market` | Real-time prices |
| `config` | Configuration and secret management |
| `import` | CSV import |
| `status` | System diagnostics |

### 3. Core Domain Layer

**Account** — Exchange, hardware wallet, software wallet, custodial service, or bank. Each account has a `sync_enabled` flag and belongs to a category.

**Holding** — Asset position: `(account_id, asset) → (quantity, avg_cost_basis)`. Quantity is the **source of truth from the last sync** for synced accounts. For manual accounts, quantity is maintained by transaction recording.

**Transaction** — Immutable record of a financial event. Types: `Buy`, `Sell`, `Transfer In`, `Transfer Out`, `Transfer Internal`, `Swap`, `Receive`, `Fee`. Every transaction is timestamped with the actual event date (not insertion time).

**Tax Lot** — Created on every acquisition (Buy, Receive, Transfer In). Tracks `quantity`, `cost_per_unit`, `acquisition_date`, `remaining_quantity`, `fully_disposed`. FIFO disposal walks lots in acquisition-date order.

**P&L Calculator** — `process_acquisition()` creates tax lots; `process_disposal()` walks FIFO lots, records realized gains in `realized_pnl`, and updates `remaining_quantity` on consumed lots.

### 4. Blockchain Layer

See [Blockchain Wallet Architecture](#blockchain-wallet-architecture) below.

### 5. Persistence Layer

SQLite via `sqlx`. All queries use parameterized statements (no SQL injection risk). Schema migrations are inline in `src/db/migrations.rs` using a `_migrations` table for versioning.

### 6. Configuration Layer

- `~/.config/cryptofolio/config.toml` — user preferences, non-secret settings
- macOS Keychain — API keys, extended public keys (xpub), any sensitive values
- Environment variables — fallback for CI/headless environments

---

## Blockchain Wallet Architecture

### BlockchainClient Trait

All four chains implement a unified trait:

```rust
trait BlockchainClient {
    fn provider_name(&self) -> &str;
    async fn health_check(&self) -> Result<HealthStatus>;
    async fn get_address_summary(&self, address: &str) -> Result<AddressSummary>;
    async fn get_transactions(&self, address: &str, since_block: Option<u64>) -> Result<Vec<WalletTransaction>>;
    async fn get_chain_extras(&self, address: &str) -> Result<Option<ChainExtras>>;
}
```

Implementations:

| Chain | Client | Provider | Notes |
|---|---|---|---|
| Bitcoin | `BlockstreamClient` | Blockstream API | No API key required |
| Ethereum | `EtherscanClient` | Etherscan V2 API | Optional API key (higher rate limits) |
| Cardano | `BlockfrostClient` | Blockfrost API | API key required for metadata |
| Solana | `SolanaRpcClient` | JSON-RPC (Helius) | RPC URL required |

### ProviderRegistry

Routes sync requests to the correct client based on chain and privacy mode:

```
PrivacyMode::Convenience → allows Public-tier providers (Blockstream, Etherscan)
PrivacyMode::Balanced    → requires at least Custom-tier (API key)
PrivacyMode::Maximum     → local nodes only
```

### SyncEngine

Orchestrates parallel wallet sync across all addresses using `tokio::task::JoinSet`. For each address:
1. Calls `get_address_summary()` → updates `holdings` table with current balances
2. Optionally calls `get_transactions()` → imports transaction history
3. Records to `sync_audit_log` for tamper-evident provenance

### Wallet Sync Data Flow

```
cryptofolio wallet sync <name>
        │
        ▼
   wallet.rs reads API keys from config/keychain (fallback: env vars)
        │
        ▼
   ProviderRegistry selects healthy client for each chain
        │
        ▼
   SyncEngine runs JoinSet across all wallet addresses (parallel)
        │
        ├─▶ get_address_summary()
        │       │
        │       ├─▶ [BTC]  Blockstream: /address/{addr}
        │       ├─▶ [ETH]  Etherscan V2: action=balance + tokentx
        │       ├─▶ [ADA]  Blockfrost: /addresses/{addr} + /assets/{unit}
        │       └─▶ [SOL]  getBalance + getTokenAccountsByOwner
        │
        ▼
   holdings.set_quantity()  — overwrites balance with latest on-chain value
        │
        ▼
   sync_audit_log insert — timestamp, provider, records_in, records_new
```

### HD Wallet (xpub) Derivation

BIP32 external-chain address derivation from xpub/ypub/zpub:

| Prefix | Path | Address Type |
|---|---|---|
| `xpub` | BIP44 `m/44'/0'/0'` | Legacy P2PKH (`1...`) |
| `ypub` | BIP49 `m/49'/0'/0'` | Wrapped SegWit P2SH-P2WPKH (`3...`) |
| `zpub` | BIP84 `m/84'/0'/0'` | Native SegWit P2WPKH (`bc1q...`) |
| `xpub` + `--address-type taproot` | BIP86 `m/86'/0'/0'` | Taproot P2TR (`bc1p...`) |

xpub keys are stored in macOS Keychain (never in config.toml or the database).

### Cardano Token Handling

Blockfrost returns native token balances as raw integer amounts. The client:
1. Fetches `GET /assets/{unit}` for each token to get `metadata.decimals`
2. Divides raw quantity by `10^decimals` to get human-readable balance
3. Uses `metadata.ticker` or `metadata.name` as the display symbol

If the metadata fetch fails (no API key, rate limit), the token is **skipped with a warning** rather than stored with 0 decimals, to prevent phantom valuations.

---

## Data Flow

### Transaction Recording (Manual)

```
tx buy BTC 0.1 --account Binance --price 95000 --date 2025-12-25
        │
        ├─▶ Validate account exists
        ├─▶ Parse date → historical timestamp (not Utc::now())
        │
        ├─▶ [if NOT --cost-basis-only]
        │       holdings.add_quantity(account, BTC, 0.1, price)
        │
        ├─▶ transactions.insert(Buy, BTC, 0.1, $95k, timestamp=2025-12-25)
        │
        └─▶ pnl.process_acquisition(tx_id, BTC, 0.1, $95k, 2025-12-25, FIFO)
                └─▶ tax_lots.insert(qty=0.1, cost=$95k, acquired=2025-12-25)
```

**Important:** On synced accounts (accounts with blockchain wallet addresses), use `--cost-basis-only` when recording historical purchases. This creates the tax lot without inflating the holdings quantity, which is already managed by wallet sync.

### Wallet Sync vs Manual Holdings

| Account type | Holdings quantity managed by | Use `tx buy` for |
|---|---|---|
| **Synced** (has wallet address) | `wallet sync` (overwrites from chain) | Cost basis only (`--cost-basis-only`) |
| **Manual** (no wallet address) | Transaction history (add/remove) | Full balance tracking |

### P&L Disposal Flow

```
tx sell BTC 0.05 --account Binance --price 100000
        │
        ├─▶ transactions.insert(Sell, BTC, 0.05, $100k)
        │
        ├─▶ pnl.process_disposal(tx_id, BTC, 0.05, $100k, FIFO)
        │       │
        │       └─▶ Walk tax_lots ordered by acquisition_date ASC
        │               ├─▶ Lot 1: acquired 2025-12-25 @ $95k, qty 0.05
        │               │       → consume 0.05, gain = (100k-95k)*0.05 = $250
        │               │       → tax_lots.set remaining_qty=0, fully_disposed=true
        │               └─▶ realized_pnl.insert(gain=$250, method=FIFO)
        │
        └─▶ holdings.remove_quantity(account, BTC, 0.05)
```

---

## Database Schema

### Core Tables

```
categories ──< accounts ──< holdings        (account owns asset positions)
                    │
                    ├──< transactions        (financial events)
                    ├──< tax_lots            (acquisition records for FIFO)
                    ├──< realized_pnl        (disposed lot records)
                    └──< sync_audit_log      (tamper-evident sync history)

accounts ──< wallet_addresses ──< blockchain_sync_state
                                        (per-address sync cursor)
```

### Key Constraints

- `holdings`: `UNIQUE(account_id, asset)` — one row per (account, asset)
- `tax_lots`: sorted by `acquisition_date` for FIFO ordering
- `sync_audit_log.account_id`: `REFERENCES accounts(id) ON DELETE CASCADE`
- `blockchain_sync_state.wallet_address_id`: `REFERENCES wallet_addresses(id) ON DELETE CASCADE`
- `wallet_sync_state`: keyed by raw address string (chain-agnostic cursor)

### Migration System

Migrations are versioned inline in `src/db/migrations.rs`. Each migration is guarded by a check against the `_migrations` table. The current schema version is checked and applied at startup — no external migration tool required.

---

## MCP Server

Located in `mcp/`. TypeScript, built with `@modelcontextprotocol/sdk`.

**Transport:** stdio (Claude spawns the process directly)

**Tool inventory:**

| Tool | Underlying CLI command |
|---|---|
| `cryptofolio_list_accounts` | `account list --json` |
| `cryptofolio_manage_account` | `account add/remove` |
| `cryptofolio_get_portfolio` | `portfolio --json` |
| `cryptofolio_get_pnl_summary` | `pnl summary --json` |
| `cryptofolio_get_unrealized_pnl` | `pnl unrealized --json` |
| `cryptofolio_get_realized_pnl` | `pnl realized --json` |
| `cryptofolio_analyze_asset` | `pnl asset <sym> --json` |
| `cryptofolio_list_transactions` | `tx list --json` |
| `cryptofolio_record_transaction` | `tx buy/sell/transfer` |
| `cryptofolio_track_conversion` | `tx swap` |
| `cryptofolio_export_transactions` | `tx export` |
| `cryptofolio_manage_wallet` | `wallet add/remove` |
| `cryptofolio_sync_wallet` | `wallet sync` |
| `cryptofolio_sync_exchange` | `sync binance` |
| `cryptofolio_get_prices` | `price <symbols> --json` |
| `cryptofolio_get_market_data` | `market <symbol> --json` |
| `cryptofolio_get_system_status` | `status --json` |
| `cryptofolio_get_audit_log` | `audit sync --json` |

**Installation:** See `mcp/README.md`. The MCP server binary path and `CRYPTOFOLIO_BIN` env var are configured in `~/Library/Application Support/Claude/claude_desktop_config.json`.

---

## AI Interface Layer

### Portfolio Skill

`.claude/skills/portfolio/SKILL.md` — invoked with `/portfolio` in Claude Code or Cowork.

**Bootstrap sequence** (parallel on invocation):
1. `cryptofolio_list_accounts` → which wallets/exchanges exist
2. `cryptofolio_get_portfolio` → current balances

**Expert behaviors:**
- Always shows cost basis alongside current value
- Flags stale wallet data before quoting numbers
- Refreshes context after any mutation (sync, tx, wallet add)
- Surfaces realized P&L on every sale
- Scope: data management only — no investment advice

**Onboarding:** If no accounts exist, walks the user through adding wallets conversationally.

### Companion Agent Pattern

The portfolio skill is the **data layer** for future companion agents (trader, portfolio manager). Consumer agents call MCP tools directly — the same `cryptofolio_*` interface. The portfolio skill adds judgment and synthesis on top of raw data. A formal snapshot schema (JSON contract for consumer agents) will be designed when the first companion agent is built.

---

## Security Architecture

### Credential Storage

| Credential | Storage | Never stored in |
|---|---|---|
| Etherscan API key | macOS Keychain | config.toml, DB, env |
| Blockfrost API key | macOS Keychain | config.toml, DB, env |
| Solana RPC URL | macOS Keychain | config.toml, DB, env |
| Binance API key | macOS Keychain | config.toml, DB, env |
| xpub / extended keys | macOS Keychain | config.toml, DB, env |

Fallback for CI/headless: environment variables (`ETHERSCAN_API_KEY`, `BLOCKFROST_API_KEY`, `SOLANA_RPC_URL`).

### Private Key Detection

The security module (`src/blockchain/security.rs`) scans all user input for private key patterns (WIF, hex, mnemonic). Private keys are rejected before storage with a clear error.

### Sync Audit Log

Every sync operation is recorded in `sync_audit_log` with: timestamp, account, address (truncated), chain, provider, action, records_in, records_new, error, duration_ms. Audit records cascade-delete with the account — they are never orphaned.

### Network

- HTTPS only for all external API calls
- HMAC-SHA256 for Binance API authentication
- No telemetry, no cloud storage, no third-party analytics

---

## Module Structure

```
src/
├── blockchain/              # On-chain data (v0.5)
│   ├── bitcoin/
│   │   ├── client.rs       # Blockstream API client
│   │   ├── xpub.rs         # BIP32 HD address derivation (xpub/ypub/zpub/taproot)
│   │   └── address.rs
│   ├── ethereum/
│   │   ├── client.rs       # Etherscan V2 client
│   │   └── address.rs
│   ├── cardano/
│   │   ├── client.rs       # Blockfrost client (balance + native tokens + metadata)
│   │   └── address.rs
│   ├── solana/
│   │   ├── client.rs       # Solana JSON-RPC client (Helius)
│   │   └── address.rs
│   ├── trait_def.rs        # BlockchainClient trait
│   ├── types.rs            # AddressSummary, WalletTransaction, WalletBalance, Chain
│   ├── provider.rs         # ProviderRegistry, PrivacyMode routing
│   ├── sync.rs             # SyncEngine (JoinSet parallel sync)
│   └── security.rs         # Private key detection
│
├── cli/                    # Command-line interface
│   ├── commands/
│   │   ├── account.rs      # account add/remove/show/list
│   │   ├── wallet.rs       # wallet add/remove/sync + API key setup
│   │   ├── tx.rs           # tx buy/sell/transfer/swap/list/export
│   │   ├── portfolio.rs    # portfolio view
│   │   ├── pnl.rs          # realized/unrealized P&L
│   │   ├── holdings.rs     # holdings management
│   │   ├── audit.rs        # audit sync/coverage/errors
│   │   ├── sync.rs         # exchange sync (Binance)
│   │   ├── import.rs       # CSV import
│   │   ├── config.rs       # config management + keychain
│   │   ├── status.rs       # system diagnostics
│   │   └── ...
│   ├── mod.rs              # CLI structure, clap definitions, AccountTypeArg
│   └── output.rs           # Formatting utilities
│
├── core/                   # Domain models
│   ├── account.rs          # Account, AccountType, AccountConfig
│   ├── holdings.rs         # Holding model
│   ├── transaction.rs      # Transaction, TransactionType
│   ├── currency.rs         # Currency, ExchangeRate
│   ├── portfolio.rs        # Portfolio aggregation
│   └── pnl/
│       ├── calculator.rs   # PnLCalculator (FIFO process_acquisition/disposal)
│       └── mod.rs
│
├── db/                     # Persistence
│   ├── migrations.rs       # Inline schema migrations (versioned)
│   ├── accounts.rs         # AccountRepository
│   ├── holdings.rs         # HoldingRepository (set_quantity, add_quantity)
│   ├── transactions.rs     # TransactionRepository
│   ├── tax_lots.rs         # TaxLotRepository
│   ├── realized_pnl.rs     # RealizedPnlRepository
│   ├── sync_state.rs       # SyncState repository
│   └── ...
│
├── exchange/               # Exchange integrations
│   └── binance/
│       ├── client.rs       # REST + WebSocket client
│       ├── sync.rs         # Balance + trade sync
│       └── import.rs       # CSV trade history import
│
├── config/                 # Configuration
│   ├── settings.rs         # config.toml parsing
│   ├── secrets.rs          # Secret key detection/validation
│   ├── keychain.rs         # Keychain abstraction
│   └── keychain_macos.rs   # macOS Security framework integration
│
├── error.rs                # CryptofolioError enum
├── lib.rs                  # Library entry point (pub re-exports)
└── main.rs                 # Binary entry point

mcp/                        # MCP server (TypeScript)
├── src/
│   ├── index.ts            # Server entry, tool registration
│   └── tools/              # One file per tool group
└── dist/                   # Built output (node dist/index.js)

.claude/
└── skills/
    └── portfolio/
        └── SKILL.md        # /portfolio Claude Code + Cowork skill
```

---

## Technology Stack

| Component | Technology | Version | Rationale |
|---|---|---|---|
| Language | Rust | 1.93.0 (pinned) | Memory safety, single binary, performance |
| CLI | clap | 4.x | Derive macros, subcommands |
| Async | Tokio | 1.x | JoinSet for parallel sync |
| HTTP | reqwest | 0.11.x | Async, TLS |
| Database | SQLite + sqlx | 0.7.x | Embedded, compile-time queries |
| Decimals | rust_decimal | 1.x | Financial precision |
| Bitcoin | bitcoin crate | 0.31.x | BIP32 derivation, address types |
| Crypto | blake2 + bech32 | - | Cardano CIP-14 fingerprints |
| MCP Server | TypeScript + @modelcontextprotocol/sdk | - | Stdio transport |
| Serialization | serde / serde_json | 1.x | JSON output |
| Datetime | chrono | 0.4.x | Timezone-aware timestamps |
| Keychain | Security.framework | - | macOS native credential store |

---

## Design Decisions

### Holdings Ownership: Sync vs Manual

Synced accounts (those with blockchain wallet addresses) have their holdings quantity managed entirely by `wallet sync`. Manually recording a `tx buy` on a synced account for cost basis purposes must use `--cost-basis-only` to avoid double-counting. Manual accounts (no wallet address) derive quantity from cumulative transaction history.

### FIFO Tax Lots

Every acquisition creates a tax lot. Disposals (Sell, Transfer Out) walk lots in acquisition-date order, reducing `remaining_quantity` and recording realized P&L. This enables accurate capital gains tracking including short-term vs long-term classification.

### Local-First

All data lives in SQLite at `~/.config/cryptofolio/database.sqlite`. No cloud sync, no telemetry. The user owns their data.

### Read-Only Exchange Access

Binance integration is read-only (balances, trade history). No trading or withdrawal capability — scoped to portfolio tracking only.

### Secrets in Keychain, Never in Files

API keys and extended public keys are stored in macOS Keychain. Config files and the database never contain credentials. Environment variables are an explicit fallback for CI environments only.

---

*For the data model detail, see [DATA_MODEL.md](DATA_MODEL.md).*  
*For the wallet sync architecture detail, see [WALLET_ARCHITECTURE_v0.5.0.md](WALLET_ARCHITECTURE_v0.5.0.md).*
