# Cryptofolio Stability Plan
**Version:** 1.0
**Date:** 2026-03-19
**Goal:** Maximize stability by completing and validating existing features before adding new ones

---

## 🎯 Strategy Overview

**Philosophy:** Complete → Validate → Stabilize → Expand

We have two major releases (v0.4.0 Binance, v0.5.0 Cardano) that are code-complete but unvalidated. Before adding new blockchains or features, we must ensure the foundation is solid.

---

## 📊 Current State

### ✅ Completed & Released
- **v0.2.0:** Multi-currency support
- **v0.3.1:** Touch ID/Keychain + P&L engine
- **v0.4.0:** Binance Deep Integration (341 tests, code complete)

### ⚠️ Completed but Blocked
- **v0.5.0:** Cardano wallet tracking
  - **Blocker:** Task #16 - API key configuration missing
  - **Impact:** Users cannot test ANY Cardano features
  - **Code Status:** 100% complete, 6/7 BDD tests fixed

### 🧪 Test Suite Health
- **Unit Tests:** 203 tests, 100% passing ✅
- **Integration Tests:** 138 tests, 100% passing ✅
- **BDD Tests:** 31/37 scenarios passing (84%)
  - 5 skipped (expected - requires manual setup)
  - 1 failing (Bech32 checksum validation)

### 🚨 Critical Gaps
1. **API Key Configuration** - Blocks Cardano validation
2. **Manual Validation** - v0.4.0 and v0.5.0 untested with real APIs
3. **Test Coverage** - 1 BDD test still failing

---

## 📋 Tier 1: Critical Blockers (Do These First)

### Task #16: Implement API Key Configuration
**Priority:** 🔴 CRITICAL
**Status:** Pending
**Estimated Effort:** 3-4 hours
**Blocks:** All Cardano features, v0.5.0 release

#### Why This is Priority #1
1. **100% blocking** - Cardano is completely unusable without it
2. **Infrastructure debt** - All Cardano code done but can't be tested
3. **Reusable** - Will support future API integrations (Etherscan, etc.)
4. **High leverage** - Unblocks multiple downstream tasks

#### Implementation Requirements

**Must Support:**
- ✅ Blockfrost API keys (Cardano)
- ✅ Config file storage (`~/.config/cryptofolio/config.toml`)
- ✅ Environment variable fallback
- ✅ Secure storage option (leverage existing keychain)

**File Structure:**
```toml
# ~/.config/cryptofolio/config.toml
[api_keys]
blockfrost_mainnet = "mainnet..."
blockfrost_preprod = "preprod..."
# Future: etherscan, polygonscan, etc.
```

**CLI Commands:**
```bash
# Set API key (interactive)
cryptofolio config set blockfrost-key

# Set API key (non-interactive)
cryptofolio config set blockfrost-key preprod123abc

# Set API key (secure storage)
cryptofolio config set blockfrost-key --secure

# Show configured keys (masked)
cryptofolio config list

# Environment variable fallback
export BLOCKFROST_API_KEY=preprod123abc
cryptofolio wallet sync 'Cardano Wallet'
```

**Integration Points:**
- `src/blockchain/cardano/client.rs` - BlockfrostClient::new() should read config
- `src/cli/commands/wallet.rs` - sync_cardano_wallet() should pass API key
- `tests/step_definitions/cardano_steps.rs` - Mock should work without API key

**Success Criteria:**
- [ ] User can set Blockfrost API key via CLI
- [ ] User can set Blockfrost API key via environment variable
- [ ] Config file is created and persisted correctly
- [ ] Cardano wallet sync works with configured API key
- [ ] BDD tests continue to pass with mocks (no API key needed)
- [ ] Documentation updated in `TESTNET_SETUP_GUIDE_CARDANO.md`

**Testing Checklist:**
```bash
# Test 1: Set via CLI
cryptofolio config set blockfrost-key preprod_test123

# Test 2: Verify storage
cat ~/.config/cryptofolio/config.toml

# Test 3: Sync with API key
cryptofolio wallet sync 'My Cardano Wallet'

# Test 4: Environment variable override
export BLOCKFROST_API_KEY=preprod_override123
cryptofolio wallet sync 'My Cardano Wallet'

# Test 5: BDD tests still work
cargo test --test bdd -- "Cardano"
```

