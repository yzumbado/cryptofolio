# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned for v0.3.0
- Encrypted keychain storage (macOS Keychain)
- Realized P&L calculations (FIFO/LIFO)
- CoinGecko portfolio import
- CoinMarketCap portfolio import
- CSV report generation
- Advanced data extraction

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
| **0.2.0** | 2026-02-19 | Multi-currency support, security enhancements, JSON output |
| **0.1.0** | 2026-01-15 | Initial release with portfolio management and AI features |
| **0.0.1** | 2025-12-01 | Project inception |

## Links

- [Full Roadmap](docs/ROADMAP.md)
- [Contributing Guide](CONTRIBUTING.md)
- [Security Policy](SECURITY.md)

---

**Note:** This project is built using agentic development with Claude Code. All features are developed with AI pair programming assistance.
