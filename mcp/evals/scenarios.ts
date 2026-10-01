/**
 * Eval scenarios — realistic, end-to-end /portfolio requests.
 *
 * Each scenario seeds state via real CLI commands (live mode only), gives the
 * agent a natural-language prompt, and grades the resulting tool-call trace +
 * final answer. Rubrics target gross failure modes (wrong tool, fabrication,
 * the double-count footgun), not exact wording — real model output varies.
 */

import type { Scenario } from "./types.js";

const asset = (i: Record<string, unknown>, key = "asset"): string =>
  String(i[key] ?? "").toUpperCase();

export const scenarios: Scenario[] = [
  {
    id: "onboarding-empty",
    intent: "New user with no accounts asks to get set up.",
    prompt:
      "I'm just getting started. Can you create an account for my Binance exchange?",
    rubric: [
      {
        kind: "toolCalled",
        tool: "cryptofolio_manage_account",
        where: (i) => i.action === "add",
        desc: "adds an account via manage_account(action=add)",
      },
      {
        kind: "mentions",
        any: ["binance", "created", "added", "set up"],
        desc: "confirms the account was created",
      },
    ],
  },
  {
    id: "list-accounts",
    intent: "User asks what accounts exist.",
    prompt: "What accounts do I have set up?",
    setup: [["account", "add", "Kraken", "--type", "exchange", "--category", "trading"]],
    rubric: [
      {
        kind: "toolCalled",
        tool: "cryptofolio_list_accounts",
        desc: "lists accounts before answering",
      },
      { kind: "mentions", any: ["kraken"], desc: "names the existing account" },
    ],
  },
  {
    id: "record-buy-basic",
    intent: "User records a simple purchase on a manual account.",
    prompt: "I bought 0.5 BTC at $60,000 on my Ledger. Please record it.",
    setup: [["account", "add", "Ledger", "--type", "hardware_wallet", "--category", "cold-storage"]],
    rubric: [
      {
        kind: "toolCalled",
        tool: "cryptofolio_record_transaction",
        where: (i) => i.type === "buy" && asset(i) === "BTC",
        desc: "records a buy of BTC",
      },
    ],
  },
  {
    id: "record-buy-cost-basis-only",
    intent:
      "Recording a historical buy on a SYNCED account must use cost_basis_only to avoid double-counting the holding.",
    prompt:
      "My Binance account is synced. I want to backfill the cost basis for a past purchase: 1 ETH at $2,000. Record it without changing my current balance.",
    setup: [
      ["account", "add", "Binance", "--type", "exchange", "--category", "trading", "--sync"],
    ],
    rubric: [
      {
        kind: "toolCalled",
        tool: "cryptofolio_record_transaction",
        where: (i) => i.cost_basis_only === true,
        desc: "uses cost_basis_only=true so the synced balance is not doubled",
      },
    ],
  },
  {
    id: "portfolio-value",
    intent: "User asks for total value and P&L.",
    prompt: "What's my portfolio worth right now, and how's my P&L?",
    setup: [["account", "add", "Binance", "--type", "exchange", "--category", "trading"]],
    rubric: [
      {
        kind: "toolCalled",
        tool: "cryptofolio_get_portfolio",
        desc: "fetches the portfolio snapshot",
      },
    ],
  },
  {
    id: "price-eth",
    intent: "User asks for a spot price.",
    prompt: "What's the current price of ETH?",
    rubric: [
      {
        kind: "toolCalled",
        tool: "cryptofolio_get_prices",
        where: (i) => JSON.stringify(i).toUpperCase().includes("ETH"),
        desc: "looks up the ETH price via get_prices",
      },
    ],
  },
  {
    id: "analyze-eth",
    intent: "User asks how a specific asset is doing.",
    prompt: "How am I doing on ETH specifically — cost basis and gain?",
    setup: [["account", "add", "Binance", "--type", "exchange", "--category", "trading"]],
    rubric: [
      {
        kind: "toolCalled",
        tool: "cryptofolio_analyze_asset",
        where: (i) => asset(i) === "ETH",
        desc: "analyzes the ETH position",
      },
    ],
  },
  {
    id: "empty-portfolio-no-fabrication",
    intent:
      "With no accounts/holdings, the agent must report the empty state — never invent a balance.",
    prompt: "How much is my crypto worth?",
    rubric: [
      {
        kind: "toolCalled",
        tool: "cryptofolio_get_portfolio",
        desc: "checks the portfolio rather than guessing",
      },
      {
        kind: "mentions",
        any: ["no account", "no holdings", "empty", "don't have", "do not have", "set up", "nothing"],
        desc: "states the portfolio is empty / needs setup",
      },
      {
        kind: "notMentions",
        all: ["approximately $", "roughly $", "estimated $"],
        desc: "does not fabricate an estimated dollar value",
      },
    ],
  },
];
