/**
 * Unit tests for cryptofolio_get_system_status
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
import { registerStatusTool } from "../../src/tools/status.js";

const fixturesDir = join(import.meta.dirname, "../fixtures");

function loadFixture(name: string): unknown {
  return JSON.parse(readFileSync(join(fixturesDir, name), "utf-8"));
}

type ToolHandler = (args: unknown) => Promise<{ content: Array<{ type: string; text: string }> }>;
type ToolRegistry = Record<string, { handler: ToolHandler }>;

function makeServer() {
  const server = new McpServer({ name: "test", version: "0.0.1" });
  registerStatusTool(server);
  return server;
}

function getTool(server: McpServer, name: string) {
  const registry = (server as unknown as { _registeredTools: ToolRegistry })._registeredTools;
  return registry[name];
}

describe("cryptofolio_get_system_status", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("returns ready_for_use=true when accounts exist and Binance is configured", async () => {
    const mockRunCli = vi.mocked(runCli);
    mockRunCli
      .mockResolvedValueOnce(loadFixture("config-show.json"))
      .mockResolvedValueOnce(loadFixture("account-list.json"));

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_system_status");
    expect(tool).toBeDefined();

    const result = await tool!.handler({});
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      data: {
        ready_for_use: boolean;
        accounts_count: number;
        binance_configured: boolean;
        setup_steps_remaining: string[];
      };
    };

    expect(parsed.success).toBe(true);
    expect(parsed.data.ready_for_use).toBe(true);
    expect(parsed.data.accounts_count).toBe(2);
    expect(parsed.data.binance_configured).toBe(true);
    expect(parsed.data.setup_steps_remaining).toHaveLength(0);
  });

  it("returns ready_for_use=false with setup steps when no accounts exist", async () => {
    const mockRunCli = vi.mocked(runCli);
    const configNoAccounts = {
      ...(loadFixture("config-show.json") as object),
      binance: { api_key_configured: false, api_secret_configured: false },
    };
    mockRunCli
      .mockResolvedValueOnce(configNoAccounts)
      .mockResolvedValueOnce([]);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_system_status");

    const result = await tool!.handler({});
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      data: { ready_for_use: boolean; setup_steps_remaining: string[] };
    };

    expect(parsed.success).toBe(true);
    expect(parsed.data.ready_for_use).toBe(false);
    expect(parsed.data.setup_steps_remaining.length).toBeGreaterThan(0);
    expect(parsed.data.setup_steps_remaining[0]).toContain(
      "cryptofolio_manage_account"
    );
  });

  it("returns an error response when the binary is not found", async () => {
    const { CliError } = await import("../../src/cli.js");
    vi.mocked(runCli).mockRejectedValueOnce(
      new CliError(127, "command not found: cryptofolio", "config")
    );

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_system_status");

    const result = await tool!.handler({});
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
    };

    expect(parsed.success).toBe(false);
  });
});
