/**
 * cryptofolio_get_mining_pnl — DePIN mining P&L statement
 */

import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { runCli } from "../cli.js";
import {
  buildSuccess,
  toContent,
  handleCliError,
  outputEnvelopeNote,
} from "../formatters/response.js";
import type { CliMiningPnl } from "../types.js";

export function registerGetMiningPnlTool(server: McpServer): void {
  server.tool(
    "cryptofolio_get_mining_pnl",
    "Return the DePIN mining P&L statement: earned-token revenue at current fair market value, hardware depreciation, operating profit, and capital recovery. " +
      outputEnvelopeNote(
        "{revenue_usd, depreciation_usd, opex_usd, operating_profit_usd, hardware_cost_usd, net_book_value_usd, capital_recovered_percent}"
      ),
    {},
    async () => {
      try {
        // `mining-pnl` takes no options; runCli adds --json --quiet.
        const raw = await runCli(["mining-pnl"]);
        const pnl = raw as CliMiningPnl;

        return toContent(
          buildSuccess(
            pnl,
            `Mining operating profit: $${pnl.operating_profit_usd}` +
              ` (revenue $${pnl.revenue_usd}, depreciation $${pnl.depreciation_usd},` +
              ` capital recovered ${pnl.capital_recovered_percent}%).`
          )
        );
      } catch (err) {
        return toContent(handleCliError(err, "cryptofolio_get_mining_pnl"));
      }
    }
  );
}
