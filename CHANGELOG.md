# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned for Future Releases
- CoinGecko portfolio import
- CoinMarketCap portfolio import
- CSV report generation
- Advanced P&L reporting (per-asset, per-year)
- MCP server for AI assistant integration (v0.6.0)

## [0.5.0] - 2026-04-01

### Added — Multi-chain Wallet Tracking

#### Blockchain Client Architecture
- `BlockchainClient` trait — unified async interface for all chain clients
- `ProviderRegistry` — health-check–driven provider selection with 60s cache
  and `PrivacyMode` (Strict / Balanced / Convenience)
- `SyncEngine` — parallel sync via `JoinSet`, `MultiProgress` progress bars,
  block-height watermarks in `wallet_sync_state` table

#### Chain Support
- **Bitcoin** — Blockstream.info, all address types (P2PKH, P2SH, Bech32, Taproot)
  xpub/zpub HD wallet derivation (BIP44/BIP84)
- **Ethereum** — Etherscan API, ETH balance + ERC-20 token detection
- **Cardano** — Blockfrost API, ADA + native tokens, stake delegation info
- **Solana** — User-supplied JSON-RPC, SOL + SPL tokens (Jupiter metadata cache),
  stake accounts

#### New Commands
- `cryptofolio wallet add` — add single-address or HD wallets
- `cryptofolio wallet list` — list wallets with blockchain filter
- `cryptofolio wallet show` — show wallet detail
- `cryptofolio wallet sync` — sync balances and transactions from blockchain
- `cryptofolio wallet remove` — remove wallet
- `cryptofolio audit sync` — view sync audit log with per-address history
- `cryptofolio audit coverage` — show which addresses are synced vs. never-synced
- `cryptofolio audit errors` — show sync error history

#### Security
- Private key guard — rejects WIF, Ethereum raw keys, and BIP39 seed phrases
  at the `wallet add` boundary before any DB write
- xpub stored in macOS Keychain (not plaintext DB column) on macOS
- All sync operations recorded in `sync_audit_log` for tamper-evident provenance
- Address redaction in logs (first 8 + last 4 chars)

#### CI
- Nightly integration test job (`CRYPTOFOLIO_INTEGRATION_TESTS=1`) for real API
  validation against Bitcoin, Ethereum, Cardano, and Solana mainnet

## [0.4.0] - 2026-03-28

### Added - Binance Transaction History Sync

#### New Command: `sync-history`
- **`cryptofolio sync-history`** — Full transaction history import from Binance
- Syncs trades, deposits, withdrawals, fiat on-ramp orders, and internal transfers
- `--symbols BTCUSDT,ETHUSDT` — Filter to specific trading pairs (required for trades)
- `--full-history` — Re-fetch entire history (ignores incremental watermarks)
- `--from YYYY-MM-DD` — Start date for history fetch
- `--dry-run` — Preview what would be imported without writing to the database
- `--no-trades / --no-deposits / --no-withdrawals / --no-fiat / --no-transfers` — Skip individual record types

#### Binance API Client Extensions (`src/exchange/binance/client.rs`)
- `get_my_trades(symbol, from_id, start_time, limit)` — Spot trade history with fromId pagination
- `get_deposit_history(start_time, limit, offset)` — Crypto deposit history
- `get_withdrawal_history(start_time, limit, offset)` — Crypto withdrawal history
- `get_fiat_orders(order_type, start_time, rows, page)` — Fiat on/off-ramp history
- `get_transfer_history(transfer_type, start_time, size, page)` — Spot ↔ Earn internal transfers
- `get_all_coins_info()` — Full coin/network information
- `get_signed_with_params` — Private helper for HMAC-signed requests with arbitrary params

