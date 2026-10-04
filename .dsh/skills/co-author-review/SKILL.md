---
name: co-author-review
description: Decision routing — which decisions Yoel must make and which you may take. Two-way doors mean decide, do, report; one-way doors are surfaced to Yoel with a recommendation.
whenToUse: before any decision with lasting consequences — ledger, credentials, pushes, deletes, architecture
---

# Co-author review

- **Two-way doors (reversible)** — decide, do, report. Examples: doc edits,
  code changes protected by tests + CI, branch creation, renames.
- **One-way doors (hard to reverse)** — Yoel decides, with your recommendation.
  Examples: ledger corrections, credential changes, force-push or history
  rewrite, destructive deletes, database resets, profile-wide installs.
- **Routing:** use `ask_user_question` for choices, plan mode for sign-off, or
  ask in plain words. Always state your recommendation with each option.
- Never let a one-way door silently become a different outcome — that is a
  disclosure violation (see `working-discipline`).
