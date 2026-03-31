# Cardano Wallet Tracking - Validation Checklist

**Version:** v0.5.0 (Cardano Integration)
**Date:** 2026-03-22
**Tester:** _____________
**Network:** Preprod Testnet

## Pre-Validation Setup

- [ ] Cryptofolio built and installed (`cargo build --release`)
- [ ] Blockfrost account created and API key obtained
- [ ] Cardano Preprod testnet wallet created (see `TESTNET_SETUP_GUIDE_CARDANO.md`)
- [ ] Testnet ADA received (minimum 1000 tADA)
- [ ] Test database backed up (if using existing data)

**Test Wallet Address:** `addr_test1_____________________________________`
**Blockfrost Project ID:** `preprod________________________`

## 1. Basic Wallet Management

### 1.1 Add Cardano Wallet
```bash
cryptofolio wallet add 'ADA Test Wallet' \
  --blockchain cardano \
  --address addr_test1_your_address_here \
  --network testnet
```

**Expected:**
- [ ] Success message: `✓ Added wallet 'ADA Test Wallet' [TESTNET]`
- [ ] Shows address in output
- [ ] `[TESTNET]` indicator is visible

### 1.2 List Wallets
```bash
cryptofolio wallet list
```

**Expected:**
- [ ] Shows "ADA Test Wallet"
- [ ] Shows "cardano" as blockchain
- [ ] Shows full address (100+ characters starting with addr_test1)
- [ ] Displays properly formatted

### 1.3 Show Wallet Details
```bash
cryptofolio wallet show 'ADA Test Wallet'
```

**Expected:**
- [ ] Shows wallet name
- [ ] Shows blockchain type
- [ ] Shows address
- [ ] No errors

## 2. Address Validation

### 2.1 Reject Invalid Address - Wrong Prefix
```bash
cryptofolio wallet add 'Bad1' --blockchain cardano --address 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0
```

**Expected:**
- [ ] Command fails with error
- [ ] Error message: "Invalid Cardano address"

### 2.2 Reject Invalid Address - Wrong Length
```bash
cryptofolio wallet add 'Bad2' --blockchain cardano --address addr_test1qxy2
```

**Expected:**
- [ ] Command fails with error
- [ ] Error message: "Invalid Cardano address"

### 2.3 Reject Invalid Address - Uppercase Characters
```bash
cryptofolio wallet add 'Bad3' --blockchain cardano --address ADDR_TEST1QXY2KGDYGJRSQTZQ2N0YRF2493P83KKFJHX0WLH
```

**Expected:**
- [ ] Command fails with error
- [ ] Error message: "Invalid Cardano address"

### 2.4 Reject Duplicate Address
```bash
# Try adding the same address again
cryptofolio wallet add 'Duplicate' --blockchain cardano --address addr_test1_your_address_here
```

**Expected:**
- [ ] Command fails with error
- [ ] Error message contains "already exists"

## 3. Balance Syncing (Requires API Key)

⚠️ **Note:** These tests require Blockfrost API key configuration (feature in development).

### 3.1 Sync ADA Balance
```bash
cryptofolio wallet sync 'ADA Test Wallet'
```

**Expected:**
- [ ] Command succeeds (exit code 0) OR
- [ ] Shows appropriate error if API key not configured
- [ ] If successful: Shows `✓ Synced ADA balance: X.XXXXXX`
- [ ] Balance matches wallet/CardanoScan

**Actual Balance:** `_______` ADA
**Expected Balance:** `_______` ADA
**Match:** ☐ Yes ☐ No ☐ N/A (API key not configured)

### 3.2 Verify on CardanoScan
1. Visit https://preprod.cardanoscan.io/address/YOUR_ADDRESS
2. Compare balance

**CardanoScan Balance:** `_______` ADA
**Cryptofolio Balance:** `_______` ADA
**Match:** ☐ Yes ☐ No ☐ N/A

## 4. Native Token Detection

### 4.1 Wallet with No Tokens
If your wallet has no tokens yet:

```bash
cryptofolio wallet sync 'ADA Test Wallet'
```

**Expected:**
- [ ] Shows ADA balance
- [ ] Shows: `  No tokens found`

### 4.2 Get Test Tokens
Follow the guide to get testnet HOSKY, MIN, or SUNDAE tokens.

**Tokens Acquired:**
- [ ] HOSKY: `_______` amount
- [ ] MIN: `_______` amount
- [ ] SUNDAE: `_______` amount
- [ ] Other: `_______` amount

### 4.3 Detect Native Tokens
```bash
cryptofolio wallet sync 'ADA Test Wallet'
```

