# Ethereum Wallet Tracking - Validation Checklist

**Version:** v0.5.0 (Ethereum Integration)
**Date:** 2026-03-22
**Tester:** _____________
**Network:** Sepolia Testnet

## Pre-Validation Setup

- [ ] Cryptofolio built and installed (`cargo build --release`)
- [ ] Ethereum Sepolia testnet wallet created (see `TESTNET_SETUP_GUIDE_ETHEREUM.md`)
- [ ] Testnet ETH received (minimum 0.1 SepoliaETH)
- [ ] Test database backed up (if using existing data)

**Test Wallet Address:** `0x_____________________________________`

## 1. Basic Wallet Management

### 1.1 Add Ethereum Wallet
```bash
cryptofolio wallet add 'ETH Test Wallet' \
  --blockchain ethereum \
  --address 0xYourAddressHere \
  --network testnet
```

**Expected:**
- [ ] Success message: `✓ Added wallet 'ETH Test Wallet' [TESTNET]`
- [ ] Shows address in output
- [ ] `[TESTNET]` indicator is visible

### 1.2 List Wallets
```bash
cryptofolio wallet list
```

**Expected:**
- [ ] Shows "ETH Test Wallet"
- [ ] Shows "ethereum" as blockchain
- [ ] Shows full address (42 characters starting with 0x)
- [ ] Displays properly formatted

### 1.3 Show Wallet Details
```bash
cryptofolio wallet show 'ETH Test Wallet'
```

**Expected:**
- [ ] Shows wallet name
- [ ] Shows blockchain type
- [ ] Shows address
- [ ] No errors

## 2. Address Validation

### 2.1 Reject Invalid Address - Wrong Length
```bash
cryptofolio wallet add 'Bad1' --blockchain ethereum --address 0x123
```

**Expected:**
- [ ] Command fails with error
- [ ] Error message: "Invalid Ethereum address"

### 2.2 Reject Invalid Address - No 0x Prefix
```bash
cryptofolio wallet add 'Bad2' --blockchain ethereum --address 742d35Cc6634C0532925a3b844Bc9e7595f0bEb0
```

**Expected:**
- [ ] Command fails with error
- [ ] Error message: "Invalid Ethereum address"

### 2.3 Reject Invalid Address - Bad Characters
```bash
cryptofolio wallet add 'Bad3' --blockchain ethereum --address 0xZZZZ35Cc6634C0532925a3b844Bc9e7595f0bEb0
```

**Expected:**
- [ ] Command fails with error
- [ ] Error message: "Invalid Ethereum address"

### 2.4 Reject Duplicate Address
```bash
# Try adding the same address again
cryptofolio wallet add 'Duplicate' --blockchain ethereum --address 0xYourAddressHere
```

**Expected:**
- [ ] Command fails with error
- [ ] Error message contains "already exists"

## 3. Balance Syncing

### 3.1 Sync ETH Balance
```bash
cryptofolio wallet sync 'ETH Test Wallet'
```

**Expected:**
- [ ] Command succeeds (exit code 0)
- [ ] Shows: `✓ Synced ETH balance: X.XXXX`
- [ ] Balance matches MetaMask/Etherscan
- [ ] Balance has 4 decimal places

**Actual Balance:** `_______` ETH
**Expected Balance:** `_______` ETH
**Match:** ☐ Yes ☐ No

### 3.2 Verify on Etherscan
1. Visit https://sepolia.etherscan.io/address/0xYourAddress
2. Compare balance

**Etherscan Balance:** `_______` ETH
**Cryptofolio Balance:** `_______` ETH
**Match:** ☐ Yes ☐ No

## 4. ERC-20 Token Detection

### 4.1 Wallet with No Tokens
If your wallet has no tokens yet, this should show:

```bash
cryptofolio wallet sync 'ETH Test Wallet'
```

**Expected:**
- [ ] Shows ETH balance
- [ ] Shows: `  No tokens found`

### 4.2 Get Test Tokens
Follow the guide to get testnet USDC, LINK, or DAI.

**Tokens Acquired:**
- [ ] USDC: `_______` amount
- [ ] LINK: `_______` amount
- [ ] DAI: `_______` amount
- [ ] Other: `_______` amount

### 4.3 Detect ERC-20 Tokens
```bash
cryptofolio wallet sync 'ETH Test Wallet'
```

**Expected:**
- [ ] Shows ETH balance
- [ ] Shows: `✓ Found X tokens` (where X = number of tokens you have)
- [ ] Lists each token with symbol and balance
- [ ] Token balances match MetaMask

