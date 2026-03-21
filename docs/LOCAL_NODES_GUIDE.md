# Local Node Setup Guide

**Why run local nodes?**
- 🔒 **Privacy:** Your wallet addresses never leave your machine
- ⚡ **Speed:** No API rate limits, instant responses
- 💪 **Reliability:** No dependency on third-party services
- 🆓 **Free:** No API key costs

**Trade-offs:**
- 📦 **Disk space:** Bitcoin Core ~500GB, Geth ~800GB (pruned mode available)
- ⏱️ **Sync time:** Initial sync takes 1-7 days
- 💻 **Resources:** Requires decent CPU/RAM (manageable on modern MacBooks)

---

## Quick Start

```bash
# Check if you have space and resources
cryptofolio node check-requirements

# Install Bitcoin Core (recommended: start here)
cryptofolio node install bitcoin

# Install Ethereum (Geth) - optional
cryptofolio node install ethereum

# Check status
cryptofolio node status
```

---

## Bitcoin Core (Recommended - Start Here)

### Why Bitcoin Core?
- Smallest footprint (~500GB, or ~10GB pruned)
- Most widely supported
- Fastest sync (1-2 days on good connection)
- Best for Sparrow wallet integration

### Installation (macOS)

#### Option 1: Automated (Recommended)
```bash
# Cryptofolio helper (coming in v0.5.0)
cryptofolio node install bitcoin --pruned
# Downloads, installs, and configures Bitcoin Core
# Uses pruned mode (only 10GB disk space)
```

#### Option 2: Manual Installation

**Step 1: Download Bitcoin Core**
```bash
# Using Homebrew (recommended)
brew install bitcoin

# Or download from bitcoin.org
# https://bitcoincore.org/en/download/
```

**Step 2: Configure bitcoin.conf**
```bash
# Create config directory
mkdir -p ~/Library/Application\ Support/Bitcoin

# Create config file
cat > ~/Library/Application\ Support/Bitcoin/bitcoin.conf << 'EOF'
# Run as a daemon
daemon=1

# Pruned mode (saves disk space - only 10GB instead of 500GB)
prune=10000

# RPC settings for Cryptofolio
server=1
rpcuser=cryptofolio
rpcpassword=CHANGE_THIS_PASSWORD
rpcallowip=127.0.0.1
rpcport=8332

# Network settings
listen=0
maxconnections=16

# Disable wallet (we're only reading blockchain)
disablewallet=1
EOF
```

**Step 3: Start Bitcoin Core**
```bash
# Start the node
bitcoind

# Check sync status
bitcoin-cli getblockchaininfo

# Wait for sync (1-2 days initial sync)
# You can use Cryptofolio with public APIs while syncing
```

**Step 4: Configure Cryptofolio**
```bash
# Tell Cryptofolio to use your local node
cryptofolio node set bitcoin \
  --type local \
  --rpc-url http://localhost:8332 \
  --rpc-user cryptofolio \
  --rpc-password CHANGE_THIS_PASSWORD

# Test connection
cryptofolio node status bitcoin
# ✓ Bitcoin Core (height: 835,421, 100% synced)
```

### Disk Space Options

**Full Node (~500GB):**
- Stores entire blockchain history
- Best for advanced users
- Remove `prune=10000` from config

**Pruned Node (~10GB):**
- Only keeps recent blocks
- Perfect for wallet tracking
- **Recommended for most users**
- Add `prune=10000` to config

---

## Ethereum (Geth) - Optional

### Why Geth?
- Full Ethereum + ERC-20 support
- Required for rETH, RPL token tracking
- Enables privacy-first Ethereum usage

### Challenges
- ⚠️ **Larger:** ~800GB full, ~200GB pruned
- ⚠️ **Slower sync:** 3-7 days
- ⚠️ **More resources:** Higher CPU/RAM usage

### Installation (macOS)

#### Option 1: Automated (Recommended)
```bash
# Cryptofolio helper (coming in v0.5.0)
cryptofolio node install ethereum --pruned --snap
# Uses snap sync (faster) and pruned mode
```

#### Option 2: Manual Installation

**Step 1: Install Geth**
```bash
# Using Homebrew
brew tap ethereum/ethereum
brew install ethereum
```

**Step 2: Start Geth (Snap Sync + Pruned)**
```bash
# Snap sync (faster initial sync - recommended)
geth --syncmode snap \
     --http \
     --http.addr 127.0.0.1 \
     --http.port 8545 \
     --http.api eth,net,web3 \
     --gcmode archive \
     --cache 4096

# This will sync in ~1-3 days
```

**Step 3: Configure Cryptofolio**
```bash
cryptofolio node set ethereum \
  --type local \
  --rpc-url http://localhost:8545

# Test connection
cryptofolio node status ethereum
# ✓ Geth (height: 19,234,567, 100% synced)
```

### Alternative: Light Client (Coming Soon)
```bash
# Geth light client (much smaller, but less reliable)
geth --syncmode light --http --http.addr 127.0.0.1
```

---

## Solana - Optional

### Challenges
- ❌ **Very resource intensive:** Requires powerful hardware
- ❌ **Huge disk space:** ~200GB+ and growing fast
- ❌ **Complex setup:** Not recommended for most users

### Recommendation
- ✅ **Use public RPC:** Solana Foundation provides free public RPCs
- ✅ **Or use Helius/Alchemy:** Free tier is generous

