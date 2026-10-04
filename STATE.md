# Cryptofolio — STATE (current truth, one page)

> Read before proposing work. If a fact changes, update this file in the same
> change. Public-safe only: machine-specific paths and financial facts live in
> `.portfolio_private.md` (never commit that file).

## Project

- Local-first crypto portfolio tracker: Rust CLI (`cryptofolio`) + TypeScript MCP
  server (18 `cryptofolio_*` tools, stdio) + DSH agent layer. Watch-only by design:
  no signing, no broadcasting; private keys rejected at input.
- Version 0.6.0 (Cargo.toml ↔ CHANGELOG). Public repo `yzumbado/cryptofolio`.
- Ledger: append-only SQLite (DB-trigger enforced); corrections only via
  `tx_type='correction'`. FIFO cost basis; no tax layer.

## Working mode

- DSH is the primary agent tool; Claude Code / Kiro sessions may still occur.
- AGENTS.md auto-loads; skills live in `.dsh/skills/` — **verify they appear in
  the session catalog**; if missing, stop and report (mechanism check).
- MCP: the `integrations/dsh-mcp/` bundle connects cryptofolio tools to DSH; its
  local patch file is gitignored (machine paths). Rebuild `mcp/dist/` after
  changing `mcp/src/`.
- Git: branch → PR → CI green → squash-merge → delete branch. Trivial docs may go
  direct to master. **Never push unless asked.**

## Gates (run before every commit)

- Rust: `cargo fmt --check` · `cargo clippy -- -D warnings` · `cargo test --lib`
- MCP (in `mcp/`): `npm run typecheck` · `npm run lint` · `npm test`
- Privacy: `bash scripts/check_leaks.sh` (CI job `leak-check.yml` is blocking)
- CI is the final word; never bypass.

## ⚠️ Things that will make you act wrongly

1. **Two databases exist.** The live ledger is NOT `~/.config/cryptofolio` — that
   copy is stale. The real path is in the private notebook. Never delete/reset a
   database file without checking which one you're touching.
2. **`reconciliation_log` has no writer yet** — never claim balances are
   "reconciled"; report uncertainty instead.
3. **`cost_basis_only: true`** is mandatory when recording buys on synced
   accounts — without it the holding double-counts.
4. **Binary staleness:** `~/.local/bin/cryptofolio` may lag; the MCP server uses
   the repo build via `CRYPTOFOLIO_BIN`. `cargo build --release` before trusting.
5. **Prices/balances are only as fresh as the last sync** — check `last_synced`
   before quoting numbers; never fabricate or interpolate.
6. **Everything in docs/ is public** — no personal figures, addresses, or keys
   (the leak gate fails the build otherwise).

## Current focus

Working-mode layer (AGENTS.md + skills) landing · next lanes: trust-fix batches,
then the advisor design session and the scenario engine. See docs/BACKLOG.md.
