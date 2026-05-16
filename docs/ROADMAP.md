# Cryptofolio Roadmap

**Last Updated:** May 2026
**Current Version:** v0.6.0

---

## Vision

Build a **local-first, privacy-respecting cryptocurrency portfolio manager** that:
- Works entirely offline with local data
- Supports multi-currency tracking (fiat + crypto)
- Provides rich data visualization and insights
- Integrates with popular portfolio tracking platforms
- Remains simple, fast, and developer-friendly

---

## v0.2.0 (✅ Released - February 2026)

### Multi-Currency Foundation

**Features Delivered:**
- ✅ Multi-currency support (fiat, crypto, stablecoins)
- ✅ Database-driven currency model (9 pre-seeded currencies)
- ✅ Exchange rate management (manual entry + automatic storage)
- ✅ Bank account type for traditional banking
- ✅ Multi-currency cost basis tracking
- ✅ Fiat-to-fiat swap detection

**Security Enhancements:**
- ✅ Secure secret handling (`config set-secret`)
- ✅ Shell history protection
- ✅ Auto file permissions (0600 on Unix)
- ✅ Multiple input methods (stdin, file, env, interactive)

**Developer Experience:**
- ✅ JSON output for all query commands
- ✅ CSV transaction export with filtering
- ✅ Customizable number formatting
- ✅ Comprehensive help text

**Testing & Documentation:**
- ✅ 110+ tests (26 currency-specific)
- ✅ Complete documentation suite
- ✅ Validation guide with 11 test scenarios
- ✅ Multi-currency implementation guide

**Metrics:**
- 26 files changed
- 2,405 lines added
- 100% test pass rate
- Built with AI pair programming

---

## v0.3.1 (✅ Released - March 2026)

### Keychain Security & P&L Foundation

**Features Delivered:**
- ✅ **macOS Keychain Integration**
  - OS-encrypted storage for API keys and secrets
  - Touch ID support with three security levels
  - Automatic migration wizard (`config migrate-to-keychain`)
  - Session caching (15-minute timeout)
  - FFI bindings to Security.framework
- ✅ **P&L Engine Foundation**
  - Tax lot tracking infrastructure (FIFO/LIFO)
  - Realized P&L database schema (MIGRATION_003)
  - TaxLotRepository and RealizedPnLRepository
  - P&L Calculator module with matching logic
  - Cost basis method support (FIFO, LIFO)
- ✅ **Quality Improvements**
  - Comprehensive test suite (259 tests total)
  - 175 unit tests (+206% increase)
  - 84 integration tests
  - 95-100% coverage on critical code paths
  - Repository layer tests (71 tests across 6 repos)
  - Core module tests (18 tests)
  - CLI output tests (30 tests)

**Technical Achievements:**
- Database migration 003 (tax_lots, realized_pnl tables)
- FFI bindings for native macOS security
- Systematic test coverage improvement
- 100% test pass rate

**Metrics:**
- Total tests: 259 (175 unit + 84 integration)
- Test pass rate: 100%
- Critical code coverage: 95-100%
- Development approach: AI pair programming with Claude Code

---

## v0.4.0 (✅ Released - March 2026)

### Binance Deep Integration

**Features Delivered:**
- ✅ `sync-history` command — full Binance transaction history import
- ✅ Spot trades, deposits, withdrawals, fiat orders, internal transfers
- ✅ Incremental sync with watermarks (fetches only new records)
- ✅ Duplicate detection via `external_id` (safe to re-run)
- ✅ `--dry-run` preview mode
- ✅ 44 new integration tests, 341 total tests (100% passing)
- ✅ Keychain access fix for macOS without code signing

---

## v0.5.0 (✅ Released - April 2026)

### Multi-Chain Wallet Tracking

**Features Delivered:**
- ✅ `BlockchainClient` trait — unified async interface for all chain clients
- ✅ `ProviderRegistry` — health-check-driven provider selection with `PrivacyMode`
- ✅ `SyncEngine` — parallel sync via `JoinSet`, block-height watermarks
- ✅ Bitcoin — Blockstream.info, all address types, xpub/zpub HD derivation
- ✅ Ethereum — Etherscan API, ETH + ERC-20 token detection
- ✅ Cardano — Blockfrost API, ADA + native tokens, stake delegation
- ✅ Solana — JSON-RPC, SOL + SPL tokens (Jupiter metadata cache), stake accounts
- ✅ Private key guard at `wallet add` boundary
- ✅ xpub stored in macOS Keychain (not plaintext DB)
- ✅ `sync_audit_log` for tamper-evident sync provenance
- ✅ `audit sync`, `audit coverage`, `audit errors` commands

---

## v0.5.1 (✅ Released - May 2026)

### Onboarding Bug Fixes

**Features Delivered:**
- ✅ Taproot xpub support (P2TR address derivation)
- ✅ Blockfrost keychain lookup during wallet sync
- ✅ `PrivacyMode::Balanced` now correctly uses Convenience for public APIs
- ✅ 10 total bug fixes from real onboarding sessions

---

## v0.6.0 (✅ Released - May 2026)

### MCP Server & AI-Native Portfolio Management

**Features Delivered:**
- ✅ MCP server (`mcp/`) with 18 `cryptofolio_*` tools via stdio protocol
- ✅ `/portfolio` Claude Code skill — natural language portfolio management
- ✅ Code quality: removed incomplete `src/ai/` module, xpub panic safety, health check constructors, dead code removal
- ✅ CI fixes: linux-amd64 + macos-arm64 cross-compile targets only, removed tarpaulin, fixed 6h timeout
- ✅ Bug fixes: `wallet list --json` trailing commas, `pnl realized` empty output, `sync_exchange` account flag

