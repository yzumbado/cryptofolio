# Cryptofolio Conversational CLI

**One-Pager: AI-Powered Portfolio Management**

---

## Problem Statement

### Current State
Users must remember exact CLI syntax and flags to interact with their portfolio:
```bash
cryptofolio tx buy BTC 0.1 --account Binance --price 95000 --notes "DCA purchase"
```

### Pain Points

1. **Cognitive Load**: Users must memorize command structures, flag names, and required arguments
2. **Context Switching**: Users think in natural language ("I bought some Bitcoin") but must translate to CLI syntax
3. **Error-Prone**: Typos in flags, forgotten required arguments, wrong order
4. **No Guidance**: CLI doesn't help users discover what they can do or suggest next steps
5. **Transactional**: Each command is isolated; no conversational flow for complex operations

### User Expectation
Users want to interact with their portfolio the way they'd talk to a financial advisor:
> "I bought 0.1 BTC today on Binance for $9,500"

---

## Proposed Solution

### Conversational CLI Mode

An AI-powered interactive shell that understands natural language and executes portfolio operations through guided conversation.

```
$ cryptofolio

  Cryptofolio v0.2.0 - AI Portfolio Assistant
  Portfolio: $61,442.89 (+109.57%)
  Type 'help' for commands or just describe what you want to do.

you> I bought some bitcoin today

  I can help you record that transaction. Let me ask a few questions:

  Which account did you buy it on?
  > Binance

  How much BTC did you buy?
  > 0.1

  What was the price per BTC?
  > 95000

  Here's what I'll record:
    Buy 0.1 BTC @ $95,000.00 on Binance
    Total: $9,500.00

  Confirm? [Y/n] y

  Recorded: buy 0.1 BTC @ $95,000.00 in 'Binance'

you> what's my portfolio looking like?

  PORTFOLIO OVERVIEW
  Total Value:     $70,942.89
  Cost Basis:      $38,817.39
  Unrealized P&L:  +$32,125.50 (+82.76%)

  Your largest holding is BTC (78% of portfolio).
  ETH is up 12% since you bought it.

you> move half my binance btc to cold storage

  I'll help you transfer BTC from Binance to Cold Storage.

  You have 0.19121 BTC on Binance.
  Half would be 0.095605 BTC.

  Transfer 0.095605 BTC from Binance to Cold Storage? [Y/n] y

  Recorded: transfer 0.095605 BTC from 'Binance' to 'Cold Storage'

you> exit
```

---

## High-Level Architecture

```
+------------------------------------------------------------------+
|                         USER INTERFACE                            |
|  +------------------------------------------------------------+  |
|  |                    Interactive Shell                        |  |
|  |  - REPL loop (rustyline/reedline)                          |  |
|  |  - Command history & completion                             |  |
|  |  - Status bar (portfolio value, connection status)          |  |
|  +------------------------------------------------------------+  |
+------------------------------------------------------------------+
                               |
                               v
+------------------------------------------------------------------+
|                      INPUT PROCESSOR                              |
|  +--------------------+    +----------------------------------+  |
|  |  Command Detector  |    |     Natural Language Parser     |  |
|  |                    |    |                                  |  |
|  |  Detects if input  |    |  - Intent classification        |  |
|  |  is a CLI command  |    |  - Entity extraction            |  |
|  |  (price, portfolio)|    |  - Context awareness            |  |
|  +--------------------+    +----------------------------------+  |
|           |                              |                       |
|           v                              v                       |
|  +------------------+         +------------------------+         |
|  | Direct Execution |         |    AI Conversation     |         |
|  | (existing CLI)   |         |    Engine (LLM API)    |         |
|  +------------------+         +------------------------+         |
+------------------------------------------------------------------+
                               |
                               v
+------------------------------------------------------------------+
|                    CONVERSATION MANAGER                           |
|  +------------------------------------------------------------+  |
|  |  - Maintains conversation context                           |  |
|  |  - Tracks pending operations                                |  |
|  |  - Manages confirmation flows                               |  |
|  |  - Handles multi-turn dialogues                             |  |
|  +------------------------------------------------------------+  |
|                               |                                  |
|       +-------------------+---+-------------------+              |
|       v                   v                       v              |
|  +---------+      +--------------+      +------------------+     |
|  | Clarify |      | Confirm      |      | Execute          |     |
|  | (ask ?) |      | (show plan)  |      | (run command)    |     |
|  +---------+      +--------------+      +------------------+     |
+------------------------------------------------------------------+
                               |
                               v
+------------------------------------------------------------------+
|                      EXECUTION LAYER                              |
|  +------------------------------------------------------------+  |
|  |                  Existing Cryptofolio Core                  |  |
|  |  - Account management                                       |  |
|  |  - Holdings tracking                                        |  |
|  |  - Transaction recording                                    |  |
|  |  - Exchange sync (Binance)                                  |  |
|  |  - Portfolio calculations                                   |  |
|  +------------------------------------------------------------+  |
+------------------------------------------------------------------+
```

---

## End-to-End Example: Recording a Bitcoin Purchase

This example traces a complete user interaction through all layers of the system.

### Scenario
User just bought 0.15 BTC on Binance at $94,500 and wants to record it.

---

### Step 1: User Interface Layer

**What the user sees:**
```
$ cryptofolio

  Cryptofolio v0.2.0 - AI Portfolio Assistant
  Portfolio: $61,442.89 (+109.57%)

you> I just bought some bitcoin on binance
```

**What happens internally:**
```rust
// Interactive Shell (REPL)
loop {
    // Display prompt
    let input = readline.readline("you> ")?;

    // User types: "I just bought some bitcoin on binance"
    // Send to Input Processor
    let response = input_processor.process(&input, &context).await?;

    // Display response
    println!("{}", response);
}
```

---

### Step 2: Input Processor Layer

**Command Detector** checks if input matches a CLI command pattern:
```rust
fn is_cli_command(input: &str) -> bool {
    let cli_patterns = ["price", "portfolio", "holdings", "account", "tx", "sync"];
    let first_word = input.split_whitespace().next().unwrap_or("");
    cli_patterns.contains(&first_word.to_lowercase().as_str())
}

// "I just bought some bitcoin on binance" -> false (not a CLI command)
// Falls through to Natural Language Parser
```

