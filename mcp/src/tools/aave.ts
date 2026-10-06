/**
 * cryptofolio_aave_health — Aave V3 position health (collateral, debt, health factor)
 */

import { z } from "zod";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { runCli } from "../cli.js";
import {
  buildSuccess,
  toContent,
  handleCliError,
  outputEnvelopeNote,
  TOOL_OUTPUT_SCHEMA,
} from "../formatters/response.js";

interface AaveHealthPosition {
  address: string;
  collateral_usd: string;
  debt_usd: string;
  available_borrows_usd: string;
  liquidation_threshold_pct: string;
  ltv_pct: string;
  health_factor: string | null; // null = no debt ("infinite" health)
}

export function registerAaveHealthTool(server: McpServer): void {
  server.registerTool(
    "cryptofolio_aave_health",
    {
      description:
        "Fetch Aave V3 position health for a tracked Ethereum wallet or address: collateral, debt, available borrows, LTV, liquidation threshold, and health factor. Read-only, from the protocol's own getUserAccountData view (watch-only, no keys, no spending). " +
        outputEnvelopeNote(
          "{positions: [{address, collateral_usd, debt_usd, available_borrows_usd, liquidation_threshold_pct, ltv_pct, health_factor}]}"
        ),
      inputSchema: {
        wallet: z
          .string()
          .optional()
          .describe("Wallet name (resolved from the accounts/wallets tables)"),
        address: z.string().optional().describe("Direct Ethereum address"),
      },
      outputSchema: TOOL_OUTPUT_SCHEMA,
    },
    async ({ wallet, address }) => {
      try {
        const args = ["aave", "health"];
        if (wallet) args.push("--wallet", wallet);
        if (address) args.push("--address", address);

        const raw = await runCli(args);
        const positions = (raw as AaveHealthPosition[]) ?? [];

        const atRisk = positions.filter((p) => {
          if (p.health_factor == null) return false; // no debt => no liquidation risk
          const hf = Number(p.health_factor);
          return Number.isFinite(hf) && hf < 1;
        }).length;

        return toContent(
          buildSuccess(
            { positions },
            `${positions.length} Aave position(s).` +
              (atRisk > 0
                ? ` ⚠️ ${atRisk} with health factor below 1.0 (liquidation risk).`
                : "")
          )
        );
      } catch (err) {
        return toContent(handleCliError(err, "cryptofolio_aave_health"));
      }
    }
  );
}
