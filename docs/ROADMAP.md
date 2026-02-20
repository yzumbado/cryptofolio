# Cryptofolio Roadmap

**Last Updated:** February 2026
**Current Version:** v0.2.0

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

## v0.3.0 (🎯 Planned - Q2 2026)

### Security & Data Integration

**Focus:** Secure credential storage and portfolio data interoperability

### Security
- [ ] **macOS Keychain Integration**
  - Encrypted storage for API keys and secrets
  - Automatic migration from plaintext config
  - Secure retrieval for API operations
  - No Windows/Linux keychain support (out of scope)

### P&L & Accounting
- [ ] **Realized P&L Calculations**
  - FIFO (First In, First Out) method
  - LIFO (Last In, First Out) method
  - Average cost method (already implemented for unrealized)
  - Per-transaction realized gains/losses
  - Year-to-date realized P&L summary

### Portfolio Import
- [ ] **CoinGecko Integration**
  - Import portfolio from CoinGecko
  - Sync holdings automatically
  - Map CoinGecko IDs to local assets
  - Historical data import

- [ ] **CoinMarketCap Integration**
  - Import portfolio from CoinMarketCap
  - Sync holdings automatically
  - Map CMC IDs to local assets
  - Historical data import

### Data Export & Reporting
- [ ] **CSV Report Generation**
  - Customizable templates
  - Portfolio summary reports
  - Transaction history reports
  - P&L reports (realized + unrealized)
  - Tax basis reports
  - Asset allocation reports

- [ ] **Advanced Data Extraction**
  - JSON export (enhanced with filters)
  - CSV export (all data types)
  - SQLite database export
  - Custom query export
  - Batch export scripts

### CLI Improvements
- [ ] `--quiet` flag for all commands
- [ ] Progress indicators for long-running operations
- [ ] Improved error messages
- [ ] "Did you mean?" suggestions

**Target Metrics:**
- 150+ tests
- < 200ms command response time
- Secure by default (keychain)

---

## v0.4.0 (🔬 Experimental - Q3 2026)

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

**Last Updated:** February 19, 2026
**Next Review:** May 1, 2026
