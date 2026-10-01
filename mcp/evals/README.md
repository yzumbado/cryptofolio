# Agent evals (LLM-in-the-loop)

Behavioral evals for the Cryptofolio MCP agent. Where the unit tests in
`mcp/tests/` verify each tool's plumbing (right argv, error envelope), these
verify the **agent's behavior**: does a real model, given the real tools, pick
the right tool, avoid the double-count footgun, and refuse to fabricate?

## Why this exists (the method)

Per Anthropic's agent-eval guidance:

- **Realistic, end-to-end scenarios** — actual `/portfolio` requests, not toy prompts.
- **Exercise the real tools** — the harness drives the actual MCP server over stdio,
  so the tool *descriptions and schemas* (part of what we're evaluating) are real.
- **Verifiable grading** — rubrics assert on the **tool-call trace** + final answer
  (deterministic), not vibes. See `scenarios.ts`.
- **Read transcripts** — every run is written to `evals/results/` for inspection;
  aggregate pass-rate hides failure modes.
- **Baseline → iterate** — record a baseline pass-rate, improve tools/prompts, re-measure.
  Non-deterministic, so this runs **manual / nightly and non-blocking**, never as a hard gate.
- **Trust the harness** — a `--mock` mode runs the whole pipeline with scripted model
  responses (no key, no binary), so the runner+grader are unit-testable. The grader is
  also covered by `tests/evals/grader.test.ts`.

## Running

### Mock (no key, no binary) — validates the harness itself
```bash
npm run eval:mock
```
Must be 100% — any failure means the harness, not the model, is broken.

### Live — measures the model
```bash
# 1. Build the binary and the MCP server
cargo build                       # produces target/debug/cryptofolio
cd mcp && npm run build           # produces dist/index.js

# 2. Point at them and provide a key
export ANTHROPIC_API_KEY=sk-...
export CRYPTOFOLIO_BIN=../target/debug/cryptofolio
export MCP_SERVER_ENTRY=$(pwd)/dist/index.js   # optional; defaults to dist/index.js
export EVAL_MODEL=claude-sonnet-4-6            # optional; default model

# 3. Run
npm run eval
```

Each scenario seeds an **isolated temp config dir** (so your real DB is never
touched), runs the agent loop, grades the trace, and writes a transcript to
`evals/results/`.

## Recording the baseline

After the first live run, record the aggregate pass-rate in
`docs/ROAD_TO_A.md` (the "LLM scenario pass-rate" baseline row). Re-measure after
any change to tool descriptions or the `/portfolio` skill.

## Layout

| File | Role |
|---|---|
| `types.ts` | Scenario / Check / Trace / Result types |
| `scenarios.ts` | The 8 graded scenarios + rubrics |
| `grader.ts` | Pure grading logic (unit-tested) |
| `harness.ts` | Agent loop over `Model` + `Tools` interfaces (the DI seam) |
| `mock.ts` | Scripted `Model` + `Tools` for `--mock` |
| `real.ts` | Live wiring: Anthropic Messages API + MCP client + temp env |
| `run.ts` | Entrypoint; report + transcript output |

## Type-checking & lint

```bash
npm run eval:typecheck   # tsc against evals/tsconfig.json
npm run lint             # eslint covers evals/ too
```
