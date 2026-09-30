# Spec: transfer- and correction-aware P&L backfill

Status: proposed · Owner: TBD · Branch: `feat/backfill-transfers` (off `bugfix/issues-11-13-16-18`)

## Problem

`pnl backfill` replays the ledger to rebuild `tax_lots` and `realized_pnl`, but it
only understands three transaction kinds:

```rust
// src/cli/commands/pnl.rs
match tx.tx_type.as_str() {
    "buy"  => process_acquisition(...),
    "sell" => process_disposal(...),
    "swap" => disposal + acquisition,
    _ => {}   // transfer_in / transfer_out / transfer_internal / correction / stake / ... all ignored
}
```

Because transfers and corrections are skipped, lots and holdings drift from reality
whenever coins move between the user's own accounts or a bad row is corrected:

1. **Transfers create phantom cost basis.** Coins are frequently recorded as a `buy`
   into the destination account (observed across TAO, BTC, RPL, the `LD*` Simple-Earn
   wrappers). The same coins already have a `buy` lot in the source account, so the
   backfill counts them **twice or three times**. Example measured this session:
   TAO held = 71.40, but `tax_lots` summed to ~163 across three accounts
   (Binance buys + a manual re-entry of the same buys + transfer-to-cold-storage rows,
   all typed `buy`).

2. **`correction` rows are inert.** The append-only ledger corrects an error by
   inserting a `tx_type='correction'` row (invariant #1), but backfill's `_ => {}`
   arm ignores it, so a correction never actually adjusts the rebuilt lots.

3. **`match_disposal` is per-account** (`calculator.rs`), so a disposal in one account
   cannot draw cost basis from lots sitting in another. A cross-venue sale (e.g. the
   RPL sold on Binance but partly acquired on-chain) can therefore run short of basis.

Net effect: cost-basis totals and holdings-vs-lots reconciliation are unreliable for
any asset the user moved between accounts. This is a data-model gap, not a data-entry
mistake, and must be fixed in code before any bulk ledger reconciliation is safe.

## Goals

- After `pnl backfill`, for every asset: `SUM(tax_lots.remaining_quantity)` per account
  equals `holdings.quantity` for that (account, asset), within rounding.
- Transfers move cost basis with the coins instead of minting new basis.
- `correction` rows actually adjust the rebuilt lots.
- No change to the append-only invariant: corrections stay inserts; nothing UPDATE/DELETEs
  `transactions`.

## Non-goals

- Tax-lot *reporting* semantics (short/long-term, wash sales) — unchanged.
- The separate `pnl summary` unrealized-price bug (tracked elsewhere).
- Rewriting historical rows. The fix is in replay logic + a one-time reconciliation.

## Design

### 1. Teach backfill the transfer kinds

Add arms to the backfill match. `transfer_lots(from, to, asset, qty, method)` already
exists in `calculator.rs` (line 172) and moves FIFO/LIFO lots between accounts carrying
their basis — wire it in:

```rust
"transfer_internal" => transfer_lots(from_account, to_account, from_asset, from_qty, method),
"transfer_out"      => // if to_account is a known internal account: transfer_lots;
                       // else (leaves the tracked set): move lots out, NO realized P&L
                       //   (a withdrawal is not a sale — see import_withdrawal fix, commit 3ee3099)
"transfer_in"       => // if from_account is a known internal account: handled by the paired
                       //   transfer_out; else it is an external deposit with a stated basis
                       //   -> process_acquisition at the row's price_usd (or 0 if none)
```

Pairing rule: a `transfer_internal` carries both `from_account_id` and `to_account_id`;
a `transfer_out`/`transfer_in` pair that reference each other (same asset, qty, adjacent
timestamps, or a shared `external_id`/`tx_hash`) is one internal move — collapse to a
single `transfer_lots`. Where the counterparty account is NOT in `accounts`, treat it as
leaving/entering the tracked set (move out with no P&L; move in as acquisition).

### 2. Make `correction` rows adjust lots

Define correction semantics precisely (currently only convention):

- A correction with `from_asset`+`from_quantity` **removes** that quantity of lots from
  `from_account_id` (FIFO), no realized P&L — it unwinds an over-recorded acquisition.
- A correction with `to_asset`+`to_quantity`+`price_usd` **adds** a lot (like a buy) —
  it supplies a missing acquisition/basis.
- This is enough to express "negate duplicate buy #N" (remove) and "record the real lot"
  (add) without ever mutating a historical row.

### 3. Optional: cross-account disposal fallback

When `match_disposal` cannot satisfy a sell from the disposing account's lots, and the
asset exists in a linked account, allow drawing basis from there (configurable; default
off to preserve strict per-account accounting). Needed for genuine cross-venue sales.

### 4. One-time reconciliation (data, not code)

After the code lands, reconcile the existing ledger with `correction` rows (append-only):
for each asset where `SUM(lots) != SUM(holdings)`, the operator confirms the authoritative
source (e.g. "Binance API trades are truth for TAO") and correction rows negate the
duplicates. This is a guided, reviewed step — never a blind bulk delete.

## Acceptance criteria / tests

- Unit (`calculator` + backfill): a buy in A then `transfer_internal` A→B yields ONE lot
  in B with the original basis, zero in A, and no realized P&L.
- Unit: `transfer_out` to an external (unknown) account removes the lot, records no gain.
- Unit: a `correction` removing qty reduces lots; a correction adding qty creates a lot.
- Integration: seed the TAO triple-count scenario (26 API buys + 12 duplicate manual buys
  + 3 transfers) → after backfill + reconciliation, `SUM(lots)=SUM(holdings)=71.40` and
  blended basis ≈ $256/TAO.
- Regression: RPL realized stays +$469.57; withdrawals still book no phantom loss.
- Gates: `cargo fmt --check`, `cargo clippy -D warnings`, unwrap gate (baseline 8),
  `cargo test --lib`, MCP suite.

## Risk / rollback

- Touches the value engine → land behind the full gate; snapshot the live DB before the
  one-time reconciliation (pattern already used this session:
  `database.sqlite.<tag>-<ts>`).
- Reconciliation is append-only and reversible by restoring the pre-reconciliation backup.

## References (code)

- `src/cli/commands/pnl.rs` — backfill match arms (the `_ => {}` gap).
- `src/core/pnl/calculator.rs` — `process_acquisition` / `process_disposal` /
  `transfer_lots` (172) / `match_disposal` (per-account).
- `src/exchange/binance/import.rs` — `import_withdrawal` (already fixed: withdrawals are
  not sales, commit 3ee3099).
- Invariants: `.kiro/steering/20-invariants.md` (append-only ledger, correction rows).
