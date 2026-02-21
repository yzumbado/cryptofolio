# Cryptofolio MCP Integration

**Interact with your cryptocurrency portfolio through natural conversation with Claude Desktop**

---

## Overview

The Cryptofolio MCP (Model Context Protocol) integration enables you to manage your crypto portfolio through natural language conversations with Claude Desktop. Instead of memorizing CLI commands, simply ask Claude to check prices, record transactions, analyze holdings, and more.

### What You Can Do

```
You: "What's my portfolio value?"
Claude: *checks portfolio* "Your portfolio is worth $61,442.89, up 109% from your cost basis."

You: "I just bought 0.1 BTC at $95,000 on Binance"
Claude: *records transaction* "✅ Purchase recorded! Your new BTC total: 0.19121"

You: "Help me track this conversion: 100,000 CRC → USD → USDT → BTC"
Claude: *tracks multi-currency flow* "✅ All 5 steps recorded with cost basis preserved!"
```

---

## Documentation

### 📚 Complete Documentation Set

1. **[MCP Technical Analysis](./MCP_TECHNICAL_ANALYSIS.md)** (Design & Architecture)
   - **Purpose:** Technical decision analysis and architecture design
   - **Audience:** Developers, architects, stakeholders
   - **Contents:**
     - MCP design principles (6 best practices)
     - Implementation options (Node.js vs Rust vs Direct CLI)
     - Comprehensive pros/cons analysis
     - Recommended architecture with diagrams
     - 10 curated tools design
     - Security considerations
     - 3-phase implementation roadmap (16-24 hours)

2. **[MCP User Guide](./MCP_USER_GUIDE.md)** (End-User Documentation)
   - **Purpose:** Complete guide for using Cryptofolio with Claude Desktop
   - **Audience:** End users, portfolio managers
   - **Contents:**
     - Installation and setup instructions
     - 8 detailed use cases with full conversations:
       - Morning portfolio check
       - Recording purchases
       - Multi-currency on-ramp flow (CRC → BTC)
       - Weekly DCA automation
       - Moving to cold storage
       - Tax season preparation
       - Portfolio analysis & rebalancing
       - Real-time market monitoring
     - Best practices for effective communication
     - Troubleshooting guide
     - Advanced workflows and integrations

3. **[MCP API Reference](./MCP_API_REFERENCE.md)** (Developer Reference)
   - **Purpose:** Detailed technical specification for all MCP tools
   - **Audience:** Developers, integrators, contributors
   - **Contents:**
     - Complete tool catalog (10 tools)
     - Input/output schemas for each tool
     - Request/response examples
     - Error handling specifications
     - Pagination guidelines
     - Rate limiting information
     - Multi-tool workflow examples

---

## Quick Start

### 1. Install Cryptofolio MCP Server

```bash
# Install via NPM (once published)
npm install -g cryptofolio-mcp

# Verify installation
cryptofolio-mcp --version
```

### 2. Configure Claude Desktop

Edit `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "cryptofolio": {
      "command": "cryptofolio-mcp",
      "args": []
    }
  }
}
```

### 3. Restart Claude Desktop

1. Quit Claude Desktop completely
2. Reopen Claude Desktop
3. Verify MCP server connection

### 4. Start Using

Open Claude Desktop and try:

```
You: "What tools do you have from Cryptofolio?"
Claude: *lists 10 available tools*

You: "What's my current portfolio value?"
Claude: *uses cryptofolio_get_portfolio tool and shows results*
```

---

## Design Principles

