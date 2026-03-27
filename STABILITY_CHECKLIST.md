# Stability Plan - Quick Reference Checklist

**Goal:** Complete and validate existing features before adding new ones

---

## Week 1: Foundation Stability

### Day 1-2: ⬜ Task #16 - API Key Configuration (CRITICAL)
- [ ] Design config file format (`~/.config/cryptofolio/config.toml`)
- [ ] Implement `cryptofolio config set blockfrost-key` command
- [ ] Add environment variable support (`BLOCKFROST_API_KEY`)
- [ ] Integrate with `BlockfrostClient::new()`
- [ ] Test with real Blockfrost API
- [ ] Update BDD tests (ensure mocks still work)
- [ ] Update `TESTNET_SETUP_GUIDE_CARDANO.md`
- [ ] **Success:** Cardano sync works with API key ✅

### Day 3: ⬜ Fix Bitcoin Transaction Mock
- [ ] Debug why test gets 25 transactions instead of 7
- [ ] Verify mock URL passed correctly in `common_steps.rs`
- [ ] Ensure test uses mock, not real API
- [ ] **Success:** BDD test shows exactly 7 transactions ✅

### Day 4: ⬜ Manual Validation v0.4.0 (Binance)
- [ ] Complete 15-step checklist in `VALIDATION_GUIDE_v0.4.0.md`
- [ ] Test with real Binance account
- [ ] Verify all 5 transaction types import correctly
- [ ] Test incremental sync (watermarks work)
- [ ] Document findings in `VALIDATION_RESULTS_v0.4.0.md`
- [ ] **Success:** No crashes, data matches Binance UI ✅

### Day 5: ⬜ Manual Validation v0.5.0 (Cardano)
- [ ] Complete 12-step checklist in `VALIDATION_CHECKLIST_CARDANO.md`
- [ ] Test with real Preprod wallet + Blockfrost API
- [ ] Verify ADA balance matches CardanoScan
- [ ] Test native token detection (if available)
- [ ] Test transaction import
- [ ] Document findings in `VALIDATION_RESULTS_CARDANO.md`
- [ ] **Success:** No crashes, balance accurate ✅

---

## Week 2: Validation & Polish

### Day 1: ⬜ Task #15 - Ethereum Testnet Validation
- [ ] Get Sepolia ETH from faucet
- [ ] Add Sepolia wallet to cryptofolio
- [ ] Sync balance (matches Etherscan)
- [ ] Test ERC-20 token detection
- [ ] Test transaction import with gas tracking
- [ ] Document findings in `VALIDATION_RESULTS_ETHEREUM.md`
- [ ] **Success:** All features work on Sepolia ✅

### Day 2-3: ⬜ Complete Bech32 Checksum Validation
- [ ] Research bech32 v0.11 API for Cardano
- [ ] Implement proper checksum validation
- [ ] Fix failing "bad checksum" BDD test
- [ ] Update unit tests with valid addresses
- [ ] **Success:** Checksum validation working ✅

### Day 4: ⬜ Task #14 - BDD Test Maintenance
- [ ] Fix 4 unused import warnings
- [ ] Fix 2 unused variable warnings
- [ ] Document step definition patterns
- [ ] Clean up unused mock methods
- [ ] **Success:** Zero warnings, 37/37 tests passing ✅

### Day 5: ⬜ Buffer / Release Prep
- [ ] Address any issues from validation
- [ ] Update CHANGELOG.md
- [ ] Review all documentation
- [ ] **Success:** Ready for v0.5.0 release ✅

---

## Week 3+: New Features (Ready!)

### ⬜ Task #8: Solana Wallet Tracking
- Start only after Week 2 complete ✅

### ⬜ Task #10: Portfolio Audit Command
- Start only after Week 2 complete ✅

### ⬜ Task #11: MCP v0.6.0 Design
- Start only after Week 2 complete ✅

---

## Daily Standup Questions

**What did I complete yesterday?**
- [ ] List completed checkboxes

**What am I working on today?**
- [ ] Next unchecked item in current week

**Any blockers?**
- [ ] Document in STABILITY_PLAN.md under "Risk Mitigation"

**Is the plan still accurate?**
- [ ] Update timeline if needed

---

## Current Status

**Test Suite:**
- Unit: 203/203 ✅
- Integration: 138/138 ✅
- BDD: 31/37 (84%) - Target: 37/37 (100%)

**Releases:**
- v0.4.0 (Binance): Code complete, pending validation
- v0.5.0 (Cardano): Code complete, **BLOCKED** by Task #16

**Critical Path:**
Task #16 → Cardano Validation → v0.5.0 Release → New Features

---

## Quick Commands

```bash
# Run all tests
cargo test

# Run BDD tests only
cargo test --test bdd

# Run specific blockchain tests
cargo test --lib blockchain::cardano

# Build all targets
cargo build --all-targets

# Check for warnings
cargo clippy --all-targets
```

---

**Last Updated:** 2026-03-19
**Next Milestone:** Complete Week 1 (Foundation Stability)
