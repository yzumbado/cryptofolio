/**
 * Unit tests for cryptofolio_manage_wallet and cryptofolio_sync_wallet
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
  registerManageWalletTool,
  registerSyncWalletTool,
} from "../../src/tools/wallets.js";

type ToolHandler = (args: unknown) => Promise<{ content: Array<{ type: string; text: string }> }>;
type ToolRegistry = Record<string, { handler: ToolHandler }>;

function makeServer() {
  const server = new McpServer({ name: "test", version: "0.0.1" });
  registerManageWalletTool(server);
  registerSyncWalletTool(server);
  return server;
}

function getTool(server: McpServer, name: string) {
  return (server as unknown as { _registeredTools: ToolRegistry })
    ._registeredTools[name];
}

describe("cryptofolio_manage_wallet", () => {
  beforeEach(() => vi.clearAllMocks());

  it("adds a wallet with an address", async () => {
    vi.mocked(runCli).mockResolvedValueOnce({ wallet: "My BTC", derived_addresses: null });

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_manage_wallet");

    const result = await tool!.handler({
      action: "add",
      name: "My BTC",
      blockchain: "bitcoin",
      address: "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh",
    });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.message).toContain("My BTC");
    expect(vi.mocked(runCli)).toHaveBeenCalledWith(
      expect.arrayContaining(["wallet", "add", "My BTC", "--blockchain", "bitcoin", "--address"])
    );
  });

  it("adds a wallet with an xpub and reports derived addresses", async () => {
    vi.mocked(runCli).mockResolvedValueOnce({ wallet: "Sparrow", derived_addresses: 20 });

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_manage_wallet");

    const result = await tool!.handler({
      action: "add",
      name: "Sparrow",
      blockchain: "bitcoin",
      xpub: "zpub6rFR7y4Q2AijF6Zs...",
    });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.message).toContain("20 addresses");
    expect(vi.mocked(runCli)).toHaveBeenCalledWith(
      expect.arrayContaining(["wallet", "add", "Sparrow", "--xpub"])
    );
  });

  it("returns MISSING_PARAM when name is omitted for add", async () => {
    const server = makeServer();
    const tool = getTool(server, "cryptofolio_manage_wallet");

    const result = await tool!.handler({
      action: "add",
      blockchain: "bitcoin",
      address: "bc1q...",
    });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      code: string;
    };

    expect(parsed.success).toBe(false);
    expect(parsed.code).toBe("MISSING_PARAM");
  });

  it("returns MISSING_PARAM when neither address nor xpub is provided", async () => {
    const server = makeServer();
    const tool = getTool(server, "cryptofolio_manage_wallet");

    const result = await tool!.handler({
      action: "add",
      name: "Broken",
      blockchain: "bitcoin",
    });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      code: string;
    };

    expect(parsed.success).toBe(false);
    expect(parsed.code).toBe("MISSING_PARAM");
  });

  it("lists wallets, optionally filtered by blockchain", async () => {
    vi.mocked(runCli).mockResolvedValueOnce([
      { name: "My BTC", blockchain: "bitcoin" },
      { name: "MetaMask", blockchain: "ethereum" },
    ]);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_manage_wallet");

    const result = await tool!.handler({ action: "list" });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      data: { wallets: unknown[] };
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.data.wallets).toHaveLength(2);
    expect(parsed.message).toContain("2 wallet(s)");
  });

  it("shows wallet details", async () => {
    vi.mocked(runCli).mockResolvedValueOnce({
      name: "My BTC",
      blockchain: "bitcoin",
      addresses: [{ address: "bc1q..." }],
      holdings: [{ asset: "BTC", quantity: "0.05" }],
    });

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_manage_wallet");

    const result = await tool!.handler({ action: "show", name: "My BTC" });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.message).toContain("1 address(es)");
    expect(parsed.message).toContain("1 asset(s)");
  });

  it("removes a wallet with --yes", async () => {
    vi.mocked(runCliRaw).mockResolvedValueOnce("");

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_manage_wallet");

    const result = await tool!.handler({ action: "remove", name: "Old Wallet" });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.message).toContain("removed");
    expect(vi.mocked(runCliRaw)).toHaveBeenCalledWith(
      expect.arrayContaining(["wallet", "remove", "Old Wallet", "--yes"])
    );
  });
});

describe("cryptofolio_sync_wallet", () => {
  beforeEach(() => vi.clearAllMocks());

  it("syncs a single wallet by name", async () => {
    vi.mocked(runCli).mockResolvedValueOnce({ synced: true, balance: "0.05" });

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_sync_wallet");

    const result = await tool!.handler({ wallet_name: "My BTC" });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.message).toContain("My BTC");
    expect(vi.mocked(runCli)).toHaveBeenCalledWith(
      expect.arrayContaining(["wallet", "sync", "My BTC"]),
      expect.any(Number)
    );
  });

  it("syncs all wallets when sync_all is true", async () => {
    vi.mocked(runCli).mockResolvedValueOnce({ synced: true });

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_sync_wallet");

    const result = await tool!.handler({ sync_all: true });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.message).toContain("All wallets");
    expect(vi.mocked(runCli)).toHaveBeenCalledWith(
      expect.arrayContaining(["wallet", "sync", "--all"]),
      expect.any(Number)
    );
  });

  it("returns MISSING_PARAM when neither wallet_name nor sync_all is provided", async () => {
    const server = makeServer();
    const tool = getTool(server, "cryptofolio_sync_wallet");

    const result = await tool!.handler({});
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      code: string;
    };

    expect(parsed.success).toBe(false);
    expect(parsed.code).toBe("MISSING_PARAM");
  });

  it("appends --import-history when requested", async () => {
    vi.mocked(runCli).mockResolvedValueOnce({ synced: true });

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_sync_wallet");

    await tool!.handler({ wallet_name: "My BTC", import_history: true });

    expect(vi.mocked(runCli)).toHaveBeenCalledWith(
      expect.arrayContaining(["--import-history"]),
      expect.any(Number)
    );
  });
});