**Natural Language Parser** sends to Claude API:
```rust
// System prompt for Claude
let system = r#"
You are a crypto portfolio assistant. Analyze user input and extract:
1. intent: The action the user wants (buy, sell, transfer, view_portfolio, etc.)
2. entities: Structured data (asset, quantity, price, account, etc.)
3. missing: Required fields that weren't provided

Respond in JSON format.
"#;

// Send to Claude API
let response = claude.messages.create(
    model: "claude-sonnet-4-20250514",
    system: system,
    messages: [{ role: "user", content: "I just bought some bitcoin on binance" }]
).await?;
```

**Claude API Response:**
```json
{
  "intent": "record_transaction",
  "transaction_type": "buy",
  "entities": {
    "asset": "BTC",
    "account": "Binance"
  },
  "missing": ["quantity", "price"],
  "confidence": 0.95,
  "clarification_needed": true,
  "next_question": "How much BTC did you buy?"
}
```

---

### Step 3: Conversation Manager Layer

**Conversation Manager** receives the parsed intent and manages the multi-turn flow:

```rust
struct ConversationState {
    intent: String,                    // "record_transaction"
    transaction_type: String,          // "buy"
    collected_entities: HashMap<String, Value>,
    missing_entities: Vec<String>,
    confirmation_pending: bool,
}

impl ConversationManager {
    fn process(&mut self, parsed: ParsedInput) -> ConversationAction {
        // Store what we have
        self.state.intent = parsed.intent;
        self.state.collected_entities.insert("asset", "BTC");
        self.state.collected_entities.insert("account", "Binance");
        self.state.missing_entities = vec!["quantity", "price"];

        // Missing required fields -> Ask for clarification
        if !self.state.missing_entities.is_empty() {
            return ConversationAction::Clarify {
                question: "How much BTC did you buy?",
                field: "quantity"
            };
        }
    }
}
```

**Output to user:**
```
  I'll help you record that purchase. A few quick questions:

  How much BTC did you buy?
  >
```

---

### Step 4: User Provides Quantity

**User input:**
```
  > 0.15
```

**Conversation Manager** updates state:
```rust
// Parse "0.15" as quantity
self.state.collected_entities.insert("quantity", 0.15);
self.state.missing_entities.remove("quantity");
// Still missing: ["price"]

return ConversationAction::Clarify {
    question: "What price did you pay per BTC?",
    field: "price"
};
```

**Output to user:**
```
  What price did you pay per BTC?
  >
```

---

### Step 5: User Provides Price

**User input:**
```
  > 94500
```

**Conversation Manager** - All entities collected, prepare confirmation:
```rust
self.state.collected_entities.insert("price", 94500);
self.state.missing_entities.remove("price");

// All required fields collected!
// missing_entities is now empty

// Build execution plan
let plan = ExecutionPlan {
    command: "tx buy",
    params: {
        asset: "BTC",
        quantity: 0.15,
        price: 94500,
        account: "Binance"
    },
    description: "Buy 0.15 BTC @ $94,500.00 on Binance",
    total_usd: 14175.00
};

return ConversationAction::Confirm { plan };
```

**Output to user:**
```
  Here's what I'll record:

  +------------------------------------------+
  |  Transaction: BUY                        |
  |  Asset:       BTC                        |
  |  Quantity:    0.15                       |
  |  Price:       $94,500.00 per BTC         |
  |  Account:     Binance                    |
  |  Total:       $14,175.00                 |
  +------------------------------------------+

  Confirm this transaction? [Y/n]
```

---

### Step 6: User Confirms

**User input:**
```
  > y
```

**Conversation Manager** triggers execution:
```rust
match user_input.to_lowercase().as_str() {
    "y" | "yes" | "" => {
        return ConversationAction::Execute {
            plan: self.state.pending_plan.clone()
        };
    }
    "n" | "no" => {
        return ConversationAction::Cancel {
            message: "Transaction cancelled."
        };
    }
    _ => {
        return ConversationAction::Clarify {
            question: "Please confirm with 'y' or cancel with 'n'"
        };
    }
}
```

---

### Step 7: Execution Layer

**Execution Layer** calls existing Cryptofolio core functions:

```rust
impl ExecutionLayer {
    async fn execute(&self, plan: ExecutionPlan, pool: &SqlitePool) -> Result<ExecutionResult> {
        match plan.command.as_str() {
            "tx buy" => {
                // Use existing transaction handler
                let tx_repo = TransactionRepository::new(pool);
                let holding_repo = HoldingRepository::new(pool);

                // Record the transaction
                let transaction = Transaction {
                    tx_type: TransactionType::Buy,
                    to_account_id: Some(plan.params.account_id),
                    to_asset: Some(plan.params.asset.clone()),
                    to_quantity: Some(plan.params.quantity),
                    price_usd: Some(plan.params.price),
                    timestamp: Utc::now(),
                    ..Default::default()
                };

                tx_repo.create(&transaction).await?;

                // Update holdings (reuses existing logic)
                holding_repo.add_to_holding(
                    &plan.params.account_id,
                    &plan.params.asset,
                    plan.params.quantity,
                    Some(plan.params.price)
                ).await?;

                Ok(ExecutionResult {
                    success: true,
                    message: format!(
                        "Recorded: buy {} {} @ {} in '{}'",
                        plan.params.quantity,
                        plan.params.asset,
                        format_usd(plan.params.price),
                        plan.params.account
                    ),
                    suggestion: Some("View your updated portfolio with 'portfolio'")
                })
            }
            _ => { /* handle other commands */ }
        }
    }
}
```

**Database changes:**
```sql
-- Transaction recorded
INSERT INTO transactions (tx_type, to_account_id, to_asset, to_quantity, price_usd, timestamp)
VALUES ('buy', 'binance-uuid', 'BTC', '0.15', '94500', '2026-02-07T10:30:00Z');

-- Holdings updated
UPDATE holdings
SET quantity = quantity + 0.15,
    avg_cost_basis = ((avg_cost_basis * old_qty) + (94500 * 0.15)) / (old_qty + 0.15)
WHERE account_id = 'binance-uuid' AND asset = 'BTC';
```

---

### Step 8: Response to User

**Final output:**
```
  Recorded: buy 0.15 BTC @ $94,500.00 in 'Binance'

  Your Binance BTC holdings: 0.24121 BTC ($22,782.83)

  Tip: View your full portfolio with 'portfolio'

you>
```