**Expected:**
- [ ] Shows ADA balance
- [ ] Shows: `✓ Found X tokens` (where X = number of tokens you have)
- [ ] Lists each token with name and balance
- [ ] Token balances match wallet

**Detected Tokens:**
```
  ______: ______ (matches wallet: ☐ Yes ☐ No)
  ______: ______ (matches wallet: ☐ Yes ☐ No)
  ______: ______ (matches wallet: ☐ Yes ☐ No)
```

### 4.4 Token Decimals Handling
- [ ] Tokens display correct decimal places
- [ ] HOSKY shows 0 decimals (whole numbers)
- [ ] Other tokens show appropriate decimals
- [ ] All balances are human-readable

## 5. Transaction History Import

### 5.1 Check Current Transaction Count
Visit CardanoScan and note your transaction count: `_______` transactions

### 5.2 Import Transactions
```bash
cryptofolio wallet sync 'ADA Test Wallet' --import-history
```

**Expected:**
- [ ] Shows ADA balance
- [ ] Shows tokens (if any)
- [ ] Shows: `✓ Imported X transactions`
- [ ] Transaction count is reasonable

**Imported Transactions:** `_______`
**CardanoScan Transactions:** `_______`
**Match:** ☐ Yes ☐ No ☐ N/A

### 5.3 Re-Import (Idempotency Check)
Run the same command again:

```bash
cryptofolio wallet sync 'ADA Test Wallet' --import-history
```

**Expected:**
- [ ] Command succeeds
- [ ] Same transaction count (no duplicates created)

## 6. Multiple Syncs (Reliability)

Run sync 3 times in a row:

```bash
cryptofolio wallet sync 'ADA Test Wallet'
cryptofolio wallet sync 'ADA Test Wallet'
cryptofolio wallet sync 'ADA Test Wallet'
```

**Expected:**
- [ ] All three syncs succeed
- [ ] Same balance each time
- [ ] Same token count each time
- [ ] No errors or warnings

## 7. Blockfrost API Integration

### 7.1 API Key Validation
Test with invalid API key (if feature implemented):

**Expected:**
- [ ] Clear error message about invalid API key
- [ ] No crash or panic
- [ ] Helpful message to check API key

### 7.2 Rate Limit Handling
Run rapid syncs to test rate limiting:

```bash
for i in {1..10}; do cryptofolio wallet sync 'ADA Test Wallet'; done
```

**Expected:**
- [ ] Either all succeed, or some fail gracefully
- [ ] Error message is clear if rate limited
- [ ] No crashes or panics

### 7.3 Network Error Handling
Test with no internet connection (optional):

**Expected:**
- [ ] Clear error message about network connectivity
- [ ] No crash or panic
- [ ] Helpful troubleshooting message

## 8. Edge Cases

### 8.1 Very Small Balance
If you can, test with a very small amount:

**Test:** 1.000001 ADA

- [ ] Small balance displays correctly (not rounded to 0)
- [ ] At least 6 decimal places shown (Cardano uses lovelaces: 1 ADA = 1,000,000 lovelace)

### 8.2 Large Token Quantities
If you have large token amounts (like HOSKY with millions):

**Test:** 1,000,000+ HOSKY

- [ ] Large numbers display correctly
- [ ] No overflow or scientific notation issues
- [ ] Proper comma/decimal formatting

### 8.3 Address with No Activity
Test with a brand new, unused address:

**Expected:**
- [ ] Either shows 0 balance, or
- [ ] Clear error that address hasn't been used yet

## 9. Staking Information (If Implemented)

### 9.1 Delegated Wallet
If your wallet is delegated to a stake pool:

```bash
cryptofolio wallet sync 'Staked Wallet'
```

**Expected:**
- [ ] Shows ADA balance
- [ ] Shows: `  Delegated to: [POOL_TICKER]`
- [ ] Pool ticker is correct

**Pool Ticker:** `_______`
**Matches Wallet:** ☐ Yes ☐ No ☐ N/A (not delegated)

### 9.2 Non-Delegated Wallet
If wallet is not delegated:

**Expected:**
- [ ] Shows ADA balance
- [ ] No delegation info shown OR
- [ ] Shows "Not delegated" message

## 10. JSON Output

### 10.1 Sync with JSON Output
```bash
cryptofolio wallet sync 'ADA Test Wallet' --json
```

**Expected:**
- [ ] Valid JSON output
- [ ] Contains wallet name
- [ ] Contains address
- [ ] Contains balance
- [ ] Contains token count

