/**
 * Unit tests for cryptofolio_sync_exchange
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

import { runCliRaw, CliError } from "../../src/cli.js";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { registerSyncExchangeTool } from "../../src/tools/sync.js";

type ToolHandler = (args: unknown) => Promise<{ content: Array<{ type: string; text: string }> }>;
type ToolRegistry = Record<string, { handler: ToolHandler }>;

function makeServer() {
  const server = new McpServer({ name: "test", version: "0.0.1" });
  registerSyncExchangeTool(server);
  return server;
}

function getTool(server: McpServer, name: string) {
  return (server as unknown as { _registeredTools: ToolRegistry })
    ._registeredTools[name];
}

describe("cryptofolio_sync_exchange", () => {
  beforeEach(() => vi.clearAllMocks());

  it("syncs the default account when no account is specified", async () => {
    vi.mocked(runCliRaw).mockResolvedValueOnce("Synced 5 assets.");

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_sync_exchange");

    const result = await tool!.handler({});
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.message).toContain("synced successfully");
    expect(vi.mocked(runCliRaw)).toHaveBeenCalledWith(
      expect.arrayContaining(["sync"]),
      expect.any(Number)
    );
  });

  it("syncs a named account", async () => {
    vi.mocked(runCliRaw).mockResolvedValueOnce("Synced 3 assets.");

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_sync_exchange");

    const result = await tool!.handler({ account: "Binance" });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.message).toContain("Binance");
    expect(vi.mocked(runCliRaw)).toHaveBeenCalledWith(
      expect.arrayContaining(["sync", "Binance"]),
      expect.any(Number)
    );
  });

  it("propagates CLI errors gracefully", async () => {
    vi.mocked(runCliRaw).mockRejectedValueOnce(
      new CliError(1, "API key not configured", "sync")
    );

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_sync_exchange");

    const result = await tool!.handler({});
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
    };

    expect(parsed.success).toBe(false);
  });
});