#### Transaction Import Engine (`src/exchange/binance/import.rs`)
- `TransactionImporter` — Converts Binance API records into Cryptofolio transactions
- **Trade import**: Buy/sell detection, symbol parsing, P&L tax lot creation, USD price tracking for stable pairs
- **Deposit import**: Status filtering (completed only), holding balance updates
- **Withdrawal import**: Status filtering (completed only), balance reduction, silent handling of pre-sync history gaps
- **Fiat order import**: Credit card / bank buy orders with computed price
- **Transfer import**: Internal wallet moves (Spot ↔ Earn) recorded without balance change
- `parse_symbol` — Parses trading pairs (e.g. `BTCUSDT` → `BTC`+`USDT`), tries quote assets longest-first
- `is_usd_equivalent` — Identifies stablecoin/USD quote assets (USDT, BUSD, USDC, TUSD, USDP, DAI)
- `ms_to_datetime` — Millisecond timestamp conversion with error propagation
- Duplicate detection via `external_id` in transactions table (safe to re-run)
- External IDs: `binance-trade-{id}`, `binance-deposit-{id}`, `binance-withdrawal-{id}`, `binance-fiat-{order_no}`, `binance-transfer-{tran_id}`

#### Sync Orchestration (`src/exchange/binance/sync.rs`)
- `SyncOrchestrator` — Coordinates all endpoints with incremental watermark tracking
- `SyncOptions` — Configuration struct (which record types, symbols, start time, dry-run, full-history)
- `SyncReport` — Per-endpoint created/skipped counts, error list, totals
- Pagination per endpoint: fromId (trades), offset (deposits/withdrawals), page (fiat/transfers)
- Non-fatal error collection: individual import errors do not abort the full sync
- Default transfer types: MAIN_UMFUTURE, UMFUTURE_MAIN, MAIN_C2C, C2C_MAIN

#### Sync State Repository (`src/db/sync_state.rs`)
- `SyncStateRepository` — Persists incremental sync watermarks per account
- `SyncState` struct — Tracks `last_trade_sync`, `last_deposit_sync`, `last_withdrawal_sync`, `last_fiat_sync`, `last_transfer_sync`, `last_trade_id`, `last_sync_symbol`
- `get_or_create` — Upserts a fresh state record on first use
- `reset` — Clears all watermarks for a full re-sync
- Watermarks updated after each successful endpoint sync

#### Database Migration
- **MIGRATION_006** — New `binance_sync_state` table with per-account watermarks and index

### Testing
- **44 New Integration Tests** — `tests/binance_sync.rs` covering all import paths
  - Trade import: buy/sell, P&L, price_usd, notes, duplicate detection, dry-run
  - Deposit/withdrawal/fiat/transfer: happy paths, edge cases, status filtering
  - Mixed import sequences
  - `SyncStateRepository`: watermark updates, reset, idempotent creation
  - `SyncReport`: totals, error collection
- **Total Test Suite: 341 tests** (203 unit + 138 integration), 100% passing

### Changed
- **Version** — Updated from 0.3.1 to 0.4.0
- `src/exchange/binance/models` — Made public to support integration test fixtures
- CHANGELOG — Corrected "Planned" entries to remove already-delivered v0.4.0 items

### Fixed
- **Keychain Access Without Code Signing** (`src/config/keychain_security_cli.rs`)
  - Replaced Security Framework FFI with `security` command-line tool
  - Resolves "killed" error on macOS 26+ with ad-hoc signatures
  - Works on unsigned binaries - no Apple Developer ID required
  - Full keychain security maintained (encrypted, Touch ID protected at system level)
  - Session caching (15-minute timeout) prevents repeated prompts
  - Cross-platform ready (easy to add Linux/Windows equivalents)

- **Withdrawal Import** - Fixed Binance API datetime format change (`src/exchange/binance/models.rs`, `import.rs`)
  - Updated `BinanceWithdrawal.apply_time` from `i64` to `String` (API now returns "YYYY-MM-DD HH:MM:SS")
  - Added `parse_binance_datetime()` function for proper datetime parsing
  - Added 6 new optional fields from updated API (transfer_type, info, confirm_no, wallet_type, tx_key, complete_time)
  - Added API response logging on parse failures to catch future API changes early
  - Tested with live Binance account: 17 withdrawals imported successfully

## [0.3.1] - 2026-03-01

