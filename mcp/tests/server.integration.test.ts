/**
 * In-process integration test for the machine-readable tool output contract.
 *
 * Why this exists: tests/tools/*.test.ts call `tool.handler(...)` directly, so
 * they bypass the SDK's CallToolRequest pipeline and therefore bypass
 * `McpServer.validateToolOutput()`. A tool with a broken outputSchema (or a
 * handler that forgets `structuredContent`) would still pass those unit tests
 * and fail in production.
 *
 * This test drives a real `McpServer` through a real `Client` over an
 * in-memory transport (`InMemoryTransport.createLinkedPair()`), which runs BOTH
 * validators on the real path:
 *   1. Server: `McpServer.validateToolOutput()` over each tool's `outputSchema`
 *      before the result is sent back (throws McpError -> isError result).
 *   2. Client: `AjvJsonSchemaValidator` (cached from `tools/list`) over the
 *      returned `structuredContent`.
 *
 * The two "negative control" tools at the bottom prove the server validator is
 * genuinely on this path: one omits `structuredContent` entirely, the other
 * returns content that violates the schema. Both must come back as `isError`.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("../src/cli.js", () => ({
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

import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { InMemoryTransport } from "@modelcontextprotocol/sdk/inMemory.js";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { runCli } from "../src/cli.js";
import { TOOL_OUTPUT_SCHEMA } from "../src/formatters/response.js";

import { registerStatusTool } from "../src/tools/status.js";
import {
  registerListAccountsTool,
  registerManageAccountTool,
} from "../src/tools/accounts.js";
import { registerPortfolioTool } from "../src/tools/portfolio.js";
import {
  registerGetPricesTool,
  registerGetMarketDataTool,
} from "../src/tools/prices.js";
import {
  registerListTransactionsTool,
  registerRecordTransactionTool,
  registerTrackConversionTool,
  registerExportTransactionsTool,
} from "../src/tools/transactions.js";
import {
  registerManageWalletTool,
  registerSyncWalletTool,
} from "../src/tools/wallets.js";
import {
  registerGetPnlSummaryTool,
  registerGetRealizedPnlTool,
  registerGetUnrealizedPnlTool,
  registerAnalyzeAssetTool,
  registerPnlBackfillTool,
} from "../src/tools/pnl.js";
import { registerSyncExchangeTool } from "../src/tools/sync.js";
import {
  registerAuditLogTool,
  registerGetSyncHistoryTool,
} from "../src/tools/audit.js";
import { registerGetMiningPnlTool } from "../src/tools/mining.js";
import { registerListHoldingsTool } from "../src/tools/holdings.js";
import { registerImportBinanceTool } from "../src/tools/import.js";

const EXPECTED_TOOL_COUNT = 23;

/**
 * Mirrors the registration order in src/index.ts. Kept here (rather than
 * importing src/index.ts, which connects over stdio at module load) so the test
 * controls the transport.
 */
function registerAllTools(server: McpServer): void {
  registerStatusTool(server);
  registerListAccountsTool(server);
  registerPortfolioTool(server);
  registerGetPricesTool(server);
  registerListTransactionsTool(server);
  registerRecordTransactionTool(server);
  registerManageAccountTool(server);
  registerManageWalletTool(server);
  registerSyncWalletTool(server);
  registerSyncExchangeTool(server);
  registerTrackConversionTool(server);
  registerGetPnlSummaryTool(server);
  registerGetRealizedPnlTool(server);
  registerGetUnrealizedPnlTool(server);
  registerAnalyzeAssetTool(server);
  registerExportTransactionsTool(server);
  registerAuditLogTool(server);
  registerGetMarketDataTool(server);
  registerGetSyncHistoryTool(server);
  registerListHoldingsTool(server);
  registerGetMiningPnlTool(server);
  registerPnlBackfillTool(server);
  registerImportBinanceTool(server);
}

async function makeConnectedClient(): Promise<{
  client: Client;
  server: McpServer;
}> {
  const server = new McpServer({ name: "cryptofolio-test", version: "0.0.1" });
  registerAllTools(server);

  const [clientTransport, serverTransport] =
    InMemoryTransport.createLinkedPair();
  await server.connect(serverTransport);

  const client = new Client(
    { name: "test-client", version: "0.0.1" },
    { capabilities: {} }
  );
  await client.connect(clientTransport);

  return { client, server };
}

function structured(result: { structuredContent?: unknown }): Record<
  string,
  unknown
> {
  return result.structuredContent as Record<string, unknown>;
}

function textOf(result: {
  content: Array<{ type: string; text?: string }>;
}): string {
  return result.content
    .map((c) => c.text ?? "")
    .join("\n");
}