**Dependencies:** None
**Blocked By:** None
**Blocks:** Task #5 (Cardano validation), v0.5.0 release

---

### Task: Fix Bitcoin Transaction Mock BDD Test
**Priority:** 🟡 HIGH
**Status:** Pending
**Estimated Effort:** 30 minutes
**Issue:** Test expects 7 transactions but gets 25 from real API

#### Problem
The Bitcoin "Import transaction history" BDD test is using a real Bitcoin address instead of the mock. This causes:
- Unpredictable test results (depends on blockchain state)
- Wrong transaction count (25 instead of 7)
- Slower test execution (network calls)

#### Root Cause
The `bitcoin_steps.rs` mock setup is correct, but the sync handler may not be using the mock URL correctly.

#### Solution
1. Verify mock URL is passed to Bitcoin sync in `common_steps.rs`
2. Ensure BlockstreamMock is properly initialized before sync
3. Check that test address matches mock address exactly

#### Files to Check
- `tests/step_definitions/common_steps.rs` - Sync handler for Bitcoin
- `tests/step_definitions/bitcoin_steps.rs` - Mock setup
- `tests/support/blockchain_mock.rs` - BlockstreamMock implementation

#### Success Criteria
- [ ] Bitcoin "Import transaction history" test passes
- [ ] Test shows exactly 7 transactions (5 incoming + 2 outgoing)
- [ ] Test completes in <1 second (using mock, not network)
- [ ] Mock URL is correctly passed to Bitcoin sync handler

---

## 📋 Tier 2: Validation & Confidence Building

### Task: Manual Validation of v0.4.0 (Binance)
**Priority:** 🟢 MEDIUM
**Status:** Pending
**Estimated Effort:** 1-2 hours
**Guide:** `VALIDATION_GUIDE_v0.4.0.md`

#### Objective
Verify that Binance Deep Integration works correctly with a real Binance account.

#### Prerequisites
- Binance account (testnet or real with small balances)
- API key and secret stored in keychain
- Test trading history available

#### Validation Steps (15-step checklist)
1. **Account Setup**
   - [ ] Add Binance account
   - [ ] Verify API key storage in keychain
   - [ ] Test account list display

2. **History Sync - Basic**
   - [ ] Sync single symbol (BTCUSDT)
   - [ ] Verify transaction import
   - [ ] Check transaction count accuracy

3. **History Sync - Advanced**
   - [ ] Sync multiple symbols
   - [ ] Test --full-history flag
   - [ ] Test --dry-run flag
   - [ ] Verify incremental sync (watermarks)

4. **Transaction Types**
   - [ ] Spot trades imported correctly
   - [ ] Deposits imported correctly
   - [ ] Withdrawals imported correctly
   - [ ] Fiat orders imported correctly
   - [ ] Internal transfers imported correctly

5. **Data Validation**
   - [ ] Compare transaction count with Binance UI
   - [ ] Verify transaction amounts
   - [ ] Check timestamps accuracy
   - [ ] Validate fees calculation

6. **Edge Cases**
   - [ ] Re-run sync (no duplicates)
   - [ ] Sync with no new data
   - [ ] Error handling (invalid symbol)
   - [ ] Rate limiting behavior

#### Success Criteria
- [ ] All 15 validation steps pass
- [ ] No crashes or panics
- [ ] Transaction data matches Binance UI
- [ ] Incremental sync works correctly
- [ ] No duplicate transactions created

#### Known Issues to Watch For
- Rate limiting (50k requests/day)
- Missing transaction types
- Timestamp timezone issues
- Duplicate detection failures

**After Completion:** Document findings in `VALIDATION_RESULTS_v0.4.0.md`

---

### Task #15: Validate Ethereum on Sepolia Testnet
**Priority:** 🟢 MEDIUM
**Status:** Pending
**Estimated Effort:** 1 hour

#### Objective
Verify Ethereum wallet tracking works correctly with Sepolia testnet.

