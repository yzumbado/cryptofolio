# Cryptofolio MCP Server - User Guide

**Version:** 1.0.0 (Draft)
**Last Updated:** February 19, 2026
**Purpose:** Guide for using Cryptofolio with Claude Desktop via MCP

---

## Table of Contents

- [Overview](#overview)
- [What is MCP?](#what-is-mcp)
- [Installation](#installation)
- [Getting Started](#getting-started)
- [Use Cases](#use-cases)
- [Conversation Examples](#conversation-examples)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

---

## Overview

The Cryptofolio MCP Server enables you to manage your cryptocurrency portfolio through natural conversation with Claude Desktop. Instead of remembering CLI commands, you can simply ask Claude to check prices, record transactions, analyze holdings, and more.

### What You Can Do

✅ **Portfolio Management**
- "What's my current portfolio value?"
- "Show me my Bitcoin holdings across all accounts"
- "What's my unrealized profit and loss?"

✅ **Price Checking**
- "What's the current price of Bitcoin and Ethereum?"
- "How is Solana performing today?"
- "Get market data for BTC with 24h stats"

✅ **Transaction Recording**
- "I bought 0.1 BTC at $95,000 on Binance"
- "I transferred 1 ETH from Binance to my Ledger"
- "I sold 0.5 ETH at $3,200"

✅ **Multi-Currency Flows**
- "I converted 100,000 CRC to USD to USDT to BTC, help me track this"
- "Track my Costa Rica on-ramp: CRC → USD → USDT → BTC"

✅ **Analysis & Reporting**
- "Analyze my Bitcoin holdings and performance"
- "Show my transaction history for 2024"
- "Export my transactions for tax reporting"

✅ **Account Management**
- "What accounts do I have configured?"
- "Sync my Binance account to get latest balances"

---

## What is MCP?

**MCP (Model Context Protocol)** is a standard protocol that allows AI assistants like Claude to interact with local tools and data sources. Think of it as giving Claude "superpowers" to access your applications.

### How It Works

```
You (in Claude Desktop)
    ↓ Natural language
Claude (AI Assistant)
    ↓ Chooses appropriate tool
Cryptofolio MCP Server
    ↓ Executes command
Cryptofolio CLI
    ↓ Queries database
Your Portfolio Data
    ↓ Returns results
Claude presents results to you
```

### Benefits

- **Natural Language:** No need to remember commands
- **Context Aware:** Claude remembers previous conversation
- **Multi-Step Tasks:** Claude can chain multiple operations
- **Error Recovery:** Claude helps you fix mistakes
- **Learning:** Claude explains what it's doing

---

## Installation

### Prerequisites

1. **Cryptofolio CLI** installed and working
   ```bash
   cryptofolio --version
   # Should show: cryptofolio 0.2.0
   ```

2. **Node.js 20.x or later**
   ```bash
   node --version
   # Should show: v20.x.x or higher
   ```

3. **Claude Desktop** installed
   - Download from: https://claude.ai/download

### Step 1: Install Cryptofolio MCP Server

```bash
# Install globally via NPM
npm install -g cryptofolio-mcp

# Verify installation
cryptofolio-mcp --version
```

### Step 2: Configure Claude Desktop

Edit Claude Desktop's configuration file:

**macOS/Linux:**
```bash
nano ~/Library/Application\ Support/Claude/claude_desktop_config.json
```

**Windows:**
```powershell
notepad %APPDATA%\Claude\claude_desktop_config.json
```

Add the Cryptofolio MCP server:

```json
{
  "mcpServers": {
    "cryptofolio": {
      "command": "cryptofolio-mcp",
      "args": []
    }
  }
}
```

### Step 3: Restart Claude Desktop

1. Quit Claude Desktop completely
2. Reopen Claude Desktop
3. Look for "Cryptofolio MCP Server connected" indicator

### Step 4: Test the Connection

In Claude Desktop, type:
```
What tools do you have available from Cryptofolio?
```

Claude should list 10 tools:
- cryptofolio_get_portfolio
- cryptofolio_get_prices
- cryptofolio_get_market_data
- cryptofolio_record_transaction
- cryptofolio_track_conversion
- cryptofolio_list_transactions
- cryptofolio_analyze_asset
- cryptofolio_export_transactions
- cryptofolio_list_accounts
- cryptofolio_sync_exchange

---

## Getting Started

### First Conversation

**You:** Hi Claude! Can you check my cryptocurrency portfolio?

**Claude:** I'll check your Cryptofolio portfolio for you.

*[Claude uses cryptofolio_get_portfolio tool]*

**Claude:** Here's your current portfolio:

**Portfolio Overview:**
- **Total Value:** $61,442.89
- **Cost Basis:** $29,317.39
- **Unrealized P&L:** +$32,125.50 (+109.57%)

**Holdings by Account:**

**Binance (Trading)**
- BTC: 0.09121 ($6,407.86)
- ETH: 2.4594 ($5,136.11)
- NIGHT: 26,016.95 ($1,396.97)

**Ledger (Cold Storage)**
- BTC: 0.09112651 ($6,402.00)

You're up over 109% on your investments! Your portfolio is performing very well. Would you like me to analyze any specific asset or account?

---

### Understanding Claude's Tool Usage

When Claude uses Cryptofolio tools, you'll see indicators like:

```
🔧 Using tool: cryptofolio_get_portfolio
```

This transparency helps you understand what Claude is doing with your data.

---

## Use Cases

### Use Case 1: Morning Portfolio Check

**Scenario:** You want to quickly check your portfolio value and any significant price movements before starting your day.

**Conversation:**

**You:** Good morning! What's my portfolio looking like today?

**Claude:** Good morning! Let me check your portfolio.

*[Uses: cryptofolio_get_portfolio]*

**Claude:** Your portfolio is currently worth **$61,442.89**, up **$32,125.50 (+109.57%)** from your cost basis of $29,317.39.

**You:** How did Bitcoin perform overnight?

**Claude:** Let me get the 24-hour market data for Bitcoin.

*[Uses: cryptofolio_get_market_data with asset: "BTC"]*

**Claude:** Bitcoin (BTC) 24h Performance:
- Current Price: $70,253.98
- 24h Change: +$2,145.00 (+2.25%)
- 24h High: $71,500.00
- 24h Low: $68,000.00
- Volume: 12,543.57 BTC

Bitcoin had a good night, up 2.25%! Your BTC holdings (0.18233651 BTC across Binance and Ledger) gained approximately $391 in value overnight.

**You:** Nice! Thanks.

---

### Use Case 2: Recording a Purchase

**Scenario:** You just bought Bitcoin on Binance and want to record the transaction with proper cost basis.

**Conversation:**

**You:** I just bought 0.1 BTC at $95,000 on Binance.

**Claude:** I'll record that purchase for you.

*[Uses: cryptofolio_record_transaction with:*
- *type: "buy"*
- *asset: "BTC"*
- *quantity: "0.1"*
- *price: "95000"*
- *account: "Binance"]*

**Claude:** ✅ Transaction recorded successfully!

**Purchase Details:**
- Asset: BTC
- Quantity: 0.1
- Price: $95,000.00
- Account: Binance
- Total Cost: $9,500.00

**Updated BTC Holdings:**
- New Quantity: 0.19121 BTC
- New Average Cost Basis: $87,937.50
- Current Value: $13,437.52

Your Bitcoin holdings are now worth $13,437.52 with an unrealized gain of $5,500.02 (+69.33%). Would you like me to sync your Binance account to verify the balance?

**You:** Yes, please sync Binance.

**Claude:** I'll sync your Binance account now.

*[Uses: cryptofolio_sync_exchange with account: "Binance"]*

**Claude:** ✅ Binance sync complete!

**Synced Holdings:**
- BTC: 0.19121000 ✓ (matches recorded amount)
- ETH: 2.4594
- USDT: 500.00

Everything looks good! Your recorded BTC purchase matches the Binance API balance.

---

### Use Case 3: Multi-Currency On-Ramp Flow

**Scenario:** You're in Costa Rica and want to track the complete flow from CRC → USD → USDT → BTC with proper cost basis at each step.

**Conversation:**

**You:** I need help tracking a multi-step conversion. I'm converting 100,000 CRC (Costa Rican Colones) to Bitcoin. The flow is:
1. Bank converts CRC to USD at rate 550 CRC/USD
2. I send USD to Lulubit (on-ramp service)
3. Buy USDT with USD on Lulubit (3% fee)
4. Transfer USDT to Binance (0.1 USDT network fee)
5. Buy BTC with USDT on Binance

Can you help me track this entire flow?

**Claude:** Absolutely! This is a perfect use case for multi-currency tracking. Let me break this down step by step and record each transaction. First, let me make sure you have all the necessary accounts set up.

*[Uses: cryptofolio_list_accounts]*

**Claude:** I see you have:
- Banco Nacional (bank account)
- Lulubit (exchange)
- Binance (exchange)

Perfect! Now let's track this conversion flow. Let me record each step:

**Step 1: Bank Conversion (CRC → USD)**

Starting with 100,000 CRC in your bank account. At exchange rate of 550 CRC/USD, you'll get:
100,000 ÷ 550 = 181.82 USD

*[Uses: cryptofolio_record_transaction with:*
- *type: "swap"*
- *from_asset: "CRC"*
- *from_quantity: "100000"*
- *to_asset: "USD"*
- *to_quantity: "181.82"*
- *account: "Banco Nacional"]*

**Claude:** ✅ Step 1 recorded. Exchange rate 550 CRC/USD automatically stored.

**Step 2: Transfer USD to Lulubit**

*[Uses: cryptofolio_record_transaction with:*
- *type: "transfer"*
- *asset: "USD"*
- *quantity: "181.82"*
- *from_account: "Banco Nacional"*
- *to_account: "Lulubit"]*

**Claude:** ✅ Step 2 recorded. USD transferred to Lulubit.

**Step 3: Buy USDT (3% fee included)**

With 181.82 USD and 3% fee, you get:
181.82 × 0.97 = 176.37 USDT (I'll use 176 as you mentioned)

*[Uses: cryptofolio_record_transaction with:*
- *type: "swap"*
- *from_asset: "USD"*
- *from_quantity: "181.82"*
- *to_asset: "USDT"*
- *to_quantity: "176"*
- *account: "Lulubit"]*

**Claude:** ✅ Step 3 recorded. USD converted to USDT (3% fee included in rate).

**Step 4: Transfer USDT to Binance**

*[Uses: cryptofolio_record_transaction with:*
- *type: "transfer"*
- *asset: "USDT"*
- *quantity: "176"*
- *from_account: "Lulubit"*
- *to_account: "Binance"*
- *fee: "0.1"*
- *notes: "ERC-20 network fee"]*

**Claude:** ✅ Step 4 recorded. USDT transferred to Binance (0.1 USDT fee).
Net amount received: 175.9 USDT

**Step 5: Buy BTC with USDT**

Let me check current BTC price first.

*[Uses: cryptofolio_get_prices with assets: ["BTC"]*

**Claude:** Current BTC price: $70,253.98

With 175.9 USDT, you can buy approximately:
175.9 ÷ 70,253.98 = 0.002503 BTC

Should I record this final purchase? Please confirm the exact BTC amount you received.

**You:** Yes, I received exactly 0.0025 BTC.

**Claude:** Perfect! Recording the final BTC purchase.

*[Uses: cryptofolio_record_transaction with:*
- *type: "swap"*
- *from_asset: "USDT"*
- *from_quantity: "175.9"*
- *to_asset: "BTC"*
- *to_quantity: "0.0025"*
- *account: "Binance"]*

**Claude:** ✅ All steps recorded successfully!

**Complete Conversion Summary:**

**Journey:** CRC → USD → USDT → BTC

| Step | From | To | Amount | Rate/Fee |
|------|------|-----|--------|----------|
| 1 | 100,000 CRC | 181.82 USD | Bank conversion | 550 CRC/USD |
| 2 | 181.82 USD | 181.82 USD | Transfer | No fee |
| 3 | 181.82 USD | 176 USDT | On-ramp swap | 3% fee |
| 4 | 176 USDT | 175.9 USDT | Transfer | 0.1 USDT fee |
| 5 | 175.9 USDT | 0.0025 BTC | Exchange swap | Market rate |

**Final Result:**
- **Initial Investment:** 100,000 CRC (~$181.82 USD)
- **Final Holding:** 0.0025 BTC
- **Cost Basis:** Your BTC now has complete cost basis tracking in both CRC and USD
- **Total Fees:** ~5.82 USD (3% on-ramp + network fees)

**Cost Basis Tracking:**
- Cost in CRC: 100,000 CRC
- Cost in USD: $181.82
- Current BTC value: $175.63 (0.0025 × $70,253.98)

All transactions are now in your database with proper cost basis at each step. This will be helpful for tax reporting!

**You:** Excellent! That's exactly what I needed.

---

### Use Case 4: Weekly DCA (Dollar-Cost Averaging)

**Scenario:** Every week you buy a fixed amount of Bitcoin. You want Claude to help track this consistently.

**Conversation:**

**You:** It's Sunday, time for my weekly DCA. I buy $100 worth of Bitcoin every week on Binance. Can you help me record this?

**Claude:** Absolutely! Let me check the current Bitcoin price and record your DCA purchase.

*[Uses: cryptofolio_get_prices with assets: ["BTC"]*

**Claude:** Current BTC price: $70,253.98

With $100, you can buy:
$100 ÷ $70,253.98 = 0.00142341 BTC

Let me record this purchase.

*[Uses: cryptofolio_record_transaction with:*
- *type: "buy"*
- *asset: "BTC"*
- *quantity: "0.00142341"*
- *price: "70253.98"*
- *account: "Binance"*
- *notes: "Weekly DCA - 2026-02-19"]*

**Claude:** ✅ Weekly DCA recorded!

**Purchase Details:**
- Date: February 19, 2026
- Amount: 0.00142341 BTC
- Price: $70,253.98
- Cost: $100.00
- Account: Binance

**Updated BTC Holdings:**
- Total BTC: 0.19263341
- Average Cost: $87,456.23
- Current Value: $13,537.60

This is DCA purchase #12 this year. Your consistent buying strategy is working well! Would you like me to analyze your DCA performance over time?

**You:** Yes, show me my DCA performance.

**Claude:** I'll analyze your Bitcoin holdings with a focus on your 2026 purchases.

*[Uses: cryptofolio_analyze_asset with asset: "BTC", year: 2026]*

**Claude:** **Bitcoin DCA Analysis (2026)**

**Strategy Overview:**
- Total Purchases: 12 (weekly)
- Total Invested: $1,200.00
- Average Purchase Price: $69,234.45
- Current BTC Price: $70,253.98

**Performance:**
- BTC Accumulated (2026): 0.01733521 BTC
- Current Value: $1,218.12
- Gain: +$18.12 (+1.51%)

**Purchase Price Range:**
- Lowest Buy: $64,500.00 (January 15)
- Highest Buy: $72,800.00 (February 5)
- Average: $69,234.45

**DCA Benefit:**
Your average cost of $69,234.45 is 1.5% below the current price. By buying consistently regardless of price, you've smoothed out volatility and accumulated 0.01733521 BTC at a favorable average.

Keep up the disciplined DCA strategy! 🎯

---

### Use Case 5: Moving to Cold Storage

**Scenario:** You've accumulated enough Bitcoin on Binance and want to transfer it to your Ledger hardware wallet for security.

**Conversation:**

**You:** I want to move 0.15 BTC from Binance to my Ledger for cold storage. Help me track this.

**Claude:** Good security practice! Let me record this transfer from Binance to your Ledger. I'll need to account for the network fee. What was the transaction fee?

**You:** 0.00005 BTC network fee.

**Claude:** Got it. Recording the transfer now.

*[Uses: cryptofolio_record_transaction with:*
- *type: "transfer"*
- *asset: "BTC"*
- *quantity: "0.15"*
- *from_account: "Binance"*
- *to_account: "Ledger Nano X"*
- *fee: "0.00005"*
- *notes: "Moving to cold storage for security"]*

**Claude:** ✅ Transfer recorded successfully!

**Transfer Summary:**
- From: Binance (hot wallet)
- To: Ledger Nano X (cold storage)
- Amount Sent: 0.15 BTC
- Network Fee: 0.00005 BTC
- Amount Received: 0.14995 BTC

**Updated Holdings:**

**Binance:**
- BTC: 0.04263341 (was 0.19263341)
- Value: $2,995.67

**Ledger Nano X:**
- BTC: 0.24107651 (was 0.09112651)
- Value: $16,936.23

**Security Improvement:**
- Cold Storage: 85% of BTC holdings
- Hot Wallet: 15% of BTC holdings

Good move! The majority of your Bitcoin is now in secure cold storage. Your cost basis has been preserved across the transfer, so your P&L tracking remains accurate.

**You:** Perfect. Can you verify the Binance balance is correct?

**Claude:** I'll sync with Binance API to verify.

*[Uses: cryptofolio_sync_exchange with account: "Binance"]*

**Claude:** ✅ Binance sync complete!

**Verified Holdings:**
- BTC: 0.04263341 ✓ (matches recorded amount)
- ETH: 2.4594
- USDT: 500.00

Your recorded transfer is accurate! The Binance balance matches perfectly.

---

### Use Case 6: Tax Season Preparation

**Scenario:** It's tax time and you need to export all your 2024 transactions for your accountant.

**Conversation:**

**You:** I need to export all my 2024 cryptocurrency transactions for taxes. Can you help?

**Claude:** Absolutely! I'll export all your 2024 transactions to a CSV file that your accountant can use.

*[Uses: cryptofolio_export_transactions with year: 2024]*

**Claude:** ✅ Export complete!

**Tax Export Summary:**
- **File:** `/Users/you/.config/cryptofolio/exports/transactions_2024.csv`
- **Transactions:** 127 records
- **Date Range:** 2024-01-01 to 2024-12-31

**Transaction Breakdown:**
- Buy: 52 transactions
- Sell: 8 transactions
- Transfer: 45 transactions
- Swap: 22 transactions

**Assets Traded:**
- BTC: 67 transactions
- ETH: 34 transactions
- USDT: 18 transactions
- Other: 8 transactions

The CSV file includes all necessary fields for tax reporting:
- Date & Time
- Transaction Type
- Asset
- Quantity
- Price (USD)
- Fee
- Account
- Cost Basis
- Notes

Would you like me to also export specific assets or accounts separately?

**You:** Yes, can you export just my Bitcoin trades for 2024?

**Claude:** I'll export BTC-specific transactions for 2024.

*[Uses: cryptofolio_export_transactions with asset: "BTC", year: 2024]*

**Claude:** ✅ BTC export complete!

**File:** `/Users/you/.config/cryptofolio/exports/btc_transactions_2024.csv`
**Records:** 67 BTC transactions

This file contains only Bitcoin transactions, making it easier for your accountant to calculate BTC-specific capital gains.

**You:** Great! One more thing - what were my realized gains in 2024?

**Claude:** Let me analyze your 2024 transactions to calculate realized gains.

*[Uses: cryptofolio_list_transactions with year: 2024]*

**Claude:** Based on your 2024 transactions:

**Realized Gains (2024):**
- **Sell Transactions:** 8
- **Total Proceeds:** $15,420.00
- **Cost Basis:** $12,800.00
- **Realized Gain:** $2,620.00

**Note:** This is a simplified calculation. For accurate tax reporting, consult with your tax professional. They may use different cost basis methods (FIFO, LIFO, specific identification) that could affect your actual tax liability.

**Recommendation:** Provide both CSV files to your accountant along with this summary.

---

### Use Case 7: Portfolio Analysis & Rebalancing

**Scenario:** You want to understand your portfolio allocation and decide if you should rebalance.

**Conversation:**

**You:** Can you analyze my portfolio and show me the asset allocation?

**Claude:** I'll analyze your complete portfolio for you.

*[Uses: cryptofolio_get_portfolio]*

**Claude:** **Portfolio Analysis**

**Total Portfolio Value:** $61,442.89

**Asset Allocation:**

| Asset | Quantity | Value | % of Portfolio | Unrealized P&L |
|-------|----------|-------|----------------|----------------|
| BTC | 0.28371 | $19,931.90 | 32.4% | +$7,431.90 (+59.5%) |
| ETH | 2.4594 | $5,136.11 | 8.4% | +$1,136.11 (+28.4%) |
| USDT | 500.00 | $500.00 | 0.8% | $0.00 (0%) |
| NIGHT | 26,016.95 | $1,396.97 | 2.3% | — |
| Cash (USD) | — | $34,477.91 | 56.1% | — |

**Account Distribution:**
- Binance (Hot): $10,035.65 (16.3%)
- Ledger (Cold): $16,936.23 (27.6%)
- Bank Accounts: $34,471.01 (56.1%)

**Observations:**
1. **Heavy Cash Position:** 56% in cash/fiat (mainly USD and CRC)
2. **Bitcoin Dominant:** BTC is your largest crypto holding (32.4%)
3. **Ethereum Secondary:** ETH is 8.4% of portfolio
4. **Alt Exposure:** Minimal (NIGHT is 2.3%)

**You:** That's a lot of cash. Should I deploy some of it into Bitcoin?

**Claude:** That's a strategic question that depends on your investment goals and risk tolerance, but I can help you model different scenarios:

**Scenario 1: Deploy 50% of cash into BTC**
- Amount to invest: $17,238.96
- BTC to acquire: ~0.2453 BTC (at current price $70,253.98)
- New BTC allocation: Would increase from 32.4% to ~45.2%
- New cash allocation: Would decrease from 56.1% to ~28.1%

**Scenario 2: Deploy 25% into BTC, 25% into ETH**
- BTC investment: $8,619.48 → ~0.1227 BTC
- ETH investment: $8,619.48 → ~4.1268 ETH
- New BTC allocation: ~38.4%
- New ETH allocation: ~16.2%
- New cash allocation: ~28.1%

Would you like me to record any purchases? Remember: This is not financial advice. Consider your risk tolerance, time horizon, and overall financial situation before making investment decisions.

**You:** Let's go with scenario 1. I'll buy $17,000 worth of BTC on Binance now.

**Claude:** I'll record that purchase. First, let me get the current BTC price.

*[Uses: cryptofolio_get_prices with assets: ["BTC"]*

**Claude:** Current BTC price: $70,253.98

Recording your purchase of $17,000 worth of BTC...

*[Uses: cryptofolio_record_transaction]*

**Claude:** ✅ Purchase recorded!

You've deployed 49.3% of your cash into Bitcoin. Your new portfolio allocation:
- BTC: ~44.8%
- Cash: ~28.4%
- Other crypto: ~9.8%

Much more balanced! Would you like me to sync Binance to verify the purchase?

---

### Use Case 8: Real-Time Market Monitoring

**Scenario:** Bitcoin is volatile today and you want Claude to help you monitor the price movements.

**Conversation:**

**You:** Bitcoin seems volatile today. What's happening?

**Claude:** Let me check the current market data for Bitcoin.

*[Uses: cryptofolio_get_market_data with asset: "BTC"]*

**Claude:** **Bitcoin Market Data (Real-Time)**

**Current Price:** $70,253.98

**24-Hour Statistics:**
- Change: +$2,145.00 (+2.25%)
- High: $71,500.00
- Low: $68,000.00
- Range: $3,500.00 (4.89%)
- Volume: 12,543.57 BTC ($881,236,432)

**Analysis:**
Bitcoin is experiencing moderate volatility today with a $3,500 range (~5%). The overall trend is positive (+2.25%) with the current price near the middle of the day's range.

**Impact on Your Holdings:**
- Your BTC: 0.28371 BTC
- Current Value: $19,931.90
- 24h Change: +$638.35 (+3.3%)

The volatility is typical for Bitcoin. Your holdings gained $638 in the last 24 hours.

**You:** If it drops to $68,000, how much would I lose?

**Claude:** Let me calculate that for you:

**Scenario: BTC drops to $68,000**
- Current Price: $70,253.98
- Target Price: $68,000.00
- Price Drop: -$2,253.98 (-3.21%)

**Impact on Your Portfolio:**
- Current BTC Value: $19,931.90
- Value at $68,000: $19,292.28
- Paper Loss: -$639.62 (-3.21%)

**Portfolio Impact:**
- Total Portfolio: $61,442.89
- After BTC drop: $60,803.27
- Overall Impact: -1.04%

Given your diversified portfolio (BTC is 32.4%), a 3.2% drop in Bitcoin would only affect your total portfolio by ~1%.

**You:** That's reassuring. I'll hold.

**Claude:** Good decision! Your cost basis on BTC is $62,500.00, so even at $68,000 you'd still be up 8.8%. No need to panic sell during normal volatility.

---

## Best Practices

### Effective Communication with Claude

#### ✅ Do's

**Be Specific:**
- ✅ "I bought 0.1 BTC at $95,000 on Binance today"
- ❌ "I bought some Bitcoin"

**Provide Context:**
- ✅ "This is my weekly DCA purchase"
- ❌ Just stating the transaction without context

**Ask Follow-Up Questions:**
- ✅ "Can you verify this by syncing Binance?"
- ✅ "What's my new average cost basis?"

**Request Analysis:**
- ✅ "Analyze my BTC holdings over the past year"
- ✅ "Show me my best and worst performing assets"

#### ❌ Don'ts

**Don't Be Vague:**
- ❌ "What's my crypto worth?"
  - Better: "What's my total portfolio value?"

**Don't Assume Claude Remembers Past Sessions:**
- ❌ "Record that BTC purchase from yesterday"
  - Better: "I bought 0.1 BTC yesterday at $94,500 on Binance, please record it"

**Don't Skip Important Details:**
- ❌ "I transferred some BTC to my Ledger"
  - Better: "I transferred 0.15 BTC from Binance to Ledger with 0.00005 BTC fee"

### Security Best Practices

#### Protect Your API Keys

❌ **Never** ask Claude to:
- Show your Binance API keys
- Modify configuration files directly
- Access system settings

✅ **Safe operations:**
- Sync exchange balances (uses existing API keys)
- View portfolio (read-only)
- Record transactions (local database only)

#### Verify Large Transactions

**Before recording significant purchases/sales:**
1. Ask Claude to show current price
2. Calculate expected quantity
3. Verify the math
4. Record the transaction
5. Sync exchange to confirm

**Example:**
```
You: "I'm about to buy $10,000 of BTC. What should I get?"
Claude: "At current price $70,253.98, you'll get 0.1423 BTC"
You: "I got 0.1420 BTC after fees. Record this."
```

### Data Hygiene

#### Regular Syncs

**Recommendation:** Sync exchange accounts weekly
```
You: "Sync all my exchange accounts"
```

This ensures your recorded holdings match actual balances and catches any missed transactions.

#### Transaction Notes

**Always add context to transactions:**
- DCA purchases: "Weekly DCA - 2026-02-19"
- Transfers: "Moving to cold storage"
- Swaps: "Portfolio rebalancing"
- Sales: "Taking profits at ATH"

This helps with:
- Tax preparation
- Performance analysis
- Understanding your strategy over time

### Conversation Continuity

#### Start of Month Routine

```
You: "Hi Claude, it's the start of a new month. Can you:
1. Show me last month's portfolio performance
2. List my top 5 transactions
3. Check current allocation vs target (60% BTC, 30% ETH, 10% cash)"
```

#### Weekly Check-In

```
You: "Weekly check-in:
1. What's my portfolio value?
2. What are the prices of BTC and ETH?
3. Record my weekly DCA if it's Sunday"
```

---

## Troubleshooting

### Common Issues

#### Issue 1: Claude Says Tool is Unavailable

**Symptoms:**
```
Claude: "I don't have access to the cryptofolio_get_portfolio tool"
```

**Solutions:**
1. **Check MCP Server Connection:**
   - Look for MCP server indicator in Claude Desktop
   - Restart Claude Desktop

2. **Verify Configuration:**
   ```bash
   cat ~/Library/Application\ Support/Claude/claude_desktop_config.json
   ```
   Should show cryptofolio in mcpServers

3. **Check MCP Server Installation:**
   ```bash
   cryptofolio-mcp --version
   ```

4. **View Claude Desktop Logs:**
   - macOS: `~/Library/Logs/Claude/`
   - Look for MCP connection errors

#### Issue 2: Tool Returns Error

**Symptoms:**
```
Claude: "I encountered an error: Account 'Binance' not found"
```

**Solutions:**
1. **List Available Accounts:**
   ```
   You: "What accounts do I have configured?"
   ```

2. **Create Missing Account:**
   ```
   You: "I need to create a Binance account"
   ```
   Then use Cryptofolio CLI directly:
   ```bash
   cryptofolio account add "Binance" --type exchange --category trading --sync
   ```

3. **Verify Cryptofolio CLI Works:**
   ```bash
   cryptofolio portfolio --json
   ```
   If this fails, MCP server will also fail

#### Issue 3: Sync Fails

**Symptoms:**
```
Claude: "Sync failed: API authentication error"
```

**Solutions:**
1. **Check API Keys:**
   ```bash
   cryptofolio config show
   ```
   Verify binance.api_key and binance.api_secret are set

2. **Test API Keys:**
   ```bash
   cryptofolio sync --account "Binance"
   ```

3. **Reconfigure API Keys:**
   ```bash
   cryptofolio config set-secret binance.api_key
   cryptofolio config set-secret binance.api_secret
   ```

#### Issue 4: Slow Responses

**Symptoms:**
- Claude takes >10 seconds to respond
- Tools timing out

**Solutions:**
1. **Check Cryptofolio Database:**
   ```bash
   sqlite3 ~/.config/cryptofolio/database.sqlite "PRAGMA integrity_check;"
   ```

2. **Optimize Database:**
   ```bash
   sqlite3 ~/.config/cryptofolio/database.sqlite "VACUUM;"
   ```

3. **Check System Resources:**
   - Node.js process using too much memory?
   - Too many MCP servers running?

#### Issue 5: Incorrect Data

**Symptoms:**
```
Claude: "Your portfolio value is $61,442.89"
You: "That doesn't seem right"
```

**Solutions:**
1. **Sync Exchange Accounts:**
   ```
   You: "Sync all exchange accounts"
   ```

2. **List Holdings:**
   ```
   You: "Show me all my holdings by account"
   ```

3. **Check Recent Transactions:**
   ```
   You: "Show my last 10 transactions"
   ```

4. **Verify in CLI:**
   ```bash
   cryptofolio portfolio
   cryptofolio holdings list
   ```

### Getting Help

#### Debug Mode

Enable debug logging in MCP server:

```bash
# Set environment variable before starting Claude Desktop
export CRYPTOFOLIO_MCP_DEBUG=true
```

Then check logs for detailed tool execution information.

#### Report Issues

If you encounter bugs:

1. **Gather Information:**
   - Claude Desktop version
   - Cryptofolio version: `cryptofolio --version`
   - MCP server version: `cryptofolio-mcp --version`
   - Error message from Claude
   - MCP server logs

2. **Create GitHub Issue:**
   - Repository: https://github.com/yourusername/cryptofolio-mcp
   - Include all information from step 1
   - Describe expected vs actual behavior

3. **Community Support:**
   - GitHub Discussions
   - Discord server (link in README)

---

## Advanced Usage

### Custom Workflows

#### Automated Weekly Report

Create a weekly ritual:

```
You: "Weekly report for February 12-19:
1. Portfolio value vs last week
2. Best performing asset
3. Total unrealized P&L
4. Record my weekly DCA"
```

Claude will execute all four tasks sequentially.

#### Multi-Step Analysis

```
You: "I want to analyze if I should take profits on Bitcoin:
1. Show my BTC cost basis
2. Calculate tax implications if I sell 25% at current price
3. Show BTC's performance over the last 3 months
4. What would my portfolio allocation be after selling?"
```

#### Portfolio Rebalancing Assistant

```
You: "Help me rebalance to 60% BTC, 30% ETH, 10% stablecoins:
1. Show current allocation
2. Calculate required trades
3. Draft the transactions (don't execute yet)
4. Estimate total fees"
```

### Integration with Other Tools

#### Morning Briefing

Combine Cryptofolio with other MCP tools:

```
You: "Morning briefing:
1. Check my Cryptofolio portfolio
2. Get latest crypto news (using news MCP tool)
3. Check my calendar for today (using calendar MCP tool)
4. Summarize anything urgent"
```

#### Investment Journal

```
You: "Create an investment journal entry:
1. What's my current portfolio value?
2. What transactions did I make this week?
3. What's my YTD performance?
Save this to my Notes app"
```

---

## Privacy & Data

### What Data is Accessed?

The Cryptofolio MCP server accesses:
- ✅ Portfolio holdings (read-only)
- ✅ Transaction history (read-only)
- ✅ Account information (read-only)
- ✅ Current prices (via Binance API)
- ✅ Exchange balances (via Binance API when syncing)

### What Data is NOT Accessed?

- ❌ API keys (stored in Cryptofolio config, not exposed)
- ❌ Private keys / wallet seeds
- ❌ Personal information beyond what's in Cryptofolio

### Data Storage

- **Local Only:** All portfolio data stays on your machine
- **No Cloud Sync:** MCP server doesn't send data to external servers
- **Claude Desktop:** Conversation history stored locally by Claude Desktop
- **Anthropic:** Claude processes your requests but doesn't store portfolio data

### Best Practices

1. **Don't Share Screenshots:** Portfolio values are private
2. **Use Secure Device:** Only use MCP on trusted computers
3. **Regular Backups:** Back up `~/.config/cryptofolio/` regularly
4. **Log Out:** Quit Claude Desktop when not in use

---

## FAQ

### Q: Can Claude automatically trade for me?

**A:** No. The MCP server is read-only for exchange interactions. Claude can:
- ✅ Record transactions you've made
- ✅ Sync balances from exchanges
- ❌ Execute trades on your behalf
- ❌ Withdraw funds

This is intentional for security.

### Q: Does this work with exchanges other than Binance?

**A:** Currently, Cryptofolio v0.2.0 only integrates with Binance for automatic syncing. However, you can manually track holdings from any exchange (Coinbase, Kraken, etc.) by recording transactions.

**Coming in v0.3:**
- CoinGecko portfolio import
- CoinMarketCap portfolio import

### Q: How much does this cost?

**A:**
- **Cryptofolio:** Free (MIT License)
- **Cryptofolio MCP Server:** Free (MIT License)
- **Claude Desktop:** Requires Claude Pro subscription ($20/month)
- **Binance API:** Free (no additional cost)

### Q: Can I use this on mobile?

**A:** Not yet. The MCP protocol currently only works with Claude Desktop (macOS/Windows/Linux). Mobile support is not available as of February 2026.

### Q: What if I don't have any crypto yet?

**A:** You can still use Cryptofolio MCP to:
- Check current cryptocurrency prices
- Learn about portfolio management
- Track paper trading / testnet activities
- Explore features before investing real money

Use Binance testnet mode:
```bash
cryptofolio config use-testnet
```

### Q: Is my API key safe?

**A:** Yes, with caveats:
- ✅ MCP server doesn't access or expose API keys
- ✅ Keys stored in Cryptofolio config with 0600 permissions
- ✅ MCP server runs locally (no network exposure)
- ⚠️ v0.2.0 stores keys in plaintext (encrypted keychain coming in v0.3)
- ⚠️ **Always use READ-ONLY API keys**

### Q: Can I contribute to the MCP server?

**A:** Yes! The Cryptofolio MCP server is open source:
- Repository: https://github.com/yourusername/cryptofolio-mcp
- Contributing guide: CONTRIBUTING.md
- Issues: https://github.com/yourusername/cryptofolio-mcp/issues

We welcome:
- Bug reports
- Feature requests
- Code contributions
- Documentation improvements

---

## What's Next?

### Roadmap

**v1.1 (Q2 2026):**
- Portfolio alerts ("notify me if BTC drops below $65k")
- Scheduled reports (daily/weekly summaries)
- More detailed analytics

**v1.2 (Q3 2026):**
- Integration with Cryptofolio v0.3 features:
  - CoinGecko import
  - CoinMarketCap import
  - CSV report generation

**v2.0 (Q4 2026):**
- Multi-user support
- Portfolio sharing (read-only links)
- Advanced tax reporting tools

### Feedback

We'd love to hear from you!

- **Feature Requests:** https://github.com/yourusername/cryptofolio-mcp/discussions
- **Bug Reports:** https://github.com/yourusername/cryptofolio-mcp/issues
- **General Feedback:** hello@cryptofolio.dev

---

## Support

### Resources

- **Documentation:** https://docs.cryptofolio.dev
- **GitHub:** https://github.com/yourusername/cryptofolio
- **Discord:** https://discord.gg/cryptofolio

### Getting Help

1. Check [Troubleshooting](#troubleshooting) section
2. Search [GitHub Issues](https://github.com/yourusername/cryptofolio-mcp/issues)
3. Ask in [GitHub Discussions](https://github.com/yourusername/cryptofolio-mcp/discussions)
4. Join our [Discord community](https://discord.gg/cryptofolio)

---

**Happy tracking! 🚀**

*Built with ❤️ by the Cryptofolio community*
*Powered by Claude Desktop and MCP*
