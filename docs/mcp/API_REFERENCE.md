# Cryptofolio MCP Server - API Reference

**Version:** 1.0.0 (Draft)
**Last Updated:** February 19, 2026
**Protocol:** Model Context Protocol (MCP) v1.0

---

## Table of Contents

- [Overview](#overview)
- [Connection](#connection)
- [Tool Catalog](#tool-catalog)
- [Error Handling](#error-handling)
- [Rate Limiting](#rate-limiting)
- [Examples](#examples)

---

## Overview

The Cryptofolio MCP Server exposes 10 curated tools following MCP best practices. All tools return structured JSON responses with success indicators and helpful error messages.

### Design Principles

1. **Outcomes Over Operations:** Tools aggregate related operations
2. **Flat Arguments:** No nested objects, top-level primitives only
3. **Instructions as Context:** Rich docstrings for agent understanding
4. **Ruthless Curation:** Limited to 10 essential tools
5. **Discovery-Oriented Naming:** `cryptofolio_{action}_{resource}` pattern
6. **Pagination:** Large result sets include metadata

### Response Format

All tools return:
```typescript
{
  success: boolean,
  [data]: any,        // Tool-specific data
  message?: string,   // Human-readable message
  error?: string      // Error message if success = false
}
```

---

## Connection

### Server Information

```json
{
  "name": "cryptofolio",
  "version": "1.0.0",
  "protocol_version": "1.0",
  "capabilities": {
    "tools": {}
  }
}
```

### Transport

MCP server communicates via stdio (standard input/output):

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

---

## Tool Catalog

### 1. cryptofolio_get_portfolio

**Description:** Get complete portfolio overview with unrealized P&L across all accounts.

**When to Use:**
- User asks "What's my portfolio?"
- User wants to see total value
- User wants P&L summary

**Input Schema:**
```typescript
{
  account?: string  // Optional: filter by specific account name
}
```

**Output:**
```typescript
{
  success: true,
  portfolio: {
    total_value_usd: string,      // Total portfolio value (decimal as string)
    cost_basis: string,            // Total cost basis
    unrealized_pnl: string,        // Unrealized profit/loss
    unrealized_pnl_percent: string, // P&L percentage
    accounts: Array<{
      name: string,                // Account name
      type: string,                // exchange | hardware_wallet | software_wallet | bank
      holdings: Array<{
        asset: string,             // Asset symbol (BTC, ETH, etc.)
        quantity: string,          // Quantity (decimal as string)
        price_usd: string,         // Current USD price
        value_usd: string,         // Current value (quantity × price)
        cost_basis?: string,       // Average cost basis
        pnl?: string,              // Unrealized P&L for this holding
        pnl_percent?: string       // P&L percentage
      }>
    }>
  }
}
```

**Example Request:**
```json
{
  "name": "cryptofolio_get_portfolio",
  "arguments": {}
}
```

**Example Response:**
```json
{
  "success": true,
  "portfolio": {
    "total_value_usd": "61442.89",
    "cost_basis": "29317.39",
    "unrealized_pnl": "32125.50",
    "unrealized_pnl_percent": "109.57",
    "accounts": [
      {
        "name": "Binance",
        "type": "exchange",
        "holdings": [
          {
            "asset": "BTC",
            "quantity": "0.09121000",
            "price_usd": "70253.98",
            "value_usd": "6407.86",
            "cost_basis": "5500.00",
            "pnl": "907.86",
            "pnl_percent": "16.51"
          }
        ]
      }
    ]
  }
}
```

**Errors:**
```json
{
  "success": false,
  "error": "Account 'NonExistent' not found. Available accounts: Binance, Ledger, MetaMask"
}
```

---

### 2. cryptofolio_get_prices

**Description:** Get current cryptocurrency prices from Binance (Spot and Alpha markets).

**When to Use:**
- User asks "What's the price of Bitcoin?"
- User wants to check multiple prices
- User needs current market rates

**Input Schema:**
```typescript
{
  assets: string[]  // Array of asset symbols (e.g., ["BTC", "ETH", "SOL"])
}
```

**Output:**
```typescript
{
  success: true,
  prices: Array<{
    symbol: string,     // Asset symbol
    price_usd: string,  // Current USD price
    source: string,     // Data source (e.g., "binance", "binance-alpha")
    timestamp: string   // ISO 8601 timestamp
  }>
}
```

**Example Request:**
```json
{
  "name": "cryptofolio_get_prices",
  "arguments": {
    "assets": ["BTC", "ETH", "NIGHT"]
  }
}
```

**Example Response:**
```json
{
  "success": true,
  "prices": [
    {
      "symbol": "BTC",
      "price_usd": "70253.98",
      "source": "binance",
      "timestamp": "2026-02-19T10:30:00Z"
    },
    {
      "symbol": "ETH",
      "price_usd": "2088.30",
      "source": "binance",
      "timestamp": "2026-02-19T10:30:00Z"
    },
    {
      "symbol": "NIGHT",
      "price_usd": "0.05",
      "source": "binance-alpha",
      "timestamp": "2026-02-19T10:30:00Z"
    }
  ]
}
```

**Errors:**
```json
{
  "success": false,
  "error": "Price not found for INVALIDCOIN. Check symbol and try again."
}
```

---

### 3. cryptofolio_get_market_data

**Description:** Get detailed 24-hour market statistics for a cryptocurrency.

**When to Use:**
- User asks "How is Bitcoin performing?"
- User wants 24h high/low/volume
- User wants detailed market analysis

**Input Schema:**
```typescript
{
  asset: string  // Asset symbol (e.g., "BTC")
}
```

**Output:**
```typescript
{
  success: true,
  market: {
    symbol: string,              // Asset symbol
    price_usd: string,           // Current price
    change_24h: string,          // Absolute price change
    change_percent_24h: string,  // Percentage change
    high_24h: string,            // 24h high
    low_24h: string,             // 24h low
    volume_24h: string,          // 24h volume
    quote_volume_24h: string,    // 24h quote volume (USD)
    timestamp: string            // ISO 8601 timestamp
  }
}
```

**Example Request:**
```json
{
  "name": "cryptofolio_get_market_data",
  "arguments": {
    "asset": "BTC"
  }
}
```

**Example Response:**
```json
{
  "success": true,
  "market": {
    "symbol": "BTC",
    "price_usd": "70253.98",
    "change_24h": "2145.00",
    "change_percent_24h": "2.25",
    "high_24h": "71500.00",
    "low_24h": "68000.00",
    "volume_24h": "12543.57",
    "quote_volume_24h": "881236432.00",
    "timestamp": "2026-02-19T10:30:00Z"
  }
}
```

---

### 4. cryptofolio_record_transaction

**Description:** Record a buy, sell, transfer, or swap transaction with automatic cost basis tracking.

**When to Use:**
- User says "I bought 0.1 BTC"
- User describes a sale, transfer, or swap
- User wants to record historical transaction

**Input Schema:**
```typescript
{
  type: "buy" | "sell" | "transfer" | "swap",
  asset: string,              // Primary asset
  quantity: string,           // Quantity (decimal as string)
  price?: string,             // Price in USD (required for buy/sell)
  account?: string,           // Account name (required for buy/sell)
  from_account?: string,      // Source account (required for transfer)
  to_account?: string,        // Destination account (required for transfer)
  from_asset?: string,        // Source asset (for swap)
  from_quantity?: string,     // Source quantity (for swap)
  to_asset?: string,          // Destination asset (for swap)
  to_quantity?: string,       // Destination quantity (for swap)
  fee?: string,               // Transaction fee
  fee_asset?: string,         // Fee asset (defaults to main asset)
  timestamp?: string,         // ISO 8601 timestamp (defaults to now)
  notes?: string              // Optional notes
}
```

**Output:**
```typescript
{
  success: true,
  transaction_id: number,
  message: string,
  updated_holdings?: {
    asset: string,
    new_quantity: string,
    new_cost_basis: string
  }
}
```

**Example Request (Buy):**
```json
{
  "name": "cryptofolio_record_transaction",
  "arguments": {
    "type": "buy",
    "asset": "BTC",
    "quantity": "0.1",
    "price": "95000",
    "account": "Binance",
    "notes": "Weekly DCA purchase"
  }
}
```

**Example Response:**
```json
{
  "success": true,
  "transaction_id": 123,
  "message": "Recorded buy: 0.1 BTC @ $95,000.00 in 'Binance'",
  "updated_holdings": {
    "asset": "BTC",
    "new_quantity": "0.19121",
    "new_cost_basis": "87500.00"
  }
}
```

**Example Request (Transfer):**
```json
{
  "name": "cryptofolio_record_transaction",
  "arguments": {
    "type": "transfer",
    "asset": "BTC",
    "quantity": "0.15",
    "from_account": "Binance",
    "to_account": "Ledger Nano X",
    "fee": "0.00005",
    "notes": "Moving to cold storage"
  }
}
```

**Errors:**
```json
{
  "success": false,
  "error": "Account 'Binance' not found. Available accounts: Ledger, MetaMask. Create account with cryptofolio CLI: cryptofolio account add Binance --type exchange"
}
```

---

### 5. cryptofolio_track_conversion

**Description:** Track multi-currency conversion flow with automatic exchange rate storage (e.g., CRC → USD → USDT → BTC).

**When to Use:**
- User describes fiat-to-crypto on-ramp
- User mentions multi-step conversion
- User is tracking Costa Rica flow or similar

**Input Schema:**
```typescript
{
  description: string,  // Natural language description of flow
  steps: Array<{
    from: string,       // Source currency/asset
    to: string,         // Destination currency/asset
    amount: string,     // Amount being converted
    rate?: string,      // Exchange rate (for fiat conversions)
    account?: string,   // Account where conversion happens
    fee?: string,       // Fee (if any)
    notes?: string      // Step-specific notes
  }>
}
```

**Output:**
```typescript
{
  success: true,
  transactions: Array<{
    id: number,
    type: string,
    from: string,
    to: string
  }>,
  final_holding: {
    asset: string,
    quantity: string,
    cost_basis_original: string,    // Cost in original currency
    cost_basis_usd: string,          // Cost in USD
    exchange_rates_stored: number    // Number of rates stored
  }
}
```

**Example Request:**
```json
{
  "name": "cryptofolio_track_conversion",
  "arguments": {
    "description": "Costa Rica on-ramp: CRC to BTC",
    "steps": [
      {
        "from": "CRC",
        "to": "USD",
        "amount": "100000",
        "rate": "550",
        "account": "Banco Nacional",
        "notes": "Bank conversion"
      },
      {
        "from": "USD",
        "to": "USD",
        "amount": "181.82",
        "account": "Banco Nacional",
        "notes": "Transfer to Lulubit"
      },
      {
        "from": "USD",
        "to": "USDT",
        "amount": "181.82",
        "account": "Lulubit",
        "notes": "3% fee on-ramp"
      },
      {
        "from": "USDT",
        "to": "USDT",
        "amount": "176",
        "fee": "0.1",
        "account": "Binance",
        "notes": "Network transfer"
      },
      {
        "from": "USDT",
        "to": "BTC",
        "amount": "175.9",
        "account": "Binance"
      }
    ]
  }
}
```

**Example Response:**
```json
{
  "success": true,
  "transactions": [
    { "id": 124, "type": "swap", "from": "CRC", "to": "USD" },
    { "id": 125, "type": "transfer", "from": "USD", "to": "USD" },
    { "id": 126, "type": "swap", "from": "USD", "to": "USDT" },
    { "id": 127, "type": "transfer", "from": "USDT", "to": "USDT" },
    { "id": 128, "type": "swap", "from": "USDT", "to": "BTC" }
  ],
  "final_holding": {
    "asset": "BTC",
    "quantity": "0.0025",
    "cost_basis_original": "100000 CRC",
    "cost_basis_usd": "181.82",
    "exchange_rates_stored": 1
  }
}
```

---

### 6. cryptofolio_list_transactions

**Description:** List transaction history with pagination and filtering.

**When to Use:**
- User asks "Show my transactions"
- User wants to review recent activity
- User needs transaction history for analysis

**Input Schema:**
```typescript
{
  limit?: number,      // Max results (default: 50, max: 200)
  offset?: number,     // Pagination offset (default: 0)
  account?: string,    // Filter by account
  asset?: string,      // Filter by asset
  type?: "buy" | "sell" | "transfer" | "swap",  // Filter by type
  from_date?: string,  // ISO 8601 date (filter start)
  to_date?: string     // ISO 8601 date (filter end)
}
```

**Output:**
```typescript
{
  success: true,
  transactions: Array<{
    id: number,
    type: string,
    asset: string,
    quantity: string,
    price_usd?: string,
    account?: string,
    from_account?: string,
    to_account?: string,
    fee?: string,
    timestamp: string,
    notes?: string
  }>,
  metadata: {
    total_count: number,   // Total matching transactions
    limit: number,         // Requested limit
    offset: number,        // Current offset
    has_more: boolean,     // More results available?
    next_offset: number    // Offset for next page
  }
}
```

**Example Request:**
```json
{
  "name": "cryptofolio_list_transactions",
  "arguments": {
    "limit": 50,
    "asset": "BTC",
    "type": "buy"
  }
}
```

**Example Response:**
```json
{
  "success": true,
  "transactions": [
    {
      "id": 123,
      "type": "buy",
      "asset": "BTC",
      "quantity": "0.1",
      "price_usd": "95000",
      "account": "Binance",
      "timestamp": "2026-02-19T10:30:00Z",
      "notes": "Weekly DCA"
    }
  ],
  "metadata": {
    "total_count": 127,
    "limit": 50,
    "offset": 0,
    "has_more": true,
    "next_offset": 50
  }
}
```

**Pagination Example:**
```typescript
// First page
{ limit: 50, offset: 0 }  // Returns items 1-50

// Second page (using next_offset from metadata)
{ limit: 50, offset: 50 }  // Returns items 51-100

// Third page
{ limit: 50, offset: 100 }  // Returns items 101-127 (last page)
```

---

### 7. cryptofolio_analyze_asset

**Description:** Get comprehensive analysis of a specific asset (holdings, transactions, P&L).

**When to Use:**
- User asks "Analyze my Bitcoin"
- User wants deep dive on specific asset
- User wants performance metrics

**Input Schema:**
```typescript
{
  asset: string,     // Asset symbol (e.g., "BTC")
  year?: number      // Optional: filter transactions by year
}
```

**Output:**
```typescript
{
  success: true,
  analysis: {
    asset: string,
    total_quantity: string,
    total_cost_basis: string,
    current_value: string,
    unrealized_pnl: string,
    unrealized_pnl_percent: string,
    accounts: Array<{
      name: string,
      quantity: string,
      percentage: string  // % of total holdings
    }>,
    transactions_summary: {
      total_buys: number,
      total_sells: number,
      total_transfers: number,
      total_swaps: number,
      avg_buy_price: string,
      avg_sell_price?: string,
      first_purchase_date: string,
      last_activity_date: string
    },
    performance?: {
      ytd_return: string,
      ytd_return_percent: string,
      all_time_return: string,
      all_time_return_percent: string
    }
  }
}
```

**Example Request:**
```json
{
  "name": "cryptofolio_analyze_asset",
  "arguments": {
    "asset": "BTC",
    "year": 2026
  }
}
```

**Example Response:**
```json
{
  "success": true,
  "analysis": {
    "asset": "BTC",
    "total_quantity": "0.19121",
    "total_cost_basis": "87500.00",
    "current_value": "13437.72",
    "unrealized_pnl": "5937.72",
    "unrealized_pnl_percent": "67.86",
    "accounts": [
      { "name": "Binance", "quantity": "0.09121", "percentage": "47.7" },
      { "name": "Ledger", "quantity": "0.10000", "percentage": "52.3" }
    ],
    "transactions_summary": {
      "total_buys": 12,
      "total_sells": 0,
      "total_transfers": 3,
      "total_swaps": 0,
      "avg_buy_price": "87500.00",
      "first_purchase_date": "2026-01-05T10:00:00Z",
      "last_activity_date": "2026-02-19T10:30:00Z"
    },
    "performance": {
      "ytd_return": "5937.72",
      "ytd_return_percent": "67.86",
      "all_time_return": "5937.72",
      "all_time_return_percent": "67.86"
    }
  }
}
```

---

### 8. cryptofolio_export_transactions

**Description:** Export transactions to CSV file for tax reporting or analysis.

**When to Use:**
- User says "Export for taxes"
- User needs CSV for accountant
- User wants to analyze in Excel/Google Sheets

**Input Schema:**
```typescript
{
  year?: number,       // Filter by year (e.g., 2024)
  account?: string,    // Filter by account
  asset?: string,      // Filter by asset
  from_date?: string,  // ISO 8601 date
  to_date?: string     // ISO 8601 date
}
```

**Output:**
```typescript
{
  success: true,
  csv_path: string,    // Absolute path to generated CSV
  row_count: number,   // Number of transactions exported
  message: string,
  filters_applied: {
    year?: number,
    account?: string,
    asset?: string,
    date_range?: string
  }
}
```

**Example Request:**
```json
{
  "name": "cryptofolio_export_transactions",
  "arguments": {
    "year": 2024
  }
}
```

**Example Response:**
```json
{
  "success": true,
  "csv_path": "/Users/user/.config/cryptofolio/exports/transactions_2024.csv",
  "row_count": 127,
  "message": "Exported 127 transactions to CSV",
  "filters_applied": {
    "year": 2024
  }
}
```

**CSV Format:**
```csv
date,type,asset,quantity,price_usd,fee,account,from_account,to_account,cost_basis,notes
2024-01-15T10:30:00Z,buy,BTC,0.1,45000,0,Binance,,,4500,"First purchase"
2024-02-01T14:20:00Z,transfer,BTC,0.05,,,Binance,Binance,Ledger,0.00001,"Moving to cold storage"
```

---

### 9. cryptofolio_list_accounts

**Description:** List all configured accounts with their types and sync status.

**When to Use:**
- User asks "What accounts do I have?"
- User wants to see account setup
- User needs to verify account names for transactions

**Input Schema:**
```typescript
{}  // No parameters
```

**Output:**
```typescript
{
  success: true,
  accounts: Array<{
    name: string,
    type: "exchange" | "hardware_wallet" | "software_wallet" | "bank",
    category?: string,  // e.g., "trading", "cold-storage", "banking"
    sync_enabled: boolean,
    created_at: string
  }>
}
```

**Example Request:**
```json
{
  "name": "cryptofolio_list_accounts",
  "arguments": {}
}
```

**Example Response:**
```json
{
  "success": true,
  "accounts": [
    {
      "name": "Binance",
      "type": "exchange",
      "category": "trading",
      "sync_enabled": true,
      "created_at": "2026-01-01T00:00:00Z"
    },
    {
      "name": "Ledger Nano X",
      "type": "hardware_wallet",
      "category": "cold-storage",
      "sync_enabled": false,
      "created_at": "2026-01-05T12:00:00Z"
    },
    {
      "name": "Banco Nacional",
      "type": "bank",
      "category": "banking",
      "sync_enabled": false,
      "created_at": "2026-02-01T08:00:00Z"
    }
  ]
}
```

---

### 10. cryptofolio_sync_exchange

**Description:** Sync holdings from exchange API (currently supports Binance).

**When to Use:**
- User says "Sync my Binance"
- User wants to verify balances
- User wants to update holdings automatically

**Input Schema:**
```typescript
{
  account?: string  // Optional: specific account (defaults to all sync-enabled)
}
```

**Output:**
```typescript
{
  success: true,
  synced_accounts: Array<{
    account: string,
    assets_synced: number,
    holdings: Array<{
      asset: string,
      quantity: string,
      value_usd?: string
    }>
  }>
}
```

**Example Request:**
```json
{
  "name": "cryptofolio_sync_exchange",
  "arguments": {
    "account": "Binance"
  }
}
```

**Example Response:**
```json
{
  "success": true,
  "synced_accounts": [
    {
      "account": "Binance",
      "assets_synced": 3,
      "holdings": [
        { "asset": "BTC", "quantity": "0.09121000" },
        { "asset": "ETH", "quantity": "2.4594" },
        { "asset": "USDT", "quantity": "500.00" }
      ]
    }
  ]
}
```

**Errors:**
```json
{
  "success": false,
  "error": "Binance API authentication failed. Please check API keys with 'cryptofolio config show' and verify READ permissions are enabled."
}
```

---

## Error Handling

### Error Response Format

All errors follow this format:
```typescript
{
  success: false,
  error: string,      // Human-readable error message
  code?: string,      // Machine-readable error code (optional)
  details?: object    // Additional error context (optional)
}
```

### Common Error Codes

| Code | Description | Example |
|------|-------------|---------|
| `ACCOUNT_NOT_FOUND` | Account doesn't exist | "Account 'Binance' not found" |
| `ASSET_NOT_FOUND` | Asset symbol invalid | "Asset 'INVALIDCOIN' not found" |
| `API_ERROR` | Exchange API failure | "Binance API authentication failed" |
| `VALIDATION_ERROR` | Input validation failed | "Quantity must be positive number" |
| `CLI_ERROR` | Cryptofolio CLI error | "Database connection failed" |
| `PERMISSION_ERROR` | Insufficient permissions | "Cannot sync: API keys not configured" |

### Error Message Guidelines

**✅ Good Error Messages (Agent-Friendly):**
```json
{
  "success": false,
  "error": "Account 'MyWallet' not found. Available accounts: Binance, Ledger, MetaMask. Create a new account with: cryptofolio account add 'MyWallet' --type hardware_wallet"
}
```

**❌ Bad Error Messages:**
```json
{
  "success": false,
  "error": "ERROR: ACCOUNT_NOT_FOUND"
}
```

Good messages:
- Explain what went wrong
- List available options
- Suggest how to fix
- Use natural language

---

## Rate Limiting

### Binance API

**Limits:**
- Price data: 1200 requests/minute
- Account data: 1200 requests/minute
- Order data: 50 requests/10 seconds

**MCP Server Behavior:**
- Caches prices for 10 seconds
- Caches market data for 30 seconds
- No rate limiting on local database operations

### Local Operations

No rate limits for:
- `cryptofolio_get_portfolio`
- `cryptofolio_list_transactions`
- `cryptofolio_list_accounts`
- `cryptofolio_record_transaction`

---

## Examples

### Example 1: Complete DCA Workflow

```typescript
// Step 1: Check current price
{
  "name": "cryptofolio_get_prices",
  "arguments": { "assets": ["BTC"] }
}
// Response: BTC = $70,253.98

// Step 2: Record purchase
{
  "name": "cryptofolio_record_transaction",
  "arguments": {
    "type": "buy",
    "asset": "BTC",
    "quantity": "0.00142341",
    "price": "70253.98",
    "account": "Binance",
    "notes": "Weekly DCA"
  }
}
// Response: Transaction #123 recorded

// Step 3: Verify with sync
{
  "name": "cryptofolio_sync_exchange",
  "arguments": { "account": "Binance" }
}
// Response: BTC: 0.00142341 ✓
```

### Example 2: Multi-Page Transaction History

```typescript
// Page 1: Get first 50 transactions
{
  "name": "cryptofolio_list_transactions",
  "arguments": { "limit": 50, "offset": 0 }
}
// Response: metadata.has_more = true, next_offset = 50

// Page 2: Get next 50
{
  "name": "cryptofolio_list_transactions",
  "arguments": { "limit": 50, "offset": 50 }
}
// Response: metadata.has_more = true, next_offset = 100

// Page 3: Get remaining
{
  "name": "cryptofolio_list_transactions",
  "arguments": { "limit": 50, "offset": 100 }
}
// Response: metadata.has_more = false (last page)
```

### Example 3: Portfolio Analysis Chain

```typescript
// Step 1: Get portfolio overview
{
  "name": "cryptofolio_get_portfolio",
  "arguments": {}
}

// Step 2: Analyze top asset
{
  "name": "cryptofolio_analyze_asset",
  "arguments": { "asset": "BTC" }
}

// Step 3: Get market data
{
  "name": "cryptofolio_get_market_data",
  "arguments": { "asset": "BTC" }
}

// Step 4: Export for records
{
  "name": "cryptofolio_export_transactions",
  "arguments": { "asset": "BTC", "year": 2026 }
}
```

---

## Version History

### v1.0.0 (Draft - February 2026)
- Initial API design
- 10 curated tools
- MCP v1.0 protocol compliance
- Pagination support
- Multi-currency tracking

---

## References

- [MCP Specification](https://modelcontextprotocol.io/)
- [MCP Best Practices](https://www.philschmid.de/mcp-best-practices)
- [Cryptofolio CLI Documentation](../README.md)
- [MCP Technical Analysis](./MCP_TECHNICAL_ANALYSIS.md)
- [MCP User Guide](./MCP_USER_GUIDE.md)

---

**Document Status:** Draft
**Feedback:** https://github.com/yourusername/cryptofolio-mcp/issues
