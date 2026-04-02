/**
 * cryptofolio_get_system_status
 *
 * Aggregates config show + account list to give the LLM a complete picture
 * of whether cryptofolio is configured and ready to use. Called first in
 * every new session and as part of the onboarding flow.
 */

import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { runCli } from "../cli.js";
import {
  buildSuccess,
  buildError,
  toContent,
  handleCliError,
} from "../formatters/response.js";
import type { CliConfigOutput, CliAccount } from "../types.js";

interface SystemStatus {
  config_path: string;
  db_path: string;
  accounts_count: number;
  accounts: Array<{ name: string; type: string; category: string }>;
  binance_configured: boolean;
  blockfrost_configured: boolean;
  default_account: string | null;
  testnet_mode: boolean;
  ready_for_use: boolean;
  setup_steps_remaining: string[];
}

export function registerStatusTool(server: McpServer): void {
  server.tool(
    "cryptofolio_get_system_status",
    "Check whether cryptofolio is configured and ready to use. Returns account count, API key status, and a checklist of remaining setup steps. Call this first in every new session.",
    {},
    async () => {
      try {
        const [configRaw, accountsRaw] = await Promise.all([
          runCli(["config", "show"]),
          runCli(["account", "list"]),
        ]);

        const config = configRaw as CliConfigOutput;
        const accounts = (accountsRaw as CliAccount[]) ?? [];

        const setup_steps_remaining: string[] = [];

        if (accounts.length === 0) {
          setup_steps_remaining.push(
            "Create at least one account using cryptofolio_manage_account"
          );
        }

        const hasExchangeAccount = accounts.some(
          (a) => a.account_type === "exchange"
        );
        if (
          hasExchangeAccount &&
          (!config.binance.api_key_configured ||
            !config.binance.api_secret_configured)
        ) {
          setup_steps_remaining.push(
            "Configure Binance API keys by running in your terminal: " +
              "cryptofolio config set-secret binance.api_key && " +
              "cryptofolio config set-secret binance.api_secret"
          );
        }

        const status: SystemStatus = {
          config_path: config.paths.config_dir + "/config.toml",
          db_path: config.paths.database,
          accounts_count: accounts.length,
          accounts: accounts.map((a) => ({
            name: a.name,
            type: a.account_type,
            category: a.category,
          })),
          binance_configured:
            config.binance.api_key_configured &&
            config.binance.api_secret_configured,
          blockfrost_configured:
            config.blockfrost.mainnet_api_key_configured,
          default_account: config.general.default_account,
          testnet_mode: config.general.use_testnet,
          ready_for_use:
            accounts.length > 0 && setup_steps_remaining.length === 0,
          setup_steps_remaining,
        };

        const message =
          status.ready_for_use
            ? `Cryptofolio is ready. ${accounts.length} account(s) configured.`
            : `Setup incomplete. ${setup_steps_remaining.length} step(s) remaining.`;

        return toContent(buildSuccess(status, message));
      } catch (err) {
        return toContent(
          handleCliError(err, "cryptofolio_get_system_status")
        );
      }
    }
  );
}
