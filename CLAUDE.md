# Cryptofolio — Claude Code Guide

## Quick start

```
/portfolio
```

Loads the **Portfolio Agent** — natural language interface to wallets, P&L, transactions, and sync. Requires the MCP server running (see `mcp/README.md`).

## Project map

```
src/                    Rust CLI binary (cryptofolio)
├── main.rs             Entry point, CLI dispatch
├── error.rs            Single CryptofolioError enum — all errors live here
├── core/               Domain models (Account, Transaction, Holdings, P&L)
├── db/                 SQLite repositories + schema
│   ├── schema.rs       Single source of truth for all tables
│   └── SPEC.md         Database design decisions
├── blockchain/         On-chain wallet sync (Bitcoin, Ethereum, Cardano, Solana)
│   ├── trait_def.rs    BlockchainClient trait — all clients implement this
│   ├── types.rs        Shared types (Chain enum, WalletTransaction, etc.)
│   └── {chain}/        Per-chain: address.rs, client.rs, mod.rs
├── exchange/           Exchange integrations (Binance)
├── cli/                Command handlers + output formatting
│   ├── output.rs       All display/format functions — use these, don't invent new ones
│   └── commands/       One file per subcommand group
├── config/             Config loading and macOS Keychain access
└── shell/              Interactive shell

mcp/src/               TypeScript MCP server
├── index.ts            Tool registration (4 phases)
├── cli.ts              CLI subprocess wrapper — single choke point
├── types.ts            Shared TS types
├── tools/              One file per tool group (accounts, wallets, transactions…)
└── formatters/
    └── response.ts     buildSuccess / buildError / toContent / handleCliError
```

## Running tests

```bash
# Unit + repository tests (fast, in-memory DB)
cargo test --lib

# BDD acceptance tests
cargo test --test bdd

# Integration tests (real APIs, needs keys)
CRYPTOFOLIO_INTEGRATION_TESTS=1 \
  ETHERSCAN_API_KEY=... \
  BLOCKFROST_API_KEY=... \
  SOLANA_RPC_URL=... \
  cargo test --test blockchain_clients
```

---

## Rust conventions

### Error handling

All errors flow through `CryptofolioError` in `src/error.rs`. Never create ad-hoc error strings — add a variant if one doesn't fit.

```rust
// Good
return Err(CryptofolioError::AccountNotFound(name.to_string()));

// Good — use ? for propagation in library code
let account = repo.get(&id).await?;

// Bad — only acceptable in tests
let account = repo.get(&id).await.unwrap();
```

**Rule:** `.unwrap()` is acceptable only in test code. In `src/` library and CLI code, always use `?` or return a descriptive error variant.

### Decimal and monetary values

**Storage:** All monetary amounts are stored as `TEXT` in SQLite using `rust_decimal::Decimal::to_string()`. Never use `REAL`, `FLOAT`, or `f64` for money.

**Parsing:** Use `Decimal::from_str()`, not `.parse::<f64>()` — float parse truncates precision.

**Formatting — always use `cli/output.rs` functions:**

| Function | Use for |
|---|---|
| `format_usd(value)` | USD amounts — always 2 decimal places with `$` |
| `format_quantity(value)` | Crypto amounts — adaptive: `≥1000` → 2dp, `≥1` → 4dp, `<1` → 8dp |
| `format_decimal(value, decimals)` | Custom precision |
| `format_pnl(value)` | P&L with color (green positive, red negative) |

Never format decimals directly with `format!("{:.2}", value)` in command handlers — call the output functions.

**JSON output:** Decimals serialize as strings (`"quantity": "1.5"`, not `"quantity": 1.5`). This is intentional — preserves precision. Do not change it.

### Timestamps

- Always use `chrono::Utc` — never `Local`.
- Store as RFC 3339 string: `timestamp.to_rfc3339()`
- Parse: `DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&Utc))`
- If a parse fails in a repository, return `Err(CryptofolioError::DateParse(...))` — do not silently fall back to `Utc::now()`.

### Enums

Every domain enum follows this exact pattern — `as_str()`, `from_str()`, `display_name()`:

```rust
impl MyEnum {
    pub fn as_str(&self) -> &'static str { ... }      // DB/wire value
    pub fn from_str(s: &str) -> Option<Self> { ... }  // accepts aliases
    pub fn display_name(&self) -> &'static str { ... } // user-facing label
}
```

See `TransactionType` in `src/core/transaction.rs` as the canonical example. Every new enum type that touches the DB must follow this pattern.

### `#![allow(dead_code)]`

Several modules have this at the top. It is intentional — these modules export types consumed by the MCP server and tests, not by the binary directly. Do not remove it.

### Repository pattern

Every DB table has a corresponding repository in `src/db/`:

