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

import { runCli } from "../../src/cli.js";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  registerListTransactionsTool,
  registerRecordTransactionTool,
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
});
