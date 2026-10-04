/**
 * Unit tests for transaction tools
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";

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
  registerListTransactionsTool,
  registerRecordTransactionTool,
  registerTrackConversionTool,
  registerExportTransactionsTool,
} from "../../src/tools/transactions.js";

const fixturesDir = join(import.meta.dirname, "../fixtures");

function loadFixture(name: string): unknown {
  return JSON.parse(readFileSync(join(fixturesDir, name), "utf-8"));
}

type ToolHandler = (args: unknown) => Promise<{ content: Array<{ type: string; text: string }> }>;
type ToolRegistry = Record<string, { handler: ToolHandler }>;

function makeServer() {
  const server = new McpServer({ name: "test", version: "0.0.1" });
  registerListTransactionsTool(server);
  registerRecordTransactionTool(server);
  registerTrackConversionTool(server);
  registerExportTransactionsTool(server);
  return server;
}

function getTool(server: McpServer, name: string) {
  return (server as unknown as { _registeredTools: ToolRegistry })
    ._registeredTools[name];
}

/** Emulate `cryptofolio tx list --limit N`: returns the first N rows. */
function mockTxList(rows: Array<Record<string, unknown>>): void {
  vi.mocked(runCli).mockImplementation(async (args: string[]) => {
    const i = args.indexOf("--limit");
    const n = i === -1 ? rows.length : Number(args[i + 1] ?? rows.length);
    return rows.slice(0, n);
  });
}

describe("cryptofolio_list_transactions", () => {
  // reset (not just clear) invalidates mockImplementation between tests
  beforeEach(() => vi.resetAllMocks());

  it("returns paginated transaction list", async () => {
    vi.mocked(runCli).mockResolvedValueOnce(loadFixture("tx-list.json"));

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_list_transactions");

    const result = await tool!.handler({ limit: 50, offset: 0 });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      data: { items: unknown[]; has_more: boolean; total_fetched: number };
    };

    expect(parsed.success).toBe(true);
    expect(parsed.data.items).toHaveLength(2);
    expect(parsed.data.has_more).toBe(false);
  });

  it("filters by asset client-side", async () => {
    vi.mocked(runCli).mockResolvedValueOnce(loadFixture("tx-list.json"));

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_list_transactions");

    const result = await tool!.handler({ asset: "BTC", limit: 50, offset: 0 });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      data: { items: unknown[] };
    };

    expect(parsed.success).toBe(true);
    expect(parsed.data.items).toHaveLength(2);
  });

  it("applies pagination offset correctly", async () => {
    const threeItems = Array.from({ length: 3 }, (_, i) => ({
      id: i + 1,
      timestamp: "2024-01-01T00:00:00Z",
      tx_type: "buy",
      to_asset: "BTC",
      to_quantity: "0.1",
    }));
    vi.mocked(runCli).mockResolvedValueOnce(threeItems);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_list_transactions");

    const result = await tool!.handler({ limit: 2, offset: 1 });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      data: { items: unknown[]; offset: number; limit: number };
    };

    expect(parsed.success).toBe(true);
    expect(parsed.data.items).toHaveLength(2);
  });

  it("applies offset once to the asset-filtered set (regression)", async () => {
    // Mixed assets in CLI order; BTC matches are rows 1, 3 and 4.
    mockTxList([
      { id: 1, to_asset: "BTC" },
      { id: 2, to_asset: "ETH" },
      { id: 3, to_asset: "BTC" },
      { id: 4, to_asset: "BTC" },
      { id: 5, to_asset: "SOL" },
    ]);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_list_transactions");

    const result = await tool!.handler({ asset: "BTC", limit: 2, offset: 1 });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      data: {
        items: Array<{ id: number }>;
        total_fetched: number;
        has_more: boolean;
      };
    };

    expect(parsed.success).toBe(true);
    // offset=1 must skip ONE matching row: [1, 3, 4] → [3, 4] (not [3]).
    expect(parsed.data.items.map((t) => t.id)).toEqual([3, 4]);
    expect(parsed.data.total_fetched).toBe(3);
    expect(parsed.data.has_more).toBe(false);
    // First window is needed+1 (offset+limit+1 = 4); it grows once because 4
    // rows only held 3 matches, and the 8-row window runs out of data at 5.
    expect(vi.mocked(runCli)).toHaveBeenNthCalledWith(1, [
      "tx", "list", "--limit", "4",
    ]);
    expect(vi.mocked(runCli)).toHaveBeenNthCalledWith(2, [
      "tx", "list", "--limit", "8",
    ]);
  });

  it("applies offset once when no asset filter is set", async () => {
    mockTxList(
      Array.from({ length: 5 }, (_, i) => ({ id: i + 1, to_asset: "BTC" }))
    );

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_list_transactions");

    const result = await tool!.handler({ limit: 2, offset: 1 });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      data: {
        items: Array<{ id: number }>;
        total_fetched: number;
        has_more: boolean;
        next_offset?: number;
      };
    };

    expect(parsed.success).toBe(true);
    expect(parsed.data.items.map((t) => t.id)).toEqual([2, 3]);
    // One probe row beyond the page (offset+limit+1 = 4) proves more data exists.
    expect(parsed.data.total_fetched).toBe(4);
    expect(parsed.data.has_more).toBe(true);
    expect(parsed.data.next_offset).toBe(3);
    // One fetch: needed+1 rows were returned, so the page plus the probe fit
    // in the first window.
    expect(vi.mocked(runCli)).toHaveBeenCalledTimes(1);
    expect(vi.mocked(runCli)).toHaveBeenNthCalledWith(1, [
      "tx", "list", "--limit", "4",
    ]);
  });

  it("stops growing the window when the CLI runs out of data", async () => {
    mockTxList([
      { id: 1, to_asset: "BTC" },
      { id: 2, to_asset: "ETH" },
      { id: 3, to_asset: "BTC" },
      { id: 4, to_asset: "ETH" },
    ]);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_list_transactions");

    const result = await tool!.handler({ asset: "BTC", limit: 2, offset: 2 });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      data: {
        items: unknown[];
        total_fetched: number;
        has_more: boolean;
      };
    };

    expect(parsed.success).toBe(true);
    // Only 2 BTC rows exist and both are skipped — empty page, no runaway loop.
    expect(parsed.data.items).toEqual([]);
    expect(parsed.data.total_fetched).toBe(2);
    expect(parsed.data.has_more).toBe(false);
    // The first window (offset+limit+1 = 5) already exceeded the 4 rows the CLI
    // has, so the loop stops after one fetch.
    expect(vi.mocked(runCli)).toHaveBeenCalledTimes(1);
    expect(vi.mocked(runCli)).toHaveBeenNthCalledWith(1, [
      "tx", "list", "--limit", "5",
    ]);
  });
});

