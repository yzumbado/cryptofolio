# Mining & Earned-Asset Accounting Model

**Status:** adopted 2026-10-03 · **Scope:** DePIN miners (GEODNET, Wingbits) and any
future earned/mined asset. **Audience:** portfolio owner + engineers extending the tracker.

This document defines how `cryptofolio` represents **assets you produced** (mined/earned
tokens) versus **capital equipment you bought to produce them** (miners). It is the
professional treatment: cost attaches to what you *bought*; revenue attaches to what you
*produced*. The two never mix on one line.

---

## 1. The core principle

> **Earned tokens are revenue at $0 cost basis. The hardware is a capital asset carried at
> cost and depreciated. They meet only in a mining P&L statement — never on a token's cost
> line.**

Putting the $2,600 miner cost onto the GEOD/WINGS tokens (the amateur move) makes the tokens
look like they "cost" ~$1 each and are "down 74%", when in reality they are pure margin and
the *hardware* is the thing being recovered over time.

---

## 2. The three ledgers

### 2.1 Earned tokens — Revenue, zero cost basis
- **Where:** the on-chain wallet that receives them (`GN-01-Avioneta` → GEOD,
  `WB-01-Avioneta` → WINGS).
- **Cost basis:** `$0`. Every token mined is income at fair value on the day received; with
  no tax layer we keep the clean form — basis `0`, so 100% of current value is income earned.
- **Representation in the tracker:** `tax_lots.acquisition_price = '0'` for the earned
  quantity. Unrealized "P&L %" is therefore not meaningful (you cannot lose % on a $0-cost
  asset) — the number that matters is **income earned to date = current value + value of any
  already sold**.

### 2.2 Mining hardware — Capital asset (PP&E), depreciated
- **Where:** the `DePIN Hardware` account (type `bank`, category `banking`), as
  pseudo-assets `MINER-GEODNET-LOC1` and `MINER-WINGBITS-LOC2`.
- **Cost:** $1,300 each, $2,600 total, booked at acquisition.
- **Depreciation:** straight-line over a **useful life** (default **5 years = 60 months**;
  3 years is the aggressive alternative). Monthly expense = cost / months.
  - 5-year: $2,600 / 60 = **$43.33/mo**
  - 3-year: $2,600 / 36 = **$72.22/mo**
- **Net book value (NBV):** `cost − accumulated depreciation`, floored at $0 (or salvage).

### 2.3 Mining P&L — where revenue meets cost
Produced on demand (report, not stored state):

```
DePIN Mining Operation — P&L (period / cumulative)
  Revenue (tokens mined, FMV at receipt):   + token income
  Less: hardware depreciation:              − accumulated dep.
  Less: electricity / internet (optional):  − opex
  ───────────────────────────────────────────────────────────
  Operating profit / (loss)

  Memo — capital recovery:
    Hardware at cost:          $2,600
    Accumulated depreciation:  −$X
    Net book value:            $Y
    Tokens earned to date:     $Z   (Z / 2,600 = % of capital recovered)
```

---

## 3. Data architecture

Nothing new in the schema is required for the MVP — the model maps onto existing tables:

| Concept | Storage | Notes |
|---|---|---|
| Earned token (zero cost) | `tax_lots.acquisition_price='0'` on the wallet account | income, not a purchase |
| Hardware at cost | `holdings` row in `DePIN Hardware`, `avg_cost_basis` = unit cost | pseudo-asset `MINER-*` |
| Hardware acquisition | append-only `transactions` row, `tx_type='buy'`, `notes` = HWID/location | provenance |
| Depreciation parameters | `transactions` row `tx_type='correction'`, `notes` = JSON `{useful_life_months, in_service_date, method:'straight_line'}` | one per miner; append-only, so a revised schedule is a NEW correction row that supersedes |
| Depreciation at a date | **computed**, not stored: `min(cost, cost/life_months × months_in_service)` | keeps the ledger append-only and avoids monthly write jobs |

**Why computed, not posted monthly:** the ledger is append-only and we avoid a cron writing
60 depreciation rows per miner. NBV is a pure function of (cost, in-service date, useful life,
valuation date). A report derives it live.

**Pricing guard:** `MINER-*` pseudo-assets must never be priced by a market feed. The
valuation pipeline treats a `MINER-*` symbol as **"carry at NBV"** (cost − computed
depreciation), the way `defi.rs::classify` already special-cases Aave/Earn wrappers.

---

## 4. Procedures (repeatable)

### P1 — Record a new mined-token batch (or re-sync)
1. Sync the wallet (chain truth) — balance already includes newly mined tokens.
2. Ensure the token's `tax_lots` carry `acquisition_price='0'` for the full held quantity.
   (New rewards inherit $0 automatically if the lot is a single $0 lot sized to holdings.)
3. Income to date = held value + value of any sold batches (from `sell` tx revenue).

### P2 — Add a mining rig (capital asset)
1. `DePIN Hardware` account must exist (type `bank`, category `banking`).
2. Insert a `holdings` row `MINER-<NET>-<LOC>` qty 1, `avg_cost_basis` = unit cost.
3. Append a `transactions` `buy` row with the HWID + in-service date in `notes`.
4. Append a `correction` row with the depreciation JSON (life, in-service date, method).

### P3 — Produce the mining P&L / capital-recovery view
1. For each miner: `NBV = max(0, cost − cost/life_months × months_since_in_service)`.
2. Revenue = cumulative token income (held FMV + sold revenue).
3. Operating profit = revenue − accumulated depreciation − opex (if tracked).
4. Capital recovered % = tokens earned to date / hardware cost.

### P4 — Revise a depreciation schedule
Append a NEW `correction` row with updated JSON; the latest row for that miner wins. Never
edit the old one (append-only ledger).

---

## 5. Current state (2026-10-03)

- GEOD/WINGS: being moved to **$0 cost** (income). ← this change
- Hardware: $2,600 at cost in `DePIN Hardware`. Depreciation params: **to be set** (default
  5-year straight-line, in-service date = earliest reward date per wallet).
- Tokens earned to date ≈ **$672** → **25.8% of the $2,600 recovered.**

---

## 6. Non-goals / honest limits

- **Per-reward FMV history IS available** (corrected 2026-10-03): each DePIN reward lands as a
  dated daily transfer on the token ACCOUNT (not the owner wallet — the owner-level signature
  list misses them). The GEOD token account shows ~10-12 GEOD/day, one tx per day, each with
  an exact `blockTime`. Income-by-period is therefore reconstructable: book each reward at its
  FMV on its receipt date. Procedure P5 below covers it.
- **Opex (power/internet) is NOT tracked, by design** (owner decision 2026-10-03): the miners
  run in two homes the owner already owns and already pays electricity + internet for. There
  is **no incremental cost** attributable to mining, so opex = $0 and the operating profit is
  revenue − depreciation only. Revisit only if a dedicated/metered cost ever arises.
- This is management accounting for decision-making, not a tax filing. No wash-sale logic.

---

## 7. Procedure P5 — reconstruct dated reward income (per-period)

1. Resolve the token ACCOUNT for the mint on the miner wallet (`getTokenAccountsByOwner`).
2. Page `getSignaturesForAddress` on the TOKEN ACCOUNT (not the owner) — one tx per daily
   reward.
3. For each tx, `getTransaction` and diff pre/post token balances for the owner → the reward
   amount; `blockTime` → the date.
4. Price each reward at the token's FMV on that date (historical price) → dated income.
5. Sum by period for an income-by-month view; cumulative sum = total income earned (exact,
   not just current-FMV approximation).
