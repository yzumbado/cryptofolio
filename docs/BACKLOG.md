# Cryptofolio — Development Backlog (public)

**Purpose:** the living record of **app/engineering** work — what's done and what's
queued, so development continues across sessions and tools (DSH, Claude Code,
KiroCrew). Committed to git — this file is public, so it contains no personal
figures, addresses, keys, or investment plans. That material lives in the
**private notebook** `.portfolio_private.md` (gitignored): §1 financial state,
§2 portfolio-management backlog, §3 privacy/clean-up log. The leak gate
(`scripts/check_leaks.sh` + CI `leak-check.yml`) enforces the boundary.

**Last updated:** 2026-10-05

**Tier legend:** 🤖 = delegable to a subagent (cheap model) · 🧠 = main agent ·
👤 = needs user design/approval. Estimate: S < 15 min, M < 1 h, L multi-session.
Tasks block via `Depends on` notes when relevant; **decompose only on activation —
never design ahead.**

---

## ✅ Done (recent → older)

### Session 2026-10-05 (DSH, fiat-tracing close + Aave health)
- **Lulubit fiat on/off-ramp importer** (#51): `import-lulubit` CSV (`receive`/`swap`/
  `dispose`), external-id idempotent; records the USD→USDT on-ramp + egress so Lulubit
  USD nets to $0. Extended with `transfer` (pass-2a USDT→Binance) and external `send`
  op (#52).
- **ERC-20 token-transfer history in Ethereum sync** (#53): `tokentx` rows were only
  aggregated into balances; now mapped to `WalletTransaction` (direction, decimal
  scaling, scam skip, unique `external_id = hash-contract`). Aave aTokens/debt/GHO
  now reconcile with on-chain history.
- **Aave health factor**: `aave health` CLI + `cryptofolio_aave_health` MCP tool
  (24th tool) — on-chain `getUserAccountData` via a generic client `eth_call`;
  `src/core/aave.rs` decodes collateral/debt/available-borrows (8dp), LTV +
  liquidation-threshold (percent), health factor (1e18). Pure decode is unit-tested.
- **Debt-as-liability**: confirmed already implemented (KiroCrew `core/defi.rs`
  `DefiKind::Debt` → negative value); no code needed — verified live via `get_portfolio`
  (`variableDebtEthGHO` shows `defi_kind:"debt"`, negative current_value).

### Session 2026-10-04 (DSH, trust-fix batch 3)
- **T11** dated on-chain reward income: Solana token-account history → `persist_rewards`
  (`receive` rows, `external_id` dedup, `chain_verified`), `select_revenue` with explicit
  `RevenueBasis` (dated stream wins, current-FMV fallback); current-price approximation
  surfaced in `--json` (`revenue_basis`) + human output. Watch-only, +wiremock tests.
- **T20** MCP display formatters (`formatUsd/Pct/Quantity/SignedUsd`) now exact via
  decimal.ts (BigInt); differential-verified over 6,621 values.
- **T21** all 23 tools migrated to `registerTool` with a real permissive `outputSchema`;
  `toContent` emits `structuredContent`; InMemoryTransport integration test proves
  `validateToolOutput` runs (incl. negative controls). 159 MCP tests.
- **T23** dropped unused `security-framework`/`core-foundation` direct deps.
- Leak-guard false positive fixed (import-binance test used a machine-style absolute
  path → `/tmp`); gate ordering corrected: stage, then leak-check, then commit.

### Session 2026-10-04 (DSH, trust-fix batch 2 + P7 docs)
- **T6** exact decimal-string math in MCP (BigInt, half-up like rust_decimal): track_conversion
  multiply, unrealized-total summation; `MISSING_PARAM` code unified; runCli/runCliRaw
  hardening incl. dead execa ENOENT path (exit 127 + binary hint). +37 tests.
- **T7** all 23 tool descriptions tightened with envelope contract; outputSchema deferred
  (SDK `server.tool()` cannot carry it — hard-throw on missing structuredContent, documented
  per-tool instead); evals TOOL_NAMES + README counts → 23.
- **T8** MCP exposes mining-pnl, sync-history, holdings list, pnl backfill (--yes), import-binance
  → 23 tools total. +16 tests.
- **T13** daily-refresh cron artifacts: scripts/portfolio-refresh.sh + launchd plist (in-repo,
  NOT installed).
- **T14** already present at HEAD — consolidated cost-basis/P&L lines into one tested headline.
- **T15** Binance "Order No" column-offset fix RESCUED from stash@{0} (sole copy), ported + 4 tests.
- **T16** `transactions.created_at` default → RFC 3339 (fresh DBs); tolerant parser kept for
  legacy live-ledger rows.
- **T18** MCP list_transactions `has_more` honest on both paths (needed+1 probe).
- **T19** (new, found by T7) `cryptofolio_analyze_asset` was effectively broken — `pnl by-asset`
  now has a `--json` branch (AssetPnlOutput) + serde test.
- **P7** docs refresh: ARCHITECTURE, DATA_MODEL, CHANGELOG Unreleased (#31–#46), ROADMAP,
  SECURITY, Dockerfile rust 1.93.0; dsh-mcp bundle tool count → 23.

### Session 2026-10-04 (DSH, trust-fix batch 1)
- **T2** timestamp parse: silent `Utc::now()` fallback → `DateParse` error; `created_at`
  accepts SQLite `CURRENT_TIMESTAMP` format via two-format parser (malformed still errors).
- **T3** blockchain sync: removed EXISTS pre-check (dedup now via `UNIQUE(external_id)` +
  `is_unique_violation()`); `.ok()` swallows on watermark/balances/tx writes now propagate
  as `SyncError`; `write_audit_log` stays fire-and-forget (documented). +4 DB tests.
- **T4** `status --json` via global flag + `print_json`; `SystemStatus`/`ProviderStatus`
  now `Serialize`; both dispatch sites (main + shell) threaded.
- **T5** MCP list_transactions: offset applied once to the asset-filtered set via
  window-growth fetch loop (cap 5000); +3 regression tests.
- **T9** `@anthropic-ai/sdk` → devDependencies (evals-only); lockfile updated.

### Session 2026-10-03 (DSH)
- **Working-mode layer (trimmed inferenceFlow adoption)**: AGENTS.md + STATE.md,
  skills `working-discipline` / `plan-before-build` / `co-author-review` /
  `cryptofolio-conventions`, CLAUDE.md → pointer. Mechanisms verified live
  (AGENTS.md auto-loaded, skills discovered by the DSH catalog).
- **Leak remediation (Phase 0)** — **merged as PR #45** ✅: identified public-repo leak —
  personal figures in `docs/BACKLOG.md`, `docs/MINING_ASSET_ACCOUNTING.md`, `docs/design/*`.
  Redacted public copies; real values moved to `.portfolio_private.md`. Added `.gitignore`
  hardening (TDR/, prototypes/, .kiro/, .portfolio_*.json/md, .sqlx/, *.docx) and a
  deterministic CI gate (`scripts/check_leaks.sh` + `leak-check.yml`, blocking).
- **Repo hygiene** — merged in #45 ✅: deleted stale scripts (sign/setup_sqlx/test_keychain),
  Feb-era docs (docs/mcp, CONVERSATIONAL_CLI, SECURE_SECRETS), validation/, coverage dump,
  .sqlx cache, legacy keychain FFI modules; SECURITY.md rewritten; currencies.rs `query!` →
  runtime queries (deterministic build, no DATABASE_URL/.sqlx); local branches/worktrees cleaned.
- **DSH MCP bundle (Phase 1)** — merged in #45 ✅: `integrations/dsh-mcp/` connects the 18
  `cryptofolio_*` tools to the DSH profile via `@deepseek-ai/dsh-mcp-client` (stdio); verified
  live returning real ledger data. Rebuilt `mcp/dist/` (was stale vs `mcp/src/`).

### KiroCrew sessions (pre-DSH)
- Mining accounting model (`docs/MINING_ASSET_ACCOUNTING.md`, `cryptofolio mining-pnl`).
- Aave aTokens displayed as underlying (wstETH/rETH/USDT). Staked SOL in balance.
- Cost basis set for all reconstructed positions (details in private notes).
- Binance outflow audit — all withdrawn coins reconciled; chain-verified notes in ledger.
- Portfolio reconstruction complete; ledger replayed via `pnl backfill` + reconciliation.

### Earlier
- Engine fixes #34; CI fixes #35/#36/#37; trade/convert pairing #38/#40.

---

## 📋 Open queue (priority order)

### P1 — Repo hygiene (deep review actions) — ✅ DONE 2026-10-03

| ID | Action | Status |
|---|---|---|
| D1 | Removed coverage_output.txt (tracked tarpaulin dump) | ✅ untracked+deleted |
| D2 | Cryptofolio_Feature_Proposal.docx untracked (file kept locally, now gitignored); temp file deleted | ✅ |
| D3 | Removed .sqlx/ + setup_sqlx.sh | ✅ |
| D4 | Deleted sign.sh | ✅ |
| D5 | Deleted test_keychain.sh | ✅ |
| D6 | Deleted docs/mcp/* (Feb design artifacts) | ✅ |
| D7 | Deleted docs/CONVERSATIONAL_CLI.md | ✅ |
| D8 | SECURITY.md rewritten to current reality; docs/SECURE_SECRETS.md deleted | ✅ |
| D9 | Deleted validation/ | ✅ |
| D10 | Deleted legacy keychain FFI modules (cargo gates green) | ✅ |
| D11 | Local cleanup: .claude/worktrees/* removed, 3 merged claude/* branches deleted | ✅ |
| D12 | History scrub: **decided — skip for now** (accept history, move forward redacted) | ⏸️ decided |
| D13 | Branch `docs/portfolio-roadmap` carries 2 unmerged docs (CAPABILITY_AND_GAPS.md, REBUILD_PROCEDURE.md, 778 lines). Review → leak-scan → decide merge/redact/delete | 👤 | S |

### P2 — DSH alignment

| ID | Action | Tier | Size |
|---|---|---|---|
| S1 | ✅ done — AGENTS.md + STATE.md + skills landed; CLAUDE.md → pointer. Stale-fact doc refresh remains (P7) | ✅ | — |
| S2 | `.dsh/skills/portfolio/SKILL.md` — port ledger-keeper skill; change "no investment advice" refusal to a hand-off to the advisor skill | 🧠 | M |
| S3 | `.dsh/skills/investment-advisor/SKILL.md` — the decision assistant: thesis building, plan design, validation/invalidation scenarios, honest plan tracking. **Design session with user first** (see P6) | 👤→🧠 | L |
| S4 | ✅ done — as `working-discipline` (rituals/guardrails) + `cryptofolio-conventions` (technical rules) skills | ✅ | — |
| S5 | Dedicated DSH presets: "Ledger", "Advisor" (read-only cryptofolio tools only — structural guarantee), "Maintainer" | 🧠 | M |
| S6 | README: remove Claude badge/Desktop sections + Ollama/AI-mode claims; add DSH section | 🤖 | S |
| S7 | Revisit session journal after 3 sessions of real use (inferenceFlow pattern; deliberately deferred) | 👤 | S |

### P3 — Decision engine (user's core goal)

| ID | Action | Tier | Size |
|---|---|---|---|
| A1 | `price_targets` table + `cryptofolio scenario` CLI: project net worth, P&L-at-target, yield, Aave health factor under bear/base/bull (productize the TDR prototype). *Current health-factor tracking is done (`aave health`); the scenario projection remains.* | 🧠 | L |
| A2 | MCP tool `cryptofolio_project_scenarios` + output schema | 🤖 | S |
| A3 | Advisor skill consumes live scenarios; per-asset thesis → plan → validation/invalidation triggers → honest review loop | 👤→🧠 | L |
| A4 | Watchlist alerts + strategy drift reports (later) | 🧠 | M |

### P4 — DSH web UI panel

| ID | Action | Tier | Size |
|---|---|---|---|
| W1 | UI plugin: portfolio dashboard panel in DSH Web (theme tokens; evolve prototypes/portfolio-dashboard.html) | 🧠 | L |
| W2 | Panel reads data via CLI/DB read path — define contract first | 👤→🧠 | M |

### P5 — Trust foundation & MCP fixes

| ID | Action | Tier | Size |
|---|---|---|---|
| T1 | Wire reconciliation_log (schema-only today; no writer) — "trust before acting" needs data | 🧠 | M |
| T2 | Fix timestamp parse silent fallback to Utc::now() in transactions repo | ✅ done 2026-10-04 | S |
| T3 | Fix blockchain sync: EXISTS-before-INSERT + swallowed errors (.ok()) | ✅ done 2026-10-04 | S |
| T4 | `status --json` support (only command without it) | ✅ done 2026-10-04 | S |
| T5 | MCP: fix list_transactions double-applied offset on asset filter | ✅ done 2026-10-04 | S |
| T6 | MCP: float math → Decimal in track_conversion / unrealized totals; MISSING_PARAM(S) consistency; runCli {message} fallback hardening | ✅ done 2026-10-04 | M |
| T7 | MCP: add output schemas + tighten tool descriptions (agent-experience standards); rewrite docs/mcp API reference against reality | ✅ done 2026-10-04 (outputSchema deferred — SDK `server.tool()` can't carry it; envelope documented per-tool instead, see T21) | M |
| T8 | MCP: expose missing CLI surface — mining-pnl, sync-history, import-binance, holdings, pnl backfill | ✅ done 2026-10-04 (23 tools) | M |
| T9 | Move @anthropic-ai/sdk to devDependencies (evals-only) | ✅ done 2026-10-04 | S |
| T10 | Binance client: stop eprintln-ing raw signed responses; keychain backend: stop passing secrets as argv | 🧠 | S |
| T11 | Wire dated on-chain reward income (Solana token-account daily pulls) into mining-pnl | ✅ done 2026-10-04 (dated reward rows + `select_revenue` + explicit current-price approximation; see T24) | M |
| T12 | Filter scam airdrop tokens from holdings display (token list lives in private notebook §2) | 🤖 | S |
| T13 | Daily portfolio-refresh cron (sync wallets + prices) | ✅ done 2026-10-04 (script + plist in scripts/; NOT installed — install is a user step) | S |
| T14 | Show cost basis / unrealized P&L headline in portfolio summary top-line | ✅ done 2026-10-04 (was already at HEAD; consolidated to one line + tests) | S |
| T15 | Rescue stashed Binance spot-trade fix: "Order No" column-offset detection (stash@{1}) — verify vs real exports, add tests | ✅ done 2026-10-04 (found in stash@{0}, ported + 4 tests) | S |
| T16 | Schema: `created_at` default → RFC 3339 (`strftime('%Y-%m-%dT%H:%M:%SZ','now')`), then strict parse in transactions repo and delete `parse_db_timestamp` two-format parser. Needs dev-DB reset (no migrations) | ✅ done 2026-10-04 (default changed; parser KEPT — live ledger still has legacy rows and there is no migration mechanism, see T22) | S |
| T17 | Blockchain sync: wrap per-address persist + watermark in one DB transaction (per-address atomicity; today a mid-batch failure leaves earlier rows committed) | 🧠 | M |
| T18 | MCP list_transactions: `has_more` is always false on the unfiltered path (fetch window == offset+limit); fetch `needed+1` so the "call again" hint actually fires | ✅ done 2026-10-04 | S |
| T19 | `cryptofolio_analyze_asset` was returning a "no structured output" marker (pnl by-asset had no `--json` branch) | ✅ fixed 2026-10-04 (AssetPnlOutput JSON branch + serde test) | S |
| T20 | MCP display formatters (`formatters/numbers.ts`) still use parseFloat for USD/pct/quantity — display-only, but the header claim "never floats" is now wrong | ✅ done 2026-10-04 (exact via decimal.ts; differential-verified over 6621 values) | S |
| T21 | Real MCP outputSchema: migrate tools to `registerTool` + emit `structuredContent` in toContent, permissive object schema, in-process server test (unit tests bypass validation — see T7 note) | ✅ done 2026-10-04 (all 23 tools + InMemoryTransport integration test with negative controls) | M |
| T22 | Live-DB `created_at` default is still legacy (schema is IF NOT EXISTS, no migrations). Decide: table-rebuild migration vs accept both formats forever (parser already accepts both) | ✅ decided 2026-10-04 — NO migration; RFC 3339 default stays for new rows; tolerant read parser kept for legacy rows (read-compat only) | S |
| T23 | Drop unused `security-framework` / `core-foundation` macOS deps from Cargo.toml (keychain now shells out to the `security` CLI) | ✅ done 2026-10-04 (crates remain transitively via reqwest's TLS stack) | S |
| T24 | Mining rewards: every incoming SPL transfer to a tracked token account is booked as a reward — a DEX purchase into that same ATA would be misclassified. Add a distributor allow-list; also cap/reduce the per-token-account rescan cost (>1000 sigs, N getTransaction calls when the owner has no new txs) | ✅ decided 2026-10-04 — no allow-list: GEOD and WINGS arrive at two dedicated miner wallets (names in private notebook §1) the owner never buys into; reward = incoming DePIN-token transfer to those wallets. Document assumption in code. Rescan-cost follow-up stays open | M |

### P6 — Advisor skill design session (with user)
Agenda: persona & scope (professional trader + long-term investor equivalent);
thesis format; plan fields (allocations, entries, exits, timeframes); validation /
invalidation triggers; honesty rules (no moving goalposts, log thesis outcomes);
which tools it may call (read-only ledger + web research + TDR/).
**Design outputs are personal → they land in `.portfolio_private.md` §2, never in tracked files.**

### P7 — Documentation refresh — ✅ DONE 2026-10-04
U3 ARCHITECTURE.md → v0.6 reality · U4 DATA_MODEL.md (tx_type enum, migrations refs) ·
U5 CHANGELOG Unreleased (PRs #31–44) · U6 ROADMAP.md (DSH + advisor direction) ·
U8 SECURITY.md (versions, contact) · U9 Dockerfile rust 1.93. All 🤖-friendly.

---

## 🔑 Key facts (don't re-derive)

- Live DB path: in `.portfolio_private.md` (machine-specific; NOT ~/.config — that copy is stale).
- FIFO cost basis; no tax layer. Ledger is append-only (correct via tx_type='correction').
- CI gates: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --lib`,
  integration tests, unwrap gate (baseline 8), mcp typecheck+lint+vitest, **leak-check**.
- MCP server: stdio only; `CRYPTOFOLIO_BIN` env picks the binary; rebuild with `npm run build` in mcp/.
- Never fabricate lots — only from authoritative data (Binance export or on-chain tx).
- `trust_level` ∈ {exchange_verified, chain_verified, manual, unverified}; `cost_basis_method` lowercase.
- Archive DB before edits (`_archive/database.pre-*`).
- Personal data (figures, wallet names, investment plans, decisions) → `.portfolio_private.md`, never committed docs (CI-enforced). This file holds engineering tasks only.
