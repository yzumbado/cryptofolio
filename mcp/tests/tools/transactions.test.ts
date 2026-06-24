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

describe("cryptofolio_list_transactions", () => {
  beforeEach(() => vi.clearAllMocks());

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

  it("returns MISSING_PARAMS error when buy is missing price", async () => {
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
    expect(parsed.code).toBe("MISSING_PARAMS");
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

  it("returns MISSING_PARAMS when transfer is missing from_account", async () => {
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
    expect(parsed.code).toBe("MISSING_PARAMS");
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
