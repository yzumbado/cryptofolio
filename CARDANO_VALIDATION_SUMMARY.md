# Cardano Integration - Ready for Validation

**Version:** v0.5.0
**Date:** 2026-03-22
**Status:** ✅ Code Complete - Ready for Manual Testing

## What's Ready

### ✅ Implementation Complete
- **BlockfrostClient** - Full Blockfrost API integration (437 lines)
- **Native Token Support** - Automatic detection and metadata lookup
- **Wallet Sync** - ADA balance + tokens + transaction history
- **Testnet Support** - Preprod and Preview testnets fully supported
- **BDD Tests** - 13 scenarios created (ready to run)

### ✅ Documentation Complete
- **Setup Guide:** `TESTNET_SETUP_GUIDE_CARDANO.md` (284 lines)
- **Validation Checklist:** `VALIDATION_CHECKLIST_CARDANO.md` (547 lines)
- **Feature Documentation:** Complete with examples

## Key Differences from Bitcoin/Ethereum

| Aspect | Bitcoin | Ethereum | Cardano |
|--------|---------|----------|---------|
| **API Provider** | Blockstream (public) | Etherscan (public) | Blockfrost (API key required) |
| **API Key** | Not required | Not required | **Required** (free tier available) |
| **Testnet** | Bitcoin Testnet | Sepolia | Preprod / Preview |
| **Tokens** | None | ERC-20 (smart contract) | Native tokens (protocol-level) |
| **Decimals** | Always 8 | Variable (6-18) | Variable (0-18) |
| **Address Format** | bc1.../tb1... | 0x... (42 chars) | addr1.../addr_test1... (100+ chars) |
| **Faucets** | Limited | Many options | Multiple reliable faucets |
| **Setup Time** | 10-60 min | 10-60 min | 15-70 min (includes API key) |
| **Unique Features** | HD wallets | Gas tracking | Staking delegation |

## Validation Process

### Step 1: Get Blockfrost API Key (5 minutes)
Follow `TESTNET_SETUP_GUIDE_CARDANO.md` Section 1:

1. Sign up at https://blockfrost.io/ (free)
2. Create "Cardano Preprod" project
3. Copy API key (format: `preprod...`)
4. **Configure in Cryptofolio:** ✅ Feature now available!
   - Environment variable: `export BLOCKFROST_API_KEY=preprod...`
   - Config file: `cryptofolio config set blockfrost.preprod_api_key preprod...`
   - Secure keychain (macOS): `cryptofolio config set-secret blockfrost.preprod_api_key`

### Step 2: Setup Preprod Wallet (10-15 minutes)
Follow `TESTNET_SETUP_GUIDE_CARDANO.md` Section 2:

1. **Install Eternl Wallet** (recommended) or Nami/Flint/Yoroi
2. **Create testnet wallet** and save 24-word phrase
3. **Switch to Preprod network** in wallet settings
4. **Copy address** (starts with `addr_test1`)

### Step 3: Get Testnet ADA (1-60 minutes)
Follow `TESTNET_SETUP_GUIDE_CARDANO.md` Section 3:

**Working Faucets:**
- https://docs.cardano.org/cardano-testnet/tools/faucet/ (10,000 tADA)
- https://testnets.cardano.org/en/testnets/cardano/tools/faucet/
- https://faucet.gimbalabs.com/ (requires Discord)
- Blockfrost faucet (requires Blockfrost account)

### Step 4: Validation (30-45 minutes)
Follow `VALIDATION_CHECKLIST_CARDANO.md`:

- [ ] Add wallet and verify `[TESTNET]` indicator
- [ ] Test address validation (reject invalid formats)
- [ ] Sync ADA balance (requires API key feature)
- [ ] Test native token detection (optional)
- [ ] Import transaction history
- [ ] Test multiple syncs (reliability)
- [ ] Test JSON output
- [ ] Performance metrics

### Step 5: Document Results
Fill out the validation checklist and report:
- ✅ Issues found (if any)
- ✅ Performance metrics
- ✅ Overall assessment
- ⚠️ Note API key configuration status

## Implementation Details

### Address Validation
```rust
// Validates Cardano addresses
// - Mainnet: addr1..., stake1...
// - Testnet: addr_test1..., stake_test1...
// - Length: 100+ characters
// - Format: Bech32 encoding
```

### Blockfrost Client
```rust
pub struct BlockfrostClient {
    base_url: String,        // Preprod or mainnet
    api_key: Option<String>, // Required for real usage
}

// Key methods:
// - get_address_info() - Balance + tokens + stake info
// - get_native_tokens() - Fetch and parse native tokens with metadata
// - get_transactions() - Transaction history
// - get_token_metadata() - Token name and decimals
```

### Native Token Support
- Automatic token discovery from address
- Metadata lookup (name, decimals)
- Balance calculation (quantity / 10^decimals)
- Sorted by display name

### Lovelace Conversion
```
1 ADA = 1,000,000 lovelaces
Balance in ADA = lovelace_amount / 1,000,000
```

## Test Commands

