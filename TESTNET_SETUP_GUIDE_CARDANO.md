# Cardano Preprod Testnet Setup Guide

This guide provides step-by-step instructions for testing Cryptofolio's Cardano wallet tracking features using the Preprod testnet.

## Prerequisites

- Cryptofolio installed and configured
- Internet connection
- Email address (for Blockfrost API key)
- Basic understanding of Cardano addresses

## Important: Blockfrost API Key Required

**Unlike Bitcoin and Ethereum, Cardano requires an API key for blockchain access.**

Blockfrost provides:
- ✅ Free tier: 50,000 requests/day
- ✅ Testnet access included
- ✅ No credit card required
- ⏱️ Setup time: 2-3 minutes

## Step 1: Get a Blockfrost API Key

### 1.1 Sign Up for Blockfrost

1. Visit https://blockfrost.io/
2. Click "Sign Up" (top right)
3. Enter your email address
4. Verify email (check spam folder)
5. Complete registration

### 1.2 Create a Project

1. Log in to Blockfrost dashboard
2. Click "Add Project"
3. Select **"Cardano Preprod"** network (NOT mainnet)
4. Give it a name: "Cryptofolio Testing"
5. Click "Create"

### 1.3 Copy Your API Key

1. Click on your project
2. Find "Project ID" field
3. Click to copy (format: `preprodXXXXXXXXXXXXXXXXXXXXX`)
4. **Save this key** - you'll need it later

### 1.4 Configure Cryptofolio with Your API Key ✅

You now have **three ways** to provide your Blockfrost API key:

#### Option A: Environment Variable (Simplest)
```bash
export BLOCKFROST_API_KEY=preprod...YOUR_KEY_HERE...
cryptofolio wallet sync 'Cardano Wallet'
```

**Pros:** Quick for testing, no permanent storage
**Cons:** Needs to be set each session

#### Option B: Config File (Persistent)
```bash
# Set for Preprod testnet
cryptofolio config set blockfrost.preprod_api_key preprod...YOUR_KEY...

# Or for mainnet (when ready)
cryptofolio config set blockfrost.mainnet_api_key mainnet...YOUR_KEY...

# View configured keys
cryptofolio config show
```

**Pros:** Persists across sessions
**Cons:** Stored in plaintext in `~/.config/cryptofolio/config.toml`

#### Option C: Secure Keychain (macOS Only - Recommended)
```bash
# Interactive prompt (most secure)
cryptofolio config set-secret blockfrost.preprod_api_key

# Or pipe from secure source
echo "preprod...YOUR_KEY..." | cryptofolio config set-secret blockfrost.preprod_api_key
```

**Pros:** Encrypted by macOS, supports Touch ID
**Cons:** macOS only

**⚠️ Security Note:** Blockfrost keys are READ-ONLY by default (safe to store).

## Step 2: Get a Cardano Testnet Wallet

### Option A: Eternl Wallet (Recommended - Most Feature-Rich)

