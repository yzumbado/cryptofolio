# Cryptofolio — Claude Code Guide

## Quick start

Start a portfolio management session:

```
/portfolio
```

This loads the **Portfolio Agent** — an expert on your crypto portfolio data. It connects to the cryptofolio MCP server and gives you a natural language interface to manage wallets, track P&L, record transactions, and sync on-chain balances.

## Prerequisites

The MCP server must be running and configured in your Claude Code settings. See `mcp/README.md` for setup instructions.

## Skills

| Skill | Command | Description |
|---|---|---|
| Portfolio Agent | `/portfolio` | Manage wallets, balances, transactions, and P&L |

## Architecture

- `src/` — Rust CLI (`cryptofolio` binary)
- `mcp/` — MCP server (TypeScript, exposes `cryptofolio_*` tools)
- `.claude/skills/` — Claude Code skills (AI-native interface layer)
- `tests/` — Integration tests (`blockchain_clients.rs`) and BDD suite

## Running tests

```bash
# Unit tests
cargo test --lib

# BDD suite
cargo test --test bdd

# Integration tests (real APIs)
CRYPTOFOLIO_INTEGRATION_TESTS=1 \
  ETHERSCAN_API_KEY=... \
  BLOCKFROST_API_KEY=... \
  SOLANA_RPC_URL=... \
  cargo test --test blockchain_clients
```