---

### Complete Interaction Summary

```
you> I just bought some bitcoin on binance
                    │
                    ▼
         ┌─────────────────────┐
         │   INPUT PROCESSOR   │
         │  ─────────────────  │
         │  Intent: tx.buy     │
         │  Asset: BTC         │
         │  Account: Binance   │
         │  Missing: qty,price │
         └─────────────────────┘
                    │
                    ▼
  How much BTC did you buy?
  > 0.15
                    │
                    ▼
         ┌─────────────────────┐
         │ CONVERSATION MGR    │
         │  ─────────────────  │
         │  Collected: qty     │
         │  Missing: price     │
         └─────────────────────┘
                    │
                    ▼
  What price did you pay per BTC?
  > 94500
                    │
                    ▼
         ┌─────────────────────┐
         │ CONVERSATION MGR    │
         │  ─────────────────  │
         │  All fields ready   │
         │  → Show confirmation│
         └─────────────────────┘
                    │
                    ▼
  Confirm this transaction? [Y/n] y
                    │
                    ▼
         ┌─────────────────────┐
         │  EXECUTION LAYER    │
         │  ─────────────────  │
         │  tx_repo.create()   │
         │  holding_repo.add() │
         │  ✓ Committed to DB  │
         └─────────────────────┘
                    │
                    ▼
  Recorded: buy 0.15 BTC @ $94,500.00 in 'Binance'
```

---

### Layer Responsibilities Recap

| Layer | Responsibility | Example Action |
|-------|---------------|----------------|
| **User Interface** | Display prompt, capture input, show responses | `readline("you> ")` |
| **Input Processor** | Classify intent, extract entities via AI | `{intent: "buy", asset: "BTC"}` |
| **Conversation Manager** | Multi-turn flow, collect missing data, confirm | Ask for quantity, then price |
| **Execution Layer** | Call existing Cryptofolio functions | `tx_repo.create()`, `holding_repo.add()` |

---

## Key Components

### 1. Intent Classification
Maps natural language to portfolio operations:

| User Says | Intent | Action |
|-----------|--------|--------|
| "I bought BTC" | `tx.buy` | Record buy transaction |
| "How's my portfolio?" | `portfolio.view` | Show portfolio |
| "Move ETH to Ledger" | `holdings.move` | Transfer between accounts |
| "What's Bitcoin worth?" | `price.check` | Fetch current price |
| "Sync my Binance" | `sync` | Sync exchange holdings |

### 2. Entity Extraction
Extracts structured data from natural language:

```
Input: "I bought 0.5 ETH yesterday on Coinbase for $1,600 each"

Entities:
  - action: buy
  - asset: ETH
  - quantity: 0.5
  - account: Coinbase
  - price: 1600
  - date: yesterday (resolved to actual date)
```

### 3. Conversation Context
Maintains state across turns:

```
Context:
  - last_account: "Binance"
  - last_asset: "BTC"
  - pending_operation: null
  - conversation_history: [...]
```

### 4. Confirmation Flow
All state-changing operations require confirmation:

```
1. User expresses intent
2. System extracts entities
3. System asks for missing info
4. System shows execution plan
5. User confirms
6. System executes
7. System shows result
```

---

## AI-Assisted Command Generation

The system provides intelligent command assistance through two complementary features:

### Command Palette (Ctrl+Space)

Triggered on-demand to transform natural language into structured commands:

```
you> bought btc [Ctrl+Space]

  ┌─ Command Suggestions ─────────────────────────────────────┐
  │                                                           │
  │  Based on "bought btc":                                   │
  │                                                           │
  │  > tx buy BTC <qty> --account <account> --price <price>   │
  │    Record a BTC purchase transaction                      │
  │                                                           │
  │  > holdings add BTC <qty> --account <account>             │
  │    Add BTC to holdings without transaction                │
  │                                                           │
  │  [↑↓ navigate] [Enter select] [Esc cancel]                │
  └───────────────────────────────────────────────────────────┘
```

**Features:**
- Fuzzy matching on partial input
- Shows command template with placeholders
- Brief description of each option
- Works offline with local model

### Smart Correction (Post-Submit)

Automatically detects and suggests fixes for errors after user presses Enter:

```
you> tx buy btc 0.1 --acount Binance --price 95000
                      ^^^^^^^
  Typo detected: 'acount' → 'account'

  Corrected command:
    tx buy BTC 0.1 --account Binance --price 95000

  Run corrected command? [Y/n]
```

```
you> tx buy btc 0.1 --account Binanec --price 95000
                              ^^^^^^^
  Unknown account: 'Binanec'

  Did you mean one of these?
    > Binance (92% match)
    > Binance US (78% match)

  Use 'Binance'? [Y/n]
```

**Correction Types:**
| Type | Example | Action |
|------|---------|--------|
| Typo in flag | `--acount` → `--account` | Auto-suggest fix |
| Unknown account | `Binanec` → `Binance` | Fuzzy match existing |
| Unknown asset | `BITCONI` → `BTC` | Match known symbols |
| Missing required | `tx buy BTC` (no qty) | Prompt for missing |
| Invalid value | `--price abc` | Ask for valid number |

### Offline Operation

Both features work offline using a local LLM:

```
┌─────────────────────────────────────────────────────────────┐
│                    AI Provider Selection                     │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐   │
│  │   Online    │     │   Offline   │     │   Hybrid    │   │
│  │  Claude API │     │   Ollama    │     │  (Default)  │   │
│  └─────────────┘     └─────────────┘     └─────────────┘   │
│                                                             │
│  Hybrid Mode:                                               │
│  - Command palette → Local model (fast, free)               │
│  - Smart correction → Local model (fast, free)              │
│  - Complex NLU → Claude API (when online)                   │
│  - Fallback → Local model if API unavailable                │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

**Local Model Requirements:**
```bash
# Install Ollama
brew install ollama