describe("cryptofolio_record_transaction", () => {
  beforeEach(() => vi.clearAllMocks());

  it("records a buy transaction", async () => {
    vi.mocked(runCli).mockResolvedValueOnce(null);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_record_transaction");

    const result = await tool!.handler({
      type: "buy",
      asset: "BTC",
      quantity: "0.1",
      account: "Binance",
      price_usd: "95000",
    });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.message).toContain("BUY");
    expect(vi.mocked(runCli)).toHaveBeenCalledWith(
      expect.arrayContaining([
        "tx", "buy", "BTC", "0.1", "--account", "Binance", "--price", "95000",
      ])
    );
  });

  it("records a swap transaction", async () => {
    vi.mocked(runCli).mockResolvedValueOnce(null);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_record_transaction");

    const result = await tool!.handler({
      type: "swap",
      from_asset: "USDT",
      from_quantity: "1000",
      to_asset: "BTC",
      to_quantity: "0.01049",
      account: "Binance",
    });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
    };

    expect(parsed.success).toBe(true);
    expect(vi.mocked(runCli)).toHaveBeenCalledWith(
      expect.arrayContaining([
        "tx", "swap",
        "--from-asset", "USDT",
        "--from-quantity", "1000",
        "--to-asset", "BTC",
        "--to-quantity", "0.01049",
        "--account", "Binance",
      ])
    );
  });

  it("returns MISSING_PARAM error when buy is missing price", async () => {
    const server = makeServer();
    const tool = getTool(server, "cryptofolio_record_transaction");

    const result = await tool!.handler({
      type: "buy",
      asset: "BTC",
      quantity: "0.1",
      account: "Binance",
    });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      code: string;
    };

    expect(parsed.success).toBe(false);
    expect(parsed.code).toBe("MISSING_PARAM");
    expect(vi.mocked(runCli)).not.toHaveBeenCalled();
  });

  it("records a transfer", async () => {
    vi.mocked(runCli).mockResolvedValueOnce(null);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_record_transaction");

    const result = await tool!.handler({
      type: "transfer",
      asset: "BTC",
      quantity: "0.5",
      from_account: "Binance",
      to_account: "Ledger",
      fee: "0.0001",
    });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
    };

    expect(parsed.success).toBe(true);
    expect(vi.mocked(runCli)).toHaveBeenCalledWith(
      expect.arrayContaining([
        "tx", "transfer", "BTC", "0.5",
        "--from", "Binance",
        "--to", "Ledger",
        "--fee", "0.0001",
      ])
    );
  });

  it("records a sell transaction", async () => {
    vi.mocked(runCli).mockResolvedValueOnce(null);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_record_transaction");

    const result = await tool!.handler({
      type: "sell",
      asset: "ETH",
      quantity: "2.0",
      account: "Binance",
      price_usd: "3500",
    });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.message).toContain("SELL");
    expect(vi.mocked(runCli)).toHaveBeenCalledWith(
      expect.arrayContaining(["tx", "sell", "ETH", "2.0", "--account", "Binance", "--price", "3500"])
    );
  });

  it("records a buy with cost_basis_only flag", async () => {
    vi.mocked(runCli).mockResolvedValueOnce(null);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_record_transaction");

    await tool!.handler({
      type: "buy",
      asset: "BTC",
      quantity: "0.05",
      account: "Binance",
      price_usd: "60000",
      cost_basis_only: true,
    });

    expect(vi.mocked(runCli)).toHaveBeenCalledWith(
      expect.arrayContaining(["--cost-basis-only"])
    );
  });

  it("returns MISSING_PARAM when transfer is missing from_account", async () => {
    const server = makeServer();
    const tool = getTool(server, "cryptofolio_record_transaction");

    const result = await tool!.handler({
      type: "transfer",
      asset: "BTC",
      quantity: "0.1",
      to_account: "Ledger",
    });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      code: string;
    };

    expect(parsed.success).toBe(false);
    expect(parsed.code).toBe("MISSING_PARAM");
    expect(vi.mocked(runCli)).not.toHaveBeenCalled();
  });
});