### Added - P&L Engine Foundation
- **Tax Lot Tracking** - FIFO/LIFO cost basis matching infrastructure
- **Realized P&L Repository** - Database layer for gain/loss recording
- **Database Schema** - New `tax_lots` and `realized_pnl` tables (MIGRATION_003)
- **P&L Calculator Module** - Core business logic for profit/loss calculations
- **Repository Pattern** - Complete data access layer for P&L tracking
- **Cost Basis Methods** - Support for FIFO and LIFO matching algorithms

### Added - Quality Improvements
- **Comprehensive Test Suite** - 259 total tests (175 unit + 84 integration)
- **Test Coverage Increase** - +206% unit test growth (57 → 175 tests)
- **Critical Code Coverage** - 95-100% coverage on all financial calculations
- **Repository Tests** - 71 tests across 6 database repositories
- **Core Module Tests** - 18 tests for Currency, Account, Transaction models
- **CLI Output Tests** - 30 tests for formatting and display functions
- **Coverage Analysis** - Systematic gap identification and remediation

### Added - Keychain Security (macOS)
- **OS-Encrypted Storage** - macOS Keychain integration for API keys
- **Touch ID Support** - Three security levels (Standard, Touch ID, Touch ID Only)
- **FFI Bindings** - Native Security.framework integration
- **Migration Wizard** - Interactive `config migrate-to-keychain` command
- **Keychain Management**:
  - `config set-secret <key>` - Store secrets in keychain
  - `config keychain-status` - View all keychain secrets
  - `config upgrade-security <key>` - Increase security level
  - `config downgrade-security <key>` - Decrease security level
- **Session Caching** - 15-minute cache to prevent repeated prompts
- **Automatic Backup** - Creates config.toml.backup before migration

### Changed
- **Version** - Updated from 0.2.0 to 0.3.1
- **Test Infrastructure** - Enhanced test organization and coverage tracking
- **Documentation** - Added comprehensive quality improvement summary

### Technical
- **Database Migration 003** - P&L schema:
  - `tax_lots` table for FIFO/LIFO tracking
  - `realized_pnl` table for gain/loss records
  - Foreign key relationships to transactions
- **Repository Layer** - TaxLotRepository, RealizedPnLRepository
- **Calculator Module** - `src/core/pnl/calculator.rs` with matching logic
- **Type Safety** - All P&L calculations use rust_decimal for precision

### Testing
- **175 Unit Tests** - Comprehensive coverage of business logic
- **84 Integration Tests** - End-to-end workflow validation
- **100% Pass Rate** - All 259 tests passing
- **Coverage Report** - 22.72% overall, 95-100% on critical paths

### Security
- **Keychain Integration** - Secrets stored in OS-encrypted keychain (macOS)
- **Touch ID Protection** - Biometric authentication for sensitive operations
- **No Plaintext Secrets** - Eliminated plaintext storage in config files

## [0.3.0] - 2026-02-21

### Added - Keychain Security (Phase 2)
- **macOS Keychain Integration** - OS-encrypted storage for API keys and secrets
- **Touch ID Security Levels** - Three-tier security (Standard, Touch ID Protected, Touch ID Only)
- **Migration Wizard** - Interactive `config migrate-to-keychain` command
- **Keychain Management Commands**:
  - `config set-secret <key> [--security-level <level>]` - Store secrets in keychain
  - `config keychain-status [--json]` - View all keychain secrets with security levels
  - `config upgrade-security <key> --to <level>` - Increase security level
  - `config downgrade-security <key> --to <level>` - Decrease security level (with warning)
- **Auto-Discovery** - Scans config.toml for secrets to migrate
- **Automatic Backup** - Creates config.toml.backup before migration
- **Session Caching** - 15-minute cache to prevent repeated keychain access
- **SSH Detection** - Graceful fallback when Touch ID unavailable
- **Database Schema** - New `keychain_keys` table (MIGRATION_005) for metadata tracking
- **Security Improvements**:
  - Eliminated plaintext secrets from config.toml
  - OS-level encryption protection
  - Protected from backup exposure (Dropbox, iCloud, Time Machine)
  - Access control via macOS Keychain Services

