/**
 * cryptofolio_get_pnl_summary      — overall realized + unrealized P&L
 * cryptofolio_get_realized_pnl     — closed positions with gain/loss per disposal
 * cryptofolio_get_unrealized_pnl   — open position P&L (mark-to-market)
 * cryptofolio_analyze_asset        — deep dive on one asset: holdings + P&L
 */

import { z } from "zod";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { runCli } from "../cli.js";
import {
  buildSuccess,
  toContent,
  handleCliError,
  paginate,
} from "../formatters/response.js";
import type {
  CliPnlSummary,
  CliRealizedEntry,
  CliUnrealizedEntry,
} from "../types.js";

// ---------------------------------------------------------------------------
// cryptofolio_get_pnl_summary
// ---------------------------------------------------------------------------

export function registerGetPnlSummaryTool(server: McpServer): void {
  server.tool(
    "cryptofolio_get_pnl_summary",
    "Get a high-level P&L summary combining realized and unrealized gains/losses. Optionally filter by account or date range. Great for a quick answer to 'how am I doing overall?'",
    {
      account: z
        .string()
        .optional()
        .describe("Filter to a specific account"),
      from_date: z
        .string()
        .optional()
        .describe("Start date in YYYY-MM-DD format"),
      to_date: z
        .string()
        .optional()
        .describe("End date in YYYY-MM-DD format"),
    },
    async ({ account, from_date, to_date }) => {
      try {
        const args = ["pnl", "summary"];
        if (account) args.push("--account", account);
        if (from_date) args.push("--from", from_date);
        if (to_date) args.push("--to", to_date);

        const raw = await runCli(args);
        const summary = raw as CliPnlSummary;

        return toContent(
          buildSuccess(
            summary,
            `Realized: $${summary.total_realized} | Unrealized: $${summary.total_unrealized} | Net: $${summary.net_pnl}`
          )
        );
      } catch (err) {
        return toContent(handleCliError(err, "cryptofolio_get_pnl_summary"));
      }
    }
  );
}

// ---------------------------------------------------------------------------
// cryptofolio_get_realized_pnl
// ---------------------------------------------------------------------------

export function registerGetRealizedPnlTool(server: McpServer): void {
  server.tool(
    "cryptofolio_get_realized_pnl",
    "List closed positions with gain/loss per disposal event. Each entry shows the asset sold, proceeds, cost basis, and net gain or loss. Use this for tax year reporting.",
    {
      account: z
        .string()
        .optional()
        .describe("Filter to a specific account"),
      asset: z
        .string()
        .optional()
        .describe("Filter to a specific asset symbol"),
      from_date: z
        .string()
        .optional()
        .describe("Start date (e.g. 2024-01-01)"),
      to_date: z
        .string()
        .optional()
        .describe("End date (e.g. 2024-12-31)"),
      limit: z
        .number()
        .int()
        .min(1)
        .max(500)
        .default(50)
        .describe("Number of entries to return (default: 50)"),
      offset: z
        .number()
        .int()
        .min(0)
        .default(0)
        .describe("Entries to skip for pagination (default: 0)"),
    },
    async ({ account, asset, from_date, to_date, limit, offset }) => {
      try {
        const fetchLimit = offset + limit;
        const args = ["pnl", "realized", "--limit", String(fetchLimit)];
        if (account) args.push("--account", account);
        if (asset) args.push("--asset", asset);
        if (from_date) args.push("--from", from_date);
        if (to_date) args.push("--to", to_date);

        const raw = await runCli(args);
        const entries = (raw as CliRealizedEntry[]) ?? [];
        const page = paginate(entries, offset, limit);

        return toContent(
          buildSuccess(
            page,
            `${page.items.length} realized P&L entry(ies).`
          )
        );
      } catch (err) {
        return toContent(
          handleCliError(err, "cryptofolio_get_realized_pnl")
        );
      }
    }
  );
}

// ---------------------------------------------------------------------------
// cryptofolio_get_unrealized_pnl
// ---------------------------------------------------------------------------

export function registerGetUnrealizedPnlTool(server: McpServer): void {
  server.tool(
    "cryptofolio_get_unrealized_pnl",
    "Show open position P&L — what you would gain or lose if you sold everything today. Includes average cost basis, current price, and unrealized gain per asset.",
    {
      account: z
        .string()
        .optional()
        .describe("Filter to a specific account"),
      asset: z
        .string()
        .optional()
        .describe("Filter to a specific asset symbol"),
    },
    async ({ account, asset }) => {
      try {
        const args = ["pnl", "unrealized"];
        if (account) args.push("--account", account);
        if (asset) args.push("--asset", asset);

        const raw = await runCli(args);
        const entries = (raw as CliUnrealizedEntry[]) ?? [];

        // Compute totals
        const totalUnrealized = entries
          .reduce((sum, e) => sum + parseFloat(e.unrealized_pnl), 0)
          .toFixed(2);

        return toContent(
          buildSuccess(
            { entries, total_unrealized_pnl: totalUnrealized },
            `${entries.length} open position(s). Total unrealized P&L: $${totalUnrealized}`
          )
        );
      } catch (err) {
        return toContent(
          handleCliError(err, "cryptofolio_get_unrealized_pnl")
        );
      }
    }
  );
}

// ---------------------------------------------------------------------------
// cryptofolio_analyze_asset
// ---------------------------------------------------------------------------

export function registerAnalyzeAssetTool(server: McpServer): void {
  server.tool(
    "cryptofolio_analyze_asset",
    "Deep dive on a single asset: realized P&L, unrealized P&L, current holdings, and cost basis breakdown. Great for answering 'how is my BTC position doing?'",
    {
      asset: z
        .string()
        .describe("Asset symbol to analyze (e.g. BTC, ETH, SOL)"),
      account: z
        .string()
        .optional()
        .describe("Filter to a specific account"),
    },
    async ({ asset, account }) => {
      try {
        const args = ["pnl", "by-asset", asset];
        if (account) args.push("--account", account);

        const raw = await runCli(args);

        return toContent(
          buildSuccess(
            raw ?? {},
            `P&L analysis for ${asset}${account ? ` in ${account}` : ""}.`
          )
        );
      } catch (err) {
        return toContent(handleCliError(err, "cryptofolio_analyze_asset"));
      }
    }
  );
}