The Cryptofolio MCP server follows industry best practices from [Philipp Schmid's MCP guide](https://www.philschmid.de/mcp-best-practices):

### 1. ⚡ Outcomes Over Operations
- **Bad:** `get_btc_price`, `get_eth_price`, `get_sol_price` (3 separate tools)
- **Good:** `cryptofolio_get_prices(assets: ["BTC", "ETH", "SOL"])` (1 aggregated tool)

### 2. 📋 Flatten Arguments
- No nested objects
- Top-level primitives only
- Constrained types (Literal enums)
- Sensible defaults

### 3. 📖 Instructions as Context
- Rich docstrings with examples
- Error messages as helpful observations
- Agent-friendly documentation

### 4. ✂️ Ruthless Curation
- **Limited to 10 tools** (within 5-15 recommended range)
- Each tool has clear, focused purpose
- Excluded: Admin operations, low-level CRUD, config management

### 5. 🔍 Discovery-Oriented Naming
- All tools: `cryptofolio_{action}_{resource}`
- Prevents collision with other MCP servers
- Clear namespace for agent understanding

### 6. 📄 Pagination with Metadata
- Large result sets paginated (default: 50 items)
- Metadata includes: `has_more`, `next_offset`, `total_count`
- Agent can request additional pages as needed

---

## Tool Catalog

### 10 Curated Tools

| Tool | Purpose | Use Case |
|------|---------|----------|
| **cryptofolio_get_portfolio** | Complete portfolio overview | "What's my portfolio value?" |
| **cryptofolio_get_prices** | Current cryptocurrency prices | "What's the price of Bitcoin?" |
| **cryptofolio_get_market_data** | 24h market statistics | "How is BTC performing today?" |
| **cryptofolio_record_transaction** | Record buy/sell/transfer/swap | "I bought 0.1 BTC at $95k" |
| **cryptofolio_track_conversion** | Multi-currency on-ramp flows | "Track CRC → USD → BTC" |
| **cryptofolio_list_transactions** | Transaction history (paginated) | "Show my 2024 transactions" |
| **cryptofolio_analyze_asset** | Asset performance analysis | "Analyze my Bitcoin holdings" |
| **cryptofolio_export_transactions** | CSV export for taxes | "Export 2024 for my accountant" |
| **cryptofolio_list_accounts** | Show configured accounts | "What accounts do I have?" |
| **cryptofolio_sync_exchange** | Sync from Binance API | "Sync my Binance account" |

---

## Architecture

### High-Level Design

```
Claude Desktop (AI Assistant)
    ↓ MCP Protocol (JSON-RPC over stdio)
Node.js MCP Server (TypeScript)
    ↓ child_process (execa)
Cryptofolio CLI (Rust binary with --json)
    ↓ SQLite queries
Database (~/.config/cryptofolio/)
```

### Technology Stack

- **MCP Protocol:** v1.0
- **Server Runtime:** Node.js 20.x LTS
- **Language:** TypeScript 5.x
- **MCP SDK:** `@modelcontextprotocol/sdk`
- **Process Management:** `execa`
- **Validation:** `zod`
- **CLI Integration:** Cryptofolio v0.2.0+ (with `--json` support)

---

## Use Cases

### 1. Morning Portfolio Check
Quick overview of portfolio value, P&L, and overnight price movements.

**Conversation:**
```
You: "Good morning! What's my portfolio looking like today?"
Claude: "Your portfolio is worth $61,442.89, up $32,125.50 (+109.57%)..."
You: "How did Bitcoin perform overnight?"
Claude: "Bitcoin is up $2,145 (+2.25%) in the last 24h..."
```

### 2. Recording Purchases
Automatic cost basis tracking for new purchases.

**Conversation:**
```
You: "I just bought 0.1 BTC at $95,000 on Binance"
Claude: "✅ Transaction recorded! Your BTC holdings: 0.19121 at avg cost $87,937.50"
You: "Sync Binance to verify"
Claude: "✅ Synced! BTC balance matches: 0.19121000"
```

### 3. Multi-Currency On-Ramp
Track complex fiat-to-crypto conversion flows.

**Conversation:**
```
You: "Help me track: 100,000 CRC → USD → USDT → BTC through my bank and exchanges"
Claude: "I'll record each step with proper exchange rates and cost basis..."
         *Records 5 transactions*
         "✅ Complete! Final: 0.0025 BTC with cost basis of 100,000 CRC"
```

### 4. Tax Preparation
Export transaction history for accountant.

**Conversation:**
```
You: "Export all my 2024 transactions for taxes"
Claude: "✅ Exported 127 transactions to transactions_2024.csv"
         "Would you like me to also export just Bitcoin trades separately?"
```

[See 4 more detailed use cases in the User Guide →](./MCP_USER_GUIDE.md#use-cases)

---

## Security

### What's Protected

✅ **API Keys:** Never exposed via MCP (stored in Cryptofolio config)
✅ **Local Data:** All operations run locally (no external servers)
✅ **Read-Only Operations:** Most tools are read-only queries
✅ **Command Injection:** Prevented via `execa` array args
✅ **Input Validation:** Zod schemas validate all inputs

### What's Excluded

For security, these tools are NOT exposed via MCP:
- ❌ Config management (API key viewing/editing)
- ❌ Database administration
- ❌ System settings
- ❌ File system access

### Best Practices

1. **Use READ-ONLY Binance API keys**
2. **Keep Cryptofolio CLI updated**
3. **Verify large transactions before recording**
4. **Regularly backup database** (`~/.config/cryptofolio/`)
5. **Quit Claude Desktop when not in use**

[Complete security analysis →](./MCP_TECHNICAL_ANALYSIS.md#security-considerations)

---

## Implementation Status

### ✅ Phase 1: Design & Documentation (Complete)
- [x] MCP best practices research
- [x] Technical analysis document
- [x] User guide with 8 use cases
- [x] API reference with all 10 tools
- [x] Architecture design
- [x] Security analysis

### 🚧 Phase 2: Development (Next)
**Estimated Time:** 16-24 hours (2-3 days with AI assistance)

**Roadmap:**
1. **Phase 1: MVP** (4-6 hours)
   - Node.js/TypeScript project setup
   - MCP SDK integration
   - 3 core tools: portfolio, prices, record_transaction
   - Basic testing with Claude Desktop

2. **Phase 2: Full Suite** (8-12 hours)
   - Implement remaining 7 tools
   - Add pagination for list_transactions
   - Comprehensive error handling
   - Test suite (unit + integration)

3. **Phase 3: Distribution** (4-6 hours)
   - NPM package creation
   - Installation automation
   - Documentation finalization
   - Public release

### 📅 Timeline
- **Design:** Complete (February 19, 2026)
- **Development Start:** TBD
- **Beta Release:** TBD
- **Public Release:** TBD

---

## Contributing

We welcome contributions! Areas where you can help:

**Before Release:**
- Review technical analysis and provide feedback
- Suggest additional use cases
- Review API design
- Security review

**After Release:**
- Bug reports and fixes
- Performance optimization
- Additional tool ideas
- Documentation improvements
- Integration examples

**Repository:** https://github.com/yourusername/cryptofolio-mcp
**Issues:** https://github.com/yourusername/cryptofolio-mcp/issues
**Discussions:** https://github.com/yourusername/cryptofolio-mcp/discussions

---

## FAQ

**Q: When will this be available?**
A: Documentation is complete. Development starts next. Estimated: 2-3 weeks to beta.

**Q: Will this work on mobile?**
A: No. MCP protocol currently only works with Claude Desktop (macOS/Windows/Linux).

**Q: Does this replace the Cryptofolio CLI?**
A: No, it wraps it. The CLI remains the core tool. MCP adds a conversational interface.

**Q: Is my data safe?**
A: Yes. All operations run locally. No data leaves your machine. API keys are not exposed via MCP.

**Q: Can Claude automatically trade for me?**
A: No. For security, only read operations and manual transaction recording are supported. No automated trading.

**Q: Does this work with other exchanges?**
A: Currently only Binance for auto-sync. Manual tracking works for any exchange. More integrations in v0.3.

**Q: How much does this cost?**
A: The MCP server is free (MIT License). Requires Claude Desktop (part of Claude Pro subscription, $20/month).

---

## Resources

### Documentation
- **[Technical Analysis](./MCP_TECHNICAL_ANALYSIS.md)** - Design decisions and architecture
- **[User Guide](./MCP_USER_GUIDE.md)** - Installation, use cases, troubleshooting
- **[API Reference](./MCP_API_REFERENCE.md)** - Detailed tool specifications

### External Links
- [Model Context Protocol Specification](https://modelcontextprotocol.io/)
- [MCP Best Practices](https://www.philschmid.de/mcp-best-practices)
- [Cryptofolio Main Documentation](../README.md)
- [Cryptofolio Architecture](./ARCHITECTURE.md)

### Support
- **Issues:** https://github.com/yourusername/cryptofolio-mcp/issues
- **Discussions:** https://github.com/yourusername/cryptofolio-mcp/discussions
- **Email:** support@cryptofolio.dev

---

## Feedback

This is a **design phase** document. We'd love your input!

**What we're looking for:**
- Missing use cases?
- Tool design feedback?
- Security concerns?
- Performance considerations?
- Documentation clarity?

**How to provide feedback:**
- Open a GitHub Discussion
- Comment on the design issue
- Email the team

**Thank you for helping shape Cryptofolio MCP!** 🙏

---

**Status:** Design Complete, Awaiting Development
**Version:** 1.0.0-draft
**Last Updated:** February 19, 2026
**Built with:** Claude Sonnet 4.5