# Pull recommended model (small, fast)
ollama pull llama3.2:3b    # 2GB, good for corrections
ollama pull mistral:7b     # 4GB, better for NLU
```

**Configuration:**
```toml
# ~/.config/cryptofolio/config.toml
[ai]
mode = "hybrid"              # "online", "offline", "hybrid"
local_model = "llama3.2:3b"  # Ollama model for offline
claude_model = "claude-sonnet-4-20250514"  # For complex queries
```

---

## AI Integration Options

| Option | Pros | Cons |
|--------|------|------|
| **Claude API** | Best reasoning, tool use | Cost per request |
| **Local LLM (Ollama)** | Free, private, offline | Lower quality, resource heavy |
| **Hybrid** | Best of both | Complexity |
| **Rule-based + Templates** | Fast, free, predictable | Limited flexibility |

### Recommended: Hybrid Approach (Claude + Ollama)

**Cost-optimized routing:**
```rust
fn select_provider(task: &Task) -> Provider {
    match task.complexity {
        // Simple tasks → Local (free, fast)
        Complexity::Low => Provider::Ollama,

        // Medium tasks → Local first, Claude if needed
        Complexity::Medium => {
            if is_online() && confidence < 0.8 {
                Provider::Claude
            } else {
                Provider::Ollama
            }
        }

        // Complex multi-turn → Claude (best quality)
        Complexity::High => Provider::Claude,
    }
}
```

### Claude API Tool Definitions

```rust
// Define tools for Claude
tools: [
  {
    name: "record_transaction",
    description: "Record a buy, sell, or transfer transaction",
    parameters: { asset, quantity, price, account, type, notes }
  },
  {
    name: "view_portfolio",
    description: "Show current portfolio with P&L",
    parameters: { group_by, account, category }
  },
  {
    name: "get_price",
    description: "Get current price for a cryptocurrency",
    parameters: { symbols }
  }
]
```

---

## Machine-Readable Output & MCP Integration

**NEW in v0.2:** All query commands support `--json` flag for programmatic access and LLM/MCP integration. Transaction history can also be exported to CSV format for tax reporting and external analysis.

### JSON Output for All Commands

Every data-retrieval command can output structured JSON:

```bash
# Portfolio data
cryptofolio portfolio --json

# Holdings
cryptofolio holdings list --json
cryptofolio holdings list --account Binance --json

# Accounts
cryptofolio account list --json
cryptofolio account show "Binance" --json

# Transactions
cryptofolio tx list --json
cryptofolio tx list --limit 50 --json

# Prices
cryptofolio price BTC ETH --json

# Market data
cryptofolio market BTCUSDT --json

# Configuration
cryptofolio config show --json
```

### CSV Export for Transaction History

**NEW in v0.2:** Export transaction history to CSV format for tax reporting, spreadsheet analysis, or external tools:

```bash
# Export all transactions
cryptofolio tx export transactions.csv

# Export with filters
cryptofolio tx export 2025-btc.csv --asset BTC --from 2025-01-01 --to 2025-12-31
cryptofolio tx export binance.csv --account Binance
cryptofolio tx export recent.csv --limit 100

# Tax season workflow
cryptofolio tx export tax-year-2025.csv --from 2025-01-01 --to 2025-12-31
# Import into TurboTax, CoinTracker, or Excel for analysis
```

### Customizable Number Formatting

**NEW in v0.2:** Configure display precision for quantities and prices:

```bash
# View current formatting settings
cryptofolio config show

# Customize decimal places
cryptofolio config set display.decimals 4           # Quantity precision (default: 8)
cryptofolio config set display.price_decimals 2     # Price precision (default: 4)

# Toggle thousands separator
cryptofolio config set display.thousands_separator true   # Show 1,234.56
cryptofolio config set display.thousands_separator false  # Show 1234.56
```

### MCP (Model Context Protocol) Server Integration

Cryptofolio can be integrated as an MCP server tool for Claude Desktop or custom AI applications:

**Example MCP Tool Definition:**
```javascript
{
  "name": "crypto-portfolio",
  "tools": {
    "get_portfolio": {
      "description": "Get current cryptocurrency portfolio with P&L",
      "inputSchema": {
        "type": "object",
        "properties": {
          "account": { "type": "string", "description": "Optional account filter" }
        }
      },
      "handler": async (params) => {
        const cmd = params.account
          ? `cryptofolio portfolio --account "${params.account}" --json`
          : `cryptofolio portfolio --json`;
        const result = execSync(cmd).toString();
        return JSON.parse(result);
      }
    },
    "get_price": {
      "description": "Get current cryptocurrency prices",
      "inputSchema": {
        "type": "object",
        "properties": {
          "symbols": {
            "type": "array",
            "items": { "type": "string" },
            "description": "Crypto symbols (e.g., BTC, ETH)"
          }
        },
        "required": ["symbols"]
      },
      "handler": async (params) => {
        const cmd = `cryptofolio price ${params.symbols.join(' ')} --json`;
        const result = execSync(cmd).toString();
        return JSON.parse(result);
      }
    },
    "get_holdings": {
      "description": "List all cryptocurrency holdings",
      "inputSchema": {
        "type": "object",
        "properties": {
          "account": { "type": "string", "description": "Optional account filter" }
        }
      },
      "handler": async (params) => {
        const cmd = params.account
          ? `cryptofolio holdings list --account "${params.account}" --json`
          : `cryptofolio holdings list --json`;
        const result = execSync(cmd).toString();
        return JSON.parse(result);
      }
    }
  }
}
```

### LLM Integration Examples

**Claude Desktop with MCP:**
```
User: "What's my portfolio worth?"

[Claude uses get_portfolio tool]
{
  "total_value_usd": "112552.27",
  "unrealized_pnl": "28107.27",
  "unrealized_pnl_percent": "33.28"
}

Claude: "Your cryptocurrency portfolio is currently worth $112,552.27,
with an unrealized profit of $28,107.27 (up 33.28% from your cost basis)."
```

**Custom Python Integration:**
```python
import subprocess
import json

def ask_portfolio_question(question: str):
    # Get portfolio data
    result = subprocess.run(
        ["cryptofolio", "portfolio", "--json"],
        capture_output=True,
        text=True
    )
    portfolio = json.loads(result.stdout)

    # Send to LLM with context
    response = call_llm(
        prompt=f"""
        Portfolio data: {json.dumps(portfolio)}

        User question: {question}

        Provide a clear, concise answer.
        """
    )
    return response

# Usage
ask_portfolio_question("Which of my holdings has the best performance?")
ask_portfolio_question("Should I rebalance my portfolio?")
```

### Automation Scripts

**Portfolio Monitoring:**
```bash
#!/bin/bash
# Alert if portfolio drops below threshold