describe("MCP server output schema integration", () => {
  beforeEach(() => vi.clearAllMocks());

  it("lists all 23 tools and every one declares an outputSchema", async () => {
    const { client } = await makeConnectedClient();

    // listTools() also caches the client-side Ajv validators used below.
    const { tools } = await client.listTools();

    expect(tools).toHaveLength(EXPECTED_TOOL_COUNT);
    expect(new Set(tools.map((t) => t.name)).size).toBe(EXPECTED_TOOL_COUNT);

    for (const tool of tools) {
      expect(tool.outputSchema, `${tool.name} has no outputSchema`).toBeDefined();
      expect(typeof tool.outputSchema).toBe("object");
      // The permissive envelope always requires `success`.
      expect(
        (tool.outputSchema as { properties?: Record<string, unknown> })
          .properties
      ).toHaveProperty("success");
    }

    await client.close();
  });

  it("success path: returns structuredContent that passes the outputSchema", async () => {
    const { client } = await makeConnectedClient();

    vi.mocked(runCli).mockResolvedValueOnce([
      {
        asset: "BTC",
        quantity: "0.5",
        cost_basis: "45000",
        account: "Ledger",
        account_id: "acc-1",
      },
    ]);

    await client.listTools(); // cache validators
    const result = await client.callTool({
      name: "cryptofolio_list_holdings",
      arguments: {},
    });

    expect(result.isError).toBeFalsy();
    const sc = structured(result);
    expect(sc["success"]).toBe(true);
    expect(sc["data"]).toHaveLength(1);

    // structuredContent mirrors content[0].text exactly.
    expect(JSON.parse(textOf(result))).toEqual(sc);

    // And it satisfies the declared machine-readable schema.
    const parsed = TOOL_OUTPUT_SCHEMA.safeParse(sc);
    expect(parsed.success).toBe(true);

    await client.close();
  });

  it("error path: cryptofolio_get_pnl_summary returns valid structuredContent", async () => {
    const { client } = await makeConnectedClient();

    const { CliError } = await import("../src/cli.js");
    vi.mocked(runCli).mockRejectedValueOnce(
      new CliError(1, "database is locked", "pnl summary")
    );

    await client.listTools();
    const result = await client.callTool({
      name: "cryptofolio_get_pnl_summary",
      arguments: {},
    });

    // Tools return error envelopes (not isError), so the server validates them.
    expect(result.isError).toBeFalsy();
    const sc = structured(result);
    expect(sc["success"]).toBe(false);
    expect(sc["error"]).toContain("database is locked");
    expect(sc["code"]).toBe("CLI_ERROR_1");
    expect(JSON.parse(textOf(result))).toEqual(sc);
    expect(TOOL_OUTPUT_SCHEMA.safeParse(sc).success).toBe(true);

    await client.close();
  });

  it("negative control: the server rejects a result with no structuredContent", async () => {
    const server = new McpServer({ name: "cryptofolio-neg-1", version: "0.0.1" });
    server.registerTool(
      "test_output_missing",
      {
        description: "test tool that omits structuredContent",
        inputSchema: {},
        outputSchema: TOOL_OUTPUT_SCHEMA,
      },
      async () => ({ content: [{ type: "text", text: "{}" }] })
    );
    const [clientTransport, serverTransport] =
      InMemoryTransport.createLinkedPair();
    await server.connect(serverTransport);
    const client = new Client(
      { name: "neg-client-1", version: "0.0.1" },
      { capabilities: {} }
    );
    await client.connect(clientTransport);
    await client.listTools();

    const result = await client.callTool({
      name: "test_output_missing",
      arguments: {},
    });

    expect(result.isError).toBe(true);
    expect(textOf(result)).toContain("no structured content");

    await client.close();
  });

  it("negative control: the server rejects structuredContent that violates the schema", async () => {
    const server = new McpServer({ name: "cryptofolio-neg-2", version: "0.0.1" });
    server.registerTool(
      "test_output_invalid",
      {
        description: "test tool that returns schema-violating structuredContent",
        inputSchema: {},
        outputSchema: TOOL_OUTPUT_SCHEMA,
      },
      async () => ({
        content: [{ type: "text", text: "{}" }],
        structuredContent: { success: "not-a-boolean" },
      })
    );
    const [clientTransport, serverTransport] =
      InMemoryTransport.createLinkedPair();
    await server.connect(serverTransport);
    const client = new Client(
      { name: "neg-client-2", version: "0.0.1" },
      { capabilities: {} }
    );
    await client.connect(clientTransport);
    await client.listTools();

    const result = await client.callTool({
      name: "test_output_invalid",
      arguments: {},
    });

    expect(result.isError).toBe(true);
    expect(textOf(result)).toContain("Invalid structured content");

    await client.close();
  });
});
