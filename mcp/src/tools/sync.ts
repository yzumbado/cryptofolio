/**
 * cryptofolio_sync_exchange — pull live balances and history from Binance
 */

import { z } from "zod";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { runCliRaw, SYNC_TIMEOUT_MS } from "../cli.js";
import {
  buildSuccess,
  toContent,
  handleCliError,
} from "../formatters/response.js";

export function registerSyncExchangeTool(server: McpServer): void {
  server.tool(
    "cryptofolio_sync_exchange",
    "Pull current balances from a Binance exchange account via API. Requires Binance API keys to be configured (run cryptofolio config set-secret binance.api_key in your terminal first).",
    {
      account: z
        .string()
        .optional()
        .describe(
          "Account name to sync. Omit to sync the default account."
        ),
    },
    async ({ account }) => {
      try {
        const args = ["sync"];
        if (account) args.push("--account", account);

        // sync command does not have --json output; use runCliRaw
        const output = await runCliRaw(args, SYNC_TIMEOUT_MS);

        return toContent(
          buildSuccess(
            { output: output || "Sync completed." },
            account
              ? `Exchange account "${account}" synced successfully.`
              : "Exchange account synced successfully."
          )
        );
      } catch (err) {
        return toContent(handleCliError(err, "cryptofolio_sync_exchange"));
      }
    }
  );
}