```rust
pub struct FooRepository<'a> {
    pool: &'a SqlitePool,  // always borrowed, never owned
}

impl<'a> FooRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self { Self { pool } }

    pub async fn list(&self) -> Result<Vec<Foo>> { ... }
    pub async fn get(&self, id: &str) -> Result<Option<Foo>> { ... }
    pub async fn insert(&self, item: &Foo) -> Result<i64> { ... }
}
```

All methods are `async` and return `Result<T>`. Borrow the pool — do not clone or own it.

### Transaction types

`TransactionType` enum in `src/core/transaction.rs` must always match the CHECK constraint in `src/db/schema.rs`. Current types:

| Rust variant | DB string | Aliases in `from_str` |
|---|---|---|
| `Buy` | `buy` | — |
| `Sell` | `sell` | — |
| `TransferIn` | `transfer_in` | `deposit` |
| `TransferOut` | `transfer_out` | `withdrawal`, `send` |
| `TransferInternal` | `transfer_internal` | `transfer` |
| `Swap` | `swap` | `trade` |
| `Stake` | `stake` | — |
| `Unstake` | `unstake` | — |
| `Earn` | `earn` | `reward`, `interest` |
| `Receive` | `receive` | `incoming` |
| `Fee` | `fee` | — |
| `Airdrop` | `airdrop` | — |
| `Correction` | `correction` | — |

If you add a new type, update both the enum AND the schema CHECK constraint AND all match arms (the compiler will catch non-exhaustive matches).

---

## Database conventions

### Schema

All tables are defined in `src/db/schema.rs`. The function `schema::create(&pool)` is called once on startup. Every statement is `CREATE TABLE IF NOT EXISTS` — safe to re-run.

**To reset the schema during development:** `rm ~/.config/cryptofolio/database.sqlite`

Never write `ALTER TABLE` or numbered migrations. Just edit `schema.rs` and reset the DB.

### Deduplication

Two UNIQUE constraints on `transactions` prevent duplicate imports — enforced by SQLite, not application logic:

- `tx_hash TEXT UNIQUE` — canonical dedup key for on-chain events (same hash regardless of which API returned it)
- `external_id TEXT UNIQUE` — dedup key for exchange events (Binance order ID, etc.)

A duplicate insert hits the constraint and returns a `sqlx::Error`. Catch it and skip — do not check for existence before inserting.

### Immutable ledger

**Never UPDATE or DELETE rows in `transactions`.** If something was recorded wrongly, insert a new row with `tx_type = 'correction'`. This preserves the audit trail and makes cost basis replay deterministic.

### Trust levels

Every transaction row has a `trust_level` column:

| Value | Meaning |
|---|---|
| `chain_verified` | tx_hash confirmed on-chain |
| `exchange_verified` | source is exchange API with signed response |
| `manual` | entered by user |
| `unverified` | default — not yet confirmed |

Agents must check `trust_level` and `reconciliation_log.status` before acting on balances.

### Test setup

```rust
#[tokio::test]
async fn my_test() -> Result<()> {
    let pool = cryptofolio::db::init_memory_pool().await?;
    // pool has full schema, no migrations needed
    Ok(())
}
```

For integration tests that share setup: use `tests/common::setup_test_db()`.

---

## CLI conventions

### Adding a new command

Every subcommand must have:

1. `--json` flag — outputs `serde_json::to_string_pretty(&result)` to stdout
2. `--quiet` flag — suppresses progress messages
3. `after_help` with at least one usage example
4. Input validation before any DB operation
5. Human-readable output using `cli/output.rs` formatters (not raw `println!`)

Global flags already wired on every command: `--json`, `--quiet`, `--verbose`, `--no-color`, `--testnet`, `--yes`, `--dry-run`.

### Output rules

- Human output: colored tables via `colored` crate, using `cli/output.rs` helpers
- JSON output (`--json`): always `serde_json::to_string_pretty()` — never manual string building
- Errors: `eprintln!` to stderr, `process::exit(1)` — do not print errors to stdout
- Dry-run: describe what would happen, make no DB writes

### CLI argument naming

- Flags: `--kebab-case` (e.g. `--by-account`, `--from-date`)
- Short flags: single letter (`-a`, `-v`, `-q`) only for the most common options
- No underscores in flag names (`--by_account` is wrong)

---

## MCP tool conventions

### Naming

`cryptofolio_<verb>_<resource>` — verb first, resource second.

```
cryptofolio_list_accounts      ✅
cryptofolio_manage_account     ✅
cryptofolio_sync_wallet        ✅
cryptofolio_accounts_list      ❌  (resource before verb)
```

### Tool structure

