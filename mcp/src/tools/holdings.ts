/**
 * cryptofolio_list_holdings — raw positions with quantity and cost basis
 */

import { z } from "zod";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { runCli } from "../cli.js";
import {
  buildSuccess,
  toContent,
  handleCliError,
  outputEnvelopeNote,
} from "../formatters/response.js";
import type { CliHolding } from "../types.js";

export function registerListHoldingsTool(server: McpServer): void {
  server.tool(
    "cryptofolio_list_holdings",
    "List raw holdings across accounts with quantity and average cost basis, optionally filtered by account; use cryptofolio_get_portfolio for live USD values and unrealized P&L. " +
      outputEnvelopeNote("[{asset, quantity, cost_basis, account, account_id}]"),
    {
      account: z
        .string()
        .optional()
        .describe("Filter to a specific account name"),
    },
    async ({ account }) => {
      try {
        const args = ["holdings", "list"];
        if (account) args.push("--account", account);

        const raw = await runCli(args);
        const holdings = Array.isArray(raw) ? (raw as CliHolding[]) : [];

        return toContent(
          buildSuccess(
            holdings,
            `${holdings.length} holding(s)` +
              (account ? ` in "${account}".` : ".")
          )
        );
      } catch (err) {
        return toContent(handleCliError(err, "cryptofolio_list_holdings"));
      }
    }
  );
}
