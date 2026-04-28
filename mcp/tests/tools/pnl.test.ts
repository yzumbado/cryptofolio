/**
 * Unit tests for P&L tools:
 * cryptofolio_get_pnl_summary, cryptofolio_get_realized_pnl,
 * cryptofolio_get_unrealized_pnl, cryptofolio_analyze_asset
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
  registerGetPnlSummaryTool,
  registerGetRealizedPnlTool,
  registerGetUnrealizedPnlTool,
  registerAnalyzeAssetTool,
} from "../../src/tools/pnl.js";

type ToolHandler = (args: unknown) => Promise<{ content: Array<{ type: string; text: string }> }>;
type ToolRegistry = Record<string, { handler: ToolHandler }>;

function makeServer() {
  const server = new McpServer({ name: "test", version: "0.0.1" });
  registerGetPnlSummaryTool(server);
  registerGetRealizedPnlTool(server);
  registerGetUnrealizedPnlTool(server);
  registerAnalyzeAssetTool(server);
  return server;
}

function getTool(server: McpServer, name: string) {
  return (server as unknown as { _registeredTools: ToolRegistry })
    ._registeredTools[name];
}

describe("cryptofolio_get_pnl_summary", () => {
  beforeEach(() => vi.clearAllMocks());

  it("returns summary with realized, unrealized, and net P&L", async () => {
    vi.mocked(runCli).mockResolvedValueOnce({
      total_realized: "1200.00",
      total_unrealized: "800.50",
      net_pnl: "2000.50",
    });

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_pnl_summary");

    const result = await tool!.handler({});
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.message).toContain("1200.00");
    expect(parsed.message).toContain("800.50");
    expect(parsed.message).toContain("2000.50");
  });

  it("passes account and date filters to CLI", async () => {
    vi.mocked(runCli).mockResolvedValueOnce({
      total_realized: "0.00",
      total_unrealized: "0.00",
      net_pnl: "0.00",
    });

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_pnl_summary");

    await tool!.handler({ account: "Binance", from_date: "2024-01-01", to_date: "2024-12-31" });

    expect(vi.mocked(runCli)).toHaveBeenCalledWith(
      expect.arrayContaining(["pnl", "summary", "--account", "Binance", "--from", "2024-01-01", "--to", "2024-12-31"])
    );
  });
});

describe("cryptofolio_get_realized_pnl", () => {
  beforeEach(() => vi.clearAllMocks());

  it("returns paginated realized P&L entries", async () => {
    vi.mocked(runCli).mockResolvedValueOnce([
      { asset: "BTC", proceeds: "30000", cost_basis: "20000", gain_loss: "10000" },
      { asset: "ETH", proceeds: "2000", cost_basis: "1500", gain_loss: "500" },
    ]);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_realized_pnl");

    const result = await tool!.handler({ limit: 50, offset: 0 });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      data: { items: unknown[]; total: number };
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.data.items).toHaveLength(2);
    expect(parsed.message).toContain("2 realized");
  });

  it("returns empty list gracefully", async () => {
    vi.mocked(runCli).mockResolvedValueOnce([]);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_realized_pnl");

    const result = await tool!.handler({ limit: 50, offset: 0 });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      data: { items: unknown[] };
    };

    expect(parsed.success).toBe(true);
    expect(parsed.data.items).toHaveLength(0);
  });
});

describe("cryptofolio_get_unrealized_pnl", () => {
  beforeEach(() => vi.clearAllMocks());

  it("returns open positions and computes total unrealized P&L", async () => {
    vi.mocked(runCli).mockResolvedValueOnce([
      { asset: "BTC", quantity: "0.5", unrealized_pnl: "5000.00" },
      { asset: "ETH", quantity: "2.0", unrealized_pnl: "-200.00" },
    ]);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_unrealized_pnl");

    const result = await tool!.handler({});
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      data: { entries: unknown[]; total_unrealized_pnl: string };
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.data.entries).toHaveLength(2);
    expect(parseFloat(parsed.data.total_unrealized_pnl)).toBeCloseTo(4800, 0);
    expect(parsed.message).toContain("2 open position(s)");
  });

  it("returns zero total for empty portfolio", async () => {
    vi.mocked(runCli).mockResolvedValueOnce([]);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_unrealized_pnl");

    const result = await tool!.handler({});
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      data: { total_unrealized_pnl: string };
    };

    expect(parsed.success).toBe(true);
    expect(parsed.data.total_unrealized_pnl).toBe("0.00");
  });
});

describe("cryptofolio_analyze_asset", () => {
  beforeEach(() => vi.clearAllMocks());

  it("returns per-asset analysis", async () => {
    vi.mocked(runCli).mockResolvedValueOnce({
      asset: "BTC",
      realized_pnl: "10000.00",
      unrealized_pnl: "5000.00",
      holdings: [{ account: "Binance", quantity: "0.5" }],
    });

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_analyze_asset");

    const result = await tool!.handler({ asset: "BTC" });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.message).toContain("BTC");
    expect(vi.mocked(runCli)).toHaveBeenCalledWith(
      expect.arrayContaining(["pnl", "by-asset", "BTC"])
    );
  });

  it("includes account filter when provided", async () => {
    vi.mocked(runCli).mockResolvedValueOnce({ asset: "ETH" });

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_analyze_asset");

    await tool!.handler({ asset: "ETH", account: "Ledger" });

    expect(vi.mocked(runCli)).toHaveBeenCalledWith(
      expect.arrayContaining(["pnl", "by-asset", "ETH", "--account", "Ledger"])
    );
  });
});
