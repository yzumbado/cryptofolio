/**
 * Unit tests for cryptofolio_list_accounts and cryptofolio_manage_account
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
  registerListAccountsTool,
  registerManageAccountTool,
} from "../../src/tools/accounts.js";

const fixturesDir = join(import.meta.dirname, "../fixtures");

function loadFixture(name: string): unknown {
  return JSON.parse(readFileSync(join(fixturesDir, name), "utf-8"));
}

type ToolHandler = (args: unknown) => Promise<{ content: Array<{ type: string; text: string }> }>;
type ToolRegistry = Record<string, { handler: ToolHandler }>;

function makeServer() {
  const server = new McpServer({ name: "test", version: "0.0.1" });
  registerListAccountsTool(server);
  registerManageAccountTool(server);
  return server;
}

function getTool(server: McpServer, name: string) {
  return (server as unknown as { _registeredTools: ToolRegistry })
    ._registeredTools[name];
}

describe("cryptofolio_list_accounts", () => {
  beforeEach(() => vi.clearAllMocks());

  it("returns account list", async () => {
    vi.mocked(runCli).mockResolvedValueOnce(loadFixture("account-list.json"));

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_list_accounts");

    const result = await tool!.handler({});
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      data: { accounts: unknown[] };
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.data.accounts).toHaveLength(2);
    expect(parsed.message).toContain("2 account(s)");
  });

  it("returns empty list gracefully", async () => {
    vi.mocked(runCli).mockResolvedValueOnce([]);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_list_accounts");

    const result = await tool!.handler({});
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      data: { accounts: unknown[] };
    };

    expect(parsed.success).toBe(true);
    expect(parsed.data.accounts).toHaveLength(0);
  });
});

describe("cryptofolio_manage_account", () => {
  beforeEach(() => vi.clearAllMocks());

  it("adds an account with all required params", async () => {
    vi.mocked(runCli).mockResolvedValueOnce(null);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_manage_account");

    const result = await tool!.handler({
      action: "add",
      name: "Binance",
      account_type: "exchange",
      category: "trading",
    });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.message).toContain("Binance");
    expect(vi.mocked(runCli)).toHaveBeenCalledWith(
      expect.arrayContaining(["account", "add", "Binance", "--type", "exchange", "--category", "trading"])
    );
  });

  it("returns MISSING_PARAM error when account_type is omitted", async () => {
    const server = makeServer();
    const tool = getTool(server, "cryptofolio_manage_account");

    const result = await tool!.handler({
      action: "add",
      name: "Test",
      category: "trading",
    });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      code: string;
    };

    expect(parsed.success).toBe(false);
    expect(parsed.code).toBe("MISSING_PARAM");
  });

  it("auto-creates category on CategoryNotFound error then retries", async () => {
    const { CliError } = await import("../../src/cli.js");
    vi.mocked(runCli)
      .mockRejectedValueOnce(
        new CliError(1, "category 'trading' not found", "account")
      )
      .mockResolvedValueOnce(null);

    vi.mocked(runCliRaw).mockResolvedValueOnce("");

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_manage_account");

    const result = await tool!.handler({
      action: "add",
      name: "TestExchange",
      account_type: "exchange",
      category: "trading",
    });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
    };

    expect(parsed.success).toBe(true);
    expect(vi.mocked(runCliRaw)).toHaveBeenCalledWith(
      expect.arrayContaining(["category", "add", "trading"])
    );
    expect(vi.mocked(runCli)).toHaveBeenCalledTimes(2);
  });

  it("removes an account with --yes flag", async () => {
    vi.mocked(runCliRaw).mockResolvedValueOnce("");

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_manage_account");

    const result = await tool!.handler({ action: "remove", name: "OldAccount" });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.message).toContain("removed");
    expect(vi.mocked(runCliRaw)).toHaveBeenCalledWith(
      expect.arrayContaining(["account", "remove", "OldAccount", "--yes"])
    );
  });
});
