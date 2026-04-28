/**
 * Unit tests for cryptofolio_get_prices and cryptofolio_get_market_data
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("../../src/cli.js", () => ({
  runCli: vi.fn(),
  runCliRaw: vi.fn(),
  CliError: class CliError extends Error {
    constructor(
      public exitCode: number,
      public stderr: string,
      public command: string,
      public hint?: string
    ) {
      super(`cryptofolio ${command} failed (exit ${exitCode}): ${stderr}`);
      this.name = "CliError";
    }
  },
  SYNC_TIMEOUT_MS: 120_000,
}));

import { runCli } from "../../src/cli.js";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  registerGetPricesTool,
  registerGetMarketDataTool,
} from "../../src/tools/prices.js";

type ToolHandler = (args: unknown) => Promise<{ content: Array<{ type: string; text: string }> }>;
type ToolRegistry = Record<string, { handler: ToolHandler }>;

function makeServer() {
  const server = new McpServer({ name: "test", version: "0.0.1" });
  registerGetPricesTool(server);
  registerGetMarketDataTool(server);
  return server;
}

function getTool(server: McpServer, name: string) {
  return (server as unknown as { _registeredTools: ToolRegistry })
    ._registeredTools[name];
}

describe("cryptofolio_get_prices", () => {
  beforeEach(() => vi.clearAllMocks());

  it("returns prices for multiple assets", async () => {
    vi.mocked(runCli).mockResolvedValueOnce([
      { symbol: "BTC", price: "65000.00" },
      { symbol: "ETH", price: "3200.00" },
    ]);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_prices");

    const result = await tool!.handler({ assets: ["BTC", "ETH"] });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      data: { prices: unknown[] };
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.data.prices).toHaveLength(2);
    expect(parsed.message).toContain("BTC: $65000.00");
    expect(parsed.message).toContain("ETH: $3200.00");
    expect(vi.mocked(runCli)).toHaveBeenCalledWith(
      expect.arrayContaining(["price", "BTC", "ETH"])
    );
  });

  it("returns a single price", async () => {
    vi.mocked(runCli).mockResolvedValueOnce([{ symbol: "SOL", price: "150.00" }]);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_prices");

    const result = await tool!.handler({ assets: ["SOL"] });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.message).toContain("SOL: $150.00");
  });
});

describe("cryptofolio_get_market_data", () => {
  beforeEach(() => vi.clearAllMocks());

  it("returns market data with 24h stats", async () => {
    vi.mocked(runCli).mockResolvedValueOnce({
      symbol: "BTC",
      price: "65000.00",
      ticker_24h: {
        price_change_percent: "2.5",
        high_24h: "66000.00",
        low_24h: "63000.00",
        volume: "25000.00",
      },
    });

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_market_data");

    const result = await tool!.handler({ asset: "BTC" });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.message).toContain("BTC: $65000.00");
    expect(parsed.message).toContain("2.5%");
    expect(parsed.message).toContain("66000.00");
    expect(vi.mocked(runCli)).toHaveBeenCalledWith(
      expect.arrayContaining(["market", "BTC", "--24h"])
    );
  });

  it("returns basic market data when 24h stats are absent", async () => {
    vi.mocked(runCli).mockResolvedValueOnce({
      symbol: "ADA",
      price: "0.45",
      ticker_24h: null,
    });

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_market_data");

    const result = await tool!.handler({ asset: "ADA" });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.message).toContain("ADA: $0.45");
    expect(parsed.message).not.toContain("24h change");
  });
});