describe("cryptofolio_track_conversion", () => {
  beforeEach(() => vi.clearAllMocks());

  it("records a multi-step conversion and reports step counts", async () => {
    vi.mocked(runCli).mockResolvedValue(null);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_track_conversion");

    const result = await tool!.handler({
      description: "Monthly DCA: CRC to BTC",
      steps: [
        { from: "CRC", to: "USD", amount: "500000", rate: "0.001" },
        { from: "USD", to: "BTC", amount: "500", rate: "0.0000153" },
      ],
    });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      data: { steps_recorded: string[]; errors: string[] };
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.data.steps_recorded).toHaveLength(2);
    expect(parsed.data.errors).toHaveLength(0);
    expect(parsed.message).toContain("2/2");
    expect(vi.mocked(runCli)).toHaveBeenCalledTimes(2);
  });

  it("computes to_quantity with exact decimal math, not floats", async () => {
    vi.mocked(runCli).mockResolvedValue(null);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_track_conversion");

    await tool!.handler({
      description: "Large conversion",
      steps: [{ from: "ABC", to: "XYZ", amount: "100000000", rate: "1.1" }],
    });

    // (100000000 * 1.1).toFixed(8) === "110000000.00000001" with floats.
    expect(vi.mocked(runCli)).toHaveBeenCalledWith(
      expect.arrayContaining(["--to-quantity", "110000000.00000000"])
    );
    expect(vi.mocked(runCli)).not.toHaveBeenCalledWith(
      expect.arrayContaining(["--to-quantity", "110000000.00000001"])
    );
  });

  it("emits an 8dp to_quantity for the 0.1 x 0.3 case", async () => {
    vi.mocked(runCli).mockResolvedValue(null);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_track_conversion");

    await tool!.handler({
      description: "Small conversion",
      steps: [{ from: "CRC", to: "USD", amount: "0.1", rate: "0.3" }],
    });

    expect(vi.mocked(runCli)).toHaveBeenCalledWith(
      expect.arrayContaining(["--to-quantity", "0.03000000"])
    );
    const args = vi.mocked(runCli).mock.calls[0]?.[0] ?? [];
    expect(args.join(" ")).not.toContain("0.030000000000000002");
  });

  it("keeps to_quantity = amount when rate is absent or empty", async () => {
    vi.mocked(runCli).mockResolvedValue(null);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_track_conversion");

    await tool!.handler({
      description: "No rate",
      steps: [
        { from: "CRC", to: "USD", amount: "12345.6789" },
        { from: "USD", to: "USDT", amount: "12.5", rate: "" },
      ],
    });

    expect(vi.mocked(runCli)).toHaveBeenNthCalledWith(
      1,
      expect.arrayContaining(["--to-quantity", "12345.6789"])
    );
    expect(vi.mocked(runCli)).toHaveBeenNthCalledWith(
      2,
      expect.arrayContaining(["--to-quantity", "12.5"])
    );
  });

  it("records partial success when one step fails", async () => {
    vi.mocked(runCli)
      .mockResolvedValueOnce(null)
      .mockRejectedValueOnce(new Error("account not found"));

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_track_conversion");

    const result = await tool!.handler({
      description: "Test partial",
      steps: [
        { from: "CRC", to: "USD", amount: "100", account: "LocalExchange" },
        { from: "USD", to: "BTC", amount: "100", account: "Missing" },
      ],
    });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      data: { steps_recorded: string[]; errors: string[] };
    };

    // Partial success: still a success envelope, but errors are reported
    expect(parsed.success).toBe(true);
    expect(parsed.data.steps_recorded).toHaveLength(1);
    expect(parsed.data.errors).toHaveLength(1);
  });

  it("returns CONVERSION_FAILED error envelope when all steps fail", async () => {
    vi.mocked(runCli).mockRejectedValue(new Error("network unavailable"));

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_track_conversion");

    const result = await tool!.handler({
      description: "Failing conversion",
      steps: [
        { from: "CRC", to: "USD", amount: "100" },
      ],
    });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      code: string;
    };

    expect(parsed.success).toBe(false);
    expect(parsed.code).toBe("CONVERSION_FAILED");
  });
});

