/**
 * cryptofolio_get_portfolio — full portfolio snapshot with unrealized P&L
 */

import { z } from "zod";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { runCli } from "../cli.js";
import {
  buildSuccess,
  toContent,
  handleCliError,
} from "../formatters/response.js";
import type { CliPortfolio } from "../types.js";

export function registerPortfolioTool(server: McpServer): void {
  server.tool(
    "cryptofolio_get_portfolio",
    "Get the current portfolio snapshot including all asset holdings, current values, cost basis, and unrealized P&L. Optionally filter by account or category.",
    {
      account: z
        .string()
        .optional()
        .describe("Filter to a specific account name"),
      category: z
        .string()
        .optional()
        .describe(
          'Filter to a category (e.g. "trading", "cold-storage")'
        ),
      by_account: z
        .boolean()
        .optional()
        .describe("Group holdings by account (default: false)"),
    },
    async ({ account, category, by_account }) => {
      try {
        const args = ["portfolio"];
        if (account) args.push("--account", account);
        if (category) args.push("--category", category);
        if (by_account === true) args.push("--by-account");

        const raw = await runCli(args);
        const portfolio = raw as CliPortfolio;

        return toContent(
          buildSuccess(
            portfolio,
            `Portfolio total: $${portfolio.total_value_usd} USD` +
              ` (unrealized P&L: ${portfolio.unrealized_pnl_percent}%)`
          )
        );
      } catch (err) {
        return toContent(handleCliError(err, "cryptofolio_get_portfolio"));
      }
    }
  );
}
