# Bitcoin Testnet Setup Guide

**Tested and verified instructions for creating a Bitcoin testnet wallet and getting testnet coins.**

---

## Why Use Testnet?

- ✅ **Safe Testing** - Test Cryptofolio's blockchain sync without risking real Bitcoin
- ✅ **Free Coins** - Get testnet BTC from faucets (no real value)
- ✅ **Full Features** - Same blockchain behavior as mainnet
- ⚠️ **No Real Value** - Testnet coins cannot be converted to real Bitcoin

---

## Step 1: Create Testnet Wallet (Blockstream Green)

**Recommended:** Use Blockstream Green - official wallet from the Blockstream team (makers of the Blockstream API we use).

### Download Blockstream Green

- **iOS:** [App Store](https://apps.apple.com/app/id1402243590)
- **Android:** [Google Play](https://play.google.com/store/apps/details?id=com.greenaddress.greenbits_android_wallet)
- **Desktop:** [blockstream.com/green](https://blockstream.com/green/)

### Setup Testnet Wallet

Follow the official guide: [Set up testnet wallet – Blockstream Help Center](https://help.blockstream.com/hc/en-us/articles/4408499482009-Set-up-testnet-wallet)

**Quick Steps:**

1. **Open Blockstream Green app**

2. **Enable Testnet Mode:**
   - Tap the **Wallet icon** (top-right corner)
   - Tap the **three dots** (⋮) to open App Settings
   - Toggle **"Enable testnet"** ON
   - Tap **Save**

3. **Create Testnet Wallet:**
   - Tap the **Wallet icon** again
   - Tap **"Set up a New Wallet"** (bottom of screen)
   - Tap **"Get Started"**
   - Tap **"Set up Mobile Wallet"**
   - Choose **"Testnet"** as the network
   - **Save your recovery phrase** (24 words) - IMPORTANT!
   - Complete the setup

4. **Get Your Testnet Address:**
   - Open your testnet wallet
   - Tap **"Receive"**
   - Copy the address (starts with `tb1`, `m`, or `n`)
   - Example: `tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx`

   ⚠️ **IMPORTANT - HD Wallet Addresses:**
   - Blockstream Green creates **multiple addresses** (HD wallet)
   - Each "Receive" may show a **different address**
   - When you receive coins, check the **"Transactions"** tab to see which **specific address** received them
   - Add the **address that received the coins** to Cryptofolio, not just any address from "Receive"

---

## Step 2: Get Testnet Bitcoin

**⏱ Note:** Getting testnet BTC can take time (10-60 minutes) depending on:
- Network congestion
- Faucet availability
- Confirmation times

### Working Testnet Faucets (2026)

**Recommended (Actively Maintained):**

1. **https://bitcoinfaucet.uo1.net/**
   - Reliable and actively maintained
   - Typical amount: 0.001 - 0.01 tBTC
   - Wait time: 10-30 minutes

2. **https://testnet.help/en/btcfaucet/testnet**
   - Bitcoin Testnet4 faucet
   - Good availability

3. **https://tatum.io/faucets**
   - Amount: 0.00001 tBTC every 24 hours
   - Multiple blockchain support

4. **https://faucet.triangleplatform.com/bitcoin/testnet**
   - Multi-chain testnet faucet

**Full List:** https://faucet-list.com/testnet-faucets/bitcoin

### How to Request Testnet Coins

1. **Copy your testnet address** from Blockstream Green
2. **Visit a faucet** (e.g., https://bitcoinfaucet.uo1.net/)
3. **Paste your address** in the field
4. **Complete any CAPTCHA** if required
5. **Click "Get testnet coins"** or similar button
6. **Wait for confirmation** (10-60 minutes)
   - The transaction needs to be mined
   - Check status in Blockstream Green or on https://blockstream.info/testnet/

---

## Step 3: Add Wallet to Cryptofolio

Once you have your testnet address:

```bash
# Add testnet wallet
cryptofolio wallet add "Blockstream Testnet" --blockchain bitcoin \
  --address tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx

# Expected output:
# [OK] ✓ Added wallet 'Blockstream Testnet' (bitcoin address [TESTNET])
# [INFO]   Address: tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx
# [INFO]   ⚠️  This is a TESTNET address
```

### Verify Testnet Detection

```bash
cryptofolio wallet list --blockchain bitcoin
```

**Expected output:**
```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Wallets
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Blockstream Testnet (software_wallet)
  ₿ bitcoin tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx [TESTNET] 🟡
```

✅ Look for the yellow **[TESTNET]** label to confirm proper detection.

---

## Step 4: Sync Blockchain Data

### Before Coins Arrive (Empty Wallet)

```bash
cryptofolio wallet sync "Blockstream Testnet"
```

**Expected output:**
```
Syncing Blockstream Testnet (bitcoin)...
[INFO]   Using testnet API
[OK] ✓ Synced BITCOIN balance: 0.00000000
[INFO]   Transactions: 0
[INFO]   Total received: 0.00000000 BTC
[INFO]   Total sent: 0.00000000 BTC
```

### After Coins Arrive (With Balance)

Wait 10-60 minutes after requesting from faucet, then sync again:

```bash
cryptofolio wallet sync "Blockstream Testnet"
```

**Expected output:**
```
Syncing Blockstream Testnet (bitcoin)...
[INFO]   Using testnet API
[OK] ✓ Synced BITCOIN balance: 0.00100000
[INFO]   Transactions: 1
[INFO]   Total received: 0.00100000 BTC
[INFO]   Total sent: 0.00000000 BTC
```

### Check Transaction History

```bash
cryptofolio wallet sync "Blockstream Testnet" --import-history
```

**Expected output:**
```
Syncing Blockstream Testnet (bitcoin)...
[INFO]   Using testnet API
[OK] ✓ Synced BITCOIN balance: 0.00100000
[INFO]   Transactions: 1
[INFO]   Total received: 0.00100000 BTC
[INFO]   Total sent: 0.00000000 BTC
[OK] ✓ Imported 1 transactions
```

---

## Step 5: Return Testnet Coins (When Done)

**⚠️ IMPORTANT:** Please return testnet coins when you're done testing. This helps others who need testnet BTC.

### Blockstream Return Address

```
tb1qerzrlxcfu24davlur5sqmgzzgsal6wusda40er
```

### How to Return Coins

**Using Blockstream Green:**

1. Open your testnet wallet in Blockstream Green
2. Tap **"Send"**
3. Paste return address: `tb1qerzrlxcfu24davlur5sqmgzzgsal6wusda40er`
4. Enter amount (or tap "Send All")
5. Set fee (minimum is fine for testnet)
6. Confirm and send

**Why Return Coins?**
- Testnet BTC is a limited community resource
- Helps developers and testers worldwide
- Takes only 1-2 minutes
- Good Bitcoin etiquette 🤝

---

## Troubleshooting

### "Address not found" Error

**Problem:** Wallet sync shows "Failed to sync" or "Address not found"

**Solution:**
- If you just created the address, wait a few minutes
- If you haven't received coins yet, the address won't appear on blockchain explorers
- Try again after requesting testnet coins

### Faucet Not Working

**Problem:** Faucet says "Out of coins" or "Try again later"

**Solution:**
- Try a different faucet from the list above
- Faucets refill periodically, try again in a few hours
- Some faucets have daily limits per IP address

### Coins Not Arriving

**Problem:** Requested testnet BTC 30+ minutes ago, still not received

**Solution:**
1. **Check transaction status:**
   - Go to https://blockstream.info/testnet/
   - Search for your address
   - See if transaction exists and confirmation count

2. **Wait longer:**
   - Testnet can be slow (blocks every 10+ minutes)
   - May need 1-6 confirmations
   - Can take up to 1 hour

3. **Try another faucet:**
   - First faucet might be empty
   - Try 2-3 different faucets

### Wrong Network Error

**Problem:** "This is a mainnet address" or not showing [TESTNET] label

**Solution:**
- Double-check address starts with `tb1`, `m`, `n`, or `2`
- If starts with `bc1`, `1`, or `3` - that's mainnet!
- Re-create wallet making sure to select "Testnet" network

### Balance Shows Zero Despite Receiving Coins

**Problem:** Blockstream Green shows coins received, but Cryptofolio shows 0.00000000 balance

**Cause:** HD wallets (like Blockstream Green) generate **multiple addresses**. You may have added address A to Cryptofolio, but the coins were received on address B.

**Solution:**
1. **In Blockstream Green**, tap "Transactions" tab
2. Tap the transaction that received the testnet coins
3. Look at the **"Received to"** address
4. **That's the address** you need to add to Cryptofolio

**Example:**
```bash
# You might have added this address (from "Receive" button):
tb1qy8y72ly6g2kml0myedqasjjad4az97h52m3rm9

# But coins were actually received on this address:
tb1qe5vpytpg8lvj98etygqe2pd547w4cccy2d4mxc

# Fix: Remove wrong address and add the correct one
cryptofolio wallet remove "Blockstream Testnet" --yes
cryptofolio wallet add "Blockstream Testnet" --blockchain bitcoin \
  --address tb1qe5vpytpg8lvj98etygqe2pd547w4cccy2d4mxc
```

**Future Feature:** xpub support will track all addresses automatically

---

## Verification Checklist

Before considering your testnet setup complete:

- ✅ Blockstream Green installed and testnet mode enabled
- ✅ Testnet wallet created with recovery phrase saved
- ✅ Testnet address copied (starts with `tb1`, `m`, `n`, or `2`)
- ✅ Testnet coins requested from faucet
- ✅ Wallet added to Cryptofolio with [TESTNET] label
- ✅ Sync shows "Using testnet API"
- ✅ Balance appears after waiting for confirmations
- ✅ Transaction history imports correctly

---

## Quick Reference

### Testnet Address Formats
- **Bech32 SegWit:** `tb1q...` or `tb1p...` (most common)
- **Legacy P2PKH:** `m...` or `n...`
- **P2SH:** `2...`

### Working Faucets (2026)
1. https://bitcoinfaucet.uo1.net/ ⭐ Recommended
2. https://testnet.help/en/btcfaucet/testnet
3. https://tatum.io/faucets
4. https://faucet.triangleplatform.com/bitcoin/testnet

### Return Address
```
tb1qerzrlxcfu24davlur5sqmgzzgsal6wusda40er
```

### Cryptofolio Commands
```bash
# Add wallet
cryptofolio wallet add "Test" --blockchain bitcoin --address <tb1...>

# List wallets
cryptofolio wallet list --blockchain bitcoin

# Sync wallet
cryptofolio wallet sync "Test"

# Import history
cryptofolio wallet sync "Test" --import-history

# Sync all testnet wallets
cryptofolio wallet sync --all
```

---

## Additional Resources

**Official Documentation:**
- [Set up testnet wallet – Blockstream Help Center](https://help.blockstream.com/hc/en-us/articles/4408499482009-Set-up-testnet-wallet)
- [What is a Testnet wallet? – Blockstream Help Center](https://help.blockstream.com/hc/en-us/articles/4408407302809-What-is-a-Testnet-wallet)
- [Bitcoin Testnet Explorer - Blockstream.info](https://blockstream.info/testnet/)

**Testnet Faucets:**
- [Bitcoin (BTC) Testnet Faucets 2026 | Faucet-List.com](https://faucet-list.com/testnet-faucets/bitcoin)
- [Bitcoin Testnet Faucet](https://bitcoinfaucet.uo1.net/)
- [Bitcoin Testnet4 Faucet](https://testnet.help/en/btcfaucet/testnet)

**Cryptofolio Documentation:**
- [BLOCKCHAIN_SYNC.md](BLOCKCHAIN_SYNC.md) - Complete blockchain sync guide
- [TESTNET_SUPPORT.md](TESTNET_SUPPORT.md) - Technical testnet details
- [BITCOIN_WALLET_COMPLETE.md](BITCOIN_WALLET_COMPLETE.md) - Implementation details

---

## Summary

**Tested Path (Works in 2026):**

1. **Install Blockstream Green** (mobile or desktop)
2. **Enable testnet mode** in settings
3. **Create testnet wallet** and save recovery phrase
4. **Get testnet address** (starts with tb1, m, or n)
5. **Request coins** from https://bitcoinfaucet.uo1.net/
6. **Wait 10-60 minutes** for confirmation
7. **Add to Cryptofolio** with `wallet add` command
8. **Sync blockchain data** with `wallet sync` command
9. **Return coins** to `tb1qerzrlxcfu24davlur5sqmgzzgsal6wusda40er` when done

**Time Required:** 15-90 minutes (mostly waiting for testnet confirmations)

**Difficulty:** Easy ⭐⭐☆☆☆

Happy testing! 🚀