**Detected Tokens:**
```
  ______: ______ (matches MetaMask: ☐ Yes ☐ No)
  ______: ______ (matches MetaMask: ☐ Yes ☐ No)
  ______: ______ (matches MetaMask: ☐ Yes ☐ No)
```

### 4.4 Token Decimals Handling
- [ ] USDC shows 2 decimal places (e.g., `100.00`)
- [ ] LINK shows 2 decimal places (e.g., `10.00`)
- [ ] DAI shows 2 decimal places (e.g., `50.00`)
- [ ] All balances are human-readable (not in wei)

## 5. Transaction History Import

### 5.1 Check Current Transaction Count
Visit Etherscan and note your transaction count: `_______` transactions

### 5.2 Import Transactions
```bash
cryptofolio wallet sync 'ETH Test Wallet' --import-history
```

**Expected:**
- [ ] Shows ETH balance
- [ ] Shows tokens (if any)
- [ ] Shows: `✓ Imported X transactions`
- [ ] Transaction count is reasonable

**Imported Transactions:** `_______`
**Etherscan Transactions:** `_______`
**Match:** ☐ Yes ☐ No (minor differences OK due to internal txs)

### 5.3 Re-Import (Idempotency Check)
Run the same command again:

```bash
cryptofolio wallet sync 'ETH Test Wallet' --import-history
```

**Expected:**
- [ ] Command succeeds
- [ ] Same transaction count (no duplicates created)

## 6. Multiple Syncs (Reliability)

Run sync 3 times in a row:

```bash
cryptofolio wallet sync 'ETH Test Wallet'
cryptofolio wallet sync 'ETH Test Wallet'
cryptofolio wallet sync 'ETH Test Wallet'
```

**Expected:**
- [ ] All three syncs succeed
- [ ] Same balance each time
- [ ] Same token count each time
- [ ] No errors or warnings

## 7. Edge Cases

### 7.1 Very Small Balance
If you can, send a very small amount to test precision:

**Test:** 0.00001 ETH

- [ ] Small balance displays correctly (not rounded to 0)
- [ ] At least 4-5 decimal places shown

### 7.2 Rate Limit Handling
Run rapid syncs to test rate limiting:

```bash
for i in {1..10}; do cryptofolio wallet sync 'ETH Test Wallet'; done
```

**Expected:**
- [ ] Either all succeed, or some fail gracefully
- [ ] Error message is clear if rate limited
- [ ] No crashes or panics

## 8. JSON Output

### 8.1 Sync with JSON Output
```bash
cryptofolio wallet sync 'ETH Test Wallet' --json
```

**Expected:**
- [ ] Valid JSON output
- [ ] Contains wallet name
- [ ] Contains address
- [ ] Contains balance
- [ ] Contains token count

### 8.2 Parse JSON
Try parsing the output:

```bash
cryptofolio wallet sync 'ETH Test Wallet' --json | jq '.'
```

**Expected:**
- [ ] Valid JSON (jq doesn't error)
- [ ] Data is structured correctly

## 9. Mainnet Warning (Optional)

### 9.1 Add Mainnet Wallet (DO NOT SYNC)
```bash
cryptofolio wallet add 'Mainnet Test' \
  --blockchain ethereum \
  --address 0xYourMainnetAddress
```

**Expected:**
- [ ] Adds successfully
- [ ] No `[TESTNET]` indicator
- [ ] Could sync against mainnet Etherscan (don't actually do it unless you want to)

**Note:** Skip actual mainnet sync unless you have a real wallet you want to test with.

## 10. Cleanup

### 10.1 Remove Test Wallet
```bash
cryptofolio wallet remove 'ETH Test Wallet' --yes
```

**Expected:**
- [ ] Wallet removed successfully
- [ ] Confirmation message shown

### 10.2 Verify Removal
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

## Issues Found

List any bugs, unexpected behavior, or UX issues:

```
1. _______________________________________________________________
2. _______________________________________________________________
3. _______________________________________________________________
```

## Overall Assessment

**Ethereum Wallet Tracking:**
- ☐ ✅ Ready for release
- ☐ ⚠️ Ready with minor issues (document above)
- ☐ ❌ Not ready (blocking issues found)

**Tester Notes:**
```
_____________________________________________________________________
_____________________________________________________________________
_____________________________________________________________________
```

**Tester Signature:** _______________ **Date:** _______________

---

## Return Testnet Funds (Optional)

If you're done testing, consider returning testnet ETH to help others:

```bash
# Send remaining SepoliaETH to the zero address
# (Do this from MetaMask or another wallet, not Cryptofolio)
# To: 0x0000000000000000000000000000000000000000
```

Or keep it for future testing! 🚀
