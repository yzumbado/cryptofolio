# Road to A-Tier — Execution Playbook

> **Audience:** an implementing agent (e.g. Sonnet) executing the remaining
> A-tier work. This is the *how*; [`ROAD_TO_A.md`](ROAD_TO_A.md) is the *what*
> (the scorecard). Do them together: pick the next task here, do it, check its
> box in the scorecard, in the same PR.
>
> **Golden rule:** small, verified steps. One task = one branch = one PR. Never
> batch unrelated tasks. If a task balloons or you hit a real fork, STOP and ask
> — do not guess on schema, ledger, or destructive behavior.

---

## Operating procedure (read once, follow every task)

1. **Branch** off the current branch: `git checkout -b a-tier/<task-id>` (e.g. `a-tier/w2.1-mcp-ci`).
2. **Make the change** described in the task section below. Stay in scope.
3. **Run the verification ritual** (below) until it is fully green.
4. **Update the scorecard** (`docs/ROAD_TO_A.md`): tick the box, name the proving test/gate.
5. **Commit** with a clear message ending in the Co-Authored-By trailer. Open a PR.

### Verification ritual (must ALL pass before you call a task done)

Rust (run from repo root):
```bash
cargo fmt                       # auto-format FIRST
cargo fmt -- --check            # must show no diffs
cargo clippy --quiet 2>&1 | grep -cE '^warning:|^error:'   # must print 0
cargo test --lib                # must be all-green (>=298)
cargo test --test bdd 2>&1 | tail -3   # known: 2 Cardano display scenarios fail (pre-existing, flagged)
```
MCP (run from `mcp/`):
```bash
npm ci
npm run typecheck
npm run lint
npm test
```

### Hard-won rules (violating these has already cost us time)
- **CI is GitHub Actions** (`.github/workflows/ci.yml`) and runs `cargo clippy -- -D warnings`. A single clippy warning fails CI. Always end at 0 warnings. Use `cargo clippy --fix --allow-dirty --lib` for trivial ones.
- **`cargo fmt` before every commit.** A stray format diff fails CI.
- **NEVER run `git stash`** to "peek at baseline" — it reverts your uncommitted work. Use a separate `git worktree` or just reason from the diff.
- **The ledger is immutable.** Do not add `UPDATE`/`DELETE` on `transactions`, and do not use `INSERT OR REPLACE` on it — triggers will `RAISE(ABORT)`. Corrections are new `tx_type='correction'` rows. Accounts are archived, not deleted.
- **Money is `TEXT` + `rust_decimal::Decimal`.** Never `f64`. Decimals serialize as JSON strings on purpose.
- **Don't expand scope.** If you spot an unrelated bug, note it in the PR description; don't fix it here.
- **Adding a field to a struct** (e.g. `Transaction`, `BinanceCsvRow`) means updating every construction site and every `SELECT`/`INSERT` column list. Build will tell you each one — fix them all, don't shortcut.

---

## TASK W2.1 — Put the MCP layer in CI  *(Sonnet)*

**Why:** the 53 TypeScript tests run locally only; half the product is ungated.

**File:** `.github/workflows/ci.yml` (read it first to match style/indentation).

**Steps:**
1. Add a new job `mcp-test` parallel to the existing `test` job.
2. It must: checkout, set up Node 20, `cd mcp`, `npm ci`, `npm run typecheck`, `npm run lint`, `npm test`.

