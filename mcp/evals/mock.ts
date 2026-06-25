/**
 * Mock Model + Tools — let the harness run end-to-end with no API key and no
 * binary. This validates the SCAFFOLDING (loop + grader + scenarios are wired
 * correctly), not the model. Each script produces a correct trace for its
 * scenario, so `eval:mock` should report ~100% — any failure means the harness
 * (not the model) is broken.
 */

import type { Model, ModelContext, ModelTurn, Tools } from "./harness.js";
import type { ToolCall, ToolDef, ToolResult } from "./types.js";

const TOOL_NAMES = [
  "cryptofolio_get_system_status",
  "cryptofolio_list_accounts",
  "cryptofolio_manage_account",
  "cryptofolio_get_portfolio",
  "cryptofolio_get_prices",
  "cryptofolio_get_market_data",
  "cryptofolio_list_transactions",
  "cryptofolio_record_transaction",
  "cryptofolio_track_conversion",
  "cryptofolio_export_transactions",
  "cryptofolio_manage_wallet",
  "cryptofolio_sync_wallet",
  "cryptofolio_sync_exchange",
  "cryptofolio_get_pnl_summary",
  "cryptofolio_get_realized_pnl",
  "cryptofolio_get_unrealized_pnl",
  "cryptofolio_analyze_asset",
  "cryptofolio_get_audit_log",
];

export class MockTools implements Tools {
  async list(): Promise<ToolDef[]> {
    return TOOL_NAMES.map((name) => ({
      name,
      description: `mock ${name}`,
      inputSchema: { type: "object", properties: {} },
    }));
  }

  async call(call: ToolCall): Promise<ToolResult> {
    // Canned success payload; scenario grading keys off the call, not the result.
    return { name: call.name, output: '{"success":true,"data":{}}', isError: false };
  }
}

interface Script {
  calls: ToolCall[];
  finalText: string;
}

const SCRIPTS: Record<string, Script> = {
  "onboarding-empty": {
    calls: [
      {
        name: "cryptofolio_manage_account",
        input: { action: "add", name: "Binance", account_type: "exchange", category: "trading" },
      },
    ],
    finalText: "I've created your Binance exchange account. You're set up.",
  },
  "list-accounts": {
    calls: [{ name: "cryptofolio_list_accounts", input: {} }],
    finalText: "You have one account: Kraken (exchange).",
  },
  "record-buy-basic": {
    calls: [
      {
        name: "cryptofolio_record_transaction",
        input: { type: "buy", asset: "BTC", quantity: "0.5", price_usd: "60000", account: "Ledger" },
      },
    ],
    finalText: "Recorded your buy of 0.5 BTC at $60,000 on Ledger.",
  },
  "record-buy-cost-basis-only": {
    calls: [
      {
        name: "cryptofolio_record_transaction",
        input: {
          type: "buy",
          asset: "ETH",
          quantity: "1",
          price_usd: "2000",
          account: "Binance",
          cost_basis_only: true,
        },
      },
    ],
    finalText: "Backfilled the cost basis for 1 ETH at $2,000 without changing your synced balance.",
  },
  "portfolio-value": {
    calls: [{ name: "cryptofolio_get_portfolio", input: {} }],
    finalText: "Your portfolio is worth $X with unrealized P&L of Y%.",
  },
  "price-eth": {
    calls: [{ name: "cryptofolio_get_prices", input: { symbols: ["ETH"] } }],
    finalText: "ETH is currently trading around the latest quoted price.",
  },
  "analyze-eth": {
    calls: [{ name: "cryptofolio_analyze_asset", input: { asset: "ETH" } }],
    finalText: "Here's your ETH position: cost basis and gain summarized.",
  },
  "empty-portfolio-no-fabrication": {
    calls: [{ name: "cryptofolio_get_portfolio", input: {} }],
    finalText:
      "You don't have any accounts set up yet, so there's nothing to value. Want to add one?",
  },
};

/**
 * Mock model: step 0 emits the scripted tool calls, step 1 emits the final text.
 */
export class MockModel implements Model {
  async step(ctx: ModelContext): Promise<ModelTurn> {
    const script = SCRIPTS[ctx.scenarioId];
    if (!script) {
      return { finalText: "No mock script for this scenario." };
    }
    if (ctx.stepIndex === 0) {
      return { toolCalls: script.calls };
    }
    return { finalText: script.finalText };
  }
}
