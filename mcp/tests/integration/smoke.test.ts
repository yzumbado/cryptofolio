/**
 * Integration smoke test — requires a real cryptofolio binary.
 *
 * Skipped automatically unless CRYPTOFOLIO_INTEGRATION=1 is set.
 * Also requires CRYPTOFOLIO_BIN pointing to the compiled binary.
 *
 * Run with:
 *   CRYPTOFOLIO_INTEGRATION=1 CRYPTOFOLIO_BIN=/path/to/cryptofolio \
 *   npx vitest run tests/integration
 */

import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const SHOULD_RUN = process.env["CRYPTOFOLIO_INTEGRATION"] === "1";

describe.skipIf(!SHOULD_RUN)("MCP integration smoke test", () => {
  let tempDir: string;

  beforeAll(() => {
    if (!SHOULD_RUN) return;
    tempDir = mkdtempSync(join(tmpdir(), "cryptofolio-mcp-test-"));
    // Point cryptofolio to an isolated config dir
    process.env["CRYPTOFOLIO_CONFIG_DIR"] = tempDir;
  });

  afterAll(() => {
    if (tempDir) {
      try {
        rmSync(tempDir, { recursive: true, force: true });
      } catch {
        // ignore cleanup errors
      }
    }
  });

  it("runCli(['account', 'list']) returns an empty array on a fresh database", async () => {
    const { runCli } = await import("../../src/cli.js");
    const result = await runCli(["account", "list"]);
    expect(Array.isArray(result)).toBe(true);
    expect((result as unknown[]).length).toBe(0);
  });

  it("runCli(['config', 'show']) returns expected shape", async () => {
    const { runCli } = await import("../../src/cli.js");
    const result = (await runCli(["config", "show"])) as {
      general: unknown;
      binance: unknown;
      blockfrost: unknown;
      paths: { config_dir: string; database: string };
    };

    expect(result).toHaveProperty("general");
    expect(result).toHaveProperty("binance");
    expect(result).toHaveProperty("blockfrost");
    expect(result.paths.config_dir).toBeTruthy();
  });

  it("can add and list an account end-to-end", async () => {
    const { runCli, runCliRaw } = await import("../../src/cli.js");

    // Create category first (seeded by migrations, but ensure it exists)
    try {
      await runCliRaw(["category", "add", "trading"]);
    } catch {
      // already exists — ignore
    }

    // Add account
    await runCli([
      "account",
      "add",
      "IntegrationTest",
      "--type",
      "exchange",
      "--category",
      "trading",
    ]);

    // List and verify
    const accounts = (await runCli(["account", "list"])) as Array<{
      name: string;
    }>;
    expect(accounts.some((a) => a.name === "IntegrationTest")).toBe(true);

    // Cleanup
    await runCliRaw(["account", "remove", "IntegrationTest", "--yes"]);
  });
});