### 10.2 Parse JSON
Try parsing the output:

```bash
cryptofolio wallet sync 'ADA Test Wallet' --json | jq '.'
```

**Expected:**
- [ ] Valid JSON (jq doesn't error)
- [ ] Data is structured correctly

## 11. Mainnet vs Testnet

### 11.1 Testnet Address Detection
Verify testnet addresses are auto-detected:

```bash
cryptofolio wallet add 'Auto Testnet' \
  --blockchain cardano \
  --address addr_test1...
```

**Expected:**
- [ ] Automatically recognizes as testnet
- [ ] Shows [TESTNET] indicator
- [ ] No need to specify --network testnet flag

### 11.2 Mainnet Address (Optional - DO NOT SYNC)
Try adding a mainnet address:

```bash
cryptofolio wallet add 'Mainnet Test' \
  --blockchain cardano \
  --address addr1...  # Your real mainnet address if you have one
```

**Expected:**
- [ ] Adds successfully
- [ ] No `[TESTNET]` indicator
- [ ] Could sync against mainnet (don't actually do it unless you want to)

## 12. Cleanup

### 12.1 Remove Test Wallet
```bash
cryptofolio wallet remove 'ADA Test Wallet' --yes
```

**Expected:**
- [ ] Wallet removed successfully
- [ ] Confirmation message shown

### 12.2 Verify Removal
```bash
cryptofolio wallet list
```

**Expected:**
- [ ] Test wallet no longer in list

## Performance Metrics

**Sync Speed:**
- First sync: `_______` seconds
- Subsequent sync: `_______` seconds
- With --import-history: `_______` seconds

**API Behavior:**
- Rate limit encountered: ☐ Yes ☐ No
- Any timeouts: ☐ Yes ☐ No
- Any failed requests: ☐ Yes ☐ No
- API key worked correctly: ☐ Yes ☐ No ☐ N/A

## Blockfrost Integration

**API Key Configuration:**
- [ ] API key stored securely
- [ ] Key not exposed in logs/errors
- [ ] Key easy to update/change

**API Usage:**
- Estimated requests used: `_______`
- Free tier sufficient: ☐ Yes ☐ No
- Dashboard accessible: ☐ Yes ☐ No

## Issues Found

List any bugs, unexpected behavior, or UX issues:

```
1. _______________________________________________________________
2. _______________________________________________________________
3. _______________________________________________________________
```

## Comparison with Other Blockchains

| Feature | Bitcoin | Ethereum | Cardano |
|---------|---------|----------|---------|
| Address validation | ☐ Pass ☐ Fail | ☐ Pass ☐ Fail | ☐ Pass ☐ Fail |
| Balance sync | ☐ Pass ☐ Fail | ☐ Pass ☐ Fail | ☐ Pass ☐ Fail |
| Token detection | N/A | ☐ Pass ☐ Fail | ☐ Pass ☐ Fail |
| Transaction import | ☐ Pass ☐ Fail | ☐ Pass ☐ Fail | ☐ Pass ☐ Fail |
| API key required | No | No | Yes |
| Testnet support | ☐ Pass ☐ Fail | ☐ Pass ☐ Fail | ☐ Pass ☐ Fail |

## Overall Assessment

**Cardano Wallet Tracking:**
- ☐ ✅ Ready for release
- ☐ ⚠️ Ready with minor issues (document above)
- ☐ ❌ Not ready (blocking issues found)
- ☐ ⏸️ Cannot fully test (API key feature not implemented)

**Tester Notes:**
```
_____________________________________________________________________
_____________________________________________________________________
_____________________________________________________________________
```

**Tester Signature:** _______________ **Date:** _______________

---

## API Key Configuration (For Future Implementation)

**Recommended approaches:**

1. **Environment Variable:**
```bash
export BLOCKFROST_API_KEY=preprod_your_key_here
cryptofolio wallet sync 'ADA Wallet'
```

2. **Config File:**
```toml
# ~/.config/cryptofolio/config.toml
[cardano]
blockfrost_api_key = "preprod_your_key_here"
```

3. **Interactive Prompt:**
```bash
cryptofolio config set blockfrost-key
# Prompts for key input (hidden)
```

## Return Testnet Funds (Optional)

When you're done testing, you can:
- Keep testnet ADA for future testing
- Send to another developer's testnet address
- Testnet funds have no value, so no need to "return" them

---

## Next Steps

After validation:
1. Document which features work
2. Note any API key configuration blockers
3. Test with real mainnet wallet (optional)
4. Provide feedback for improvements

🚀 **Happy Testing!**