**Pattern (adapt to the file's existing style):**
```yaml
  mcp-test:
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: mcp
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'npm'
          cache-dependency-path: mcp/package-lock.json
      - run: npm ci
      - run: npm run typecheck
      - run: npm run lint
      - run: npm test
```

**Done when:** `ci.yml` parses (valid YAML) and the steps mirror what you can run locally green in `mcp/`. NOTE: W2.2 (eslint config) must land first or `npm run lint` fails — do W2.2 before/with this.

**Guardrail:** do not touch the existing `test` job or `release.yml`.

---

## TASK W2.2 — Add the missing eslint config  *(Sonnet)*

**Why:** `mcp/package.json` runs `eslint src/ tests/` but no config file exists, so lint is meaningless.

**Files:** create `mcp/eslint.config.js` (flat config; eslint 9 style). Check `mcp/package.json` for the eslint version first.

**Pattern (TypeScript flat config):**
```js
import tseslint from "typescript-eslint";

export default tseslint.config(
  ...tseslint.configs.recommended,
  {
    files: ["src/**/*.ts", "tests/**/*.ts"],
    rules: {
      "@typescript-eslint/no-unused-vars": ["error", { argsIgnorePattern: "^_" }],
    },
  },
  { ignores: ["dist/**", "node_modules/**"] },
);
```
If `typescript-eslint` isn't a dep yet: `npm i -D typescript-eslint` (commit the package.json/lock changes).

**Done when:** `npm run lint` exits 0 in `mcp/`. Fix any real lint errors it surfaces (don't blanket-disable rules).

---

## TASK W2.3 — Production `unwrap`/`expect` CI gate  *(Sonnet)*

**Why:** lock in the audited discipline (8 known-infallible production unwraps) so it can't regress.

**Files:** create `scripts/check_unwraps.sh`; add a step to the `test` job in `.github/workflows/ci.yml`.

**Steps:**
1. First, establish the CURRENT baseline locally:
   ```bash
   python3 - <<'PY'
   import glob
   total=0
   for f in glob.glob('src/**/*.rs', recursive=True):
       s=open(f).read()
       idx=s.find('#[cfg(test)]')
       prod = s if idx==-1 else s[:idx]   # exclude test module (first one onward)
       total += prod.count('.unwrap()')+prod.count('.expect(')
   print(total)
   PY
   ```
2. Put that number as `BASELINE` in the script below (expected: 8 — confirm).
3. Create `scripts/check_unwraps.sh`:
   ```bash
   #!/usr/bin/env bash
   set -euo pipefail
   BASELINE=8
   COUNT=$(python3 - <<'PY'
   import glob
   total=0
   for f in glob.glob('src/**/*.rs', recursive=True):
       s=open(f).read()
       idx=s.find('#[cfg(test)]')
       prod = s if idx==-1 else s[:idx]
       total += prod.count('.unwrap()')+prod.count('.expect(')
   print(total)
   PY
   )
   echo "Production unwrap/expect count: $COUNT (baseline $BASELINE)"
   if [ "$COUNT" -gt "$BASELINE" ]; then
     echo "::error::unwrap/expect regression: $COUNT > $BASELINE. Return Result instead."
     exit 1
   fi
   ```
   `chmod +x scripts/check_unwraps.sh`.
4. Add to the `test` job in `ci.yml`: `- run: bash scripts/check_unwraps.sh`.

**Known limitation (document in the script comment):** the heuristic excludes everything after the first `#[cfg(test)]` in a file. Good enough as a ratchet; do not over-engineer.

**Done when:** `bash scripts/check_unwraps.sh` prints the baseline and exits 0.

---

## TASK W1.5 — Schema/enum + version drift cleanup  *(Sonnet)*

**Why:** dead values and a stale manifest mislead every future reader (human or agent).

**Steps (each independently verifiable):**
1. **Drop dead `'transfer'`** from the `tx_type` CHECK in `src/db/schema.rs` (the Rust `TransactionType::as_str()` never emits it). Keep the `from_str` alias `"transfer" => TransferInternal` in `src/core/transaction.rs` (input ergonomics). Run `cargo test --lib`.
2. **Pin the version:** set `Cargo.toml` `version` to match the latest entry in `CHANGELOG.md` (expected `0.6.x` — verify). Run `cargo build`.
3. **Backfill tags** (only if the user confirms — tagging is outward-facing): `git tag v0.5.0 <sha>` etc. **Ask before pushing tags.**

**Done when:** tests green, version matches CHANGELOG. **Guardrail:** do NOT push tags without explicit user approval.

---

## TASK W2.4 — Coverage reporting (report, don't gate yet)  *(Sonnet)*

**Why:** make coverage visible; set a floor later once baseline is known.

**Steps:**
1. Rust: add a CI step using `cargo-llvm-cov` (fast; tarpaulin was removed for timeouts). Example: `cargo install cargo-llvm-cov && cargo llvm-cov --lib --summary-only`. Make it **non-blocking** (`continue-on-error: true`).
2. MCP: `vitest run --coverage` (add `@vitest/coverage-v8` dev dep). Non-blocking.
3. Record both baseline numbers in `docs/ROAD_TO_A.md` "Baselines" table.

**Done when:** both coverage commands run and numbers are recorded. Do not set a failing threshold yet.

---

## TASK W3.1 — Programmatic agent evals for MCP tools  *(Sonnet — repetitive, well-specified)*

**Why:** verify the agent-facing contract: each tool selects the right CLI command with the right params, and rejects bad input with an actionable error.

**Files:** extend `mcp/tests/tools/*.test.ts` (one file per tool group already exists). Study `mcp/tests/tools/status.test.ts` first — copy its harness (`getTool`, fixture mocking of `runCli`).

**For EACH of the 18 tools, add ≥3 cases:**
1. **Happy path:** mock `runCli` to return a fixture; assert the tool calls `runCli` with the expected argv and returns a success envelope.
2. **Param mapping:** a non-default input (e.g. a filter/limit) produces the expected argv.
3. **Bad input:** missing required field (or a `CliError`) returns an error envelope with a helpful `hint`/code (not a raw throw).

**Pattern:**
```ts
it("manage_account remove archives via CLI", async () => {
  vi.mocked(runCliRaw).mockResolvedValueOnce("");
  const res = await getTool(server, "cryptofolio_manage_account").callback(
    { action: "remove", name: "Old" }, {} as any);
  expect(runCliRaw).toHaveBeenCalledWith(["account", "remove", "Old", "--yes"]);
  expect(res.content[0].text).toContain("archived");
});
```

**Done when:** every tool has ≥3 cases; `npm test` green; add `.rows`-style fixtures under `mcp/tests/fixtures/` as needed.

**Guardrail:** these mock the CLI — they do NOT invoke a real model. That's W3.2.

---

## TASK W3.3 — Harden the `/portfolio` skill  *(Sonnet — text edits)*

**Why:** the skill lacks failure-mode guidance.

**File:** `.claude/skills/portfolio/SKILL.md`.

**Add three rules (match the file's existing voice/format):**
1. **Error/rate-limit fallback:** if a tool returns an error or a sync times out, tell the user plainly, show partial data if available, and suggest a retry — never fabricate balances.
2. **Pagination:** for large histories, use `limit`/`offset` on `cryptofolio_list_transactions` and summarize rather than dumping everything.
3. **Staleness:** before quoting numbers, check `last_synced`; if stale (>~24h), say so and offer a refresh.

**Done when:** the three rules are present and concrete. Each should later get an eval scenario in W3.2.

**Guardrail:** keep the skill's existing scope boundaries (data-only; no trading/tax advice).

---

## LEAVE FOR THE STRONGER MODEL (Opus) — do NOT attempt with Sonnet

- **W3.2 — LLM-in-the-loop eval harness** (`mcp/evals/`). Requires designing scenarios, a grading rubric, and a runner that drives a real model. Judgment-heavy and novel; high risk of a plausible-but-wrong harness. Opus designs it; Sonnet can then ADD scenarios to an established harness.
- **W3.4 — CLAUDE.md honesty pass.** Requires deciding which documented claims are now actually *enforced* (e.g. immutability is now true; "double-entry" should be renamed "paired-column journal") and adding security/staleness/testing sections. Needs whole-system judgment. Opus owns this.

---

## When you finish a task
- Scorecard box ticked with the proving test named.
- Verification ritual fully green.
- One focused PR. Then return to the scorecard and take the next box.
