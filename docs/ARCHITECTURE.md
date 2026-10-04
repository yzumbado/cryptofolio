# Cryptofolio Architecture

**Version:** 0.6.0  
**Last Updated:** October 2026

This document describes the technical architecture of Cryptofolio, a local-first,
watch-only cryptocurrency portfolio manager built with Rust.

---

## Table of Contents

- [Overview](#overview)
- [System Layers](#system-layers)
- [Command Surface](#command-surface)
- [Core Domain Layer](#core-domain-layer)
- [Blockchain Layer](#blockchain-layer)
- [Data Flow](#data-flow)
- [Persistence](#persistence)
- [MCP Server](#mcp-server)
- [Agent Integration](#agent-integration)
- [Security Architecture](#security-architecture)
- [Module Structure](#module-structure)
- [Technology Stack](#technology-stack)
- [Design Decisions](#design-decisions)

---

## Overview

```
┌───────────────────────────────────────────────────────────────────────┐
│                         AGENT LAYER                                   │
│   DSH workspace (AGENTS.md + .dsh/skills/)  ·  /portfolio skill       │
│   ─────────────────────────────────────────────────────────────────   │
│                MCP Server (TypeScript, stdio)                         │
│              cryptofolio_* tools (23 tools)                           │
└───────────────────────────────────────────────────────────────────────┘
                                  │  spawns
                                  ▼
┌───────────────────────────────────────────────────────────────────────┐
│                         USER INTERFACE                                │
│              CLI (clap)  —  cryptofolio <command> [args]              │
│              + interactive shell (cryptofolio shell)                  │
└───────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌───────────────────────────────────────────────────────────────────────┐
│                       COMMAND HANDLERS                                │
│  price │ market │ account │ category │ holdings │ portfolio │ tx      │
│  mining-pnl │ sync │ sync-history │ import │ import-binance │ config  │
│  currency │ pnl │ wallet │ audit │ shell │ status                     │
└───────────────────────────────────────────────────────────────────────┘
                                  │
              ┌───────────────────┼───────────────────┐
              ▼                   ▼                   ▼
┌─────────────────────┐ ┌─────────────────┐ ┌─────────────────────────┐
│    CORE DOMAIN      │ │   BLOCKCHAIN     │ │    EXCHANGE             │
│  Account / Holding  │ │  ProviderRegistry│ │  Binance (Spot+Alpha)   │
│  Transaction        │ │  SyncEngine      │ │                         │
│  P&L / Tax Lots     │ │  BlockchainClient│ │                         │
│  Mining / Portfolio │ │  BTC/ETH/ADA/SOL │ │                         │
│                     │ │  /TAO clients    │ │                         │
└─────────────────────┘ └─────────────────┘ └─────────────────────────┘
              │                   │                   │
              └───────────────────┼───────────────────┘
                                  ▼
┌───────────────────────────────────────────────────────────────────────┐
│                         PERSISTENCE                                   │
│     SQLite via sqlx (default: <config dir>/database.sqlite)           │
│  accounts │ holdings │ transactions │ tax_lots │ realized_pnl         │
│  wallet_addresses │ wallet_sync_state │ binance_sync_state            │
│  sync_audit_log │ + supporting tables (see DATA_MODEL.md)             │
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

### 1. Agent Layer

The agent layer reaches Cryptofolio through the CLI — directly, or via the MCP
server which shells out to it:

- **DSH workspace** — `AGENTS.md` is auto-loaded each session and points at
  `STATE.md` and the skills under `.dsh/skills/` (`working-discipline`,
  `plan-before-build`, `co-author-review`, `cryptofolio-conventions`). DSH is the
  primary working mode; Claude Code / Kiro sessions may still occur.
- **MCP server** (`mcp/`) — a TypeScript process connected over stdio. It exposes
  23 `cryptofolio_*` tools that shell out to the configured `cryptofolio` binary
  (`CRYPTOFOLIO_BIN`, falling back to `PATH`) and return structured JSON.
- **`/portfolio` skill** (`.claude/skills/portfolio/SKILL.md`) — legacy Claude
  Code / Cowork entry point; bootstraps via the same MCP tools.

See [MCP Server](#mcp-server) and [Agent Integration](#agent-integration).

### 2. CLI Layer

Built with `clap` v4 derive macros (`src/cli/mod.rs`). `src/main.rs` parses the
command, initializes the SQLite pool, and dispatches to a handler. Global flags
apply to every command: `--no-color`, `--testnet`, `--json`, `--quiet`,
`--verbose`. `--testnet` is also set by the `CRYPTOFOLIO_TESTNET` env var.

### 3. Core Domain Layer

- **Account** — Exchange, hardware wallet, software wallet, custodial service, or
  bank. Belongs to a category; carries a `sync_enabled` flag and an `archived`
  soft-delete flag. Removal archives, never deletes.
- **Holding** — Asset position `(account_id, asset) → (quantity, avg_cost_basis)`.
  For synced accounts the quantity is set by sync; for manual accounts it is
  maintained by transaction recording.
- **Transaction** — Immutable row in an append-only ledger. 13 `tx_type` values
  including `correction`; see DATA_MODEL.md for the full list.
- **Tax Lot** — Created per acquisition; tracks `remaining_quantity` and
  `cost_basis_method` (lowercase: `fifo`, `lifo`, `average`). Disposal walks lots
  in acquisition-date order (FIFO).
- **P&L Calculator** — `process_acquisition()` creates lots; `process_disposal()`
  consumes FIFO lots and records rows in `realized_pnl`.
- **Mining** — `mining-pnl` models DePIN token revenue and hardware depreciation
  (see `docs/MINING_ASSET_ACCOUNTING.md`).

### 4. Blockchain Layer

See [Blockchain Layer](#blockchain-layer).

### 5. Persistence Layer

SQLite via `sqlx`. All queries are parameterized. The schema is defined inline in
`src/db/schema.rs` and applied with `CREATE TABLE IF NOT EXISTS` on every startup.
There is no migration system (see [Persistence](#persistence)).

### 6. Configuration Layer

- `config.toml` in the OS config directory (`AppConfig::config_path()`) — user
  preferences and non-secret settings, including `ai.*` provider config.
- macOS Keychain — API keys, extended public keys (xpub), and secrets.
- Environment variables — documented fallback for CI/headless use.

---

## Command Surface

Top-level commands (`src/cli/mod.rs`, dispatched in `src/main.rs`):

| Command | Description |
|---|---|
| `price <symbols...>` | Current spot prices |
| `market <symbol> [--24h]` | Detailed market data, optional 24h stats |
| `account` | `list` / `add` / `remove` / `show` / `address` |
| `category` | `list` / `add` / `rename` / `remove` |
| `holdings` | `list` / `add` / `remove` / `set` / `move` |
| `portfolio` | Holdings with P&L; `--by-account`, `--by-category`, `--account`, `--category` |
| `tx` | `list` / `buy` / `sell` / `transfer` / `swap` / `export` |
| `mining-pnl` | DePIN mining P&L statement |
| `sync [--account]` | Exchange balance/trade sync (Binance) |
| `sync-history --account` | Full Binance history import (trades, deposits, withdrawals, fiat, transfers) |
| `import <file> --account` | CSV import |
| `import-binance <file> --account` | Binance CSV/ZIP export import |
| `config` | `show` / `set` / `set-secret` / `use-testnet` / `use-mainnet` / keychain management |
| `currency` | `list` / `show` / `add` / `remove` / `toggle` / `set-rate` / `show-rate` |
| `pnl` | `summary` / `realized` / `unrealized` / `by-asset` / `backfill` |
| `wallet` | `add` / `list` / `show` / `sync` / `remove` |
| `audit` | `sync` / `coverage` / `errors` |
| `shell` | Interactive shell (history, tab-completion) |
| `status [--check]` | System diagnostics |

Account types (`AccountTypeArg`): `exchange`, `hardware_wallet`,
`software_wallet`, `custodial_service`, `bank`.

---

## Core Domain Layer

### Transaction types

`TransactionType` (Rust) and the `tx_type` CHECK constraint agree on 13 values:

`buy`, `sell`, `transfer_in`, `transfer_out`, `transfer_internal`, `swap`,
`stake`, `unstake`, `earn`, `receive`, `fee`, `airdrop`, `correction`.

`from_str` accepts aliases (`deposit → transfer_in`, `withdrawal`/`send →
transfer_out`, `transfer → transfer_internal`, `trade → swap`, `reward`/
`interest → earn`, `incoming → receive`), but only the canonical 13 are stored.

### FIFO cost basis

Every acquisition creates a tax lot. Disposals (`sell`, `transfer_out`, `fee`,
`unstake`) walk open lots ordered by `acquisition_date` ASC, decrement
`remaining_quantity`, set `fully_disposed`, and insert a `realized_pnl` row per
consumed lot. `cost_basis_method` is stored lowercase.

### Append-only ledger

`transactions` rows are never updated or deleted — two SQLite triggers
(`trg_transactions_no_update`, `trg_transactions_no_delete`) abort either
operation. Corrections are new rows with `tx_type = 'correction'`. Accounts are
archived instead of deleted. See DATA_MODEL.md for the correction pattern.

---

## Blockchain Layer

### BlockchainClient trait

All chains implement one trait (`src/blockchain/trait_def.rs`):
`provider_name()`, `health_check()`, `get_address_summary()`,
`get_transactions()`, and an optional `get_chain_extras()`.

| Chain | Client | Provider |
|---|---|---|
| Bitcoin | `BlockstreamClient` | Blockstream API |
| Ethereum | `EtherscanClient` | Etherscan V2 API |
| Cardano | `BlockfrostClient` | Blockfrost API |
| Solana | `SolanaRpcClient` | JSON-RPC |
| Bittensor | Taostats-backed client | Taostats API (`free + staked` TAO) |

### ProviderRegistry

Routes sync requests to an eligible provider tier (`src/blockchain/provider.rs`):

```
PrivacyMode::Strict      → Local only
PrivacyMode::Balanced    → Local → Custom (no public API)
PrivacyMode::Convenience → Local → Custom → Public
```

Tiers are ordered `Local (0) → Custom (1) → Public (2)`; results are cached for
60 seconds.

### SyncEngine

Orchestrates parallel wallet sync across all addresses with
`tokio::task::JoinSet`. For each address it fetches an address summary, updates
`holdings`, optionally imports transaction history, advances the
`wallet_sync_state` watermark, and records to `sync_audit_log`. Deduplication is
enforced by the `UNIQUE(tx_hash)` / `UNIQUE(external_id)` constraints rather than
an EXISTS pre-check.

### HD wallet (xpub) derivation

BIP32 external-chain derivation from xpub/ypub/zpub, plus Taproot:

| Prefix | Path | Address type |
|---|---|---|
| `xpub` | BIP44 `m/44'/0'/0'` | Legacy P2PKH |
| `ypub` | BIP49 `m/49'/0'/0'` | Wrapped SegWit P2SH-P2WPKH |
| `zpub` | BIP84 `m/84'/0'/0'` | Native SegWit P2WPKH |
| `xpub` + `--address-type taproot` | BIP86 `m/86'/0'/0'` | Taproot P2TR |

---

## Data Flow

### Manual transaction recording

```
tx buy BTC 0.1 --account Binance --price 95000 --date 2025-12-25
        │
        ├─▶ validate account exists
        ├─▶ parse date → event timestamp (not Utc::now())
        ├─▶ [unless --cost-basis-only] holdings.add_quantity(...)
        ├─▶ transactions.insert(Buy, ..., timestamp=2025-12-25)
        └─▶ pnl.process_acquisition(...) → tax_lots row
```

**Important:** on synced accounts (those with wallet addresses), use
`--cost-basis-only` when recording historical purchases. It creates the tax lot
without inflating the holdings quantity that sync already manages.

### Wallet sync vs manual holdings

| Account kind | Holdings quantity managed by | Use `tx buy` for |
|---|---|---|
| **Synced** (has wallet address) | `wallet sync` (overwrites from chain) | Cost basis only |
| **Manual** (no wallet address) | Transaction history | Full balance tracking |

### P&L disposal

`tx sell` inserts the transaction, calls `process_disposal()` to walk open FIFO
lots and write `realized_pnl` rows, then calls `holdings.remove_quantity()`.

---

## Persistence

Default database path is `<config dir>/database.sqlite`
(`AppConfig::database_path()`); the `CRYPTOFOLIO_DB` env var overrides it. The
live machine-specific path is not published. A second, stale
`~/.config/cryptofolio` copy may exist on a given machine — always confirm which
database you are touching before a destructive operation.

`src/db/schema.rs` is the single source of truth. Every statement is
`IF NOT EXISTS`, so `schema::create()` is idempotent. There are **no versioned
migrations** and no `_migrations` table: to reset the schema during development,
delete the database file and restart (or point `CRYPTOFOLIO_DB` at a throwaway
file). Versioned migrations are planned for v1.0.

The schema enables `PRAGMA journal_mode=WAL` and `PRAGMA foreign_keys=ON`. Eight
tables carry CHECK-constrained enums: `currencies.asset_type`, `transactions`
(`tx_type`, `source`, `trust_level`), `tax_lots.cost_basis_method`,
`address_registry.classification`/`classified_by`, `discovery_queue.status`,
`reconciliation_log.status`, `keychain_keys.storage_type`/`security_level`, and
`blockchain_nodes.node_type`. See DATA_MODEL.md for every table and column.

---

## MCP Server

Located in `mcp/`. TypeScript, built with `@modelcontextprotocol/sdk`.

**Transport:** stdio.

**Binary resolution:** `CRYPTOFOLIO_BIN`, else `cryptofolio` on `PATH`.
`runCli()` appends `--json --quiet`; `runCliRaw()` appends `--quiet` only.

**Tool inventory (23):**

| Tool | Underlying CLI command |
|---|---|
| `cryptofolio_get_system_status` | `config show` + `account list` |
| `cryptofolio_list_accounts` | `account list` |
| `cryptofolio_manage_account` | `account add` / `account remove` (+ `category add`) |
| `cryptofolio_get_portfolio` | `portfolio` |
| `cryptofolio_get_pnl_summary` | `pnl summary` |
| `cryptofolio_get_realized_pnl` | `pnl realized` |
| `cryptofolio_get_unrealized_pnl` | `pnl unrealized` |
| `cryptofolio_analyze_asset` | `pnl by-asset` |
| `cryptofolio_list_transactions` | `tx list` |
| `cryptofolio_record_transaction` | `tx buy` / `tx sell` / `tx transfer` / `tx swap` |
| `cryptofolio_track_conversion` | `tx swap` (one per step) |
| `cryptofolio_export_transactions` | `tx export` |
| `cryptofolio_manage_wallet` | `wallet add` / `list` / `show` / `remove` |
| `cryptofolio_sync_wallet` | `wallet sync` |
| `cryptofolio_sync_exchange` | `sync` |
| `cryptofolio_get_prices` | `price` |
| `cryptofolio_get_market_data` | `market` |
| `cryptofolio_get_audit_log` | `audit sync` / `coverage` / `errors` |
| `cryptofolio_get_sync_history` | `audit sync` |
| `cryptofolio_list_holdings` | `holdings list` |
| `cryptofolio_get_mining_pnl` | `mining-pnl` |
| `cryptofolio_pnl_backfill` | `pnl backfill --yes` |
| `cryptofolio_import_binance` | `import-binance` |

Rebuild `mcp/dist/` with `npm run build` after changing `mcp/src/`.

---

## Agent Integration

**DSH (primary).** `AGENTS.md` auto-loads each session and directs the agent to
`STATE.md` and the `.dsh/skills/` catalog. The `integrations/dsh-mcp/` bundle is a
configuration-only connector that wires the MCP server into a DSH profile over
stdio; tools then surface as `mcp__cryptofolio__*`. Its local patch file
(`cordis.patch.yml`) is gitignored because it contains machine paths.

**`/portfolio` skill.** `.claude/skills/portfolio/SKILL.md` remains for Claude
Code / Cowork. On invocation it calls `cryptofolio_list_accounts` and
`cryptofolio_get_portfolio` in parallel, then acts as a data-management expert:
cost basis always shown with value, data freshness checked before quoting,
context refreshed after mutations, and `cost_basis_only: true` enforced for buys
on synced accounts. It does not give investment advice.

**AI provider config.** `config.toml` supports an `ai` section (mode
`online` / `offline` / `hybrid` / `disabled`, Claude and Ollama settings). This is
diagnostic only: `status` reports provider availability, but there is no
AI-driven CLI command or Rust AI module.

---

## Security Architecture

### Credential storage

| Credential | Storage |
|---|---|
| Binance API key / secret | macOS Keychain (TOML fallback on other platforms) |
| Etherscan / Blockfrost / Solana / Taostats keys | macOS Keychain |
| Extended public keys (xpub) | macOS Keychain |

The macOS backend shells out to the system `security` tool
(`src/config/keychain_security_cli.rs`) rather than linking Security.framework
FFI directly, so it needs no code-signing entitlement. `src/config/keychain.rs`
holds the platform-independent abstraction. `keychain_keys` stores metadata only
— never the secret value.

### Private key detection

`src/blockchain/security.rs` scans user input for private-key patterns (WIF, hex,
mnemonic) and rejects them before storage. Cryptofolio is watch-only: no signing,
no broadcasting.

### Sync audit log

Every sync operation is recorded in `sync_audit_log` (timestamp, account,
address, chain, provider, action, records_in, records_new, error, duration_ms).
Rows reference `accounts(id)` and are retained because accounts are archived, not
deleted.

### Network

- HTTPS only for external API calls
- HMAC-SHA256 for Binance authentication
- No telemetry, no cloud storage, no third-party analytics

---

## Module Structure

```
src/
├── blockchain/              # On-chain data
│   ├── bitcoin/             # Blockstream client, xpub derivation, address
│   ├── ethereum/            # Etherscan V2 client, address
│   ├── cardano/             # Blockfrost client, address
│   ├── solana/              # JSON-RPC client, address
│   ├── bittensor/           # Taostats client (staked + free TAO)
│   ├── trait_def.rs         # BlockchainClient trait
│   ├── types.rs             # Chain, AddressSummary, WalletTransaction, ...
│   ├── provider.rs          # ProviderRegistry, PrivacyMode/PrivacyLevel
│   ├── sync.rs              # SyncEngine (JoinSet parallel sync)
│   └── security.rs          # Private key detection
│
├── cli/                     # Command-line interface
│   ├── commands/            # One module per command group
│   ├── mod.rs               # clap definitions, Commands enum, AccountTypeArg
│   ├── notifications.rs     # SystemStatus / ProviderStatus
│   └── output.rs            # Formatting utilities
│
├── core/                    # Domain models
│   ├── account.rs           # Account, AccountType
│   ├── holdings.rs          # Holding model
│   ├── transaction.rs       # Transaction, TransactionType (13 values)
│   ├── currency.rs          # Currency, ExchangeRate
│   ├── portfolio.rs         # Portfolio aggregation
│   ├── mining.rs            # DePIN mining P&L model
│   ├── defi.rs / dexscreener.rs / pricing.rs
│   └── pnl/                 # PnLCalculator (FIFO acquisition/disposal)
│
├── db/                      # Persistence
│   ├── schema.rs            # Inline schema (single source of truth)
│   ├── accounts.rs          # AccountRepository (incl. archive)
│   ├── holdings.rs          # HoldingRepository (set/add/remove_quantity)
│   ├── transactions.rs      # TransactionRepository
│   ├── tax_lots.rs          # TaxLotRepository
│   ├── realized_pnl.rs      # RealizedPnLRepository
│   ├── sync_state.rs        # Binance + wallet watermark repositories
│   ├── address_registry.rs / discovery_queue.rs / currencies.rs / keychain.rs
│   └── SPEC.md
│
├── exchange/                # Exchange integrations
│   └── binance/             # client, sync, CSV import, endpoints, alpha
│
├── config/                  # Configuration
│   ├── settings.rs          # config.toml parsing, paths, AiConfig
│   ├── secrets.rs           # secret detection/validation
│   ├── keychain.rs          # Keychain storage abstraction
│   ├── keychain_security_cli.rs  # macOS backend (system `security` tool)
│   └── migration.rs         # TOML → Keychain migration
│
├── shell/                   # Interactive shell (completer, shortcuts, context)
├── error.rs                 # CryptofolioError enum
├── lib.rs                   # Module map / re-exports
└── main.rs                  # Binary entry point and dispatch

mcp/                         # MCP server (TypeScript)
├── src/index.ts             # Server entry, registers 23 tools
├── src/tools/               # One module per tool group
├── src/cli.ts               # CLI wrapper (CRYPTOFOLIO_BIN, --json/--quiet)
├── tests/ · evals/          # Vitest tests and evals
└── dist/                    # Built output

integrations/dsh-mcp/        # DSH MCP bundle (config-only, stdio)

.dsh/skills/                 # DSH skills (working-discipline, plan-before-build, ...)
.claude/skills/portfolio/    # /portfolio skill
AGENTS.md · STATE.md         # Auto-loaded workspace instructions and current facts
```

---

## Technology Stack

From `Cargo.toml` (package version 0.6.0) and `mcp/package.json`:

| Component | Technology | Version |
|---|---|---|
| Language | Rust | edition 2021 |
| CLI | clap (derive) | 4.x |
| Async | Tokio | 1.x |
| HTTP | reqwest | 0.12 |
| Database | SQLite + sqlx | 0.8 |
| Decimals | rust_decimal | 1.x |
| Bitcoin | bitcoin crate | 0.32 |
| Crypto | blake2 + bech32 | - |
| Errors | thiserror | 2.x |
| Serialization | serde / serde_json / toml | 1.x / 1.x / 0.8 |
| Datetime | chrono | 0.4.x |
| MCP server | TypeScript + @modelcontextprotocol/sdk | - |

---

## Design Decisions

### Holdings ownership: sync vs manual

Synced accounts have their holdings quantity managed by `wallet sync`. Recording
a `tx buy` on a synced account for cost-basis purposes must use
`--cost-basis-only` to avoid double-counting. Manual accounts derive quantity from
cumulative transaction history.

### FIFO tax lots

Every acquisition creates a tax lot; disposals walk lots in acquisition-date
order, reducing `remaining_quantity` and recording realized P&L. `cost_basis_method`
is stored lowercase.

### Append-only ledger

The ledger is immutable at the engine level (SQLite triggers on `transactions`).
Corrections are new `tx_type='correction'` rows; accounts are archived rather than
deleted.

### Local-first

All data lives in a local SQLite file. No cloud sync, no telemetry.

### Read-only exchange access

Binance integration is read-only (balances, trades, history). No trading or
withdrawal capability — watch-only by design.

### Secrets in Keychain, never in files

API keys and extended public keys live in macOS Keychain (with a TOML fallback on
other platforms). Config files and the database never contain credentials.

---

*For the data model detail, see [DATA_MODEL.md](DATA_MODEL.md).*  
*For v0.5-era wallet sync background, see [WALLET_ARCHITECTURE_v0.5.0.md](WALLET_ARCHITECTURE_v0.5.0.md).*
