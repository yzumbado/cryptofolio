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

## AI Integration Options

| Option | Pros | Cons |
|--------|------|------|
| **Claude API** | Best reasoning, tool use | Cost per request |
| **Local LLM (Ollama)** | Free, private, offline | Lower quality, resource heavy |
| **Hybrid** | Best of both | Complexity |
| **Rule-based + Templates** | Fast, free, predictable | Limited flexibility |

### Recommended: Claude API with Tool Use

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

## Open Questions

1. **AI Provider**: Claude API vs local LLM vs hybrid?
2. **Offline Mode**: Should it work without internet (rule-based fallback)?
3. **Cost Management**: Cache responses? Rate limit AI calls?
4. **Privacy**: Send portfolio data to AI, or only command structure?
5. **Voice**: Future support for voice input?

---

*Document Version: 1.1*
*Created: 2026-02-07*
*Updated: 2026-02-07 - Added end-to-end example*