**Configuration:**
```bash
# Use public RPC (recommended)
cryptofolio node set solana \
  --type public_api \
  --rpc-url https://api.mainnet-beta.solana.com

# Or Helius (better rate limits)
cryptofolio node set solana \
  --type public_api \
  --rpc-url https://mainnet.helius-rpc.com/?api-key=YOUR_KEY \
  --api-key YOUR_KEY
```

---

## Cardano - Optional

### Options

#### Option 1: Daedalus (Easiest)
- Daedalus wallet includes a full node
- ~20GB disk space
- Syncs in ~1 day
- Has built-in RPC

**Setup:**
1. Install Daedalus from https://daedaluswallet.io
2. Let it sync
3. Configure Cryptofolio to use Daedalus node

#### Option 2: cardano-node (Advanced)
- Bare node without wallet
- More complex setup
- Better for servers

#### Option 3: Blockfrost API (Recommended)
- Free tier: 50,000 requests/day
- Easy setup
- No local resources needed

**Configuration:**
```bash
# Using Blockfrost (recommended)
cryptofolio config set-secret blockfrost.project_id
# Enter your project ID from blockfrost.io

cryptofolio node set cardano \
  --type public_api \
  --rpc-url https://cardano-mainnet.blockfrost.io/api/v0
```

---

## Hybrid Approach (Recommended)

**For most users:**
- ✅ **Bitcoin:** Local node (pruned) - Easy, small, fast
- ✅ **Ethereum:** Public API initially, local node later if needed
- ✅ **Solana:** Public RPC (free)
- ✅ **Cardano:** Blockfrost API (free tier)

**Benefits:**
- Bitcoin privacy (most important)
- Manageable disk space (~10GB)
- Works immediately
- Can upgrade to more local nodes over time

---

## Monitoring & Maintenance

### Check Node Status
```bash
# All nodes
cryptofolio node status

# Specific node
cryptofolio node status bitcoin

# Detailed info
cryptofolio node status bitcoin --verbose
```

### Sync Progress
```bash
# Watch Bitcoin Core sync
watch -n 5 'bitcoin-cli getblockchaininfo | grep -E "blocks|headers"'

# Watch Geth sync
geth attach --exec 'eth.syncing'
```

### Disk Space Management
```bash
# Check disk usage
du -sh ~/Library/Application\ Support/Bitcoin
du -sh ~/Library/Ethereum

# Bitcoin: Enable pruning if needed
# Edit bitcoin.conf and add: prune=10000
```

---

## Troubleshooting

### Bitcoin Core won't start
```bash
# Check logs
tail -f ~/Library/Application\ Support/Bitcoin/debug.log

# Common issues:
# - Port already in use: Change rpcport in config
# - Corrupted data: Delete and resync
# - Wrong permissions: chmod 600 bitcoin.conf
```

### Geth won't sync
```bash
# Check logs
geth attach --exec 'eth.syncing'

# Common issues:
# - Not enough disk space: Use --gcmode full --prune
# - Slow peers: Restart and wait
# - Corrupted database: geth removedb (warning: re-downloads everything)
```

### RPC connection failed
```bash
# Test RPC manually
curl -u cryptofolio:PASSWORD \
  -d '{"jsonrpc":"1.0","id":"test","method":"getblockchaininfo","params":[]}' \
  -H 'content-type: text/plain;' \
  http://127.0.0.1:8332/

# Check if node is running
ps aux | grep bitcoind
ps aux | grep geth
```

---

## Cost Analysis

### Disk Space (MacBook)
| Setup | Total Space | Viable on MacBook? |
|-------|-------------|-------------------|
| Bitcoin only (pruned) | ~10GB | ✅ Yes |
| Bitcoin full | ~500GB | ⚠️ Maybe (1TB SSD) |
| + Ethereum (pruned) | ~210GB | ⚠️ Tight (512GB) |
| + Ethereum full | ~1.3TB | ❌ Need external drive |
| All chains local | ~1.5TB+ | ❌ Need external drive |

### Recommendation for MacBook Users
- ✅ **256GB MacBook:** Bitcoin pruned only
- ✅ **512GB MacBook:** Bitcoin pruned + consider Ethereum later
- ✅ **1TB+ MacBook:** Bitcoin full + Ethereum pruned
- ✅ **External SSD:** Run everything if you want

---

## API Keys (Fallback)

If local nodes aren't viable, get free API keys:

### Ethereum
- **Alchemy:** https://www.alchemy.com (Free: 300M compute units/month)
- **Infura:** https://infura.io (Free: 100k requests/day)

### Solana
- **Helius:** https://helius.xyz (Free: generous)
- **QuickNode:** https://quicknode.com (Free tier available)

### Cardano
- **Blockfrost:** https://blockfrost.io (Free: 50k requests/day)

Store API keys securely:
```bash
cryptofolio config set-secret alchemy.api_key
cryptofolio config set-secret helius.api_key
cryptofolio config set-secret blockfrost.project_id
```

---

## Summary

**Recommended Setup:**
1. ✅ Start with Bitcoin Core (pruned) - 10GB, 1-2 day sync
2. ✅ Use public APIs for ETH, SOL, ADA initially
3. ✅ Add Ethereum local node later if you want more privacy
4. ✅ Monitor and maintain nodes regularly

**Commands:**
```bash
# Install Bitcoin (automated - coming in v0.5.0)
cryptofolio node install bitcoin --pruned

# Configure
cryptofolio node set bitcoin --type local --rpc-url http://localhost:8332

# Check status
cryptofolio node status

# Use Cryptofolio normally
cryptofolio wallet sync --all
cryptofolio audit
```

**Questions?** Open an issue on GitHub or check the documentation.
