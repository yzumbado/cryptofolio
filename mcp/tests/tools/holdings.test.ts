/**
 * Unit tests for cryptofolio_list_holdings
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
import { registerListHoldingsTool } from "../../src/tools/holdings.js";

type ToolHandler = (args: unknown) => Promise<{ content: Array<{ type: string; text: string }> }>;
type ToolRegistry = Record<string, { handler: ToolHandler }>;

function makeServer() {
  const server = new McpServer({ name: "test", version: "0.0.1" });
  registerListHoldingsTool(server);
  return server;
}

function getTool(server: McpServer, name: string) {
  return (server as unknown as { _registeredTools: ToolRegistry })
    ._registeredTools[name];
}

describe("cryptofolio_list_holdings", () => {
  beforeEach(() => vi.clearAllMocks());

  it("lists all holdings with quantity and cost basis", async () => {
    vi.mocked(runCli).mockResolvedValueOnce([
      {
        asset: "BTC",
        quantity: "0.5",
        cost_basis: "45000",
        account: "Ledger",
        account_id: "acc-1",
      },
      {
        asset: "ETH",
        quantity: "2.0",
        cost_basis: null,
        account: "Binance",
        account_id: "acc-2",
      },
    ]);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_list_holdings");

    const result = await tool!.handler({});
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      data: unknown[];
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.data).toHaveLength(2);
    expect(parsed.message).toContain("2 holding(s)");
    expect(vi.mocked(runCli)).toHaveBeenCalledWith(
      expect.arrayContaining(["holdings", "list"])
    );
  });

  it("passes the account filter to the CLI", async () => {
    vi.mocked(runCli).mockResolvedValueOnce([]);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_list_holdings");

    const result = await tool!.handler({ account: "Binance" });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.message).toContain("Binance");
    expect(vi.mocked(runCli)).toHaveBeenCalledWith(
      expect.arrayContaining(["holdings", "list", "--account", "Binance"])
    );
  });

  it("returns an empty list message when there are no holdings", async () => {
    vi.mocked(runCli).mockResolvedValueOnce([]);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_list_holdings");

    const result = await tool!.handler({});
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      data: unknown[];
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.data).toHaveLength(0);
    expect(parsed.message).toContain("0 holding(s)");
  });

  it("returns an error envelope when the account does not exist", async () => {
    const { CliError } = await import("../../src/cli.js");
    vi.mocked(runCli).mockRejectedValueOnce(
      new CliError(1, "Account not found: Nope", "holdings")
    );

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_list_holdings");

    const result = await tool!.handler({ account: "Nope" });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
    };

    expect(parsed.success).toBe(false);
  });
});
