# Cryptofolio MCP Server - Technical Analysis

**Date:** February 19, 2026
**Purpose:** Technical analysis for implementing Model Context Protocol (MCP) server integration with Claude Desktop
**Status:** Design Phase

---

## Executive Summary

This document analyzes technical approaches for integrating Cryptofolio with Claude Desktop via MCP (Model Context Protocol), applying industry best practices from Philipp Schmid's MCP design principles. The recommendation is a **Node.js MCP server** that wraps Cryptofolio CLI commands, following outcome-oriented design with strict tool curation.

**Recommendation:** Node.js MCP Server (Option 1)
**Rationale:** Fastest implementation, leverages existing JSON output, follows MCP best practices, maintains separation of concerns

---

## Table of Contents

- [MCP Design Principles](#mcp-design-principles)
- [Implementation Options](#implementation-options)
- [Pros & Cons Analysis](#pros--cons-analysis)
- [Recommended Architecture](#recommended-architecture)
- [Tool Design](#tool-design)
- [Security Considerations](#security-considerations)
- [Implementation Roadmap](#implementation-roadmap)

---

## MCP Design Principles

### Core Philosophy

> **MCP servers are user interfaces for AI agents, not infrastructure for developers.**

Key principle: Design for agent capabilities, not API comprehensiveness.

### Best Practices Applied

#### 1. Outcomes Over Operations
**Principle:** Aggregate related operations into single, outcome-focused tools.

**Application to Cryptofolio:**
- ❌ **Bad:** Separate tools for `get_btc_price`, `get_eth_price`, `get_sol_price`
- ✅ **Good:** Single `cryptofolio_get_prices` tool that accepts asset list
- ❌ **Bad:** `get_holdings`, `get_cost_basis`, `calculate_pnl` as separate tools
- ✅ **Good:** Single `cryptofolio_get_portfolio` that returns complete view

**Example:**
```typescript
// Bad: Multiple operations
cryptofolio_list_transactions()
cryptofolio_filter_by_asset()
cryptofolio_calculate_gains()

// Good: Single outcome
cryptofolio_analyze_asset_performance(asset: "BTC", year: 2024)
// Returns: transactions, P&L, cost basis, all in one response
```

#### 2. Flatten Arguments
**Principle:** Use top-level primitives with constrained types. Avoid nested objects.

**Application:**
```typescript
// Bad: Nested configuration
{
  transaction: {
    type: "buy",
    details: {
      asset: "BTC",
      quantity: 0.1
    },
    account: {
      name: "Binance"
    }
  }
}

// Good: Flat primitives
{
  asset: "BTC",
  quantity: 0.1,
  price: 95000,
  account: "Binance",
  type: "buy"  // Literal["buy", "sell", "transfer", "swap"]
}
```

#### 3. Instructions as Context
**Principle:** Documentation is active context consumed by agents. Error messages are observations.

**Application:**
```typescript
{
  name: "cryptofolio_record_transaction",
  description: `Record a cryptocurrency transaction with automatic cost basis tracking.

Use when: User mentions buying, selling, or transferring crypto.
Date format: ISO 8601 (2024-01-15T10:30:00Z) or omit for current time.
Price: Always in USD unless using multi-currency swap.

Examples:
- "I bought 0.1 BTC at $95,000" → type: "buy", asset: "BTC", quantity: 0.1, price: 95000
- "Transferred 1 ETH to my Ledger" → type: "transfer", asset: "ETH", from: "Binance", to: "Ledger"`,

  inputSchema: {
    // Flattened, well-documented schema
  }
}
```

**Error messages as observations:**
```typescript
// Bad: throw new Error("Account not found")
// Good:
return {
  success: false,
  message: "Account 'MyWallet' not found. Available accounts: Binance, Ledger, MetaMask. Create a new account with cryptofolio_create_account."
}
```

#### 4. Ruthless Curation
**Principle:** Limit to 5-15 focused tools. Each server addresses one primary function.

**Application to Cryptofolio:**
- **Total tools:** 10-12 (within recommended range)
- **Primary function:** Crypto portfolio management
- **Excluded:** Advanced admin operations, database maintenance, configuration management

**Tool curation rationale:**
- ✅ **Include:** Portfolio viewing, price checking, transaction recording (core user tasks)
- ❌ **Exclude:** Database migrations, config file editing, testnet switching (developer tasks)
- ✅ **Include:** Multi-currency conversions (unique value proposition)
- ❌ **Exclude:** Low-level account management, category CRUD operations

#### 5. Discovery-Oriented Naming
**Principle:** Use `{service}_{action}_{resource}` format to prevent naming collisions.

**Application:**
```typescript
// All tools prefixed with "cryptofolio_"
cryptofolio_get_portfolio()
cryptofolio_get_prices()
cryptofolio_record_transaction()
cryptofolio_track_conversion()  // Multi-currency flow
cryptofolio_export_transactions()
cryptofolio_get_market_data()
```

**Why this matters:**
- Claude Desktop may load multiple MCP servers simultaneously
- Prevents collision with `github_get_issues`, `slack_send_message`, etc.
- Clear namespace for agent to understand tool origin

#### 6. Pagination with Metadata
**Principle:** Implement pagination with `limit`, return metadata (`has_more`, `next_offset`, `total_count`).

**Application to Cryptofolio:**
```typescript
// Transactions endpoint
cryptofolio_list_transactions({
  limit: 50,        // Default: 50, Max: 200
  offset: 0,
  account?: string,
  asset?: string
})

// Response with metadata
{
  transactions: [...],  // Up to 50 items
  metadata: {
    total_count: 347,
    limit: 50,
    offset: 0,
    has_more: true,
    next_offset: 50
  }
}
```

**Why this matters:**
- Cryptofolio may have 1000+ transactions
- Loading all into context bloats agent memory
- Pagination allows incremental exploration

---

## Implementation Options

### Option 1: Node.js MCP Server (Wrapper Approach)

**Architecture:**
```
Claude Desktop
    ↓ (MCP Protocol)
Node.js MCP Server
    ↓ (exec/spawn)
Cryptofolio CLI (Rust binary)
    ↓ (--json flag)
SQLite Database
```

**Description:**
- TypeScript/Node.js server using `@modelcontextprotocol/sdk`
- Wraps Cryptofolio CLI commands via `child_process.exec()`
- Parses JSON output from `--json` flag
- Transforms responses to match MCP best practices

**Key Dependencies:**
- `@modelcontextprotocol/sdk` - MCP SDK from Anthropic
- `zod` - Schema validation for tool inputs
- `execa` - Better subprocess management than raw `exec`

**Implementation Example:**
```typescript
import { McpServer } from '@modelcontextprotocol/sdk';
import { execa } from 'execa';

const server = new McpServer({
  name: 'cryptofolio',
  version: '0.2.0'
});

server.tool({
  name: 'cryptofolio_get_portfolio',
  description: 'Get current portfolio with P&L calculations...',
  inputSchema: {
    type: 'object',
    properties: {
      account: { type: 'string', description: 'Optional account filter' }
    }
  },
  handler: async ({ account }) => {
    const args = ['portfolio', '--json'];
    if (account) args.push('--account', account);

    const { stdout } = await execa('cryptofolio', args);
    const data = JSON.parse(stdout);

    // Transform to MCP-friendly format
    return {
      success: true,
      portfolio: {
        total_value_usd: data.total_value_usd,
        unrealized_pnl: data.unrealized_pnl,
        pnl_percent: data.unrealized_pnl_percent,
        holdings: data.entries.map(e => ({
          account: e.account_name,
          assets: e.holdings
        }))
      }
    };
  }
});
```

### Option 2: Rust MCP Server (Native Integration)

**Architecture:**
```
Claude Desktop
    ↓ (MCP Protocol)
Rust MCP Server (integrated with Cryptofolio)
    ↓ (direct function calls)
SQLite Database
```

**Description:**
- Rust implementation using `mcp-server-sdk-rust` (if available) or custom JSON-RPC
- Direct integration with Cryptofolio core library
- Shared database connection pool
- No subprocess overhead

**Key Dependencies:**
- `serde_json` - JSON serialization
- `tokio` - Async runtime (already in Cryptofolio)
- Custom MCP protocol implementation or SDK

**Implementation Approach:**
```rust
// Refactor Cryptofolio to library + CLI + MCP server
cryptofolio/
├── cryptofolio-core/     # Shared business logic
│   ├── portfolio.rs
│   ├── transactions.rs
│   └── db/
├── cryptofolio-cli/      # CLI binary
│   └── main.rs
└── cryptofolio-mcp/      # MCP server binary
    └── main.rs

// MCP server uses core directly
use cryptofolio_core::portfolio;

async fn handle_get_portfolio(account: Option<String>) -> Result<Portfolio> {
    let pool = get_db_pool().await?;
    portfolio::get_portfolio(&pool, account).await
}
```

### Option 3: Direct CLI Integration (Simple Approach)

**Architecture:**
```
Claude Desktop (Bash tool)
    ↓ (shell commands)
Cryptofolio CLI
    ↓
SQLite Database
```

**Description:**
- No MCP server needed
- Claude Desktop uses built-in bash execution
- User provides commands in conversation
- Limited structure and validation

**Example Usage:**
```
User: "What's my portfolio?"
Claude: Let me check your portfolio.
[Runs: cryptofolio portfolio --json]
[Parses output and presents to user]
```

---

## Pros & Cons Analysis

### Option 1: Node.js MCP Server (Wrapper)

#### Pros
✅ **Fast Implementation** (~4-6 hours with AI assistance)
- MCP SDK available and well-documented
- No Rust changes needed
- Leverages existing `--json` output
- TypeScript provides excellent developer experience

✅ **Separation of Concerns**
- Cryptofolio CLI remains standalone
- MCP server is independent service
- Can version independently
- Easy to test in isolation

✅ **Standard Architecture**
- Follows common MCP server patterns
- Community examples available
- Easy onboarding for contributors
- Well-understood maintenance model

✅ **JSON Output Already Exists**
- All commands support `--json` flag
- No parsing logic needed
- Cryptofolio team already maintains this format

✅ **Flexible Transformation Layer**
- Can reshape responses for agent consumption
- Apply pagination easily
- Add helpful error messages
- Aggregate multiple CLI calls if needed

✅ **Easy Distribution**
- NPM package for installation
- Works on any platform with Node.js
- Simple Claude Desktop configuration

#### Cons
❌ **Subprocess Overhead**
- Each tool call spawns new process (~10-50ms latency)
- Not ideal for high-frequency operations
- Memory overhead for process creation

❌ **Extra Dependency**
- Requires Node.js runtime (in addition to Rust binary)
- Adds complexity to deployment
- Version compatibility considerations

❌ **Indirect Database Access**
- Cannot share connection pool with CLI
- Multiple SQLite connections (file locking considerations)
- No ability to optimize queries

❌ **Error Message Translation**
- Must parse CLI stderr/stdout
- Error messages designed for humans, not agents
- Requires transformation logic

❌ **Maintenance Burden**
- Two codebases to maintain
- CLI changes may require MCP server updates
- Version synchronization needed

#### Best For
- **Quick time-to-market** (production in days)
- **Separation of concerns** (independent teams)
- **Leveraging existing CLI** (no Rust changes)

---

### Option 2: Rust MCP Server (Native)

#### Pros
✅ **Performance**
- No subprocess overhead
- Direct database access with connection pooling
- Minimal latency (~1-5ms vs 10-50ms)

✅ **Shared Codebase**
- Single repository
- Shared business logic (DRY principle)
- Consistent error handling
- Type safety across CLI and MCP

✅ **Optimized Queries**
- Can create MCP-specific database queries
- Implement pagination at SQL level
- Efficient data transformations
- Custom indexes for MCP access patterns

✅ **No Runtime Dependencies**
- Single Rust binary for both CLI and MCP
- Smaller deployment footprint
- Easier distribution

✅ **Type Safety**
- Rust's type system prevents many bugs
- Compile-time guarantees
- Safer refactoring

#### Cons
❌ **Development Time** (~2-3 weeks)
- Rust MCP SDK may not exist or be immature
- Custom JSON-RPC implementation needed
- Requires significant Rust expertise
- Longer development cycle

❌ **Complexity**
- Refactoring required (library + CLI + MCP)
- More code to maintain in single repo
- Harder to test in isolation
- Steeper learning curve for contributors

❌ **MCP Protocol Maturity**
- MCP is new (2024 release)
- Rust ecosystem may lag Node.js
- Limited community examples
- Potential for protocol breaking changes

❌ **Coupling**
- CLI and MCP share same codebase
- Changes affect both surfaces
- Harder to version independently

❌ **Refactoring Risk**
- Requires restructuring existing code
- Potential for introducing bugs
- Extensive testing needed
- May delay other features

#### Best For
- **Long-term optimization** (worth 2-3 week investment)
- **Performance-critical use cases** (high-frequency trading bots)
- **Single-language maintenance** (Rust-only team)

---

### Option 3: Direct CLI Integration (No MCP Server)

#### Pros
✅ **Zero Development Time**
- Available immediately
- Uses Claude Desktop's bash capabilities
- No new code needed

✅ **No Maintenance**
- No MCP server to maintain
- No dependencies
- No version synchronization

✅ **Simple Architecture**
- Direct command execution
- No intermediate layers
- Easy to understand

#### Cons
❌ **Poor Agent Experience**
- No structured tool discovery
- Bash commands not in agent's "toolbox"
- Requires user to guide Claude
- No parameter validation

❌ **Limited Error Handling**
- Raw stderr output
- Not formatted for agents
- Hard to recover from errors

❌ **No Context Management**
- CLI output not optimized for agent context
- May dump large data into conversation
- No pagination support

❌ **Security Concerns**
- Bash injection risks if user provides input
- No sandboxing
- Full shell access

❌ **Not MCP Compliant**
- Doesn't follow MCP standards
- No metadata about capabilities
- Can't leverage MCP ecosystem

#### Best For
- **Quick testing** (validate concept)
- **Personal use** (single user, low frequency)
- **Temporary solution** (until proper MCP server ready)

---

## Recommended Architecture

### Winner: Option 1 (Node.js MCP Server)

**Decision Rationale:**
1. **Time to Value:** Production-ready in 4-6 hours (vs weeks for Rust)
2. **Best Practices Compliance:** TypeScript MCP SDK follows all 6 principles
3. **Risk Mitigation:** No changes to stable Cryptofolio CLI
4. **Community Alignment:** Majority of MCP servers use Node.js
5. **Maintenance:** Separate versioning allows independent updates

**Architecture Diagram:**
```
┌─────────────────────────────────────────────────────────────────┐
│                        Claude Desktop                           │
│  (User Interface - Anthropic's desktop app)                     │
└────────────────────────────┬────────────────────────────────────┘
                             │ MCP Protocol (JSON-RPC over stdio)
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Cryptofolio MCP Server                        │
│  (Node.js/TypeScript - @modelcontextprotocol/sdk)              │
│                                                                 │
│  ┌────────────────────────────────────────────────────────┐   │
│  │  Tool Registry (10-12 curated tools)                   │   │
│  │  - cryptofolio_get_portfolio                           │   │
│  │  - cryptofolio_get_prices                              │   │
│  │  - cryptofolio_record_transaction                      │   │
│  │  - cryptofolio_track_conversion (multi-currency)       │   │
│  │  - cryptofolio_list_transactions (paginated)           │   │
│  │  - etc.                                                │   │
│  └────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌────────────────────────────────────────────────────────┐   │
│  │  CLI Wrapper Layer (execa)                             │   │
│  │  - Argument construction                               │   │
│  │  - Process execution                                   │   │
│  │  - Error handling                                      │   │
│  └────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌────────────────────────────────────────────────────────┐   │
│  │  Response Transformer                                  │   │
│  │  - JSON parsing                                        │   │
│  │  - Pagination metadata                                 │   │
│  │  - Agent-friendly formatting                           │   │
│  └────────────────────────────────────────────────────────┘   │
└────────────────────────────┬────────────────────────────────────┘
                             │ child_process.exec()
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Cryptofolio CLI (Rust)                      │
│  Existing binary with --json support                            │
└────────────────────────────┬────────────────────────────────────┘
                             │ SQLite queries
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│              SQLite Database (~/.config/cryptofolio/)           │
│  - accounts, holdings, transactions, currencies, exchange_rates │
└─────────────────────────────────────────────────────────────────┘
```

**Technology Stack:**
- **Language:** TypeScript 5.x
- **Runtime:** Node.js 20.x LTS
- **MCP SDK:** `@modelcontextprotocol/sdk` ^1.0.0
- **Process Management:** `execa` ^8.0.0
- **Validation:** `zod` ^3.22.0
- **Testing:** `vitest` ^1.0.0

---

## Tool Design

### Curated Tool List (10 Tools)

Following **ruthless curation** principle, limit to 10 focused tools:

#### 1. `cryptofolio_get_portfolio`
**Purpose:** Get complete portfolio overview with P&L
**Use When:** User asks "What's my portfolio?" or "How much do I have?"
**Arguments:**
- `account?: string` - Optional account filter

**Response:**
```typescript
{
  success: true,
  portfolio: {
    total_value_usd: "61442.89",
    cost_basis: "29317.39",
    unrealized_pnl: "32125.50",
    pnl_percent: "109.57",
    accounts: [
      {
        name: "Binance",
        holdings: [
          { asset: "BTC", quantity: "0.09121", value_usd: "6407.86" }
        ]
      }
    ]
  }
}
```

#### 2. `cryptofolio_get_prices`
**Purpose:** Get current cryptocurrency prices
**Use When:** User asks "What's the price of Bitcoin?"
**Arguments:**
- `assets: string[]` - Array of asset symbols (BTC, ETH, etc.)

**Response:**
```typescript
{
  success: true,
  prices: [
    { symbol: "BTC", price_usd: "70253.98", source: "binance" },
    { symbol: "ETH", price_usd: "2088.30", source: "binance" }
  ]
}
```

#### 3. `cryptofolio_get_market_data`
**Purpose:** Get detailed 24h market statistics
**Use When:** User asks "How is Bitcoin performing today?"
**Arguments:**
- `asset: string` - Asset symbol (BTC, ETH, etc.)

**Response:**
```typescript
{
  success: true,
  market: {
    symbol: "BTC",
    price_usd: "70253.98",
    change_24h: "2145.00",
    change_percent_24h: "2.25",
    high_24h: "71500.00",
    low_24h: "68000.00",
    volume_24h: "12543.57"
  }
}
```

#### 4. `cryptofolio_record_transaction`
**Purpose:** Record buy/sell/transfer/swap with automatic cost basis
**Use When:** User says "I bought 0.1 BTC" or "I sold 0.5 ETH"
**Arguments:** (Flattened)
- `type: "buy" | "sell" | "transfer" | "swap"`
- `asset: string`
- `quantity: string` (decimal as string to preserve precision)
- `price?: string` (USD price, optional for transfers)
- `account?: string` (required for buy/sell)
- `from_account?: string` (required for transfer)
- `to_account?: string` (required for transfer)
- `fee?: string`
- `notes?: string`

**Response:**
```typescript
{
  success: true,
  transaction_id: 123,
  message: "Recorded buy: 0.1 BTC @ $95,000.00 in 'Binance'",
  updated_holdings: {
    asset: "BTC",
    new_quantity: "0.19121",
    new_cost_basis: "87500.00"
  }
}
```

#### 5. `cryptofolio_track_conversion`
**Purpose:** Track multi-currency on-ramp flow (CRC → USD → USDT → BTC)
**Use When:** User describes fiat-to-crypto conversion journey
**Arguments:**
- `flow_description: string` - Natural language description
- `steps: Array<{from, to, amount, rate}>`

**Example:**
```typescript
{
  flow_description: "Convert 100,000 CRC to BTC via USD and USDT",
  steps: [
    { from: "CRC", to: "USD", amount: "100000", rate: "550" },
    { from: "USD", to: "USDT", amount: "181.82" },
    { from: "USDT", to: "BTC", amount: "176" }
  ]
}
```

**Response:**
```typescript
{
  success: true,
  transactions: [
    { id: 124, type: "swap", from: "CRC", to: "USD" },
    { id: 125, type: "swap", from: "USD", to: "USDT" },
    { id: 126, type: "swap", from: "USDT", to: "BTC" }
  ],
  final_holding: {
    asset: "BTC",
    quantity: "0.0025",
    cost_basis_crc: "100000",
    cost_basis_usd: "181.82"
  }
}
```

#### 6. `cryptofolio_list_transactions`
**Purpose:** List transaction history with pagination
**Use When:** User asks "Show my recent transactions"
**Arguments:**
- `limit?: number` - Max 200, default 50
- `offset?: number` - Default 0
- `account?: string` - Filter by account
- `asset?: string` - Filter by asset
- `type?: "buy" | "sell" | "transfer" | "swap"`

**Response (with pagination metadata):**
```typescript
{
  success: true,
  transactions: [
    {
      id: 123,
      type: "buy",
      asset: "BTC",
      quantity: "0.1",
      price_usd: "95000",
      timestamp: "2024-02-15T10:30:00Z"
    }
    // ... up to 50 items
  ],
  metadata: {
    total_count: 347,
    limit: 50,
    offset: 0,
    has_more: true,
    next_offset: 50
  }
}
```

#### 7. `cryptofolio_analyze_asset`
**Purpose:** Get comprehensive asset analysis (holdings, transactions, P&L)
**Use When:** User asks "Analyze my BTC holdings"
**Arguments:**
- `asset: string`
- `year?: number` - Optional year filter for transactions

**Response (outcome-oriented, aggregated):**
```typescript
{
  success: true,
  analysis: {
    asset: "BTC",
    total_quantity: "0.19121",
    total_cost_basis: "87500.00",
    current_value: "13437.72",
    unrealized_pnl: "25937.72",
    pnl_percent: "296.43",
    accounts: [
      { name: "Binance", quantity: "0.09121" },
      { name: "Ledger", quantity: "0.10000" }
    ],
    transactions_summary: {
      total_buys: 5,
      total_sells: 0,
      total_transfers: 2,
      avg_buy_price: "87500.00"
    }
  }
}
```

#### 8. `cryptofolio_export_transactions`
**Purpose:** Export transactions to CSV for tax/analysis
**Use When:** User says "Export my 2024 transactions"
**Arguments:**
- `year?: number`
- `account?: string`
- `asset?: string`

**Response:**
```typescript
{
  success: true,
  csv_path: "/Users/user/.config/cryptofolio/exports/transactions_2024.csv",
  row_count: 123,
  message: "Exported 123 transactions to CSV"
}
```

#### 9. `cryptofolio_list_accounts`
**Purpose:** List all configured accounts
**Use When:** User asks "What accounts do I have?"
**Arguments:** None

**Response:**
```typescript
{
  success: true,
  accounts: [
    {
      name: "Binance",
      type: "exchange",
      category: "trading",
      sync_enabled: true
    },
    {
      name: "Ledger Nano X",
      type: "hardware_wallet",
      category: "cold-storage",
      sync_enabled: false
    }
  ]
}
```

#### 10. `cryptofolio_sync_exchange`
**Purpose:** Sync holdings from exchange API (Binance)
**Use When:** User says "Sync my Binance account"
**Arguments:**
- `account?: string` - Optional account name, defaults to all sync-enabled

**Response:**
```typescript
{
  success: true,
  synced_accounts: [
    {
      account: "Binance",
      assets_synced: 3,
      holdings: [
        { asset: "BTC", quantity: "0.09121000" },
        { asset: "ETH", quantity: "2.4594" },
        { asset: "USDT", quantity: "500.00" }
      ]
    }
  ]
}
```

---

## Security Considerations

### API Key Access

**Risk:** MCP server needs to execute CLI commands that may access API keys

**Mitigation:**
- MCP server runs in user context (same permissions as user)
- Cryptofolio config file already has 0600 permissions
- MCP server does NOT expose config management tools
- API keys only accessed by CLI binary, not by Node.js code

**Tools excluded for security:**
- ❌ `cryptofolio_set_api_key` - Not exposed via MCP
- ❌ `cryptofolio_show_config` - Could leak secrets
- ❌ `cryptofolio_config_*` - All config tools excluded

### Command Injection

**Risk:** User input passed to shell commands could enable injection

**Mitigation:**
```typescript
// Bad: Vulnerable to injection
exec(`cryptofolio price ${userInput}`);

// Good: Use array args with execa
execa('cryptofolio', ['price', userInput]);
// execa properly escapes arguments
```

**Additional safeguards:**
- Validate all inputs with Zod schemas
- Whitelist allowed asset symbols (BTC, ETH, etc.)
- Validate account names against database
- No direct SQL query exposure

### Data Exposure

**Risk:** Portfolio data is sensitive financial information

**Mitigation:**
- MCP server runs locally (no network exposure)
- Data stays in Claude Desktop's local context
- User controls when tools are invoked
- No telemetry or logging of responses

### Permission Model

**Principle:** MCP tools should respect user intent

**Implementation:**
- **Read operations:** No confirmation needed (portfolio, prices, list)
- **Write operations:** Return preview, user must confirm in Claude Desktop
  - "I'm about to record: buy 0.1 BTC at $95,000. Confirm?"
- **Sync operations:** Confirmation required (API calls cost money)

---

## Implementation Roadmap

### Phase 1: MVP (4-6 hours)
**Goal:** Basic portfolio interaction working in Claude Desktop

**Tasks:**
1. Set up Node.js project with TypeScript
2. Install MCP SDK and dependencies
3. Implement 3 core tools:
   - `cryptofolio_get_portfolio`
   - `cryptofolio_get_prices`
   - `cryptofolio_record_transaction`
4. Test with Claude Desktop
5. Documentation and examples

**Deliverables:**
- Working MCP server
- Claude Desktop config
- README with setup instructions

### Phase 2: Full Tool Suite (8-12 hours)
**Goal:** Complete 10-tool implementation

**Tasks:**
1. Implement remaining 7 tools
2. Add pagination to `list_transactions`
3. Add error handling and helpful messages
4. Write comprehensive tool descriptions
5. Testing suite (unit tests for each tool)

**Deliverables:**
- Complete tool suite
- Test coverage >80%
- User guide with examples

### Phase 3: Polish & Distribution (4-6 hours)
**Goal:** Production-ready release

**Tasks:**
1. NPM package creation
2. Automatic Cryptofolio binary detection
3. Installation script for Claude Desktop config
4. Error logging and diagnostics
5. Performance optimization

**Deliverables:**
- NPM package published
- One-command installation
- Troubleshooting guide

**Total Estimated Time:** 16-24 hours (2-3 days of AI-assisted development)

---

## Appendix A: Alternative Considered - Hybrid Approach

**Concept:** Node.js MCP server for most tools, direct Rust integration for high-frequency tools

**Pros:**
- Best of both worlds
- Fast implementation for most features
- Performance optimization for critical paths

**Cons:**
- Increased complexity
- Two integration patterns to maintain
- Harder to reason about system

**Verdict:** Not recommended for v1. Revisit if performance becomes issue.

---

## Appendix B: MCP Protocol Stability

**Current Status (Feb 2026):**
- MCP released by Anthropic in late 2024
- Protocol relatively stable but evolving
- Breaking changes possible in 1.x releases

**Risk Mitigation:**
- Use semantic versioning for MCP server
- Pin MCP SDK to specific version
- Monitor Anthropic's MCP changelog
- Isolate MCP protocol handling in adapter layer

**Long-term Strategy:**
- Once MCP reaches 2.0 stability, consider Rust implementation
- For now, Node.js provides flexibility to adapt to protocol changes

---

## Appendix C: Success Metrics

**How to measure MCP integration success:**

1. **User Adoption**
   - Downloads of cryptofolio-mcp NPM package
   - GitHub stars on MCP server repo
   - Claude Desktop users configuring integration

2. **Usage Patterns**
   - Most frequently called tools
   - Average tools per conversation
   - Error rates per tool

3. **User Satisfaction**
   - GitHub issues/feedback
   - User testimonials
   - Feature requests

4. **Performance**
   - Average response time per tool
   - 95th percentile latency
   - Error rate <5%

**Target (3 months post-release):**
- 100+ active users
- <100ms average response time
- >95% success rate
- Positive user feedback

---

## References

1. [MCP Best Practices - Philipp Schmid](https://www.philschmid.de/mcp-best-practices)
2. [Model Context Protocol Specification](https://modelcontextprotocol.io/)
3. [@modelcontextprotocol/sdk Documentation](https://github.com/modelcontextprotocol/sdk)
4. [Cryptofolio Architecture](./ARCHITECTURE.md)
5. [Cryptofolio v0.2.0 Documentation](../README.md)

---

**Document Status:** Ready for Review
**Next Steps:** Get stakeholder approval, proceed to Phase 1 implementation
**Contact:** See [CONTRIBUTING.md](../CONTRIBUTING.md)
