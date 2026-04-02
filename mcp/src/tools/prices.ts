/**
 * cryptofolio_get_prices — spot prices for one or more assets
 * cryptofolio_get_market_data — 24h OHLCV market statistics for one asset
 */

import { z } from "zod";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { runCli } from "../cli.js";
import {
  buildSuccess,
  toContent,
  handleCliError,
} from "../formatters/response.js";
import type { CliPrice, CliMarketOutput } from "../types.js";

// ---------------------------------------------------------------------------
// cryptofolio_get_prices
// ---------------------------------------------------------------------------

export function registerGetPricesTool(server: McpServer): void {
  server.tool(
    "cryptofolio_get_prices",
    "Get current spot prices for one or more crypto assets. Returns price in USD. Useful for quick lookups before recording transactions or analyzing portfolio value.",
    {
      assets: z
        .array(z.string())
        .min(1)
        .describe('List of asset symbols, e.g. ["BTC", "ETH", "SOL"]'),
    },
    async ({ assets }) => {
      try {
        const raw = await runCli(["price", ...assets]);
        const prices = (raw as CliPrice[]) ?? [];

        return toContent(
          buildSuccess(
            { prices },
            prices.map((p) => `${p.symbol}: $${p.price}`).join(", ")
          )
        );
      } catch (err) {
        return toContent(handleCliError(err, "cryptofolio_get_prices"));
      }
    }
  );
}

// ---------------------------------------------------------------------------
// cryptofolio_get_market_data
// ---------------------------------------------------------------------------

export function registerGetMarketDataTool(server: McpServer): void {
  server.tool(
    "cryptofolio_get_market_data",
    "Get 24-hour market statistics for a single asset: price, change, high, low, and volume.",
    {
      asset: z.string().describe('Asset symbol, e.g. "BTC"'),
    },
    async ({ asset }) => {
      try {
        const raw = await runCli(["market", asset, "--24h"]);
        const market = raw as CliMarketOutput;

        let summary = `${market.symbol}: $${market.price}`;
        if (market.ticker_24h) {
          summary +=
            ` | 24h change: ${market.ticker_24h.price_change_percent}%` +
            ` | high: $${market.ticker_24h.high_24h}` +
            ` | low: $${market.ticker_24h.low_24h}`;
        }

        return toContent(buildSuccess(market, summary));
      } catch (err) {
        return toContent(handleCliError(err, "cryptofolio_get_market_data"));
      }
    }
  );
}