### Added - Validation & Testing
- **Comprehensive Test Suite** - 9 validation test files covering all features
- **Documentation** - 2,400+ lines of testing and validation documentation
- **Real Production Migration** - Tested with actual production secrets
- **100% Test Success Rate** - All 5 critical tests passed

### Changed
- **Secret Storage** - Secrets now stored in macOS Keychain by default (macOS only)
- **Config File** - No longer contains plaintext secrets after migration
- **Security Warning** - Updated to reflect keychain availability

### Security
- **ELIMINATED**: Plaintext secret storage in TOML files
- **PROTECTED**: Secrets from file system access, backups, cloud sync
- **MITIGATED**: Malware secret theft (requires OS keychain access)

### Known Limitations
- **Touch ID Prompts**: Security levels tracked but native prompts not yet implemented
  - Reason: security-framework 2.9 lacks SecAccessControl API
  - Workaround: Secrets still OS-encrypted and secure
  - Planned: FFI bindings for v0.3.1
- **Platform Support**: Keychain features macOS-only (Linux/Windows: future support)

### Backward Compatibility
- ✅ All v0.2.0 commands work unchanged
- ✅ Existing data intact (zero data loss)
- ✅ TOML configuration continues working
- ✅ Mixed storage supported (TOML + Keychain)
- ✅ Migration is opt-in (not forced)
- ✅ JSON output format unchanged

## [0.2.0] - 2026-02-19

### Added - Multi-Currency Support
- **Multi-currency foundation** - Full support for fiat currencies alongside crypto
- **Currency database** - Pre-seeded with 9 currencies (USD, CRC, EUR, BTC, ETH, USDT, USDC, BNB, SOL)
- **Exchange rate tracking** - Manual entry with historical tracking and automatic storage
- **Bank account type** - New account type for traditional banking integration
- **Multi-currency cost basis** - Track holdings with cost in any currency (e.g., CRC, USD)
- **Currency management commands** - Complete CLI for currency operations:
  - `currency list` - List all currencies with filtering
  - `currency show` - Display currency details
  - `currency add` - Add custom currencies
  - `currency remove` - Remove currencies
  - `currency toggle` - Enable/disable currencies
  - `currency set-rate` - Set exchange rates manually
  - `currency show-rate` - View rate history
- **Fiat swap detection** - Automatic exchange rate storage for fiat-to-fiat swaps
- **Costa Rica on-ramp flow** - Complete support for CRC → USD → USDT → BTC conversions

### Added - Security Enhancements
- **Secure secret handling** - New `config set-secret` command prevents API keys in shell history
- **Multiple input methods** - Interactive, stdin, file, and environment variable support
- **Shell history protection** - Secrets never appear in bash/zsh history
- **Process list protection** - Secrets not visible in `ps` output
- **File permissions enforcement** - Automatic 0600 on Unix/macOS/Linux
- **Security warnings** - Comprehensive warnings about READ-ONLY API keys

### Added - Developer Experience
- **JSON output for all commands** - Complete machine-readable output support:
  - `portfolio --json`
  - `holdings list --json`
  - `account list --json` and `account show --json`
  - `tx list --json`
  - `config show --json`
  - `currency list --json` and `currency show-rate --json`
  - `price --json` and `market --json`
- **CSV transaction export** - Export transaction history with filtering:
  - Filter by account (`--account`)
  - Filter by asset (`--asset`)
  - Filter by date range (`--from`, `--to`)
- **Customizable number formatting** - Configure decimal precision:
  - `display.decimals` - Quantity decimal places (default: 8)
  - `display.price_decimals` - Price decimal places (default: 2)
  - `display.thousands_separator` - Use commas (default: true)

### Added - Documentation
- **Multi-currency guide** - Complete implementation documentation
- **Validation guide updates** - Added Test V10 with 10 currency test scenarios
- **README updates** - Comprehensive multi-currency section with examples
- **API integration examples** - LLM/MCP integration patterns
- **Roadmap document** - Detailed v0.3 and v0.4 plans