```bash
# 1. Add Preprod wallet
cryptofolio wallet add 'Cardano Test' \
  --blockchain cardano \
  --address addr_test1_your_address \
  --network testnet

# 2. Sync balance (requires API key configuration)
cryptofolio wallet sync 'Cardano Test'

# 3. Import history
cryptofolio wallet sync 'Cardano Test' --import-history

# 4. List wallets
cryptofolio wallet list

# 5. JSON output
cryptofolio wallet sync 'Cardano Test' --json
```

## Expected Outputs

### With ADA Only
```
✓ Synced ADA balance: 10000.000000
  No tokens found
```

### With ADA + Native Tokens
```
✓ Synced ADA balance: 9500.000000
✓ Found 3 tokens
  HOSKY: 1000000.00
  MIN: 50.00
  SUNDAE: 100.00
```

### With Staking (If Implemented)
```
✓ Synced ADA balance: 9500.000000
✓ Found 3 tokens
  HOSKY: 1000000.00
  MIN: 50.00
  SUNDAE: 100.00
  Delegated to: BLOOM
```

### With Transaction Import
```
✓ Synced ADA balance: 9500.000000
✓ Found 2 tokens
  MIN: 50.00
  SUNDAE: 100.00
✓ Imported 12 transactions
```

## Current Limitations

### ✅ API Key Configuration - IMPLEMENTED!
**Status:** ✅ Feature complete and available (Task #16)

**Available Methods:**
1. Environment variable: `export BLOCKFROST_API_KEY=preprod...`
2. Config file: `cryptofolio config set blockfrost.preprod_api_key preprod...`
3. Secure keychain (macOS): `cryptofolio config set-secret blockfrost.preprod_api_key`

**Impact:**
- ✅ Can test with real Blockfrost API
- ✅ BDD tests use mock servers (still work)
- ✅ Manual testing fully supported
- ✅ Ready for v0.5.0 validation

### ⏳ Stake Pool Delegation
**Status:** Data structures ready, API calls partially implemented.

**What Works:**
- Stake pool info structure defined
- Client method signatures exist

**What's Pending:**
- Full stake address lookup
- Pool delegation details
- Rewards information

### ✅ Transaction Import
**Status:** Implemented and ready.

**Features:**
- Fetches transaction history
- Parses fees and deposits
- Converts lovelace to ADA
- Ready to save to database

## Troubleshooting Reference

See `TESTNET_SETUP_GUIDE_CARDANO.md` for:
- ❌ "Invalid Cardano address" - Address format issues
- ❌ "Blockfrost API error: 403" - Invalid API key
- ❌ "Blockfrost API error: 404" - Address not found
- ❌ Balance shows 0 - API cache or timing
- ❌ Tokens not showing - Need token transactions
- ❌ Faucets not working - Try different faucet

## Success Criteria

**Fully Ready for Release:**
- [ ] API key configuration implemented
- [ ] All validation checklist items pass
- [ ] ADA balance matches CardanoScan
- [ ] Native tokens detected correctly
- [ ] Transaction import works
- [ ] No crashes or panics
- [ ] Reasonable sync speed (<5 seconds)

**Ready with API Key Feature Pending:**
- [ ] Code compiles without errors
- [ ] BDD tests pass (with mocks)
- [ ] Address validation works
- [ ] Wallet add/remove works
- [ ] JSON output format correct
- [ ] Documentation complete

## Next Steps After Validation

1. **If API key feature is available:**
   - Complete full manual validation
   - Test with real Preprod wallet
   - Verify all features work
   - Mark as fully validated

2. **If API key feature pending:**
   - Run BDD tests with mocks
   - Validate code quality
   - Test address validation only
   - Wait for API key feature before full validation

3. **After validation passes:**
   - Update CHANGELOG.md with v0.5.0
   - Consider release candidate
   - Move to Task #8 (Solana) or complete API key feature

## BDD Test Status

**Created:** 13 scenarios in `cardano_wallet.feature`
**Status:** Ready to run

**Scenarios:**
1. ✅ Add Cardano wallet with valid address
2. ✅ Reject invalid address - wrong prefix
3. ✅ Reject invalid address - wrong length
4. ✅ Reject invalid address - bad checksum
5. ⏳ Sync Cardano wallet from blockchain (requires mock)
6. ⏳ Detect native tokens automatically (requires mock)
7. ⏳ Sync wallet with no tokens (requires mock)
8. ⏳ Import transaction history (requires mock)
9. ✅ List wallets shows Cardano addresses
10. ✅ Cannot add duplicate Cardano address
11. ⏳ Sync wallet shows native token balances (requires mock)
12. ✅ Track testnet Cardano wallet
13. ⏳ Sync shows stake pool information (requires mock)

**Run Tests:**
```bash
cargo test --test bdd
```

## Resources

- **Blockfrost:** https://blockfrost.io/ (API provider)
- **Cardano Testnet:** https://docs.cardano.org/cardano-testnet/
- **Preprod Explorer:** https://preprod.cardanoscan.io/
- **Official Faucet:** https://docs.cardano.org/cardano-testnet/tools/faucet/
- **Eternl Wallet:** https://eternl.io/
- **Nami Wallet:** https://namiwallet.io/

---

**Ready to validate?** Start with `TESTNET_SETUP_GUIDE_CARDANO.md` 🚀

**Note:** For full validation, API key configuration feature needs to be implemented first. BDD tests with mocks can be run immediately.
