/**
 * Unit tests for cryptofolio_get_mining_pnl
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
import { registerGetMiningPnlTool } from "../../src/tools/mining.js";

type ToolHandler = (args: unknown) => Promise<{ content: Array<{ type: string; text: string }> }>;
type ToolRegistry = Record<string, { handler: ToolHandler }>;

function makeServer() {
  const server = new McpServer({ name: "test", version: "0.0.1" });
  registerGetMiningPnlTool(server);
  return server;
}

function getTool(server: McpServer, name: string) {
  return (server as unknown as { _registeredTools: ToolRegistry })
    ._registeredTools[name];
}

describe("cryptofolio_get_mining_pnl", () => {
  beforeEach(() => vi.clearAllMocks());

  it("returns the mining P&L statement with a summary message", async () => {
    vi.mocked(runCli).mockResolvedValueOnce({
      revenue_usd: "1200.50",
      depreciation_usd: "400.00",
      opex_usd: "0.00",
      operating_profit_usd: "800.50",
      hardware_cost_usd: "2400.00",
      net_book_value_usd: "2000.00",
      capital_recovered_percent: "33.4",
    });

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_mining_pnl");

    const result = await tool!.handler({});
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      data: { operating_profit_usd: string };
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.data.operating_profit_usd).toBe("800.50");
    expect(parsed.message).toContain("800.50");
    expect(parsed.message).toContain("33.4%");
    expect(vi.mocked(runCli)).toHaveBeenCalledWith(
      expect.arrayContaining(["mining-pnl"])
    );
  });

  it("returns an error envelope when the CLI fails", async () => {
    const { CliError } = await import("../../src/cli.js");
    vi.mocked(runCli).mockRejectedValueOnce(
      new CliError(1, "config could not be loaded", "mining-pnl")
    );

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_mining_pnl");

    const result = await tool!.handler({});
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
    };

    expect(parsed.success).toBe(false);
  });
});
