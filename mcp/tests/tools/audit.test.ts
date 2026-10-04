/**
 * Unit tests for cryptofolio_get_audit_log and cryptofolio_get_sync_history
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
  registerAuditLogTool,
  registerGetSyncHistoryTool,
} from "../../src/tools/audit.js";

type ToolHandler = (args: unknown) => Promise<{ content: Array<{ type: string; text: string }> }>;
type ToolRegistry = Record<string, { handler: ToolHandler }>;

function makeServer() {
  const server = new McpServer({ name: "test", version: "0.0.1" });
  registerAuditLogTool(server);
  registerGetSyncHistoryTool(server);
  return server;
}

function getTool(server: McpServer, name: string) {
  return (server as unknown as { _registeredTools: ToolRegistry })
    ._registeredTools[name];
}

describe("cryptofolio_get_audit_log", () => {
  beforeEach(() => vi.clearAllMocks());

  it("returns sync_history entries and notes error count", async () => {
    vi.mocked(runCli).mockResolvedValueOnce([
      { wallet: "My BTC", synced_at: "2026-04-01T10:00:00Z", error: null },
      { wallet: "MetaMask", synced_at: "2026-04-01T10:05:00Z", error: "timeout" },
    ]);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_audit_log");

    const result = await tool!.handler({ view: "sync_history", limit: 50, offset: 0 });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      data: { items: unknown[] };
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.data.items).toHaveLength(2);
    expect(parsed.message).toContain("1 with errors");
    expect(vi.mocked(runCli)).toHaveBeenCalledWith(
      expect.arrayContaining(["audit", "sync"])
    );
  });

  it("returns clean message when no errors in sync_history", async () => {
    vi.mocked(runCli).mockResolvedValueOnce([
      { wallet: "My BTC", synced_at: "2026-04-01T10:00:00Z", error: null },
    ]);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_audit_log");

    const result = await tool!.handler({ view: "sync_history", limit: 50, offset: 0 });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      message: string;
    };

    expect(parsed.message).not.toContain("with errors");
  });

  it("returns coverage entries with never-synced hint", async () => {
    vi.mocked(runCli).mockResolvedValueOnce([
      { address: "bc1q...", last_sync_at: "2026-04-01T10:00:00Z" },
      { address: "0xABCD", last_sync_at: null },
    ]);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_audit_log");

    const result = await tool!.handler({ view: "coverage", limit: 50, offset: 0 });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      data: { items: unknown[] };
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.data.items).toHaveLength(2);
    expect(parsed.message).toContain("never synced");
    expect(vi.mocked(runCli)).toHaveBeenCalledWith(
      expect.arrayContaining(["audit", "coverage"])
    );
  });

  it("returns errors view with clean message when no errors", async () => {
    vi.mocked(runCli).mockResolvedValueOnce([]);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_audit_log");

    const result = await tool!.handler({ view: "errors", limit: 50, offset: 0 });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.message).toContain("No sync errors");
  });

  it("filters by wallet and chain", async () => {
    vi.mocked(runCli).mockResolvedValueOnce([]);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_audit_log");

    await tool!.handler({ view: "sync_history", wallet: "My BTC", chain: "bitcoin", limit: 10, offset: 0 });

    expect(vi.mocked(runCli)).toHaveBeenCalledWith(
      expect.arrayContaining(["--wallet", "My BTC", "--chain", "bitcoin"])
    );
  });
});

describe("cryptofolio_get_sync_history", () => {
  beforeEach(() => vi.clearAllMocks());

  it("returns paginated sync events from audit sync", async () => {
    vi.mocked(runCli).mockResolvedValueOnce([
      { wallet: "My BTC", chain: "bitcoin", error: null },
      { wallet: "MetaMask", chain: "ethereum", error: "rate limited" },
    ]);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_sync_history");

    const result = await tool!.handler({ limit: 50, offset: 0 });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      data: { items: unknown[] };
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.data.items).toHaveLength(2);
    expect(parsed.message).toContain("1 with errors");
    expect(vi.mocked(runCli)).toHaveBeenCalledWith(
      expect.arrayContaining(["audit", "sync", "--limit", "50"])
    );
  });

  it("fetches offset + limit rows and slices server-side", async () => {
    vi.mocked(runCli).mockResolvedValueOnce([
      { wallet: "A" },
      { wallet: "B" },
      { wallet: "C" },
    ]);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_sync_history");

    const result = await tool!.handler({ limit: 1, offset: 2 });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      data: { items: Array<{ wallet: string }>; has_more: boolean };
    };

    expect(parsed.success).toBe(true);
    expect(parsed.data.items).toHaveLength(1);
    expect(parsed.data.items[0]?.wallet).toBe("C");
    expect(parsed.data.has_more).toBe(false);
    expect(vi.mocked(runCli)).toHaveBeenCalledWith(
      expect.arrayContaining(["audit", "sync", "--limit", "3"])
    );
  });

  it("passes wallet and chain filters to the CLI", async () => {
    vi.mocked(runCli).mockResolvedValueOnce([]);

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_sync_history");

    await tool!.handler({ wallet: "Ledger", chain: "bitcoin", limit: 10, offset: 0 });

    expect(vi.mocked(runCli)).toHaveBeenCalledWith(
      expect.arrayContaining(["audit", "sync", "--wallet", "Ledger", "--chain", "bitcoin"])
    );
  });

  it("returns an error envelope when the CLI fails", async () => {
    const { CliError } = await import("../../src/cli.js");
    vi.mocked(runCli).mockRejectedValueOnce(
      new CliError(1, "no such table: sync_audit_log", "audit")
    );

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_get_sync_history");

    const result = await tool!.handler({ limit: 50, offset: 0 });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
    };

    expect(parsed.success).toBe(false);
  });
});
