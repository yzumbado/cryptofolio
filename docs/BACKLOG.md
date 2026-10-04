# Cryptofolio — Development Backlog (public)

**Purpose:** the living record of **app/engineering** work — what's done and what's
queued, so development continues across sessions and tools (DSH, Claude Code,
KiroCrew). Committed to git — this file is public, so it contains no personal
figures, addresses, keys, or investment plans. That material lives in the
**private notebook** `.portfolio_private.md` (gitignored): §1 financial state,
§2 portfolio-management backlog, §3 privacy/clean-up log. The leak gate
(`scripts/check_leaks.sh` + CI `leak-check.yml`) enforces the boundary.

**Last updated:** 2026-10-03

**Tier legend:** 🤖 = delegable to a subagent (cheap model) · 🧠 = main agent ·
👤 = needs user design/approval. Estimate: S < 15 min, M < 1 h, L multi-session.

---

## ✅ Done (recent → older)

### Session 2026-10-03 (DSH)
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
| S1 | AGENTS.md: migrate CLAUDE.md → AGENTS.md (DSH-native), prune Claude-specific bits, fix stale facts (DB path, trait method names, network_to_chain location) | 🧠 | M |
| S2 | `.dsh/skills/portfolio/SKILL.md` — port ledger-keeper skill; change "no investment advice" refusal to a hand-off to the advisor skill | 🧠 | M |
| S3 | `.dsh/skills/investment-advisor/SKILL.md` — the decision assistant: thesis building, plan design, validation/invalidation scenarios, honest plan tracking. **Design session with user first** (see P6) | 👤→🧠 | L |
| S4 | `.dsh/skills/cryptofolio-maintainer/SKILL.md` — encode CLAUDE.md conventions + ROAD_TO_A rituals for DSH sessions | 🧠 | S |
| S5 | Dedicated DSH presets: "Ledger", "Advisor" (read-only cryptofolio tools only — structural guarantee), "Maintainer" | 🧠 | M |
| S6 | README: remove Claude badge/Desktop sections + Ollama/AI-mode claims; add DSH section | 🤖 | S |

### P3 — Decision engine (user's core goal)

| ID | Action | Tier | Size |
|---|---|---|---|
| A1 | `price_targets` table + `cryptofolio scenario` CLI: project net worth, P&L-at-target, yield, Aave health factor under bear/base/bull (productize the TDR prototype) | 🧠 | L |
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
| T2 | Fix timestamp parse silent fallback to Utc::now() in transactions repo | 🤖 | S |
| T3 | Fix blockchain sync: EXISTS-before-INSERT + swallowed errors (.ok()) | 🤖 | S |
| T4 | `status --json` support (only command without it) | 🤖 | S |
| T5 | MCP: fix list_transactions double-applied offset on asset filter | 🤖 | S |
| T6 | MCP: float math → Decimal in track_conversion / unrealized totals; MISSING_PARAM(S) consistency; runCli {message} fallback hardening | 🤖 | M |
| T7 | MCP: add output schemas + tighten tool descriptions (agent-experience standards); rewrite docs/mcp API reference against reality | 🤖 | M |
| T8 | MCP: expose missing CLI surface — mining-pnl, sync-history, import-binance, holdings, pnl backfill | 🤖 | M |
| T9 | Move @anthropic-ai/sdk to devDependencies (evals-only) | 🤖 | S |
| T10 | Binance client: stop eprintln-ing raw signed responses; keychain backend: stop passing secrets as argv | 🧠 | S |
| T11 | Wire dated on-chain reward income (Solana token-account daily pulls) into mining-pnl | 🤖 | M |
| T12 | Filter scam airdrop tokens from holdings display (token list lives in private notebook §2) | 🤖 | S |
| T13 | Daily portfolio-refresh cron (sync wallets + prices) | 🤖 | S |
| T14 | Show cost basis / unrealized P&L headline in portfolio summary top-line | 🤖 | S |
| T15 | Rescue stashed Binance spot-trade fix: "Order No" column-offset detection (stash@{1}) — verify vs real exports, add tests | 🤖 | S |

### P6 — Advisor skill design session (with user)
Agenda: persona & scope (professional trader + long-term investor equivalent);
thesis format; plan fields (allocations, entries, exits, timeframes); validation /
invalidation triggers; honesty rules (no moving goalposts, log thesis outcomes);
which tools it may call (read-only ledger + web research + TDR/).
**Design outputs are personal → they land in `.portfolio_private.md` §2, never in tracked files.**

### P7 — Documentation refresh
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
