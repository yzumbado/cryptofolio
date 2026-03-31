# Ethereum Sepolia Testnet Setup Guide

This guide provides step-by-step instructions for testing Cryptofolio's Ethereum wallet tracking features using the Sepolia testnet.

## Prerequisites

- Cryptofolio installed and configured
- Internet connection
- Basic understanding of Ethereum addresses

## Step 1: Get a Sepolia Testnet Wallet

### Option A: MetaMask (Recommended - Most Popular)

1. **Install MetaMask:**
   - Visit https://metamask.io/
   - Install browser extension (Chrome, Firefox, Brave, Edge)
   - Or download mobile app (iOS/Android)

2. **Create/Import Wallet:**
   - Click "Create a new wallet" or "Import existing wallet"
   - Follow the setup wizard
   - **IMPORTANT:** Save your seed phrase securely (offline)

3. **Switch to Sepolia Testnet:**
   - Click the network dropdown (top of MetaMask)
   - Toggle "Show test networks" in Settings → Advanced
   - Select "Sepolia test network"

4. **Copy Your Address:**
   - Click your account name to copy address
   - Format: `0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0` (42 characters)

### Option B: Rabby Wallet

1. Visit https://rabby.io/
2. Install browser extension
3. Create new wallet
4. Switch to "Sepolia" network
5. Copy your address

### Option C: Coinbase Wallet

1. Download from https://www.coinbase.com/wallet
2. Create new wallet
3. Go to Settings → Networks
4. Enable "Testnets"
5. Switch to "Sepolia"

## Step 2: Get Testnet ETH

**IMPORTANT:** Getting testnet ETH can take 10-60 minutes depending on faucet availability. Be patient!

### Working Faucets (2026)

Try these faucets in order until you receive funds:

#### 1. Alchemy Sepolia Faucet (Recommended)
- **URL:** https://sepoliafaucet.com/
- **Requirements:** Alchemy account (free)
- **Amount:** 0.5 SepoliaETH per day
- **Notes:** Most reliable, requires login

#### 2. Infura Sepolia Faucet
- **URL:** https://www.infura.io/faucet/sepolia
- **Requirements:** Infura account (free)
- **Amount:** 0.5 SepoliaETH per day
- **Notes:** Good reliability

#### 3. QuickNode Faucet
- **URL:** https://faucet.quicknode.com/ethereum/sepolia
- **Requirements:** Email verification
- **Amount:** 0.1 SepoliaETH
- **Notes:** Fast processing

#### 4. Chainlink Sepolia Faucet
- **URL:** https://faucets.chain.link/sepolia
- **Requirements:** Twitter/GitHub account
- **Amount:** 0.1 SepoliaETH
- **Notes:** Requires social verification

#### 5. Proof of Work Faucet (No Login Required)
- **URL:** https://sepolia-faucet.pk910.de/
- **Requirements:** None (but requires mining in browser)
- **Amount:** Variable (0.01-0.5 SepoliaETH)
- **Notes:** Slow but no signup required. Leave tab open while "mining"

### Tips for Getting Testnet ETH

- **Multiple faucets:** Try several faucets to get enough ETH faster
- **Wait time:** Some faucets have cooldowns (24 hours between claims)
- **Verification:** Have Twitter/GitHub accounts ready for social verification
- **Browser mining:** The PoW faucet is reliable but slow - let it run overnight

## Step 3: Get Test ERC-20 Tokens (Optional)

To test ERC-20 token tracking, you need testnet tokens:

### Option A: Sepolia Token Faucets

1. **USDC Test Token:**
   - Visit https://faucet.circle.com/
   - Select "Sepolia"
   - Request test USDC

2. **DAI Test Token:**
   - Visit https://sepolia-faucet.com/dai
   - Connect wallet
   - Request test DAI

3. **LINK Test Token:**
   - Visit https://faucets.chain.link/sepolia
   - Also provides LINK tokens along with ETH

### Option B: Uniswap Testnet Swap

1. Visit https://app.uniswap.org/
2. Switch to Sepolia network
3. Swap some testnet ETH for testnet tokens (USDC, DAI, etc.)

### Option C: Deploy Your Own Test Token