describe("cryptofolio_export_transactions", () => {
  beforeEach(() => vi.clearAllMocks());

  it("exports as CSV and returns file path", async () => {
    vi.mocked(runCliRaw).mockResolvedValueOnce("");

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_export_transactions");

    const result = await tool!.handler({ format: "csv" });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      data: { file_path: string; format: string };
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.data.format).toBe("csv");
    expect(parsed.data.file_path).toContain("transactions-");
    expect(parsed.data.file_path).toMatch(/\.csv$/);
    expect(parsed.message).toContain("Exported to:");
    expect(vi.mocked(runCliRaw)).toHaveBeenCalledWith(
      expect.arrayContaining(["tx", "export"])
    );
  });

  it("exports as JSON with date range and account filters", async () => {
    vi.mocked(runCliRaw).mockResolvedValueOnce("");

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_export_transactions");

    await tool!.handler({
      format: "json",
      from_date: "2024-01-01",
      to_date: "2024-12-31",
      account: "Binance",
      asset: "BTC",
    });

    expect(vi.mocked(runCliRaw)).toHaveBeenCalledWith(
      expect.arrayContaining([
        "--format", "json",
        "--from", "2024-01-01",
        "--to", "2024-12-31",
        "--account", "Binance",
        "--asset", "BTC",
      ])
    );
  });

  it("returns error envelope when CLI fails during export", async () => {
    const { CliError } = await import("../../src/cli.js");
    vi.mocked(runCliRaw).mockRejectedValueOnce(
      new CliError(1, "permission denied writing to exports dir", "tx export")
    );

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_export_transactions");

    const result = await tool!.handler({ format: "csv" });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
    };

    expect(parsed.success).toBe(false);
  });
});