```typescript
export function registerMyTool(server: McpServer): void {
  server.tool(
    "cryptofolio_verb_resource",
    "One sentence describing what this tool does and when to call it.",
    {
      param: z.string().describe("What this param means"),
    },
    async ({ param }) => {
      try {
        const raw = await runCli(["subcommand", param]);
        return toContent(buildSuccess(raw, "Human-readable summary."));
      } catch (err) {
        return toContent(handleCliError(err, "cryptofolio_verb_resource"));
      }
    }
  );
}
```

**Rules:**
- Always use `runCli()` — it appends `--json --quiet` automatically
- Use `runCliRaw()` only for commands that don't implement `--json` yet
- Always wrap errors with `handleCliError(err, toolName)` — never throw
- Validate inputs with Zod before calling the CLI
- Register the tool in `mcp/src/index.ts`

### Response format

All tools return the same JSON envelope:

```json
{ "success": true,  "data": { ... }, "message": "Human summary" }
{ "success": false, "error": "...",  "code": "...", "hint": "..." }
```

Use `buildSuccess`, `buildError`, `toContent` from `mcp/src/formatters/response.ts` — never construct this object manually.

---

## Blockchain client conventions

### Adding a new chain

1. Create `src/blockchain/{chain}/` with `address.rs`, `client.rs`, `mod.rs`
2. Implement `BlockchainClient` trait from `src/blockchain/trait_def.rs`
3. Add the `Chain` variant to `src/blockchain/types.rs`
4. Register the client in `src/blockchain/provider.rs`
5. Add address validation to `cli/commands/wallet.rs`

### BlockchainClient required methods

```rust
fn provider_name(&self) -> &str;
async fn health_check(&self) -> Result<HealthStatus>;
async fn get_address_info(&self, address: &str) -> Result<AddressSummary>;
async fn get_transactions(&self, address: &str) -> Result<Vec<WalletTransaction>>;
```

### Native asset decimals

```
Bitcoin:  8  decimals  (1 BTC  = 100_000_000 satoshi)
Ethereum: 18 decimals  (1 ETH  = 10^18 wei)
Cardano:  6  decimals  (1 ADA  = 1_000_000 lovelace)
Solana:   9  decimals  (1 SOL  = 1_000_000_000 lamports)
```

Always convert to the standard unit before storing. Use `Decimal::from(raw) / Decimal::from(10u64.pow(decimals))`.

### Watch-only rule

The `BlockchainClient` trait is **watch-only**. No private keys, no signing, no spending. Inputs are public addresses and xpubs only.

---

## Security rules

### API keys

API keys live in the macOS Keychain via `src/config/keychain.rs`. Never store a raw key in:
- `config.toml`
- SQLite (use `blockchain_nodes.api_key_ref` to reference a Keychain entry by name, not the key itself)
- Any log or error message
- Any `notes` field in a transaction

### Input validation

Validate all user-supplied addresses before inserting. Each chain's `validate_address()` function in `blockchain/{chain}/address.rs` is the gate. The security layer in `blockchain/security.rs` also rejects WIF private keys and seed phrases.

### No private keys in this codebase

This project is a portfolio tracker, not a wallet. No signing, no broadcasting, no key derivation beyond reading xpub for address generation.

---

## Common gotchas

1. **Decimal from string:** `Decimal::from_str("1.5").unwrap()` in tests is fine. In production, propagate the error: `Decimal::from_str(s)?`.

2. **Duplicate transactions:** Don't check for existence before inserting. Let the DB UNIQUE constraint reject duplicates and catch the `sqlx::Error`.

3. **JSON decimals are strings:** `{ "quantity": "1.5" }` not `{ "quantity": 1.5 }`. Intentional. Do not change.

4. **Timestamp fallbacks:** If a timestamp from the DB fails to parse, return an error — don't silently fall back to `Utc::now()`. Silent fallbacks mask data corruption.

5. **Account IDs are text:** They are human-readable slugs or UUIDs, not auto-increment integers. Never assume numeric.

6. **Async everywhere:** All DB operations are `async fn`. Don't accidentally call them in a sync context without `.await`.

7. **Holdings can drift:** The `holdings` table is maintained by the sync engine, not computed from `transactions`. If you suspect drift, check `reconciliation_log` to see whether computed balance matches on-chain balance.

8. **Trust before acting:** An agent querying balances must check `reconciliation_log.status`. If any relevant wallet is `unreconciled`, the agent must report uncertainty rather than act on potentially wrong data.

9. **No migrations:** The schema is in `src/db/schema.rs`. To change the schema, edit it and delete the database file. `ALTER TABLE` and numbered migrations are not used before v1.0.

10. **Match exhaustiveness:** Adding a new `TransactionType` variant will cause compile errors in match statements across the codebase. Fix all of them — the compiler will point you to each one.