If you're familiar with smart contracts, deploy a simple ERC-20 token on Sepolia for testing.

## Step 4: Add Wallet to Cryptofolio

```bash
# Add your Sepolia testnet wallet
cryptofolio wallet add 'Sepolia Test Wallet' \
  --blockchain ethereum \
  --address 0xYourAddressHere \
  --network testnet
```

Expected output:
```
✓ Added wallet 'Sepolia Test Wallet' [TESTNET]
  Address: 0xYourAddressHere
```

## Step 5: Verify Balance

```bash
# Sync your wallet to fetch current balance
cryptofolio wallet sync 'Sepolia Test Wallet'
```

Expected output:
```
✓ Synced ETH balance: 0.5000
✓ Found 2 tokens
  USDC: 100.00
  LINK: 10.00
```

## Step 6: Test Transaction History (Optional)

```bash
# Import transaction history
cryptofolio wallet sync 'Sepolia Test Wallet' --import-history
```

Expected output:
```
✓ Synced ETH balance: 0.5000
✓ Found 2 tokens
  USDC: 100.00
  LINK: 10.00
✓ Imported 5 transactions
```

## Troubleshooting

### ❌ "Invalid Ethereum address"
- **Cause:** Address format is incorrect
- **Solution:** Ethereum addresses must:
  - Start with `0x`
  - Be exactly 42 characters long (including `0x`)
  - Contain only hexadecimal characters (0-9, a-f, A-F)

### ❌ Balance shows 0.0000 but wallet has ETH
- **Cause:** API rate limit or temporary network issue
- **Solutions:**
  1. Wait 30 seconds and try `wallet sync` again
  2. Check your address on Sepolia Etherscan: https://sepolia.etherscan.io/
  3. Verify you're using the correct network (testnet flag)

### ❌ "Network error: Etherscan API error: 429"
- **Cause:** Rate limit exceeded (5 requests/second on free tier)
- **Solution:** Wait 1-2 minutes and retry

### ❌ Tokens not showing up
- **Cause:** Etherscan needs at least one token transaction to detect tokens
- **Solution:**
  1. Check if you actually received tokens (check in MetaMask)
  2. Try sending a small amount to yourself to create a transaction
  3. Wait 1-2 minutes for Etherscan to index the transaction

### ❌ Faucets not working / out of funds
- **Cause:** High demand or temporary outage
- **Solutions:**
  1. Try a different faucet from the list
  2. Try during off-peak hours (early morning UTC)
  3. Use the PoW faucet (always works but slower)
  4. Join Ethereum Discord and ask for testnet ETH

## Important Notes

### About Sepolia Testnet

- **Purpose:** Testing only - has no real value
- **Block time:** ~12 seconds (similar to mainnet)
- **Explorers:** https://sepolia.etherscan.io/
- **RPC URLs:** Public RPC endpoints available (used automatically by Cryptofolio)

### Security Considerations

- ⚠️ **Never use testnet wallets for real funds**
- ⚠️ **Don't reuse testnet addresses on mainnet**
- ✅ Testnet seed phrases are safe to share (they have no value)
- ✅ You can use the same address for multiple tests

### Cleaning Up After Testing

When you're done testing, you can optionally return testnet ETH to help others:

**Recommended Return Address:**
```
0x0000000000000000000000000000000000000000
```

Or donate to testnet faucets if they provide a return address.

## Next Steps

After successfully testing:

1. ✅ Verify ETH balance displays correctly
2. ✅ Test ERC-20 token detection
3. ✅ Import transaction history
4. ✅ List wallets to see testnet indicator
5. ✅ Try syncing multiple times to verify consistency

See `VALIDATION_CHECKLIST_ETHEREUM.md` for complete validation steps.

## Need Help?

- **Etherscan Sepolia:** https://sepolia.etherscan.io/
- **MetaMask Support:** https://support.metamask.io/
- **Ethereum Testnet Info:** https://ethereum.org/en/developers/docs/networks/#sepolia

---

**Last Updated:** 2026-03-22
**Network:** Sepolia (recommended testnet for Ethereum)
**Alternative:** Goerli (being deprecated), Holesky (validator testing)
