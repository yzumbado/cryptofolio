---
name: working-discipline
description: Cryptofolio session rituals and guardrails — how to start, decide, delegate, and close work in this repo. Load before doing any work here.
whenToUse: at session start, before committing, before delegating to subagents
---

# Working discipline

## Session start

1. Read STATE.md (current truth + act-wrongly list) and docs/BACKLOG.md.
2. Check mechanisms fired: skills present in the catalog, `mcp__cryptofolio__*`
   tools available. If a claimed mechanism is missing, stop and report.
3. Propose the session focus in plain language, citing items by number AND name.
   Execute only after Yoel confirms.

## Guardrails

- **Verification Gate:** every technical claim is backed by a cited source or a
  passing test. Guessing is not verifying.
- **Disclosure Standard:** if a task did not meet its literal goal, say so
  plainly — never report a silent substitution as success.
- **Decision Velocity:** two-way doors (reversible) — decide, do, report.
  One-way doors — get Yoel's explicit agreement first. One-way here: ledger
  corrections, credential/key changes, force-push or history rewrite,
  destructive deletes, database resets, profile-wide plugin installs.
- **Trust raw over summary:** verify the artifact exists (file written, test
  passing, tool returning data), not just that a command ran.

## Delegating to subagents (cheap models)

Give a self-contained brief: Intent · Read first · Steps · Acceptance · Rollback.
Subagents run work and gates but never commit, push, merge, or install bundles.
Verify their artifact; never trust the report alone.

## Session end

1. Update docs/BACKLOG.md (move items, note outcomes) and STATE.md if a fact changed.
2. Run the gates: cargo fmt/clippy/test · npm typecheck/lint/test · leak check.
3. Commit with a conventional message. **Do NOT push unless Yoel asks.**

## Communication

- Cite backlog items as `#NN (plain name)` — never a bare number.
- Medium/large work: use the `plan-before-build` skill — artifact table,
  non-goals, Yoel's decisions, sign-off before writing.
- Decision moments: use the `co-author-review` skill or ask Yoel directly.
