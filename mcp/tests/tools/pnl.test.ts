/**
 * Unit tests for P&L tools:
 * cryptofolio_get_pnl_summary, cryptofolio_get_realized_pnl,
 * cryptofolio_get_unrealized_pnl, cryptofolio_analyze_asset,
 * cryptofolio_pnl_backfill
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

import { runCli, runCliRaw } from "../../src/cli.js";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  registerGetPnlSummaryTool,
  registerGetRealizedPnlTool,
  registerGetUnrealizedPnlTool,
  registerAnalyzeAssetTool,
  registerPnlBackfillTool,
} from "../../src/tools/pnl.js";

type ToolHandler = (args: unknown) => Promise<{ content: Array<{ type: string; text: string }> }>;
type ToolRegistry = Record<string, { handler: ToolHandler }>;

function makeServer() {
  const server = new McpServer({ name: "test", version: "0.0.1" });
  registerGetPnlSummaryTool(server);
  registerGetRealizedPnlTool(server);
  registerGetUnrealizedPnlTool(server);
  registerAnalyzeAssetTool(server);
  registerPnlBackfillTool(server);
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

  it("returns error envelope when CLI fails", async () => {
    const { CliError } = await import("../../src/cli.js");
    vi.mocked(runCli).mockRejectedValueOnce(
      new CliError(1, "no transactions found", "pnl summary")
    );

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_pnl_summary");

    const result = await tool!.handler({});
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
    };

    expect(parsed.success).toBe(false);
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

  it("passes asset and date range filters to CLI", async () => {
    vi.mocked(runCli).mockResolvedValueOnce([
      { asset: "BTC", proceeds: "50000", cost_basis: "30000", gain_loss: "20000" },
    ]);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_realized_pnl");

    await tool!.handler({ asset: "BTC", from_date: "2024-01-01", to_date: "2024-12-31", limit: 50, offset: 0 });

    expect(vi.mocked(runCli)).toHaveBeenCalledWith(
      expect.arrayContaining(["pnl", "realized", "--asset", "BTC", "--from", "2024-01-01", "--to", "2024-12-31"])
    );
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

  it("rounds the total half-up with exact decimal math (float case)", async () => {
    // parseFloat("1.005").toFixed(2) === "1.00" in JS.
    vi.mocked(runCli).mockResolvedValueOnce([
      { asset: "AAA", quantity: "1", unrealized_pnl: "1.005" },
    ]);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_unrealized_pnl");

    const result = await tool!.handler({});
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      data: { total_unrealized_pnl: string };
    };

    expect(parsed.data.total_unrealized_pnl).toBe("1.01");
  });

  it("sums totals beyond Number.MAX_SAFE_INTEGER exactly", async () => {
    // Float addition collapses 9007199254740993 + 1 to 9007199254740992.
    vi.mocked(runCli).mockResolvedValueOnce([
      { asset: "AAA", quantity: "1", unrealized_pnl: "9007199254740993" },
      { asset: "BBB", quantity: "1", unrealized_pnl: "1" },
    ]);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_unrealized_pnl");

    const result = await tool!.handler({});
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      data: { total_unrealized_pnl: string };
    };

    expect(parsed.data.total_unrealized_pnl).toBe("9007199254740994.00");
  });

  it("fails closed when the CLI returns a non-list payload", async () => {
    vi.mocked(runCli).mockResolvedValueOnce({
      message: "Command completed (no structured output)",
    });

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_unrealized_pnl");

    const result = await tool!.handler({});
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      error: string;
    };

    expect(parsed.success).toBe(false);
    expect(parsed.error).toContain("non-list payload");
  });

  it("passes account and asset filters to CLI", async () => {
    vi.mocked(runCli).mockResolvedValueOnce([
      { asset: "SOL", quantity: "100", unrealized_pnl: "500.00" },
    ]);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_unrealized_pnl");

    await tool!.handler({ account: "Binance", asset: "SOL" });

    expect(vi.mocked(runCli)).toHaveBeenCalledWith(
      expect.arrayContaining(["pnl", "unrealized", "--account", "Binance", "--asset", "SOL"])
    );
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

  it("returns error envelope when CLI fails for unknown asset", async () => {
    const { CliError } = await import("../../src/cli.js");
    vi.mocked(runCli).mockRejectedValueOnce(
      new CliError(1, "asset 'DOGE' has no transactions", "pnl by-asset")
    );

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_analyze_asset");

    const result = await tool!.handler({ asset: "DOGE" });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
    };

    expect(parsed.success).toBe(false);
    expect(vi.mocked(runCli)).toHaveBeenCalledWith(
      expect.arrayContaining(["pnl", "by-asset", "DOGE"])
    );
  });
});

describe("cryptofolio_pnl_backfill", () => {
  beforeEach(() => vi.clearAllMocks());

  it("runs a full backfill with --yes and returns CLI output", async () => {
    vi.mocked(runCliRaw).mockResolvedValueOnce(
      "[OK] Backfill complete! Processed 3 buys, 1 sells, 0 swaps, 0 transfers, 0 corrections"
    );

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_pnl_backfill");

    const result = await tool!.handler({});
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      data: { output: string };
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.data.output).toContain("Backfill complete");
    expect(parsed.message).toContain("all accounts");
    expect(vi.mocked(runCliRaw)).toHaveBeenCalledWith(
      expect.arrayContaining(["pnl", "backfill", "--yes"])
    );
  });

  it("scopes the backfill to one account when provided", async () => {
    vi.mocked(runCliRaw).mockResolvedValueOnce("[OK] Backfill complete! Processed 1 buys");

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_pnl_backfill");

    const result = await tool!.handler({ account: "Binance" });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.message).toContain("Binance");
    expect(vi.mocked(runCliRaw)).toHaveBeenCalledWith(
      expect.arrayContaining(["pnl", "backfill", "--yes", "--account", "Binance"])
    );
  });

  it("returns error envelope when CLI fails", async () => {
    const { CliError } = await import("../../src/cli.js");
    vi.mocked(runCliRaw).mockRejectedValueOnce(
      new CliError(1, "database is locked", "pnl")
    );

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_pnl_backfill");

    const result = await tool!.handler({});
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      error: string;
    };

    expect(parsed.success).toBe(false);
    expect(parsed.error).toContain("database is locked");
  });
});
