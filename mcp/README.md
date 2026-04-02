# Cryptofolio MCP Server

LLM-driven crypto portfolio management via the Model Context Protocol.

Exposes 18 tools so Claude Desktop can manage your entire portfolio without
ever touching the CLI directly.

## Requirements

- Node.js 20+
- cryptofolio binary (compiled from this repo)

## Quick Start

### 1. Build the MCP server

```bash
cd mcp/
npm install
npm run build
```

### 2. Configure Claude Desktop

Edit `~/Library/Application Support/Claude/claude_desktop_config.json`
(macOS) and add:

```json
{
  "mcpServers": {
    "cryptofolio": {
      "command": "node",
      "args": ["/absolute/path/to/cryptofolio/mcp/dist/index.js"],
      "env": {
        "CRYPTOFOLIO_BIN": "/absolute/path/to/cryptofolio/target/release/cryptofolio"
      }
    }
  }
}
```

Replace the paths with the actual locations on your system.

### 3. Build the cryptofolio binary (if not already done)

```bash
# From the repo root
cargo build --release
# Binary will be at: target/release/cryptofolio
```

### 4. Restart Claude Desktop

Quit and reopen Claude Desktop. The cryptofolio tools will be available.

### 5. One-time secret setup (only terminal step required)

Before using Binance sync, run once in your terminal:

```bash
cryptofolio config set-secret binance.api_key
cryptofolio config set-secret binance.api_secret
```

This is the only step that cannot be done through Claude — API keys require
a hidden TTY prompt for security. Everything else can be done via Claude.

## Available Tools (18)

| Tool | Purpose |
|------|---------|
| `cryptofolio_get_system_status` | Health check — call this first in every session |
| `cryptofolio_list_accounts` | List all configured accounts |
| `cryptofolio_manage_account` | Add or remove accounts |
| `cryptofolio_get_portfolio` | Full portfolio snapshot with P&L |
| `cryptofolio_get_prices` | Spot prices for any asset(s) |
| `cryptofolio_get_market_data` | 24h OHLCV market stats |
| `cryptofolio_list_transactions` | Paginated transaction history |
| `cryptofolio_record_transaction` | Record buy/sell/transfer/swap |
| `cryptofolio_track_conversion` | Multi-step fiat-to-crypto flows |
| `cryptofolio_export_transactions` | Export CSV/JSON for tax reporting |
| `cryptofolio_manage_wallet` | Add/list/show/remove blockchain wallets |
| `cryptofolio_sync_wallet` | Sync on-chain balance and history |
| `cryptofolio_sync_exchange` | Sync Binance balances |
| `cryptofolio_get_pnl_summary` | Overall realized + unrealized P&L |
| `cryptofolio_get_realized_pnl` | Closed positions with gain/loss |
| `cryptofolio_get_unrealized_pnl` | Open position P&L |
| `cryptofolio_analyze_asset` | Deep dive on one asset |
| `cryptofolio_get_audit_log` | Blockchain sync history and errors |

## Onboarding Flow (LLM-only)

Tell Claude: *"I want to set up my crypto portfolio tracker."*

Claude will:
1. Call `cryptofolio_get_system_status` to check current state
2. Call `cryptofolio_manage_account` to create your accounts
3. Call `cryptofolio_manage_wallet` to add blockchain addresses
4. Call `cryptofolio_sync_wallet` / `cryptofolio_sync_exchange` for first sync
5. Call `cryptofolio_get_portfolio` to confirm everything is working

## Development

```bash
npm run dev          # Run with tsx (no build step)
npm test             # Unit tests
npm run typecheck    # TypeScript type check
npm run lint         # ESLint
```

### Integration tests (requires binary)

```bash
CRYPTOFOLIO_INTEGRATION=1 \
CRYPTOFOLIO_BIN=/path/to/cryptofolio \
npm run test:integration
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `CRYPTOFOLIO_BIN` | Path to the cryptofolio binary | `cryptofolio` (on PATH) |
