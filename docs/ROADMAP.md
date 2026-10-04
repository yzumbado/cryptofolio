# Cryptofolio Roadmap

**Last Updated:** October 2026
**Current Version:** v0.6.0
**Living queue:** [docs/BACKLOG.md](BACKLOG.md) — authoritative, priority-ordered
(P1…P7). This roadmap carries direction; the backlog carries tasks and status.

---

## Vision

Build a **local-first, privacy-respecting cryptocurrency portfolio manager** that:

- Keeps all data on the user's machine (no cloud sync, no telemetry)
- Tracks multi-chain and multi-currency holdings with verifiable provenance
- Is driven conversationally by an **agentic working mode** (DSH) rather than a
  click-heavy GUI
- Turns an accurate ledger into **decision support** — scenarios and theses, not
  trading automation
- Remains simple, fast, watch-only, and developer-friendly

---

## Working Mode (✅ Landed — October 2026)

DSH is now the **primary agent tool** for this repo. Claude Code / Kiro sessions
may still occur, but the working-mode layer is DSH-first:

- ✅ **AGENTS.md** — auto-loaded orientation layer (read STATE.md, read the backlog,
  load the working-discipline skill).
- ✅ **STATE.md** — one-page current truth, including the "things that will make
  you act wrongly" landmines.
- ✅ **Skills** (`.dsh/skills/`) — `working-discipline` (rituals/guardrails),
  `plan-before-build`, `co-author-review`, `cryptofolio-conventions`.
- ✅ **CLAUDE.md → pointer** — no drifting duplicate instructions.
- ✅ **DSH MCP bundle** (`integrations/dsh-mcp/`) — connects the 18
  `cryptofolio_*` tools to the DSH profile via stdio.
- ✅ **Public/private boundary** — public docs hold engineering facts only;
  personal figures live in the gitignored `.portfolio_private.md`. Enforced by
  `scripts/check_leaks.sh` + CI `leak-check.yml`.

---

## Released

| Version | Date | Highlights |
|---------|------|------------|
| **0.6.0** | 2026-05-01 | MCP server (18 tools), `/portfolio` skill, code-quality + CI fixes |
| **0.5.1** | 2026-05-01 | Taproot xpub, Blockfrost keychain, `PrivacyMode` fix, onboarding bug fixes |
| **0.5.0** | 2026-04-01 | Multi-chain wallet tracking (BTC, ETH, ADA, SOL), sync audit log |
| **0.4.0** | 2026-03-28 | Binance deep integration (`sync-history`, full import) |
| **0.3.1** | 2026-03-01 | Keychain security, P&L foundation (FIFO/LIFO) |
| **0.2.0** | 2026-02-19 | Multi-currency support, JSON output, CSV export |
| **0.1.0** | 2026-01-15 | Initial release — portfolio tracking + Binance sync |

Post-0.6.0 work (portfolio reconstruction, chain reconciliation, DePIN mining P&L,
Multi-chain corrections, leak remediation, DSH layer) is recorded in
[CHANGELOG.md](../CHANGELOG.md) under **Unreleased**.

---

## Next Lanes

Directional order. Each lane maps to a priority in [docs/BACKLOG.md](BACKLOG.md),
which is the source of truth for individual tasks and status.

### 1. Trust foundation — P5 (Trust foundation & MCP fixes)

"Trust before acting": the ledger must be provably reliable before it feeds any
advisor or scenario output.

- Wire `reconciliation_log` (schema-only today) so reconciliation is a recorded
  fact, not an assumption.
- Fix MCP correctness gaps: decimal math in `track_conversion`/unrealized totals,
  output schemas, missing CLI surface (mining-pnl, sync-history, holdings,
  `pnl backfill`).
- Harden secrets handling (no raw signed responses in logs; no secrets via argv).
- Land the trust-fix batches currently in flight as reviewable PRs.

### 2. Advisor skill design session — P6

A **design session with the owner first**, before any implementation. Agenda:
persona and scope, thesis format, plan fields (allocations, entries, exits,
timeframes), validation/invalidation triggers, honesty rules (no moving
goalposts; log thesis outcomes), and which tools the skill may call (read-only
ledger + web research + `TDR/`).

Design outputs are personal by nature and land in `.portfolio_private.md` §2 —
never in tracked files.

### 3. Decision engine / scenario engine — P3

Productize the TDR prototype into a repeatable projection pipeline:

1. **`price_targets` table + `cryptofolio scenario` CLI** — project net worth,
   P&L-at-target, yield, and Aave health factor under bear / base / bull.
2. **MCP tool `cryptofolio_project_scenarios`** — expose the same projections to
   agents with a stable output schema.
3. **Advisor consumption** — the advisor skill reads live scenarios and drives the
   per-asset loop: thesis → plan → validation/invalidation triggers → honest review.

Later, watchlist alerts and strategy-drift reports build on this.

### 4. DSH web UI panel — P4

Bring portfolio visualization into the DSH Web GUI as a panel (theme tokens;
evolve the `prototypes/portfolio-dashboard.html` direction).

- Define the data contract first: the panel reads via the CLI/DB read path.
- Keep it local-only and read-only — no new network surface.

### 5. DSH presets — P2

Ship dedicated DSH presets — **Ledger**, **Advisor** (read-only `cryptofolio_*`
tools only, a structural guarantee), and **Maintainer** — so each working mode has
its own tool boundary. Also port the ledger-keeper skill and route "no investment
advice" refusals to the advisor skill.

---

## Long-Term Vision (2027+)

- **DeFi & cross-chain depth** — protocol position tracking, NFT portfolio views,
  cross-chain aggregation on top of the existing watch-only wallet model.
- **Advanced analytics** — risk analysis and rebalancing *recommendations* (never
  automated execution) through the decision engine.
- **Community & distribution** — plugin points for custom panels, shared report
  templates, Linux ARM builds, Docker, Homebrew.

---

## Not Planned (Out of Scope)

Intentionally excluded to keep the tool simple and safe:

- ❌ **Cloud sync** — remains local-first.
- ❌ **Trading / signing / broadcasting** — watch-only by design; private keys are
  rejected at input.
- ❌ **Automated trading or DCA execution** — too risky.
- ❌ **Mobile apps** — agent + CLI focused.
- ❌ **Tax filing integration** — use exports plus dedicated tax software.
- ❌ **Price-alert notifications** — use other tools; the scenario engine covers
  forward-looking analysis.

---

## Development Philosophy

1. **Local-first** — all data stays on the user's machine.
2. **Privacy-respecting** — no telemetry, no tracking; public/private boundary is
   CI-enforced.
3. **Read-only APIs** — never request write permissions from providers.
4. **Verifiable** — claims are backed by a cited source or a passing test; the
   ledger is append-only and corrections are explicit.
5. **Agentic development** — DSH-first working mode; built with AI pair programming.
6. **Semantic versioning** — no breaking changes without a major version.

---

## Questions?

- 📖 [Documentation](.)
- 🐛 [Issues](https://github.com/yzumbado/cryptofolio/issues)

**Next review:** with the next backlog reprioritization (see
[docs/BACKLOG.md](BACKLOG.md)).
