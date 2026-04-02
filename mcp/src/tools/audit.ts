/**
 * cryptofolio_get_audit_log — view sync history, coverage, and errors
 */

import { z } from "zod";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { runCli } from "../cli.js";
import {
  buildSuccess,
  toContent,
  handleCliError,
  paginate,
} from "../formatters/response.js";
import type {
  CliAuditSyncEntry,
  CliAuditCoverageEntry,
  CliAuditErrorEntry,
} from "../types.js";

export function registerAuditLogTool(server: McpServer): void {
  server.tool(
    "cryptofolio_get_audit_log",
    "View the sync audit log for blockchain wallets. Use view='sync_history' to see recent sync operations, 'coverage' to see which addresses have been synced and up to which block, or 'errors' to see failed sync attempts.",
    {
      view: z
        .enum(["sync_history", "coverage", "errors"])
        .describe(
          '"sync_history" — recent sync operations | "coverage" — address sync coverage | "errors" — failed syncs'
        ),
      wallet: z
        .string()
        .optional()
        .describe("Filter to a specific wallet name"),
      chain: z
        .string()
        .optional()
        .describe(
          'Filter to a blockchain (e.g. "bitcoin", "ethereum", "solana", "cardano")'
        ),
      limit: z
        .number()
        .int()
        .min(1)
        .max(200)
        .default(50)
        .describe("Number of entries to return (default: 50)"),
      offset: z
        .number()
        .int()
        .min(0)
        .default(0)
        .describe("Entries to skip for pagination (default: 0)"),
    },
    async ({ view, wallet, chain, limit, offset }) => {
      try {
        const fetchLimit = offset + limit;

        switch (view) {
          case "sync_history": {
            const args = [
              "audit",
              "sync",
              "--limit",
              String(fetchLimit),
            ];
            if (wallet) args.push("--wallet", wallet);
            if (chain) args.push("--chain", chain);

            const raw = await runCli(args);
            const entries = (raw as CliAuditSyncEntry[]) ?? [];
            const page = paginate(entries, offset, limit);

            const errorCount = page.items.filter((e) => e.error).length;
            return toContent(
              buildSuccess(
                page,
                `${page.items.length} sync event(s).` +
                  (errorCount > 0
                    ? ` ${errorCount} with errors — use view='errors' for details.`
                    : "")
              )
            );
          }

          case "coverage": {
            const args = ["audit", "coverage"];
            if (wallet) args.push("--wallet", wallet);
            if (chain) args.push("--chain", chain);

            const raw = await runCli(args);
            const entries = (raw as CliAuditCoverageEntry[]) ?? [];
            const page = paginate(entries, offset, limit);

            const neverSynced = page.items.filter(
              (e) => !e.last_sync_at
            ).length;
            return toContent(
              buildSuccess(
                page,
                `${page.items.length} address(es).` +
                  (neverSynced > 0
                    ? ` ${neverSynced} never synced — call cryptofolio_sync_wallet.`
                    : "")
              )
            );
          }

          case "errors": {
            const args = ["audit", "errors", "--limit", String(fetchLimit)];

            const raw = await runCli(args);
            const entries = (raw as CliAuditErrorEntry[]) ?? [];
            const page = paginate(entries, offset, limit);

            return toContent(
              buildSuccess(
                page,
                `${page.items.length} error(s) found.` +
                  (page.items.length === 0
                    ? " No sync errors. All wallets are syncing cleanly."
                    : "")
              )
            );
          }
        }
      } catch (err) {
        return toContent(handleCliError(err, "cryptofolio_get_audit_log"));
      }
    }
  );
}
