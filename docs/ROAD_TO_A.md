# Road to A-Tier

> **Source of truth.** This is the durable scorecard for raising Cryptofolio from B+ (3.4/4.0) to
> A-tier — across both the Rust code and the agent layer (MCP server + skills). It survives context
> compaction and is followed across sessions and models. Full rationale and teaching notes live in
> the approved plan; this file is the executable checklist.

## How to use this

1. Open this file. Pick the next unchecked box (respect the execution order below).
2. Execute that one item in a focused push (use plan mode for non-trivial items).
3. In the **same PR**, check the box here and note the proving test / gate.
4. Keep this file current — it is the only place that knows how far we've come.

**Hard freeze:** no roadmap feature work (reconciliation, price-fill, web UI, sync loop) until
every box below is green.

**The A-tier triad:** *enforce* (guarantees live in the engine), *gate* (regressions are blocked
by CI), *measure* (agent behavior is evaluated, not assumed).

---

## Exit scorecard

### Code integrity
- [x] Importer surfaces a skipped-rows count; **zero silent drops**. Parser returns `ParseReport { rows, skipped: Vec<SkippedRow> }`; the command reports duplicates / parse-skips / write-errors distinctly and lists each skip with line + reason. Proven by `csv::tests::test_malformed_row_is_reported_not_dropped`.
- [x] Price **and** network captured at ingestion; survive a DB round-trip. Spot-trade/order/alpha `price` + `price_asset` and withdraw/deposit `network` now captured (`csv.rs`); `import_binance.rs` maps price → `price_usd` (USD-equivalent) or `price_currency`/`price_amount`, and resolves chain from the export's Network column via `network_to_chain` (not a coin guess). Proven by `csv::tests::test_parse_spot_trade_captures_price`, `test_parse_withdraw_csv_skips_pending` (network), and `import_binance::tests::test_price_in_usd_equivalent_populates_price_usd` / `test_price_in_non_usd_preserved_natively` / `test_network_to_chain_uses_export_value`.
- [x] Immutability **enforced by DB trigger**; an `UPDATE`/`DELETE` on `transactions` aborts (proven by `db::transactions::tests::test_ledger_is_immutable`). Coupled decision: accounts are **archived, never hard-deleted** (`archive_account`/`reactivate_account`; proven by `db::accounts::tests::test_archive_account`).
- [x] Dedup keys are identity-stable; a legitimate same-second/same-amount pair does **not** collide. `disambiguate_external_ids` gives colliding content-hash ids a stable `#N` suffix (idempotent across re-imports). Proven by `csv::tests::test_identical_rows_get_distinct_external_ids`.
- [x] Unknown Binance operations **fail closed** — `map_operation` returns `Option`; an unrecognised op becomes a reported skip (with the op name), never a silent `Earn`. Proven by `csv::tests::test_unknown_operation_fails_closed_not_earn`.
- [ ] Schema/enum drift removed (no dead `'transfer'` value); `Cargo.toml` version matches CHANGELOG; tags backfilled.
  - Dead `'transfer'` value removed from `tx_type` CHECK in `src/db/schema.rs` (W1.5, 2026-06-23). `Cargo.toml` pinned to `0.6.0` matching CHANGELOG. **Tag backfill deferred — needs explicit user approval before pushing.**

### Quality gates (regression-proof)
- [x] MCP TypeScript tests run in CI on every PR — `mcp-test` job in `.github/workflows/ci.yml` runs `npm ci/typecheck/lint/test` (Node 20). Verified locally: `npm ci` clean, 50/50 tests pass.
- [x] `eslint` config present; MCP lint green in CI — `mcp/eslint.config.js` (flat, typescript-eslint recommended); `npm run lint` exits 0 (fixed an unused import in `status.ts`).
- [x] Production `unwrap`/`expect` count gated in CI (baseline: **8** known-infallible; no regression). Gate: `bash scripts/check_unwraps.sh` in the `test` job; baseline measured 2026-06-23.
- [ ] Coverage reported for both Rust and MCP (floor gated once baseline is known).

### Agent practice
- [x] Every MCP tool has ≥3 **programmatic** eval cases (correct tool + params from structured input); suite green in CI. 66 tests (was 50), all 18 tools covered — W3.1, 2026-06-23.
- [ ] **LLM-in-the-loop** scenario suite exists (≥8 graded scenarios with a rubric); runs nightly/manual; baseline pass-rate recorded.
- [ ] `/portfolio` skill hardened: error/rate-limit fallback, pagination guidance, staleness rules — each covered by an eval scenario.
- [ ] **CLAUDE.md honesty pass**: no claim documented that isn't enforced; missing sections (secrets hygiene, staleness policy, testing strategy) added.

---

## Execution order

1. **W0** — this scorecard ✅ (you're reading it).
2. **W1.1–1.4** — integrity foundation: immutability, fidelity, fail-closed, dedup.
3. **W2.1–2.3** — CI gates: MCP-in-CI, eslint, unwrap gate.
4. **W3.1 + W3.4** — programmatic evals + CLAUDE.md honesty pass.
5. **W3.2 + W3.3** — LLM eval harness + skill hardening.
6. **W1.5 + W2.4** — drift cleanup + coverage (alongside).

---

## Baselines (recorded as we go)

| Metric | Baseline | Source |
|---|---|---|
| Production `unwrap`/`expect` | 8 (all known-infallible) | Eval 2026-06-21 |
| Rust unit tests | 298 passing (was 290) | `cargo test --lib` |
| MCP tests | 66 (all 18 tools, ≥3 cases each) | Vitest |
| LLM scenario pass-rate | _TBD_ | W3.2 |
| Rust coverage | _TBD_ | W2.4 |
| MCP coverage | _TBD_ | W2.4 |