TOTAL=$(cryptofolio portfolio --json --quiet | jq -r '.total_value_usd' | tr -d '$' | tr -d ',')

if (( $(echo "$TOTAL < 100000" | bc -l) )); then
  # Send notification
  osascript -e "display notification \"Portfolio: \$$TOTAL\" with title \"Crypto Alert\""

  # Or send to Slack
  curl -X POST $SLACK_WEBHOOK -d "{\"text\": \"Portfolio below threshold: \$$TOTAL\"}"
fi
```

**Daily Logging:**
```bash
#!/bin/bash
# Log portfolio value daily

DATE=$(date +%Y-%m-%d)
PORTFOLIO=$(cryptofolio portfolio --json)

echo "{\"date\": \"$DATE\", \"portfolio\": $PORTFOLIO}" >> ~/portfolio-history.jsonl

# Calculate 7-day change
jq -s 'if length >= 7 then
  {
    "current": .[-1].portfolio.total_value_usd,
    "week_ago": .[-7].portfolio.total_value_usd,
    "change_percent": (((.[-1].portfolio.total_value_usd | tonumber) / (.[-7].portfolio.total_value_usd | tonumber) - 1) * 100)
  }
else empty end' ~/portfolio-history.jsonl
```

### Benefits of JSON Output

| Benefit | Description |
|---------|-------------|
| **LLM Integration** | Direct integration with Claude, ChatGPT, and custom AI agents |
| **MCP Server Tools** | Build Claude Desktop tools for portfolio analysis |
| **Scriptable** | Process with `jq`, Python, Node.js, or any language |
| **Consistent Format** | All commands return predictable JSON structures |
| **Precision Preserved** | Numbers as strings to avoid floating-point issues |
| **Dashboard Ready** | Feed data to Grafana, custom monitoring, or web UIs |
| **Testing** | Validate command outputs in CI/CD pipelines |

---

## User Experience Flow

```
+-------------+     +------------------+     +---------------+
|   Start     | --> | Show Portfolio   | --> | Wait for      |
| cryptofolio |     | Summary + Prompt |     | User Input    |
+-------------+     +------------------+     +---------------+
                                                    |
                    +-------------------------------+
                    |
                    v
            +---------------+
            | Classify Input|
            +---------------+
                    |
        +-----------+-----------+
        |                       |
        v                       v
+---------------+       +---------------+
| CLI Command   |       | Natural Lang  |
| (direct exec) |       | (AI process)  |
+---------------+       +---------------+
        |                       |
        |               +-------+-------+
        |               |               |
        |               v               v
        |       +-------------+  +-------------+
        |       | Ask for     |  | Ready to    |
        |       | Missing Info|  | Execute     |
        |       +-------------+  +-------------+
        |               |               |
        |               +-------+-------+
        |                       |
        v                       v
+---------------------------------------+
|           Execute Command             |
+---------------------------------------+
                    |
                    v
+---------------------------------------+
|    Show Result + Suggest Next Steps   |
+---------------------------------------+
                    |
                    v
            (back to prompt)
```

---

## Implementation Phases

### Phase 1: Interactive Shell (No AI)
- REPL with command history
- Tab completion for commands
- Status bar with portfolio summary
- Direct CLI command execution

### Phase 2: Smart Command Parsing
- Fuzzy command matching
- Shorthand commands (`p` = `portfolio`, `b` = `buy`)
- Context-aware defaults (remember last account)

### Phase 3: AI Integration
- Claude API integration
- Natural language understanding
- Multi-turn conversation
- Tool execution

### Phase 4: Enhanced UX
- Proactive suggestions
- Portfolio insights
- Price alerts
- Anomaly detection ("Your BTC is down 15% today")

---

## Success Metrics

| Metric | Target |
|--------|--------|
| Command success rate | >95% of intents correctly understood |
| Average turns to complete action | <3 turns for common operations |
| User satisfaction | Users prefer conversational over CLI |
| Error recovery | System gracefully handles misunderstandings |

---

## NLP Test Suite & Model Evaluation

A comprehensive test framework to validate natural language understanding, compare models, and ensure quality.

### Design Principles

**Priority Order: Cost > Accuracy > Latency**

1. **Cost**: Minimize API spending; prefer local models when quality is sufficient
2. **Accuracy**: Correct intent + entity extraction is critical for user trust
3. **Latency**: Acceptable up to 2s for complex queries; <500ms for corrections

### Test Coverage Requirements

| Requirement | Description |
|-------------|-------------|
| Operations | At least 1 test per CLI operation |
| Variations | Minimum 3 natural language phrasings per operation |
| Multi-turn | Test conversation flows requiring clarification |
| Edge cases | Ambiguous inputs, typos, partial info, slang |
| Synthetic | All test data is synthetic (no real user data) |

### Test Dataset Structure

```yaml
# tests/fixtures/nlp_benchmark.yaml

metadata:
  version: "1.0"
  created: "2026-02-07"
  total_tests: 84  # 12 operations × 3 variations + 24 multi-turn + 24 edge cases

# ============================================================
# SINGLE-TURN TESTS: Intent Classification + Entity Extraction
# ============================================================

