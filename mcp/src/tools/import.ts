/**
 * cryptofolio_import_binance — import a Binance CSV/ZIP export (WRITE)
 */

import { z } from "zod";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { runCliRaw, SYNC_TIMEOUT_MS } from "../cli.js";
import {
  buildSuccess,
  toContent,
  handleCliError,
  outputEnvelopeNote,
} from "../formatters/response.js";

export function registerImportBinanceTool(server: McpServer): void {
  server.tool(
    "cryptofolio_import_binance",
    "Import a Binance CSV or ZIP export into an existing account, writing transactions to the ledger (write) unless dry_run is true. " +
      outputEnvelopeNote("{output}"),
    {
      file: z
        .string()
        .min(1)
        .describe("Path to a Binance .csv or .zip export file"),
      account: z
        .string()
        .min(1)
        .describe(
          "Account to import into; it must already exist (create with cryptofolio_manage_account first)"
        ),
      dry_run: z
        .boolean()
        .optional()
        .describe(
          "Preview what would be imported without writing anything (default: false)"
        ),
    },
    async ({ file, account, dry_run }) => {
      try {
        // `import-binance` does not implement --json; use runCliRaw.
        const args = ["import-binance", file, "--account", account];
        if (dry_run === true) args.push("--dry-run");

        const output = await runCliRaw(args, SYNC_TIMEOUT_MS);

        return toContent(
          buildSuccess(
            { output: output || "Import completed." },
            dry_run === true
              ? `Dry run for "${file}" complete — nothing was written.`
              : `"${file}" imported into "${account}".`
          )
        );
      } catch (err) {
        return toContent(handleCliError(err, "cryptofolio_import_binance"));
      }
    }
  );
}