---

## v0.7.0 (🎯 Planned - 2026)

### Visual Data Exploration Dashboard

**Focus:** Local-first visual analytics without cloud dependencies

### Dashboard Architecture
- [ ] **Local Node.js Server**
  - No external dependencies
  - Runs on localhost only
  - Auto-starts/stops with CLI
  - Reads from local SQLite database
  - No data transmission

### Visualization Features
- [ ] **Interactive Portfolio Explorer**
  - Real-time portfolio view
  - Drag-to-zoom time ranges
  - Click-through to transactions
  - Account/category filtering

- [ ] **Time-Series Analysis**
  - Historical value charts
  - P&L over time
  - Asset performance comparison
  - Drawdown visualization

- [ ] **Portfolio Composition**
  - Asset allocation pie charts
  - Account distribution
  - Category breakdown
  - Fiat vs crypto split

- [ ] **Historical Performance**
  - Total return charts
  - Per-asset returns
  - Benchmark comparison
  - Correlation matrix

### Rich Data Display
- [ ] **Charts & Graphs**
  - Line charts (time series)
  - Pie charts (composition)
  - Bar charts (comparisons)
  - Heatmaps (correlations)

- [ ] **Tables**
  - Sortable columns
  - Filterable rows
  - Export to CSV
  - Copy to clipboard

- [ ] **Summary Cards**
  - Total value
  - Today's change
  - 7-day change
  - All-time high/low

### Technical Stack
```
┌─────────────────────────────────────┐
│  Browser (localhost:3000)           │
│  ├─ React/Svelte (TBD)              │
│  ├─ Chart.js / D3.js                │
│  └─ Tailwind CSS                    │
└─────────────────────────────────────┘
              ↕ HTTP
┌─────────────────────────────────────┐
│  Node.js Server (local only)        │
│  ├─ Express.js                      │
│  ├─ SQLite reader                   │
│  └─ Read-only database access       │
└─────────────────────────────────────┘
              ↕ SQL
┌─────────────────────────────────────┐
│  SQLite Database                    │
│  ~/.config/cryptofolio/database.db  │
└─────────────────────────────────────┘
```

**Commands:**
```bash
# Start dashboard
cryptofolio dashboard start
# 🚀 Dashboard running at http://localhost:3000

# Open in browser
cryptofolio dashboard open

# Stop dashboard
cryptofolio dashboard stop
```

**Security Guarantees:**
- ✅ Local-only server (no external network access)
- ✅ Read-only database access
- ✅ No data transmission
- ✅ CORS disabled
- ✅ Auto-shutdown on inactivity

**Target Metrics:**
- < 1s dashboard load time
- < 100ms chart render time
- < 50MB memory footprint
- Zero external API calls

---

## Long-Term Vision (2027+)

### Multi-Chain DeFi Integration
- Read-only wallet tracking (Ethereum, Solana, etc.)
- DeFi protocol position tracking
- NFT portfolio tracking
- Cross-chain aggregation

### Advanced Analytics
- AI-powered insights
- Portfolio optimization suggestions
- Risk analysis
- Rebalancing recommendations

### Community Features
- Plugin system for custom dashboards
- Shared report templates
- Community-built integrations
- Educational resources

### Platform Expansion
- Linux ARM support (Raspberry Pi)
- Docker container
- Homebrew formula
- Snap package

---

## Not Planned (Out of Scope)

**Intentionally NOT included to maintain simplicity and security:**

❌ **Cloud Sync** - Remains local-first forever
❌ **Mobile Apps** - CLI-focused tool
❌ **Trading Capabilities** - Read-only portfolio tracking only
❌ **Automated Trading** - Too risky, out of scope
❌ **Windows/Linux Keychain** - macOS only for v0.3
❌ **Coinbase/Kraken Integration** - Not prioritized
❌ **Tax Filing Integration** - Too complex, use exports + tax software
❌ **Price Alerts** - Use other tools for notifications
❌ **DCA Automation** - Security risk, use manual recording

---

## Feature Requests

**Want to suggest a feature?**
1. Check [GitHub Discussions](https://github.com/yourusername/cryptofolio/discussions)
2. Search for existing requests
3. Create a new discussion with:
   - Use case description
   - Example workflow
   - Why it fits Cryptofolio's vision

**Voting:**
- React with 👍 to upvote features
- React with 👎 if you disagree
- Comment with your perspective

---

## Development Philosophy

### Core Principles

1. **Local-First** - All data stays on your machine
2. **Privacy-Respecting** - No telemetry, no tracking
3. **Read-Only APIs** - Never request write permissions
4. **Simple & Fast** - < 200ms command response
5. **Developer-Friendly** - JSON output, scriptable
6. **Agentic Development** - Built with AI pair programming

### Quality Standards

- ✅ 100% test pass rate
- ✅ No breaking changes without major version
- ✅ Comprehensive documentation
- ✅ Security-first design
- ✅ Semantic versioning

### Release Cadence

- **Major (v1.0, v2.0)** - Yearly, with breaking changes
- **Minor (v0.3, v0.4)** - Quarterly, new features
- **Patch (v0.2.1, v0.2.2)** - As needed, bug fixes

---

## Questions?

- 📖 [Documentation](.)
- 💬 [Discussions](https://github.com/yourusername/cryptofolio/discussions)
- 🐛 [Issues](https://github.com/yourusername/cryptofolio/issues)

---

**Last Updated:** May 2026
**Next Review:** August 1, 2026