single_turn_tests:

  # ----- PRICE OPERATIONS -----
  - id: price_simple_1
    input: "What's the price of Bitcoin?"
    expected:
      intent: price.check
      entities:
        symbols: [BTC]
    difficulty: easy

  - id: price_simple_2
    input: "How much is ETH right now?"
    expected:
      intent: price.check
      entities:
        symbols: [ETH]
    difficulty: easy

  - id: price_simple_3
    input: "btc price"
    expected:
      intent: price.check
      entities:
        symbols: [BTC]
    difficulty: easy

  - id: price_multiple_1
    input: "Show me prices for BTC, ETH, and SOL"
    expected:
      intent: price.check
      entities:
        symbols: [BTC, ETH, SOL]
    difficulty: medium

  # ----- BUY TRANSACTIONS -----
  - id: tx_buy_complete_1
    input: "I bought 0.5 BTC on Binance for $47,000 each"
    expected:
      intent: tx.buy
      entities:
        asset: BTC
        quantity: 0.5
        account: Binance
        price: 47000
    difficulty: easy

  - id: tx_buy_complete_2
    input: "Just grabbed 2 ETH at 3200 on my Coinbase account"
    expected:
      intent: tx.buy
      entities:
        asset: ETH
        quantity: 2
        account: Coinbase
        price: 3200
    difficulty: medium

  - id: tx_buy_complete_3
    input: "Purchased 100 SOL @ $150 via Binance"
    expected:
      intent: tx.buy
      entities:
        asset: SOL
        quantity: 100
        account: Binance
        price: 150
    difficulty: medium

  - id: tx_buy_partial_1
    input: "I bought some bitcoin today"
    expected:
      intent: tx.buy
      entities:
        asset: BTC
      missing: [quantity, account, price]
    difficulty: medium

  - id: tx_buy_partial_2
    input: "added eth to my portfolio"
    expected:
      intent: tx.buy
      entities:
        asset: ETH
      missing: [quantity, account, price]
    difficulty: hard

  # ----- SELL TRANSACTIONS -----
  - id: tx_sell_complete_1
    input: "I sold 0.1 BTC from Binance at $95,000"
    expected:
      intent: tx.sell
      entities:
        asset: BTC
        quantity: 0.1
        account: Binance
        price: 95000
    difficulty: easy

  - id: tx_sell_complete_2
    input: "Dumped my ETH at 4000 bucks each, got rid of 5 coins from Kraken"
    expected:
      intent: tx.sell
      entities:
        asset: ETH
        quantity: 5
        account: Kraken
        price: 4000
    difficulty: hard

  - id: tx_sell_complete_3
    input: "Took profits on 50 SOL at $200 on Binance"
    expected:
      intent: tx.sell
      entities:
        asset: SOL
        quantity: 50
        account: Binance
        price: 200
    difficulty: medium

  # ----- TRANSFER OPERATIONS -----
  - id: transfer_complete_1
    input: "Transfer 0.5 BTC from Binance to my Ledger"
    expected:
      intent: holdings.move
      entities:
        asset: BTC
        quantity: 0.5
        from_account: Binance
        to_account: Ledger
    difficulty: easy

  - id: transfer_complete_2
    input: "Move my ETH from the exchange to cold storage"
    expected:
      intent: holdings.move
      entities:
        asset: ETH
        from_account: exchange  # needs resolution
        to_account: cold storage  # needs resolution
      missing: [quantity]
    difficulty: medium

  - id: transfer_complete_3
    input: "Send half my BTC to the hardware wallet"
    expected:
      intent: holdings.move
      entities:
        asset: BTC
        percentage: 50
        to_account: hardware wallet  # needs resolution
      missing: [from_account]
    difficulty: hard

  # ----- PORTFOLIO VIEW -----
  - id: portfolio_view_1
    input: "Show me my portfolio"
    expected:
      intent: portfolio.view
      entities: {}
    difficulty: easy

  - id: portfolio_view_2
    input: "How am I doing? What's my P&L?"
    expected:
      intent: portfolio.view
      entities: {}
    difficulty: medium

  - id: portfolio_view_3
    input: "What's everything worth right now?"
    expected:
      intent: portfolio.view
      entities: {}
    difficulty: medium

  - id: portfolio_filtered_1
    input: "Show me just my Binance holdings"
    expected:
      intent: portfolio.view
      entities:
        account: Binance
    difficulty: medium

  # ----- HOLDINGS MANAGEMENT -----
  - id: holdings_list_1
    input: "What do I have?"
    expected:
      intent: holdings.list
      entities: {}
    difficulty: easy

  - id: holdings_list_2
    input: "List all my crypto"
    expected:
      intent: holdings.list
      entities: {}
    difficulty: easy

  - id: holdings_add_1
    input: "I have 2 BTC on my Ledger that I bought at 30k average"
    expected:
      intent: holdings.add
      entities:
        asset: BTC
        quantity: 2
        account: Ledger
        cost_basis: 30000
    difficulty: medium

  # ----- SYNC OPERATIONS -----
  - id: sync_all_1
    input: "Sync my exchanges"
    expected:
      intent: sync
      entities: {}
    difficulty: easy

  - id: sync_all_2
    input: "Update balances from Binance"
    expected:
      intent: sync
      entities:
        account: Binance
    difficulty: easy

  - id: sync_all_3
    input: "Refresh everything"
    expected:
      intent: sync
      entities: {}
    difficulty: medium

  # ----- ACCOUNT MANAGEMENT -----
  - id: account_add_1
    input: "Add a new Ledger wallet account"
    expected:
      intent: account.add
      entities:
        name: Ledger
        type: hardware_wallet
    difficulty: medium

  - id: account_list_1
    input: "What accounts do I have?"
    expected:
      intent: account.list
      entities: {}
    difficulty: easy

  # ----- MARKET DATA -----
  - id: market_data_1
    input: "Show me 24 hour stats for Bitcoin"
    expected:
      intent: market.view
      entities:
        symbol: BTC
        show_24h: true
    difficulty: medium

# ============================================================
# MULTI-TURN CONVERSATION TESTS
# ============================================================