1. **Install Eternl:**
   - Visit https://eternl.io/
   - Install browser extension (Chrome, Firefox, Brave, Edge)
   - Or download mobile app (iOS/Android)
   - Or use web version (https://eternl.io/app/mainnet/welcome)

2. **Create Testnet Wallet:**
   - Open Eternl
   - Go to Settings → Network → Select "Preprod Testnet"
   - Click "Create Wallet"
   - Choose "Create new wallet"
   - **Save your 24-word recovery phrase** (offline, secure location)
   - Set a spending password
   - Wallet created!

3. **Copy Your Address:**
   - Go to "Receive" tab
   - Click to copy address
   - Format: `addr_test1qq...` (starts with `addr_test1`)

### Option B: Nami Wallet (Simplest)

1. Visit https://namiwallet.io/
2. Install browser extension
3. Create new wallet
4. In settings, switch to "Preprod Testnet"
5. Copy receive address

### Option C: Flint Wallet

1. Visit https://flint-wallet.com/
2. Install browser extension
3. Create new wallet
4. Switch to testnet in settings
5. Copy receive address

### Option D: Yoroi (By Emurgo)

1. Visit https://yoroi-wallet.com/
2. Install browser extension or mobile app
3. Create new wallet (Cardano)
4. During setup, select "Preprod Testnet"
5. Copy receive address

### Option E: Daedalus Testnet (Full Node - Advanced)

⚠️ **Only for advanced users** - requires ~30GB disk space and full blockchain sync.

1. Visit https://daedaluswallet.io/
2. Download "Daedalus Testnet" version
3. Install and wait for blockchain sync (several hours)
4. Create wallet
5. Copy receive address

## Step 3: Get Testnet ADA

**IMPORTANT:** Getting testnet ADA can take 1-60 minutes depending on faucet availability. Be patient!

### Working Faucets (2026)

Try these faucets in order until you receive funds:

#### 1. Cardano Testnet Faucet (Recommended)
- **URL:** https://docs.cardano.org/cardano-testnet/tools/faucet/
- **Requirements:** None
- **Amount:** 10,000 tADA
- **Notes:** Official faucet, most reliable

#### 2. Testnets.cardano.org Faucet
- **URL:** https://testnets.cardano.org/en/testnets/cardano/tools/faucet/
- **Requirements:** None
- **Amount:** 10,000 tADA
- **Notes:** Same as above, alternative URL

#### 3. Gimbalabs Faucet
- **URL:** https://faucet.gimbalabs.com/
- **Requirements:** Discord verification
- **Amount:** Variable (usually 1000 tADA)
- **Notes:** Community-run, very reliable

#### 4. Blockfrost Faucet
- **URL:** https://docs.blockfrost.io/#section/Faucet
- **Requirements:** Blockfrost account
- **Amount:** 1000 tADA
- **Notes:** Can use your existing Blockfrost account

### Tips for Getting Testnet ADA

- **Multiple faucets:** You can use multiple faucets to get more tADA
- **Wait time:** Usually instant, but can take up to 5 minutes
- **Rate limits:** Most faucets limit 1 request per address per 24 hours
- **Check balance:** Use your wallet or Cardano Explorer to verify receipt

## Step 4: Get Test Native Tokens (Optional)

Cardano native tokens are different from Ethereum ERC-20s:
- No smart contracts needed
- Built into the protocol
- Any address can hold any token

### Option A: Testnet Token Faucets

1. **HOSKY Token (Popular meme token):**
   - Testnet versions may be available from community faucets
   - Check Cardano Discord channels

2. **MIN Token (Minswap DEX):**
   - Testnet versions on Preprod
   - Use Minswap testnet DEX to acquire

3. **SUNDAE Token (SundaeSwap DEX):**
   - Available on Preprod testnet
   - Use SundaeSwap testnet to acquire

### Option B: Use Testnet DEX

1. **Minswap Preprod:**
   - Visit https://preprod.minswap.org/ (if available)
   - Connect your testnet wallet
   - Swap some tADA for test tokens

2. **SundaeSwap Testnet:**
   - Visit SundaeSwap testnet (if available)
   - Connect wallet
   - Swap for tokens

### Option C: Mint Your Own (Advanced)

If you're familiar with Cardano CLI or Plutus:
1. Use `cardano-cli` to mint native tokens on Preprod
2. Only requires tADA for transaction fees
3. Great for testing custom token scenarios

## Step 5: Add Wallet to Cryptofolio

```bash
# Add your Preprod testnet wallet
cryptofolio wallet add 'Cardano Test Wallet' \
  --blockchain cardano \
  --address addr_test1_your_address_here \
  --network testnet
```

Expected output:
```
✓ Added wallet 'Cardano Test Wallet' [TESTNET]
  Address: addr_test1_your_address_here
```

## Step 6: Verify Balance (With API Key)

**Note:** Currently requires API key configuration (feature in development).

```bash
# Sync your wallet to fetch current balance
cryptofolio wallet sync 'Cardano Test Wallet'
```

Expected output:
```
✓ Synced ADA balance: 10000.000000
  No tokens found
```

Or with tokens:
```
✓ Synced ADA balance: 9500.000000
✓ Found 2 tokens
  HOSKY: 1000000.00
  MIN: 50.00
```

## Step 7: Explore Preprod Blockchain

### Cardano Explorer (Preprod)

**CardanoScan Preprod:**
- URL: https://preprod.cardanoscan.io/
- Search your address
- View transactions, tokens, staking

**Blockfrost Explorer:**
- URL: https://preprod.cardanoscan.io/ or use Blockfrost API
- Detailed transaction info

**AdaStat Preprod:**
- URL: https://preprod.adastat.net/
- Pool statistics and delegation

## Troubleshooting

### ❌ "Invalid Cardano address"
- **Cause:** Address format is incorrect
- **Solution:** Cardano addresses must:
  - Start with `addr_test1` (Preprod testnet) or `addr1` (mainnet)
  - Be 100+ characters long
  - Contain only lowercase alphanumeric characters

### ❌ "Blockfrost API error: 403"
- **Cause:** Invalid or missing API key
- **Solution:**
  1. Verify your API key is correct
  2. Ensure you created a "Preprod" project (not mainnet)
  3. Check API key configuration in cryptofolio

### ❌ "Blockfrost API error: 404"
- **Cause:** Address not found or never used
- **Solution:**
  1. Verify you copied the correct address
  2. Ensure address has received at least one transaction
  3. Wait a few minutes after first transaction for indexing

### ❌ Balance shows 0 but wallet has ADA
- **Cause:** Blockchain explorer lag or API cache
- **Solutions:**
  1. Wait 1-2 minutes and try again
  2. Check address on CardanoScan Preprod
  3. Verify correct network (Preprod vs Preview)

### ❌ Faucet not working / out of funds
- **Cause:** High demand or temporary outage
- **Solutions:**
  1. Try a different faucet from the list
  2. Try during off-peak hours (early morning UTC)
  3. Ask in Cardano Discord or Telegram channels
  4. Check official Cardano testnets documentation

### ❌ Native tokens not showing
- **Cause:** Tokens not indexed yet or no token transactions
- **Solution:**
  1. Wait 1-2 minutes after receiving tokens
  2. Verify tokens exist on CardanoScan
  3. Ensure you actually received tokens (not just ADA)

## Important Notes

### About Cardano Preprod Testnet

- **Purpose:** Testing only - has no real value
- **Block time:** ~20 seconds
- **Epoch length:** 1 day (vs 5 days on mainnet)
- **Reset:** Preprod is permanent (unlike older testnets that reset)
- **Explorers:** Multiple explorers available

### About Blockfrost API

- **Free Tier:** 50,000 requests/day
- **Rate Limits:** ~10 requests/second
- **Caching:** Responses are cached for 30-60 seconds
- **Testnets:** Preprod, Preview both supported
- **Monitoring:** Dashboard shows usage stats

### Security Considerations

- ⚠️ **Never use testnet wallets for real funds**
- ⚠️ **Don't reuse testnet seed phrases on mainnet**
- ✅ Testnet seed phrases are safe to share (they have no value)
- ✅ You can use the same address for multiple tests
- ✅ Blockfrost API keys for testnet are safe to share (but keep private as best practice)

### Staking on Testnet

Cardano testnets support staking:
- Delegate to test stake pools
- Earn test rewards (no real value)
- Test delegation features in Cryptofolio (coming soon)
- No minimum ADA required on testnet

## Next Steps

After successfully testing:

1. ✅ Verify ADA balance displays correctly
2. ✅ Test native token detection (if you have tokens)
3. ✅ Try syncing multiple times to verify consistency
4. ✅ Test with different addresses
5. ✅ Import transaction history (once implemented)

See `VALIDATION_CHECKLIST_CARDANO.md` for complete validation steps.

## Need Help?

- **Blockfrost Support:** https://docs.blockfrost.io/
- **Cardano Testnet Info:** https://docs.cardano.org/cardano-testnet/
- **Cardano Forum:** https://forum.cardano.org/
- **Cardano Stack Exchange:** https://cardano.stackexchange.com/
- **Cardano Discord:** https://discord.gg/kfATXEENPD

## Useful Resources

**Testnet Faucets:**
- https://docs.cardano.org/cardano-testnet/tools/faucet/
- https://testnets.cardano.org/en/testnets/cardano/tools/faucet/
- https://faucet.gimbalabs.com/

**Testnet Explorers:**
- https://preprod.cardanoscan.io/
- https://preprod.cexplorer.io/
- https://preprod.adastat.net/

**API Documentation:**
- https://docs.blockfrost.io/

**Wallets:**
- Eternl: https://eternl.io/
- Nami: https://namiwallet.io/
- Flint: https://flint-wallet.com/
- Yoroi: https://yoroi-wallet.com/

---

**Last Updated:** 2026-03-22
**Network:** Preprod (recommended testnet for Cardano)
**Alternative:** Preview (for testing upcoming features)
**API Provider:** Blockfrost (https://blockfrost.io/)
