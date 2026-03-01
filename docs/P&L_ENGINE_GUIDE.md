# 📊 Understanding the P&L (Profit & Loss) Engine

**A beginner's guide to understanding your crypto investment performance**

---

## Table of Contents
1. [What is the P&L Engine?](#what-is-the-pl-engine)
2. [Why You Need This (Even Without Taxes)](#why-you-need-this-even-without-taxes)
3. [Core Concepts Explained Simply](#core-concepts-explained-simply)
4. [Real-World Examples](#real-world-examples)
5. [Use Cases by User Type](#use-cases-by-user-type)
6. [How It Works](#how-it-works)
7. [Getting Started](#getting-started)

---

## What is the P&L Engine?

The **Profit & Loss (P&L) Engine** is an automatic calculator that tracks:
- ✅ How much profit or loss you've made on each trade
- ✅ Which specific coins you sold (from which purchase)
- ✅ Your overall portfolio performance
- ✅ A complete history of all your gains and losses

**Think of it as:** Your personal investment analyst that never sleeps.

### The Problem It Solves

Imagine this conversation:

> **You:** "I made $10,000 trading crypto this year!"
> **Friend:** "Nice! What's your actual profit after accounting for all your purchases?"
> **You:** "Uh... I'd have to check my spreadsheet... actually, I'm not sure..." 😅

The P&L Engine gives you the real answer instantly.

---

## Why You Need This (Even Without Taxes)

### 🌍 For Users in Tax-Free Jurisdictions

Even if your country **doesn't tax crypto** (lucky you! 🎉), this feature is incredibly valuable:

#### 1. **Know Your Real Performance**

```
Without P&L Engine:
"I have 5 BTC now, and Bitcoin went up 50% this year!"

With P&L Engine:
"I invested $100,000 total, current value is $150,000.
My actual return is +50%, but I also sold 2 BTC at a loss earlier.
My real profit considering all trades is +32%."
```

**Why it matters:** Market price ≠ your personal performance!

#### 2. **Smart Selling Decisions**

```
Scenario: Bitcoin is at $50,000. Should I sell?

Without P&L Engine:
"BTC is up! I'll sell some."

With P&L Engine:
"Let me check which batch to sell:
 - Batch 1: Bought @ $30,000 → $20k profit ✅
 - Batch 2: Bought @ $45,000 → $5k profit
 - Batch 3: Bought @ $55,000 → $5k loss ❌

I'll sell Batch 1 to lock in maximum profit!"
```

**Why it matters:** Sell strategically, not emotionally!

#### 3. **Portfolio Insights**

```bash
$ cryptofolio pnl by-asset

Your Performance by Asset:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Bitcoin (BTC)
  Invested:     $100,000
  Current:      $150,000
  Profit:       +$50,000 (+50%) ✅

Ethereum (ETH)
  Invested:     $50,000
  Current:      $40,000
  Loss:         -$10,000 (-20%) ⚠️

Solana (SOL)
  Invested:     $10,000
  Current:      $25,000
  Profit:       +$15,000 (+150%) 🚀
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Total Portfolio:
  Invested:     $160,000
  Current:      $215,000
  Net Profit:   +$55,000 (+34.4%)
```

**Why it matters:** See what's working and what's not!

#### 4. **Financial Planning**

Know exactly:
- 💰 How much profit you can withdraw without touching your principal
- 📊 Your true ROI (Return on Investment)
- 📈 Historical performance trends
- 🎯 Whether you're meeting your investment goals

#### 5. **Professional Investor Mindset**

Track metrics like a pro:
- **Win Rate:** What % of your trades are profitable?
- **Average Gain:** How much do you make on winners?
- **Average Loss:** How much do you lose on losers?
- **Best/Worst Performers:** Which assets drive your returns?

### 💼 For Users Who Pay Crypto Taxes

All the above benefits, PLUS:

#### 6. **Accurate Tax Filing**
```bash
$ cryptofolio pnl summary --year 2024

2024 Tax Summary
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Short-term gains:    $15,000
Long-term gains:     $35,000
Total gains:         $50,000
Losses:              -$5,000
Net taxable gain:    $45,000

Estimated tax (25%): $11,250
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

$ cryptofolio tx export tax-report-2024.csv
✓ Report ready for your accountant
```

#### 7. **Tax Optimization**
- Choose FIFO vs LIFO to minimize taxes
- Track holding periods (short-term vs long-term capital gains)
- Harvest losses to offset gains
- Plan year-end tax strategy

#### 8. **Audit Protection**
- Complete transaction history
- Clear cost basis for every sale
- Exportable reports for tax authorities
- Peace of mind during tax season

---

## Core Concepts Explained Simply

### 1. Tax Lots (Investment Batches)

**What is it?**
A "tax lot" is simply a record of when you bought crypto.

**Example:**
```
Jan 1:  Buy 1 BTC @ $30,000  → Tax Lot #1
Feb 1:  Buy 1 BTC @ $40,000  → Tax Lot #2
Mar 1:  Buy 1 BTC @ $35,000  → Tax Lot #3
```

**Why track this?**
When you sell 0.5 BTC, the software knows exactly which batch you're selling from!

**Real-world analogy:**
Like tracking multiple savings accounts. If you have three accounts and withdraw $100, you need to know which account it came from.

---

### 2. FIFO vs LIFO (Which Coins Do I Sell First?)

#### FIFO: First In, First Out
**Rule:** Sell your **oldest** coins first
**Analogy:** Like a grocery store - old milk at the front, fresh milk at the back

**Example:**
```
You own:
  Lot 1: 1 BTC bought @ $30,000 (Jan)
  Lot 2: 1 BTC bought @ $40,000 (Feb)

You sell 0.5 BTC @ $50,000

FIFO sells from Lot 1 (oldest):
  Cost:   $15,000 (0.5 × $30,000)
  Sale:   $25,000 (0.5 × $50,000)
  Profit: $10,000 ✅
```

**When to use:**
- ✅ Tax-free countries (doesn't matter!)
- ✅ Long-term holding strategy
- ✅ Most common method worldwide

#### LIFO: Last In, First Out
**Rule:** Sell your **newest** coins first
**Analogy:** Like a stack of plates - take from the top

**Example:**
```
You own:
  Lot 1: 1 BTC bought @ $30,000 (Jan)
  Lot 2: 1 BTC bought @ $40,000 (Feb)

You sell 0.5 BTC @ $50,000

LIFO sells from Lot 2 (newest):
  Cost:   $20,000 (0.5 × $40,000)
  Sale:   $25,000 (0.5 × $50,000)
  Profit: $5,000 ✅ (smaller profit = less tax)
```

**When to use:**
- ✅ Tax optimization (minimize current year taxes)
- ✅ Rising market (newer coins have higher cost basis)
- ✅ Short-term trading

**Important:** Most tax jurisdictions require you to pick ONE method and stick with it per asset.

---

### 3. Realized vs Unrealized Gains

#### Unrealized Gains (Paper Profits)
**What is it?** Profit you *could* make if you sold now
**Status:** You still own the crypto
**Tax implication:** No tax owed (yet)

**Example:**
```
You bought:  1 BTC @ $30,000
Now worth:   1 BTC @ $50,000
Unrealized:  $20,000 profit

You haven't sold, so:
  ✓ No tax owed
  ✓ No real profit yet
  ✓ Price could go up or down
```

#### Realized Gains (Actual Profits)
**What is it?** Profit you *actually* made by selling
**Status:** You sold the crypto
**Tax implication:** Tax owed (in most countries)

**Example:**
```
You bought:  1 BTC @ $30,000
You sold:    1 BTC @ $50,000
Realized:    $20,000 profit

You sold, so:
  ✓ Real cash profit
  ✓ Tax owed (if applicable)
  ✓ Locked in forever
```

**The P&L Engine tracks both:**
```bash
$ cryptofolio pnl unrealized
Current Holdings: $100,000 unrealized gain

$ cryptofolio pnl realized
2024 Sales: $45,000 realized gain
```

---

### 4. Cost Basis

**What is it?**
The original price you paid for your crypto (plus fees).

**Why it matters:**
Your profit = Sale Price - Cost Basis

**Example:**
```
Purchase: 1 BTC @ $30,000 + $50 fee = $30,050 cost basis
Sale:     1 BTC @ $50,000 - $75 fee = $49,925 sale price
Profit:   $49,925 - $30,050 = $19,875
```

**The P&L Engine:**
- ✅ Tracks cost basis automatically
- ✅ Includes all fees
- ✅ Handles complex scenarios (multiple purchases, partial sales)

---

## Real-World Examples

### Example 1: Active Trader

**Profile:** Sarah, day trader in Portugal (crypto tax-free)

**Challenge:**
"I make 50-100 trades per month. I have no idea if I'm actually profitable!"

**Solution with P&L Engine:**
```bash
$ cryptofolio pnl summary --month 2024-03

March 2024 Trading Summary
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Total trades:        87
Winning trades:      52 (60% win rate) ✅
Losing trades:       35 (40% loss rate)

Average win:         $450
Average loss:        -$200
Largest win:         $2,100
Largest loss:        -$800

Net profit:          $16,400 (+12.3% return)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Best performer:      SOL (+$5,400)
Worst performer:     DOGE (-$1,200)
```

**Benefit:**
Sarah now knows exactly which strategies work and can improve her win rate!

---

### Example 2: Long-Term Holder

**Profile:** John, HODL investor in El Salvador (crypto tax-free)

**Challenge:**
"I bought Bitcoin in 2020, 2021, and 2023. I want to sell some but keep my best-performing lots."

**Solution with P&L Engine:**
```bash
$ cryptofolio pnl unrealized --asset BTC

Your Bitcoin Holdings
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Lot 1: 0.5 BTC bought @ $15,000 (2020)
  Current value: $25,000
  Unrealized:    +$17,500 (+233%) 🚀 ← Keep this!

Lot 2: 1.0 BTC bought @ $60,000 (2021)
  Current value: $50,000
  Unrealized:    -$10,000 (-17%) ⚠️ ← Maybe sell?

Lot 3: 0.8 BTC bought @ $30,000 (2023)
  Current value: $40,000
  Unrealized:    +$10,000 (+33%) ✅
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Strategy: Set cost basis method to "Specific ID"
          to manually choose which lot to sell!
```

**Benefit:**
John can strategically sell his worst-performing lot while keeping his 2020 Bitcoin!

---

### Example 3: Tax Compliance User

**Profile:** Maria, investor in Germany (crypto taxed)

**Challenge:**
"Tax season is a nightmare. My accountant charges $500 just to calculate my crypto taxes!"

**Solution with P&L Engine:**
```bash
$ cryptofolio pnl summary --year 2024

2024 Tax Report (Germany)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Held < 1 year (taxable):       $15,000
Held > 1 year (tax-free):      $30,000  ✅

Taxable income:                $15,000
Tax rate (42%):                $6,300
Tax owed:                      $6,300
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

$ cryptofolio tx export tax-report-2024-germany.csv
✓ Ready for Elster (German tax software)

Accountant fee saved: $500 💰
```

**Benefit:**
Maria files her taxes in 30 minutes instead of paying her accountant!

---

### Example 4: Portfolio Rebalancing

**Profile:** Alex, diversified investor in Singapore (crypto tax-free)

**Challenge:**
"I want to rebalance my portfolio but don't know which assets to sell."

**Solution with P&L Engine:**
```bash
$ cryptofolio pnl by-asset

Portfolio Performance
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Asset    Invested   Current    Gain/Loss    % Return
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
BTC      $50,000    $75,000    +$25,000     +50%  ✅
ETH      $30,000    $45,000    +$15,000     +50%  ✅
SOL      $10,000    $35,000    +$25,000     +250% 🚀
ADA      $10,000    $6,000     -$4,000      -40%  ⚠️
DOGE     $5,000     $3,000     -$2,000      -40%  ⚠️
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Total    $105,000   $164,000   +$59,000     +56%

Strategy: SOL is 21% of portfolio (target: 10%)
          → Sell $18,000 of SOL
          → Buy more BTC/ETH to rebalance
```

**Benefit:**
Alex makes data-driven decisions instead of guessing!

---

## Use Cases by User Type

### 🆓 Crypto Tax-Free Countries
*Users in: Portugal, Germany (>1yr), Singapore, Switzerland, etc.*

**Primary Benefits:**
1. ✅ **Performance Tracking** - Know your real returns
2. ✅ **Investment Strategy** - Data-driven decisions
3. ✅ **Portfolio Analysis** - Identify winners/losers
4. ✅ **Financial Planning** - Track profit vs principal
5. ✅ **Professional Reports** - Impress your financial advisor

**Key Commands:**
```bash
# Check overall performance
cryptofolio pnl summary

# See unrealized gains (what if I sold now?)
cryptofolio pnl unrealized

# Compare assets
cryptofolio pnl by-asset

# Track specific account
cryptofolio pnl summary --account Binance
```

---

### 💼 Crypto Tax Countries
*Users in: USA, UK, Canada, Australia, most of Europe, etc.*

**Primary Benefits:**
1. ✅ **Tax Compliance** - Accurate tax filing
2. ✅ **Tax Optimization** - Choose FIFO/LIFO strategically
3. ✅ **Audit Protection** - Complete transaction history
4. ✅ **Loss Harvesting** - Offset gains with losses
5. ✅ **Holding Period Tracking** - Short vs long-term gains
6. ✅ **Year-End Planning** - Know tax liability in advance

**Key Commands:**
```bash
# Annual tax summary
cryptofolio pnl summary --year 2024

# Export for tax software
cryptofolio tx export tax-2024.csv --year 2024

# Check current year liability
cryptofolio pnl realized --year 2024

# Compare FIFO vs LIFO
cryptofolio pnl summary --method fifo
cryptofolio pnl summary --method lifo
```

---

### 🏢 Professional Traders

**Primary Benefits:**
1. ✅ **Win Rate Analysis** - % of profitable trades
2. ✅ **Risk/Reward Metrics** - Average win vs average loss
3. ✅ **Strategy Testing** - Compare different approaches
4. ✅ **Performance Attribution** - Which assets drive returns?
5. ✅ **Monthly Reports** - Track progress over time

**Key Commands:**
```bash
# Monthly performance
cryptofolio pnl summary --month 2024-03

# Trading statistics
cryptofolio pnl stats

# Best/worst trades
cryptofolio pnl trades --sort profit --limit 10

# Compare months
cryptofolio pnl compare --from 2024-01 --to 2024-03
```

---

### 👴 Long-Term Investors (HODLers)

**Primary Benefits:**
1. ✅ **Lot Tracking** - Know your oldest holdings
2. ✅ **Strategic Selling** - Choose which lot to sell
3. ✅ **Time-Based Analysis** - Holding period returns
4. ✅ **Accumulation Tracking** - DCA (Dollar Cost Averaging) analysis
5. ✅ **Legacy Planning** - Know exact cost basis for heirs

**Key Commands:**
```bash
# See all lots (batches)
cryptofolio pnl lots --asset BTC

# Check oldest holdings
cryptofolio pnl lots --sort date

# Unrealized gains by lot
cryptofolio pnl unrealized --detailed

# Track DCA performance
cryptofolio pnl dca --asset BTC
```

---

## How It Works

### The Automatic Process

```
┌─────────────────────────────────────────────────────────┐
│  Step 1: You Buy Crypto                                 │
└─────────────────────────────────────────────────────────┘
                         ↓
    cryptofolio tx buy BTC 1.0 --price 30000
                         ↓
┌─────────────────────────────────────────────────────────┐
│  P&L Engine Creates a Tax Lot                          │
│  ✓ Lot #1: 1.0 BTC @ $30,000                           │
│  ✓ Stored in database                                   │
│  ✓ Ready for future calculations                        │
└─────────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────┐
│  Step 2: You Sell Crypto                                │
└─────────────────────────────────────────────────────────┘
                         ↓
    cryptofolio tx sell BTC 0.5 --price 50000
                         ↓
┌─────────────────────────────────────────────────────────┐
│  P&L Engine Calculates Automatically                    │
│  1. Which lot to sell from? → Lot #1 (FIFO)            │
│  2. Cost basis? → $15,000 (0.5 × $30,000)              │
│  3. Sale amount? → $25,000 (0.5 × $50,000)             │
│  4. Profit? → $10,000                                   │
│  ✓ Records realized P&L                                 │
│  ✓ Updates remaining lot: 0.5 BTC @ $30,000            │
└─────────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────┐
│  Step 3: You Check Results Anytime                      │
└─────────────────────────────────────────────────────────┘
                         ↓
    cryptofolio pnl summary
                         ↓
         ┌──────────────────────────────┐
         │  Instant Reports:            │
         │  • Realized gains: $10,000   │
         │  • Unrealized gains: $10,000 │
         │  • Total return: +66%        │
         │  • Current holdings: 0.5 BTC │
         └──────────────────────────────┘
```

### Behind the Scenes: Database Tables

The P&L Engine uses two main tables:

#### 1. `tax_lots` Table
Stores each purchase batch:
```sql
| id | asset | quantity | price   | date       | remaining |
|----|-------|----------|---------|------------|-----------|
| 1  | BTC   | 1.0      | 30,000  | 2024-01-01 | 0.5       |
| 2  | BTC   | 1.0      | 40,000  | 2024-02-01 | 1.0       |
| 3  | ETH   | 10.0     | 2,000   | 2024-03-01 | 10.0      |
```

#### 2. `realized_pnl` Table
Records each sale result:
```sql
| id | asset | quantity | cost    | revenue | profit  | date       |
|----|-------|----------|---------|---------|---------|------------|
| 1  | BTC   | 0.5      | 15,000  | 25,000  | 10,000  | 2024-03-15 |
```

**You don't need to understand this!** The software handles everything automatically.

---

## Getting Started

### Step 1: Enable the P&L Engine

The P&L Engine is **automatic**! Just start using Cryptofolio normally:

```bash
# Buy crypto (creates tax lots automatically)
cryptofolio tx buy BTC 1.0 --price 30000 --account Binance

# Sell crypto (calculates P&L automatically)
cryptofolio tx sell BTC 0.5 --price 50000 --account Binance
```

### Step 2: Choose Your Cost Basis Method

**For tax-free countries:** Use FIFO (default) - doesn't matter!
```bash
cryptofolio config set cost_basis_method fifo
```

**For tax countries:** Consult your accountant, but FIFO is most common:
```bash
# Options: fifo, lifo, specific-id
cryptofolio config set cost_basis_method fifo
```

### Step 3: Review Your Performance

```bash
# Quick overview
cryptofolio pnl summary

# Detailed by asset
cryptofolio pnl by-asset

# Check unrealized gains
cryptofolio pnl unrealized

# Tax year summary (if applicable)
cryptofolio pnl summary --year 2024
```

### Step 4: Export Reports

```bash
# CSV for spreadsheet analysis
cryptofolio tx export portfolio-2024.csv

# Tax report (if applicable)
cryptofolio tx export tax-report-2024.csv --year 2024

# Full database backup
cryptofolio tx export backup.sql --format sql
```

---

## Common Questions (FAQ)

### Q1: I don't pay crypto taxes. Is this still useful?

**Absolutely!** The P&L Engine is primarily a **portfolio tracking tool**. Tax reporting is just one feature.

**Benefits without taxes:**
- Know your real investment returns
- Make smart selling decisions
- Track portfolio performance
- Identify winning/losing strategies
- Professional investment reports

Think of it like a fitness tracker - you don't need to be training for a marathon to benefit from knowing your steps!

---

### Q2: What if I already have existing holdings?

**No problem!** Add your historical purchases:

```bash
# Add old purchases (backdate them)
cryptofolio tx buy BTC 2.0 --price 15000 --date 2020-01-15

# The P&L Engine will create lots retroactively
# Future sales will use these for calculations
```

Or import from CSV:
```bash
cryptofolio import transactions-history.csv
```

---

### Q3: Can I change my cost basis method later?

**Yes, but be careful:**
- The software recalculates everything
- In tax countries, you may need to stick with one method
- Changing methods affects reported profits

```bash
# Switch methods
cryptofolio config set cost_basis_method lifo

# Compare before committing
cryptofolio pnl summary --method fifo
cryptofolio pnl summary --method lifo
```

---

### Q4: What about crypto-to-crypto trades?

**Fully supported!** Swaps are treated as:
1. Sell the old crypto (realizes P&L)
2. Buy the new crypto (creates new lot)

```bash
cryptofolio tx swap --from BTC 0.5 --to ETH 10.0
```

The P&L Engine automatically:
- Calculates profit/loss on BTC sale
- Creates new ETH tax lot
- Tracks both transactions

---

### Q5: I made hundreds of trades. How do I track everything?

**That's the beauty of automation!** The P&L Engine handles unlimited trades:

```bash
# Import from exchange
cryptofolio import binance-history.csv

# Or add manually (creates lots automatically)
cryptofolio tx buy BTC 1.0 --price 30000
cryptofolio tx sell BTC 0.5 --price 35000
cryptofolio tx buy BTC 2.0 --price 32000
# ... etc

# View summary instantly
cryptofolio pnl summary
```

---

### Q6: What if I move crypto between my wallets/accounts?

**Transfers don't trigger P&L:**

```bash
# Transfer between accounts (no sale)
cryptofolio tx transfer BTC 1.0 \
  --from Binance \
  --to ColdWallet

# P&L Engine knows this isn't a sale
# Tax lot stays intact, just changes account
```

---

### Q7: Can I track multiple portfolios separately?

**Yes!** Use accounts:

```bash
# Trading portfolio
cryptofolio pnl summary --account Binance

# HODL portfolio
cryptofolio pnl summary --account ColdWallet

# Combined view
cryptofolio pnl summary
```

---

### Q8: What about staking rewards or airdrops?

**Track as income:**

```bash
# Staking reward (creates new lot at current price)
cryptofolio tx income ETH 0.5 --price 2000 --type staking

# Airdrop (creates lot at $0 cost basis)
cryptofolio tx income UNI 400 --price 5 --type airdrop

# When you sell these, P&L is calculated normally
```

---

### Q9: How accurate is this for tax filing?

**Very accurate**, but:
- ✅ Use for tax preparation and estimates
- ✅ Export reports for your accountant
- ⚠️ Always verify with a tax professional
- ⚠️ Tax laws vary by country

The P&L Engine follows standard accounting practices (FIFO/LIFO), but tax rules are complex. Use it as a tool to **save time**, not replace professional advice.

---

### Q10: Can I see individual trade details?

**Absolutely!**

```bash
# List all realized gains/losses
cryptofolio pnl realized --detailed

# Show specific trade
cryptofolio pnl show <transaction-id>

# Export everything
cryptofolio tx export all-trades.csv --detailed
```

---

## Advanced Features

### Specific Identification (Advanced Users)

Instead of FIFO/LIFO, manually choose which lot to sell:

```bash
# See available lots
cryptofolio pnl lots --asset BTC

# Sell specific lot
cryptofolio tx sell BTC 0.5 --lot-id 3
```

**When to use:**
- Tax optimization (sell high-cost lots to minimize gain)
- Strategic hodling (keep your best lots)
- Rebalancing without triggering large gains

---

### Tax Loss Harvesting

Sell losing positions to offset gains:

```bash
# Find your losers
cryptofolio pnl unrealized --filter losses

# Example output:
# ADA: -$5,000 unrealized loss

# Sell to realize the loss
cryptofolio tx sell ADA 1000 --price 0.50

# Now your gains are offset!
cryptofolio pnl summary
# Net gain: $10,000 - $5,000 = $5,000 (lower tax)
```

---

### Wash Sale Awareness

**Note:** In some countries (like USA), if you sell at a loss and rebuy within 30 days, the loss is disallowed.

The P&L Engine can warn you:

```bash
cryptofolio config set wash_sale_warning true

# If you try to rebuy too soon:
cryptofolio tx buy BTC 1.0 --price 30000
⚠️  Warning: Potential wash sale detected
    You sold BTC at a loss 15 days ago
    Consider waiting 15 more days
```

---

### Multi-Currency Support

Track everything in your preferred currency:

```bash
# Set base currency
cryptofolio config set base_currency EUR

# All reports show in EUR
cryptofolio pnl summary
# Realized gains: €45,000 EUR
```

---

## Visual Examples

### Example Report: Summary View

```
cryptofolio pnl summary --year 2024

╔═══════════════════════════════════════════════════════════╗
║            2024 Investment Performance Summary             ║
╚═══════════════════════════════════════════════════════════╝

Portfolio Overview
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Total Invested:        $150,000
Current Value:         $215,000
Unrealized Gain:       $65,000 (+43.3%)

Realized Performance
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Total Sold:            $75,000
Cost Basis:            $50,000
Realized Gain:         $25,000 (+50%)

Tax Information (if applicable)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Short-term gains:      $10,000 (held < 1 year)
Long-term gains:       $15,000 (held > 1 year)
Tax estimate (20%):    $5,000

Overall Performance
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Total Gain (realized + unrealized): $90,000
Total Return:          +60%
```

---

### Example Report: By Asset

```
cryptofolio pnl by-asset

╔═══════════════════════════════════════════════════════════╗
║              Performance by Asset (All Time)               ║
╚═══════════════════════════════════════════════════════════╝

Bitcoin (BTC)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Invested:              $100,000
Current holdings:      2.5 BTC ($125,000)
Sold:                  $50,000
Total value:           $175,000
Net gain:              $75,000 (+75%) ✅

  Realized:            $25,000
  Unrealized:          $50,000

Ethereum (ETH)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Invested:              $50,000
Current holdings:      25 ETH ($62,500)
Sold:                  $25,000
Total value:           $87,500
Net gain:              $37,500 (+75%) ✅

  Realized:            $12,500
  Unrealized:          $25,000

Solana (SOL)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Invested:              $10,000
Current holdings:      500 SOL ($27,500)
Sold:                  $0
Total value:           $27,500
Net gain:              $17,500 (+175%) 🚀

  Realized:            $0
  Unrealized:          $17,500

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Portfolio Total
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Total invested:        $160,000
Total value:           $290,000
Total gain:            $130,000 (+81.25%)
```

---

## Conclusion

The **P&L (Profit & Loss) Engine** transforms Cryptofolio from a simple transaction tracker into a powerful investment analysis tool.

### For Everyone:
✅ Know your real performance
✅ Make data-driven decisions
✅ Track portfolio health
✅ Professional-grade reports

### For Tax Payers:
✅ Accurate tax filing
✅ Tax optimization
✅ Audit protection
✅ Save accountant fees

### The Bottom Line:

Whether you're:
- 🌍 In a tax-free country tracking performance
- 💼 In a tax country filing returns
- 📊 A professional trader analyzing strategy
- 👴 A HODLer planning long-term

The P&L Engine gives you **clarity, control, and confidence** in your crypto investments.

---

## Next Steps

Ready to start using the P&L Engine?

1. **Read the [User Guide](./USER_GUIDE.md)** for detailed commands
2. **Check [Tax Guide](./TAX_GUIDE.md)** for country-specific advice (if applicable)
3. **Join our [Community](https://discord.gg/cryptofolio)** for tips and support

---

**Questions?** Open an issue on [GitHub](https://github.com/yourusername/cryptofolio/issues) or ask in our community!

**Found this helpful?** ⭐ Star the project on GitHub!

---

*Last updated: February 2026*
*Version: 0.3.0 (Phase 3 - P&L Engine)*