### Changed
- **Holdings table** - Added `cost_basis_currency` and `avg_cost_basis_base` columns
- **Transactions table** - Added `price_currency`, `price_amount`, `exchange_rate`, `exchange_rate_pair` columns
- **Account categories** - Added `banking` and `on-ramp` categories
- **Account types** - Added `Bank` account type
- **Error handling** - Added `InvalidInput`, `NotFound`, `AlreadyExists` error variants

### Added - Testing
- **26 new tests** - Currency-specific test coverage:
  - 12 unit tests for Currency and ExchangeRate models
  - 14 integration tests for database operations
  - Complete Costa Rica on-ramp flow validation
- **110+ total tests** - All passing with 100% success rate

### Technical
- **Database Migration 002** - Multi-currency schema:
  - `currencies` table with asset_type classification
  - `exchange_rates` table with upsert on conflict
  - Extended holdings and transactions tables
- **Type safety** - sqlx compile-time query checking for all new queries
- **Decimal precision** - Financial-grade decimal handling for exchange rates

## [0.1.0] - 2026-01-15

### Added - Initial Release
- **Portfolio management** - Track holdings across multiple accounts
- **Account types** - Exchange, hardware wallet, software wallet, custodial service
- **Category system** - Organize accounts (trading, cold storage, hot wallets, etc.)
- **Transaction tracking** - Buy, sell, transfer, swap operations
- **Cost basis tracking** - Average cost method for P&L calculations
- **Binance integration** - Auto-sync with read-only API
- **Binance Alpha support** - Fetch prices from Binance Alpha markets
- **Price checking** - Real-time cryptocurrency prices
- **Market data** - 24-hour statistics and trends
- **CSV import** - Bulk transaction import
- **Testnet support** - Practice with Binance testnet
- **Local SQLite database** - All data stored locally
- **Privacy-first** - No cloud sync, no telemetry

### Added - AI Features
- **Interactive shell** - Natural language command interface
- **Multiple AI providers**:
  - Claude (cloud) - Advanced reasoning
  - Ollama (local) - Privacy-first
  - Hybrid mode - Automatic provider selection
  - Pattern-based - Regex fallback (no AI needed)
- **Multi-turn conversations** - Context-aware interactions
- **Status command** - System diagnostics and AI provider status

### Added - CLI Features
- **JSON output** - Machine-readable output for automation
- **Quiet mode** - Suppress non-essential output
- **Dry-run mode** - Preview changes without committing
- **Confirmation prompts** - Safe destructive operations
- **Progress indicators** - Visual feedback for long operations
- **Colored output** - Syntax highlighting and status colors
- **Tab completion** - Command completion in shell mode

### Added - Documentation
- **Comprehensive README** - Use cases, examples, architecture
- **Validation guide** - Step-by-step testing instructions
- **Secure secrets guide** - API key security best practices
- **Conversational CLI guide** - AI features documentation

### Technical
- **Rust implementation** - Type-safe, fast, single binary
- **Tokio async runtime** - Efficient async operations
- **clap v4** - Modern CLI framework with derive macros
- **sqlx** - Compile-time checked SQL queries
- **rust_decimal** - Financial precision for amounts
- **TOML configuration** - Human-readable config files
- **XDG Base Directory** - Standard config/data paths

## [0.0.1] - 2025-12-01

### Added
- Initial project setup
- Basic CLI structure
- Database schema design

---

## Version History Summary

| Version | Date | Description |
|---------|------|-------------|
| **0.3.1** | 2026-03-01 | Keychain security, P&L foundation, quality improvements (259 tests) |
| **0.2.0** | 2026-02-19 | Multi-currency support, security enhancements, JSON output |
| **0.1.0** | 2026-01-15 | Initial release with portfolio management and AI features |
| **0.0.1** | 2025-12-01 | Project inception |

## Links

- [Full Roadmap](docs/ROADMAP.md)
- [Contributing Guide](CONTRIBUTING.md)
- [Security Policy](SECURITY.md)

---

**Note:** This project is built using agentic development with Claude Code. All features are developed with AI pair programming assistance.
