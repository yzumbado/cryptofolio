---
name: cryptofolio-conventions
description: The technical rules of this codebase — error handling, decimals, schema, importer, CLI, MCP, and security conventions condensed from the original CLAUDE.md. Load before changing Rust or TypeScript code.
whenToUse: before editing src/, mcp/src/, or tests; when writing new commands, tools, or DB code
---

# Cryptofolio conventions

## Rust

- **Errors:** all errors flow through `CryptofolioError` (src/error.rs). Never
  ad-hoc error strings. `.unwrap()`/`.expect()` only in tests — CI gates the
  production count (baseline 8, all provably infallible).
- **Money:** store as TEXT via `Decimal::to_string()`; never REAL/f64. Parse with
  `Decimal::from_str` (propagate errors). Format only with `cli/output.rs`
  helpers. JSON output serializes decimals as strings — intentional, don't change.
- **Timestamps:** `chrono::Utc` only; RFC 3339 strings. A failed timestamp parse
  is an error (`DateParse`), never a silent `Utc::now()` fallback.
- **Enums:** every domain enum implements `as_str()` / `from_str()` / `display_name()`.
  `TransactionType` variants must match the schema CHECK constraint exactly —
  the compiler catches non-exhaustive matches.
- **DB:** schema lives in `src/db/schema.rs` (`CREATE TABLE IF NOT EXISTS`, no
  migrations — edit + reset dev DB). `transactions` is append-only by DB trigger;
  correct via `tx_type='correction'`. Accounts archive, never hard-delete.
  Dedup via UNIQUE constraints (`tx_hash`, `external_id`) — catch the sqlx error,
  never pre-check existence. Repositories borrow `&SqlitePool`, async, `Result`.
- **Importer (Binance CSV):** fail closed — malformed/unknown rows go to
  `ParseReport.skipped` with line + reason, never silently dropped or defaulted.
  Capture provenance (price, network, tx_hash); keep external IDs identity-stable.

## CLI

- Every subcommand: `--json` (pretty serde), `--quiet`, `after_help` with an
  example, validation before DB writes, human output via `cli/output.rs` only.
- Errors to stderr + exit 1; dry-run makes no writes. Flags kebab-case.

## MCP server (mcp/)

- Tools named `cryptofolio_<verb>_<resource>`; one file per group in `src/tools/`,
  registered in `src/index.ts`. Always `runCli()` (adds `--json --quiet`); wrap
  errors with `handleCliError`. Envelope only via `formatters/response.ts`
  (`buildSuccess`/`buildError`). Zod-validate inputs before calling the CLI.
- All monetary values are strings in schemas. `CRYPTOFOLIO_BIN` selects the binary.
- Secrets never pass through TS — the Rust CLI + macOS Keychain own them.

## Blockchain

- `BlockchainClient` trait is watch-only: no keys, no signing, no spending.
- Convert chain units to standard decimals before storing (BTC 8, ETH 18,
  ADA 6, SOL 9). Validate addresses at the per-chain gate before insert.

## Security

- API keys/xpubs live in macOS Keychain, never config.toml, DB, logs, or notes.
- Never print a secret or a raw signed API response.
- Watch-only guarantee: reject WIF/seed/raw keys at input (`security.rs`).

## Git

- Branch → PR → CI green → squash-merge → delete branch; trivial docs direct to
  master. Run the gates before committing. Never push unless asked. One logical
  change per PR; update docs/BACKLOG.md in the same PR.

## Skills (.dsh/skills/)

- Frontmatter is strict YAML — avoid `key: value`-looking `: ` sequences inside
  `description`; a parse failure silently drops the skill from the DSH catalog.
  After creating/editing a skill, verify it appears in the catalog (mechanism check).