#### Prerequisites
- Sepolia testnet wallet
- Sepolia ETH from faucet (https://sepoliafaucet.com/)
- Test ERC-20 tokens (optional)

#### Validation Steps
1. **Setup**
   - [ ] Get Sepolia ETH from faucet (0.5 ETH)
   - [ ] Add Sepolia wallet to cryptofolio
   - [ ] Verify [TESTNET] indicator shows

2. **Basic Sync**
   - [ ] Sync wallet balance
   - [ ] Verify ETH balance matches Etherscan
   - [ ] Check sync speed (<5 seconds)

3. **ERC-20 Tokens**
   - [ ] Request test USDC from faucet (if available)
   - [ ] Sync wallet with tokens
   - [ ] Verify token detection
   - [ ] Check token balances

4. **Transaction History**
   - [ ] Send test transaction
   - [ ] Import transaction history
   - [ ] Verify transaction appears
   - [ ] Check gas tracking

5. **JSON Output**
   - [ ] Test --json flag
   - [ ] Verify JSON structure
   - [ ] Check all fields present

#### Success Criteria
- [ ] Balance matches Sepolia Etherscan
- [ ] ERC-20 tokens detected automatically
- [ ] Transaction import works
- [ ] Gas fees tracked correctly
- [ ] No errors or warnings

#### Testnet Resources
- **Faucet:** https://sepoliafaucet.com/
- **Explorer:** https://sepolia.etherscan.io/
- **RPC:** https://rpc.sepolia.org/

**After Completion:** Document findings in `VALIDATION_RESULTS_ETHEREUM.md`

---

### Task: Manual Validation of v0.5.0 (Cardano)
**Priority:** 🟢 MEDIUM (After Task #16)
**Status:** Blocked by Task #16
**Estimated Effort:** 1-2 hours
**Guide:** `VALIDATION_CHECKLIST_CARDANO.md`

#### Objective
Verify Cardano wallet tracking works correctly with Preprod testnet.

#### Prerequisites
- Blockfrost API key (from Task #16)
- Preprod testnet wallet (Eternl/Nami/Flint)
- Preprod ADA from faucet (10,000 tADA)

#### Validation Steps (12 sections)
1. **API Key Setup**
   - [ ] Configure Blockfrost API key
   - [ ] Verify key storage
   - [ ] Test environment variable override

2. **Wallet Setup**
   - [ ] Add Preprod wallet
   - [ ] Verify [TESTNET] indicator
   - [ ] Test address validation

3. **ADA Balance**
   - [ ] Sync ADA balance
   - [ ] Verify balance matches CardanoScan
   - [ ] Check decimal formatting (X.X format)

4. **Native Tokens**
   - [ ] Acquire test native token (optional)
   - [ ] Sync wallet with tokens
   - [ ] Verify token detection
   - [ ] Check token metadata lookup

5. **Transaction History**
   - [ ] Import transaction history
   - [ ] Verify transaction count
   - [ ] Check fee calculation
   - [ ] Test multiple syncs

6. **Stake Delegation** (if implemented)
   - [ ] Check delegation status
   - [ ] Verify pool ticker shows

7. **Performance**
   - [ ] Measure sync time
   - [ ] Test with multiple addresses
   - [ ] Check API rate limiting

8. **Error Handling**
   - [ ] Test invalid API key
   - [ ] Test invalid address
   - [ ] Test network timeout

#### Success Criteria
- [ ] All 12 validation sections pass
- [ ] Balance matches CardanoScan
- [ ] Native tokens detected correctly
- [ ] Transaction import works
- [ ] No crashes or panics
- [ ] Sync time <5 seconds

#### Testnet Resources
- **API:** https://blockfrost.io/
- **Faucet:** https://docs.cardano.org/cardano-testnet/tools/faucet/
- **Explorer:** https://preprod.cardanoscan.io/
- **Wallet:** https://eternl.io/

**After Completion:** Document findings in `VALIDATION_RESULTS_CARDANO.md`

---

## 📋 Tier 3: Polish & Enhancement

### Task: Complete Bech32 Checksum Validation
**Priority:** 🔵 LOW
**Status:** Pending
**Estimated Effort:** 1-2 hours

#### Current State
- Basic validation works (prefix, length, characters)
- Bech32 checksum validation partially implemented but deactivated
- 1 BDD test failing: "Reject invalid Cardano address - bad checksum"

#### Goal
Implement proper Bech32 checksum verification for Cardano addresses using the `bech32` crate.

#### Research Needed
1. Cardano uses Bech32 (not Bech32m)
2. Understand bech32 v0.11 API for Cardano format
3. Handle both mainnet and testnet address formats
4. Distinguish between checksum errors and format errors

#### Implementation Steps
1. **Research bech32 crate usage**
   - Review bech32 v0.11 documentation
   - Study Cardano address encoding spec
   - Check if Cardano uses custom Bech32 variant

2. **Implement checksum validation**
   - Use `bech32::decode()` correctly for Cardano
   - Handle errors appropriately
   - Preserve backward compatibility with test addresses

3. **Update tests**
   - Use real valid Cardano addresses in unit tests
   - Or generate valid test addresses
   - Ensure BDD "bad checksum" test passes

4. **Documentation**
   - Add comments explaining Bech32 format
   - Document any Cardano-specific quirks

#### Success Criteria
- [ ] All unit tests pass with real addresses
- [ ] BDD "bad checksum" test passes
- [ ] Valid addresses accepted
- [ ] Invalid checksums rejected
- [ ] Test addresses work (if needed)

#### References
- Bech32 spec: https://github.com/bitcoin/bips/blob/master/bip-0173.mediawiki
- Cardano addresses: https://cips.cardano.org/cips/cip19/

---

### Task #14: Update and Maintain BDD Tests
**Priority:** 🔵 LOW
**Status:** Ongoing
**Estimated Effort:** 1-2 hours

#### Objectives
1. Document BDD test patterns
2. Clean up warnings
3. Add missing coverage
4. Improve test maintainability

#### Specific Tasks
1. **Documentation**
   - [ ] Document step definition patterns
   - [ ] Explain mock server setup
   - [ ] Add examples of common scenarios

2. **Code Cleanup**
   - [ ] Fix unused import warnings (4 occurrences)
   - [ ] Fix unused variable warnings (2 occurrences)
   - [ ] Remove unused mock methods or mark with #[allow(dead_code)]

3. **Test Coverage**
   - [ ] Verify all blockchain features have BDD tests
   - [ ] Add missing scenarios if needed
   - [ ] Test error paths

4. **Maintainability**
   - [ ] Refactor duplicate code
   - [ ] Improve mock helpers
   - [ ] Standardize test data

#### Files to Update
- `tests/support/blockchain_mock.rs` - Remove unused methods or mark allowed
- `tests/support/ethereum_mock.rs` - Fix unused imports
- `tests/support/cardano_mock.rs` - Fix unused imports
- `tests/step_definitions/` - Document patterns

#### Success Criteria
- [ ] Zero warnings in BDD test compilation
- [ ] All features have test coverage
- [ ] Tests are maintainable and documented
- [ ] No flaky tests

---

## 🚫 Deferred Tasks (Do NOT Start Yet)

These tasks are important but should wait until the foundation is stable:

### Task #8: Implement Solana Wallet Tracking
**Status:** 🔴 DEFERRED
**Rationale:** New blockchain integration should wait until Cardano is validated

### Task #10: Implement Portfolio Audit Command
**Status:** 🔴 DEFERRED
**Rationale:** New feature should wait until existing features are validated

### Task #11: Design v0.6.0 MCP Integration
**Status:** 🔴 DEFERRED
**Rationale:** Major architecture change should wait until v0.4.0 and v0.5.0 are stable

---

## 📅 Proposed Timeline

### Week 1: Foundation Stability (Critical Path)
```
Day 1-2: Task #16 - API Key Configuration
  ├─ Design config file format
  ├─ Implement CLI commands
  ├─ Integrate with Cardano client
  └─ Update BDD tests

Day 3: Fix Bitcoin Transaction Mock
  └─ Debug and fix mock URL passing

Day 4: Manual Validation v0.4.0 (Binance)
  └─ Follow 15-step checklist

Day 5: Manual Validation v0.5.0 (Cardano)
  └─ Follow 12-step checklist
```

### Week 2: Validation & Polish
```
Day 1: Task #15 - Ethereum Testnet Validation
  └─ Test on Sepolia with real faucet

Day 2-3: Bech32 Checksum Implementation
  ├─ Research bech32 crate
  ├─ Implement validation
  └─ Update tests

Day 4: BDD Test Maintenance
  ├─ Fix warnings
  └─ Document patterns

Day 5: Buffer for issues / Release prep
```

### Week 3+: New Features (With Confidence!)
```
Ready to start:
  ├─ Task #8: Solana wallet tracking
  ├─ Task #10: Portfolio audit command
  └─ Task #11: MCP integration design
```

---

## 📊 Success Metrics

### Code Quality
- ✅ All unit tests passing (203/203)
- ✅ All integration tests passing (138/138)
- ✅ All BDD tests passing (37/37) - Target: 100%
- ✅ Zero compiler errors
- ✅ Zero critical warnings

### Feature Completeness
- ✅ v0.4.0 manually validated
- ✅ v0.5.0 manually validated
- ✅ API key configuration working
- ✅ All blockchains testable by users

### User Experience
- ✅ Clear error messages
- ✅ Complete documentation
- ✅ Validation guides accurate
- ✅ No blockers for testing

### Stability
- ✅ No known crashes
- ✅ No data corruption issues
- ✅ Predictable behavior
- ✅ Graceful error handling

---

## 🚨 Risk Mitigation

### Risk: API Key Configuration Takes Longer Than Expected
**Impact:** Medium
**Mitigation:**
- Start with simplest implementation (config file only)
- Defer secure storage to Phase 2 if needed
- Environment variable is sufficient for testing

### Risk: Validation Reveals Critical Bugs
**Impact:** High
**Mitigation:**
- Fix bugs immediately as P0 priority
- Document all issues found
- Re-run validation after fixes
- Consider delaying new features if needed

### Risk: Bech32 Validation Proves Too Complex
**Impact:** Low
**Mitigation:**
- Keep current basic validation
- Mark as "Future Enhancement"
- Document limitation in CARDANO_VALIDATION_SUMMARY.md
- Most users won't notice (Blockfrost validates too)

### Risk: Test Suite Becomes Brittle
**Impact:** Medium
**Mitigation:**
- Invest time in Task #14 (maintenance)
- Document test patterns clearly
- Add retry logic for flaky tests
- Use consistent test data

---

## 📝 Documentation Requirements

Each completed task should produce:

1. **Code Changes**
   - Well-commented code
   - Updated README if needed
   - CHANGELOG.md entry

2. **Testing Evidence**
   - Test results (pass/fail)
   - Screenshots for manual validation
   - Performance metrics

3. **User Documentation**
   - Updated guides (if applicable)
   - New examples (if applicable)
   - Migration notes (if applicable)

4. **Validation Reports**
   - `VALIDATION_RESULTS_v0.4.0.md`
   - `VALIDATION_RESULTS_ETHEREUM.md`
   - `VALIDATION_RESULTS_CARDANO.md`

---

## ✅ Definition of Done

A task is considered **DONE** when:

1. ✅ **Code Complete**
   - All code written and reviewed
   - Compiles without errors
   - No critical warnings

2. ✅ **Tests Pass**
   - Unit tests pass
   - Integration tests pass
   - BDD tests pass (if applicable)

3. ✅ **Validated**
   - Manual testing completed
   - Edge cases tested
   - Performance acceptable

4. ✅ **Documented**
   - Code comments added
   - User guide updated
   - Validation report written

5. ✅ **Committed**
   - Changes committed to git
   - Branch merged to master
   - CHANGELOG.md updated

---

## 🎯 North Star Goal

**By the end of this plan:**
- ✅ v0.4.0 and v0.5.0 are fully validated and stable
- ✅ Users can test all features with real APIs
- ✅ Test suite is 100% passing
- ✅ Documentation is complete and accurate
- ✅ Foundation is solid for future expansion

**Then and only then**, we confidently add Solana, portfolio audit, and MCP integration.

---

## 📞 Questions or Issues?

If you encounter blockers or need clarification:
1. Document the issue in the task section above
2. Update the risk mitigation section
3. Adjust timeline if needed
4. Communicate changes to stakeholders

**Remember:** Stability first, features second. A solid foundation enables faster feature development later.

---

**Last Updated:** 2026-03-19
**Next Review:** After Week 1 (Foundation Stability)