multi_turn_tests:

  - id: multi_buy_flow_1
    name: "Complete buy with clarification"
    difficulty: medium
    turns:
      - user: "I bought some bitcoin"
        expected_response:
          type: clarify
          question_about: [quantity, account, price]

      - user: "0.1 BTC"
        expected_response:
          type: clarify
          question_about: [account, price]
          entities_so_far:
            asset: BTC
            quantity: 0.1

      - user: "on Binance at 94k"
        expected_response:
          type: confirm
          entities_complete:
            asset: BTC
            quantity: 0.1
            account: Binance
            price: 94000

      - user: "yes"
        expected_response:
          type: execute
          command: "tx buy BTC 0.1 --account Binance --price 94000"

  - id: multi_buy_flow_2
    name: "Buy with context from previous turn"
    difficulty: hard
    turns:
      - user: "I use Binance mostly"
        expected_response:
          type: acknowledge
          context_update:
            default_account: Binance

      - user: "Just bought 0.5 ETH at 3200"
        expected_response:
          type: confirm
          entities_complete:
            asset: ETH
            quantity: 0.5
            account: Binance  # inferred from context
            price: 3200

  - id: multi_transfer_flow_1
    name: "Transfer with percentage"
    difficulty: hard
    turns:
      - user: "Move half my Bitcoin to cold storage"
        expected_response:
          type: clarify
          needs_resolution:
            - "Which account to transfer from?"
            - "Which cold storage account?"
          entities_so_far:
            asset: BTC
            percentage: 50

      - user: "From Binance to my Ledger"
        expected_response:
          type: confirm
          calculation_shown: true  # should show actual amount
          entities_complete:
            asset: BTC
            quantity: "<calculated from 50%>"
            from_account: Binance
            to_account: Ledger

  - id: multi_correction_flow_1
    name: "User corrects mid-conversation"
    difficulty: hard
    turns:
      - user: "I sold 1 ETH at 4000"
        expected_response:
          type: clarify
          question_about: [account]

      - user: "Actually wait, it was 1.5 ETH"
        expected_response:
          type: acknowledge
          entities_updated:
            quantity: 1.5  # corrected
          question_about: [account]

      - user: "on Coinbase"
        expected_response:
          type: confirm
          entities_complete:
            asset: ETH
            quantity: 1.5
            account: Coinbase
            price: 4000

  - id: multi_cancel_flow_1
    name: "User cancels operation"
    difficulty: easy
    turns:
      - user: "Buy 1 BTC at 95000 on Binance"
        expected_response:
          type: confirm

      - user: "no wait, cancel that"
        expected_response:
          type: cancelled
          message: "Operation cancelled"

  - id: multi_sequential_ops_1
    name: "Multiple operations in sequence"
    difficulty: hard
    turns:
      - user: "Buy 1 ETH at 3200 on Binance"
        expected_response:
          type: confirm

      - user: "y"
        expected_response:
          type: execute

      - user: "Now show me my portfolio"
        expected_response:
          type: execute
          command: "portfolio"

# ============================================================
# EDGE CASE TESTS
# ============================================================

edge_case_tests:

  # ----- AMBIGUOUS INPUTS -----
  - id: edge_ambiguous_1
    input: "bitcoin"
    expected:
      intent: ambiguous
      possible_intents: [price.check, portfolio.view]
      clarification: "Would you like to check BTC price or view your BTC holdings?"
    difficulty: hard

  - id: edge_ambiguous_2
    input: "Binance"
    expected:
      intent: ambiguous
      possible_intents: [sync, portfolio.view, holdings.list]
    difficulty: hard

  # ----- TYPOS AND MISSPELLINGS -----
  - id: edge_typo_1
    input: "I bougth 0.1 BTC on Binace"
    expected:
      intent: tx.buy
      entities:
        asset: BTC
        quantity: 0.1
        account: Binance  # corrected from "Binace"
      corrections_applied: ["bougth→bought", "Binace→Binance"]
    difficulty: medium

  - id: edge_typo_2
    input: "protfolio"
    expected:
      intent: portfolio.view
      corrections_applied: ["protfolio→portfolio"]
    difficulty: easy

  # ----- SLANG AND INFORMAL -----
  - id: edge_slang_1
    input: "wen moon?"
    expected:
      intent: price.check
      entities: {}
      # or could be portfolio.view
    difficulty: hard

  - id: edge_slang_2
    input: "gonna stack some sats"
    expected:
      intent: tx.buy
      entities:
        asset: BTC  # "sats" = satoshis = BTC
    difficulty: hard

  - id: edge_slang_3
    input: "HODL check"
    expected:
      intent: portfolio.view
    difficulty: hard

  # ----- NUMBERS IN DIFFERENT FORMATS -----
  - id: edge_number_format_1
    input: "bought 0.00001 BTC at 95,000 dollars"
    expected:
      intent: tx.buy
      entities:
        asset: BTC
        quantity: 0.00001
        price: 95000
    difficulty: medium

  - id: edge_number_format_2
    input: "sold half a bitcoin at ninety-five thousand"
    expected:
      intent: tx.sell
      entities:
        asset: BTC
        quantity: 0.5
        price: 95000
    difficulty: hard

  - id: edge_number_format_3
    input: "bought 1M sats at 95k"
    expected:
      intent: tx.buy
      entities:
        asset: BTC
        quantity: 0.01  # 1M sats = 0.01 BTC
        price: 95000
    difficulty: hard

  # ----- MISSING CRITICAL INFO -----
  - id: edge_vague_1
    input: "add to my portfolio"
    expected:
      intent: unclear
      clarification: "What would you like to add?"
    difficulty: hard

  - id: edge_vague_2
    input: "update it"
    expected:
      intent: unclear
      clarification: "What would you like to update?"
    difficulty: hard

  # ----- NEGATIVE / INVALID -----
  - id: edge_invalid_1
    input: "sell -5 BTC"
    expected:
      intent: tx.sell
      validation_error: "Quantity cannot be negative"
    difficulty: medium

  - id: edge_invalid_2
    input: "buy BTC at zero dollars"
    expected:
      intent: tx.buy
      validation_error: "Price must be greater than zero"
    difficulty: medium

  # ----- NON-CRYPTO REQUESTS -----
  - id: edge_offtopic_1
    input: "What's the weather like?"
    expected:
      intent: out_of_scope
      response: "I can only help with cryptocurrency portfolio management."
    difficulty: easy

  - id: edge_offtopic_2
    input: "Tell me a joke"
    expected:
      intent: out_of_scope
    difficulty: easy
```

### Evaluation Metrics (Priority Order)

```
┌─────────────────────────────────────────────────────────────────┐
│  EVALUATION PRIORITY                                            │
│                                                                 │
│  1. COST          How much does it cost per 1000 queries?      │
│  ════════════════════════════════════════════════════════════  │
│                                                                 │
│  2. ACCURACY      Does it understand the user correctly?        │
│  ────────────────────────────────────────────────────────────  │
│     • Intent Accuracy     = Correct intent / Total tests       │
│     • Entity Precision    = Correct entities / Extracted       │
│     • Entity Recall       = Correct entities / Expected        │
│     • Clarification Score = Asked right questions when needed  │
│                                                                 │
│  3. LATENCY       How fast is the response?                     │
│  ────────────────────────────────────────────────────────────  │
│     • P50 latency   (median)                                   │
│     • P95 latency   (tail)                                     │
│     • P99 latency   (worst case)                               │
└─────────────────────────────────────────────────────────────────┘
```

### Test Runner & Reporting

```bash
# Run full benchmark
cryptofolio test nlp --all

# Run against specific model
cryptofolio test nlp --model claude-sonnet-4
cryptofolio test nlp --model ollama:llama3.2

