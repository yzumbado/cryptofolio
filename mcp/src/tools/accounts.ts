/**
 * cryptofolio_list_accounts — list all configured accounts
 * cryptofolio_manage_account — add or remove an account
 *
 * cryptofolio_manage_account handles the category dependency automatically:
 * if a CategoryNotFound error occurs during account add, it creates the
 * category first and retries.
 */

import { z } from "zod";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { runCli, runCliRaw } from "../cli.js";
import {
  buildSuccess,
  toContent,
  handleCliError,
  buildError,
} from "../formatters/response.js";
import type { CliAccount } from "../types.js";

// ---------------------------------------------------------------------------
// cryptofolio_list_accounts
// ---------------------------------------------------------------------------

export function registerListAccountsTool(server: McpServer): void {
  server.tool(
    "cryptofolio_list_accounts",
    "List all configured accounts (exchanges, wallets, custodial services). Returns account names, types, categories, and sync status. Call this before referencing an account by name in other tools.",
    {},
    async () => {
      try {
        const raw = await runCli(["account", "list"]);
        const accounts = (raw as CliAccount[]) ?? [];

        return toContent(
          buildSuccess(
            { accounts },
            `${accounts.length} account(s) found.`
          )
        );
      } catch (err) {
        return toContent(handleCliError(err, "cryptofolio_list_accounts"));
      }
    }
  );
}

// ---------------------------------------------------------------------------
// cryptofolio_manage_account
// ---------------------------------------------------------------------------

const ACCOUNT_TYPES = [
  "exchange",
  "hardware_wallet",
  "software_wallet",
  "custodial_service",
  "bank",
] as const;

export function registerManageAccountTool(server: McpServer): void {
  server.tool(
    "cryptofolio_manage_account",
    'Add or remove a portfolio account. Use action "add" to create a new account (exchange, hardware wallet, etc.) or "remove" to delete one. This is required during initial setup — create at least one account before recording any transactions.',
    {
      action: z
        .enum(["add", "remove"])
        .describe('Operation to perform: "add" or "remove"'),
      name: z.string().describe("Account name (e.g. Binance, Ledger)"),
      account_type: z
        .enum(ACCOUNT_TYPES)
        .optional()
        .describe(
          "Required when action=add. One of: exchange, hardware_wallet, software_wallet, custodial_service, bank"
        ),
      category: z
        .string()
        .optional()
        .describe(
          'Required when action=add. Category name (e.g. "trading", "cold-storage", "hot-wallets"). Will be created automatically if it does not exist.'
        ),
      sync_enabled: z
        .boolean()
        .optional()
        .describe(
          "Whether to enable automatic sync for this account (default: false)"
        ),
      testnet: z
        .boolean()
        .optional()
        .describe("Mark this account as a testnet account (default: false)"),
    },
    async ({ action, name, account_type, category, sync_enabled, testnet }) => {
      try {
        if (action === "add") {
          if (!account_type) {
            return toContent(
              buildError(
                'account_type is required when action is "add".',
                "MISSING_PARAM",
                `Provide one of: ${ACCOUNT_TYPES.join(", ")}`
              )
            );
          }
          if (!category) {
            return toContent(
              buildError(
                'category is required when action is "add".',
                "MISSING_PARAM",
                'Common categories: "trading", "cold-storage", "hot-wallets"'
              )
            );
          }

          const args = [
            "account",
            "add",
            name,
            "--type",
            account_type,
            "--category",
            category,
          ];
          if (sync_enabled === true) args.push("--sync");
          if (testnet === true) args.push("--testnet");

          try {
            await runCli(args);
          } catch (err: unknown) {
            // Auto-create the category if it doesn't exist, then retry
            const msg = err instanceof Error ? err.message : String(err);
            if (
              msg.toLowerCase().includes("category") &&
              (msg.toLowerCase().includes("not found") ||
                msg.toLowerCase().includes("does not exist"))
            ) {
              await runCliRaw(["category", "add", category]);
              await runCli(args);
            } else {
              throw err;
            }
          }

          return toContent(
            buildSuccess(
              { name, account_type, category, sync_enabled: sync_enabled ?? false },
              `Account "${name}" created successfully.`
            )
          );
        } else {
          // action === "remove"
          await runCliRaw(["account", "remove", name, "--yes"]);
          return toContent(
            buildSuccess(
              { name },
              `Account "${name}" removed successfully.`
            )
          );
        }
      } catch (err) {
        return toContent(handleCliError(err, "cryptofolio_manage_account"));
      }
    }
  );
}
