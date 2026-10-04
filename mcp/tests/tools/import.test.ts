/**
 * Unit tests for cryptofolio_import_binance
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
import { registerImportBinanceTool } from "../../src/tools/import.js";

type ToolHandler = (args: unknown) => Promise<{ content: Array<{ type: string; text: string }> }>;
type ToolRegistry = Record<string, { handler: ToolHandler }>;

function makeServer() {
  const server = new McpServer({ name: "test", version: "0.0.1" });
  registerImportBinanceTool(server);
  return server;
}

function getTool(server: McpServer, name: string) {
  return (server as unknown as { _registeredTools: ToolRegistry })
    ._registeredTools[name];
}

describe("cryptofolio_import_binance", () => {
  beforeEach(() => vi.clearAllMocks());

  it("passes the file path through untouched and imports into an account", async () => {
    vi.mocked(runCliRaw).mockResolvedValueOnce(
      "[OK] Imported 42 transactions (3 duplicates, 0 parse-skipped, 0 write-errors)"
    );

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_import_binance");

    const result = await tool!.handler({
      file: "/tmp/Binance history 2024.csv",
      account: "Binance",
    });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      data: { output: string };
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.data.output).toContain("Imported 42 transactions");
    expect(parsed.message).toContain("Binance history 2024.csv");
    expect(vi.mocked(runCliRaw)).toHaveBeenCalledWith(
      [
        "import-binance",
        "/tmp/Binance history 2024.csv",
        "--account",
        "Binance",
      ],
      expect.any(Number)
    );
  });

  it("adds --dry-run and reports that nothing was written", async () => {
    vi.mocked(runCliRaw).mockResolvedValueOnce(
      "Dry run — 10 rows would be imported, 1 skipped during parsing. Not writing anything."
    );

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_import_binance");

    const result = await tool!.handler({
      file: "history.zip",
      account: "Binance",
      dry_run: true,
    });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      message: string;
    };

    expect(parsed.success).toBe(true);
    expect(parsed.message).toContain("nothing was written");
    expect(vi.mocked(runCliRaw)).toHaveBeenCalledWith(
      ["import-binance", "history.zip", "--account", "Binance", "--dry-run"],
      expect.any(Number)
    );
  });

  it("returns an error envelope when the file is missing or the account is unknown", async () => {
    vi.mocked(runCliRaw).mockRejectedValueOnce(
      new CliError(1, "File not found: nope.csv", "import-binance")
    );

    const server = makeServer();
    const tool = getTool(server, "cryptofolio_import_binance");

    const result = await tool!.handler({ file: "nope.csv", account: "Binance" });
    const parsed = JSON.parse(result.content[0]?.text ?? "{}") as {
      success: boolean;
      error: string;
      code: string;
    };

    expect(parsed.success).toBe(false);
    expect(parsed.error).toContain("File not found");
    expect(parsed.code).toBe("CLI_ERROR_1");
  });
});
