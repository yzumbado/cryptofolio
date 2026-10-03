# Cryptofolio — Working Backlog

**Purpose:** the living record of what's done and what's queued, so work continues
across sessions. Committed to git (durable, versioned). The agent's briefing points
here. Update it as items move.

**Last updated:** 2026-10-03

---

## ✅ Done (recent → older)

### Session 2026-10-03 — reconciliation + mining accounting
- **Mining accounting model** — `docs/MINING_ASSET_ACCOUNTING.md`; tokens = $0-cost
  income, hardware = depreciating capital, `cryptofolio mining-pnl` command. **PR #43 merged.**
  Live: +$462 operating profit, 26.1% of $2,600 capital recovered.
- **Aave aTokens display as underlying** (wstETH/rETH/USDT, not aEth*). **PR #42 merged.**
- **Staked SOL in balance + Stake program-id fix** (was invisible ~$7.9k). **PR #41 merged.**
- **Cost basis set:** wstETH $29,469 (exact, USDT legs); rETH $10,778 (certain FIFO lots,
  ETH-equiv price — rETH was supplied to Aave, not sold).
- **SOL lot dedup** — removed duplicate aggregate lot; correct basis $5,122 (65.66 @ $78.01).
- **GEOD/WINGS → $0 cost** (income); miners → $2,600 capital with 5yr straight-line depreciation.
- **Binance outflow audit** — all 9 withdrawn coins reconciled; USDT→Aave + ADA→NIGHT traced
  on-chain; 3 chain-verified audit notes in ledger. No unidentified wallets.
- **ZEC/NEAR/LINK** confirmed present (Binance Simple Earn, LD* wrappers).
- Portfolio net worth ~$111–115k (price-dependent); realized P&L −$1,935.

### Earlier
- Engine fixes #34; CI fixes #35/#36/#37; trade/convert pairing #38/#40.

---

## 📋 Open queue (priority order)

### P1 — security (flagged, awaiting user)
- [ ] **Rotate the plaintext Claude API key** in `config.toml [ai]` and move it to Keychain
  (`src/config/keychain.rs` supports it). Do NOT commit/echo the value.

### P2 — accounting completeness
- [ ] **P5: dated reward income** — wire the on-chain daily-reward pull (token ACCOUNT, not
  owner wallet; ~10-12 GEOD/day with blockTime) into `mining-pnl` for a real income-by-month
  view. `core::mining::RewardEvent` already exists; needs the Solana fetch + historical price.
- [ ] Decide depreciation life: currently 5-year straight-line; 3-year is the alternative.

### P3 — hygiene / display
- [ ] Filter scam airdrop tokens (CAT/HEX/DIXT/yRise/YES/xAI) out of raw holdings (excluded
  from value already; still clutter the holdings list).
- [ ] Drop two already-merged git stashes.

### P4 — automation / UX
- [ ] Daily portfolio-refresh cron (sync wallets + prices).
- [ ] Wire cost basis into the portfolio summary top-line (unrealized P&L headline).
- [ ] Recreate the `/portfolio` workflow as a KiroCrew agent/skill (from briefing).

---

## 🔑 Key facts (don't re-derive)
- Live DB: `~/Library/Application Support/cryptofolio/database.sqlite` (NOT ~/.config).
- FIFO cost basis; no tax layer. Ledger is append-only (correct via tx_type='correction').
- CI gate before push: `cargo fmt -- --check`, `cargo clippy -- -D warnings`,
  `cargo test --lib`, AND integration tests. Unwrap baseline = 8.
- Never fabricate lots — only from authoritative data (Binance export or on-chain tx).
- `trust_level` ∈ {exchange_verified, chain_verified, manual, unverified};
  `cost_basis_method` lowercase (fifo/lifo/average).
- Archive DB before edits (`_archive/database.pre-*`).