# Run specific test category
cryptofolio test nlp --category single_turn
cryptofolio test nlp --category multi_turn
cryptofolio test nlp --category edge_cases

# Generate comparison report
cryptofolio test nlp --compare claude-sonnet-4,claude-haiku,ollama:mistral
```

### Sample Benchmark Report

```
╔═══════════════════════════════════════════════════════════════════════╗
║                    NLP BENCHMARK REPORT                               ║
║                    Generated: 2026-02-07 14:30:00                     ║
╠═══════════════════════════════════════════════════════════════════════╣
║                                                                       ║
║  Test Suite: v1.0 (84 tests)                                         ║
║  Categories: 36 single-turn, 24 multi-turn, 24 edge cases            ║
║                                                                       ║
╠═══════════════════════════════════════════════════════════════════════╣
║                                                                       ║
║  COST ANALYSIS (per 1000 queries)                         [Priority 1]║
║  ─────────────────────────────────────────────────────────────────── ║
║                                                                       ║
║  Model                    Input Tokens   Output Tokens   Total Cost   ║
║  ──────────────────────────────────────────────────────────────────  ║
║  ollama:llama3.2:3b       ~150 avg       ~50 avg         $0.00       ║
║  ollama:mistral:7b        ~150 avg       ~50 avg         $0.00       ║
║  claude-haiku             ~150 avg       ~50 avg         $1.25       ║
║  claude-sonnet-4          ~150 avg       ~50 avg         $4.50       ║
║                                                                       ║
╠═══════════════════════════════════════════════════════════════════════╣
║                                                                       ║
║  ACCURACY METRICS                                         [Priority 2]║
║  ─────────────────────────────────────────────────────────────────── ║
║                                                                       ║
║  Model               Intent   Entity   Entity   Clarify   Overall    ║
║                       Acc     Prec     Recall   Score     Score      ║
║  ──────────────────────────────────────────────────────────────────  ║
║  claude-sonnet-4     98.8%    97.2%    96.5%    94.0%     96.6%  ██████████ ║
║  claude-haiku        95.2%    93.1%    91.8%    88.5%     92.2%  █████████░ ║
║  ollama:mistral:7b   89.3%    85.6%    82.4%    78.2%     83.9%  ████████░░ ║
║  ollama:llama3.2:3b  82.1%    78.4%    75.2%    70.1%     76.5%  ███████░░░ ║
║                                                                       ║
║  By Difficulty:                                                       ║
║  ──────────────────────────────────────────────────────────────────  ║
║                      Easy     Medium   Hard                          ║
║  claude-sonnet-4     100%     98.5%    94.2%                         ║
║  claude-haiku        98.5%    95.0%    86.7%                         ║
║  ollama:mistral:7b   95.0%    88.2%    72.5%                         ║
║  ollama:llama3.2:3b  90.0%    82.1%    62.3%                         ║
║                                                                       ║
╠═══════════════════════════════════════════════════════════════════════╣
║                                                                       ║
║  LATENCY METRICS                                          [Priority 3]║
║  ─────────────────────────────────────────────────────────────────── ║
║                                                                       ║
║  Model                    P50        P95        P99                  ║
║  ──────────────────────────────────────────────────────────────────  ║
║  ollama:llama3.2:3b      85ms       150ms      280ms     ████░░░░░░  ║
║  ollama:mistral:7b       180ms      320ms      580ms     ██████░░░░  ║
║  claude-haiku            220ms      450ms      820ms     ███████░░░  ║
║  claude-sonnet-4         380ms      720ms      1.2s      █████████░  ║
║                                                                       ║
╠═══════════════════════════════════════════════════════════════════════╣
║                                                                       ║
║  RECOMMENDATION                                                       ║
║  ─────────────────────────────────────────────────────────────────── ║
║                                                                       ║
║  Based on priority (Cost > Accuracy > Latency):                      ║
║                                                                       ║
║  • Command Palette / Corrections: ollama:mistral:7b                  ║
║    - Free, 83.9% accuracy (sufficient for suggestions)               ║
║    - 180ms median latency (acceptable)                               ║
║                                                                       ║
║  • Complex NLU / Multi-turn: claude-haiku                            ║
║    - $1.25/1K queries (10x cheaper than Sonnet)                      ║
║    - 92.2% accuracy (good enough for most cases)                     ║
║    - Fall back to Sonnet for hard cases (confidence < 0.8)           ║
║                                                                       ║
║  Estimated monthly cost (1000 queries/month):                        ║
║    - 70% local (corrections): $0.00                                  ║
║    - 25% Haiku (medium NLU): $0.31                                   ║
║    - 5% Sonnet (complex): $0.23                                      ║
║    - TOTAL: ~$0.54/month                                             ║
║                                                                       ║
╚═══════════════════════════════════════════════════════════════════════╝
```

### CI/CD Integration

```yaml
# .github/workflows/nlp-tests.yml
name: NLP Benchmark

on:
  push:
    paths:
      - 'src/ai/**'
      - 'tests/fixtures/nlp_benchmark.yaml'
  schedule:
    - cron: '0 0 * * 0'  # Weekly

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Run NLP Tests (Local Model)
        run: |
          ollama pull llama3.2:3b
          cargo run -- test nlp --model ollama:llama3.2

      - name: Run NLP Tests (Claude)
        env:
          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
        run: |
          cargo run -- test nlp --model claude-haiku

      - name: Check Accuracy Threshold
        run: |
          # Fail if accuracy drops below 80%
          ACCURACY=$(cat results.json | jq '.overall_accuracy')
          if (( $(echo "$ACCURACY < 0.80" | bc -l) )); then
            echo "Accuracy regression: $ACCURACY < 0.80"
            exit 1
          fi

      - name: Upload Report
        uses: actions/upload-artifact@v4
        with:
          name: nlp-benchmark-report
          path: benchmark_report.html
```

---

## Open Questions

1. **AI Provider**: Claude API vs local LLM vs hybrid?
2. **Offline Mode**: Should it work without internet (rule-based fallback)?
3. **Cost Management**: Cache responses? Rate limit AI calls?
4. **Privacy**: Send portfolio data to AI, or only command structure?
5. **Voice**: Future support for voice input?

---

*Document Version: 1.2*
*Created: 2026-02-07*
*Updated: 2026-02-07 - Added end-to-end example, AI-assisted commands, NLP test suite*
