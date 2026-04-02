/**
 * Unit tests for cryptofolio_get_portfolio
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
import { registerPortfolioTool } from "../../src/tools/portfolio.js";

const fixturesDir = join(import.meta.dirname, "../fixtures");

function loadFixture(name: string): unknown {
  return JSON.parse(readFileSync(join(fixturesDir, name), "utf-8"));
}

type ToolHandler = (args: unknown) => Promise<{ content: Array<{ type: string; text: string }> }>;
type ToolRegistry = Record<string, { handler: ToolHandler }>;

function makeServer() {
  const server = new McpServer({ name: "test", version: "0.0.1" });
  registerPortfolioTool(server);
  return server;
}

function getTool(server: McpServer, name: string) {
  return (server as unknown as { _registeredTools: ToolRegistry })
    ._registeredTools[name];
}

describe("cryptofolio_get_portfolio", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("returns portfolio data with totals", async () => {
    vi.mocked(runCli).mockResolvedValueOnce(loadFixture("portfolio.json"));

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_portfolio");
    expect(tool).toBeDefined();

    const result = await tool!.handler({});
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      data: { total_value_usd: string; entries: unknown[] };
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.data.total_value_usd).toBe("52341.87");
    expect(parsed.data.entries).toHaveLength(1);
    expect(parsed.message).toContain("52341.87");
  });

  it("passes --account flag when account is specified", async () => {
    vi.mocked(runCli).mockResolvedValueOnce(loadFixture("portfolio.json"));

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_portfolio");

    await tool!.handler({ account: "Binance" });

    expect(vi.mocked(runCli)).toHaveBeenCalledWith(
      expect.arrayContaining(["--account", "Binance"])
    );
  });

  it("returns error response when CLI fails", async () => {
    const { CliError } = await import("../../src/cli.js");
    vi.mocked(runCli).mockRejectedValueOnce(
      new CliError(1, "account not found", "portfolio")
    );

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_portfolio");

    const result = await tool!.handler({});
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
    };

    expect(parsed.success).toBe(false);
  });
});
