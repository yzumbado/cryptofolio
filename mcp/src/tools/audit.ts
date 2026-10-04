/**
 * cryptofolio_get_sync_history  — recent blockchain sync operations
 * cryptofolio_get_audit_log     — view sync history, coverage, and errors
 */

import { z } from "zod";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { runCli } from "../cli.js";
import {
  buildSuccess,
  toContent,
  handleCliError,
  paginate,
  outputEnvelopeNote,
  TOOL_OUTPUT_SCHEMA,
} from "../formatters/response.js";
import type {
  CliAuditSyncEntry,
  CliAuditCoverageEntry,
  CliAuditErrorEntry,
} from "../types.js";

export function registerAuditLogTool(server: McpServer): void {
  server.registerTool(
    "cryptofolio_get_audit_log",
    {
      description: "Return one view of the wallet sync audit log — recent sync operations, address coverage, or failed syncs — optionally filtered by wallet and chain, using offset/limit pagination. " +
        outputEnvelopeNote(
          "{items, total_fetched, offset, limit, has_more, next_offset?}"
        ),
      inputSchema: {
        view: z
          .enum(["sync_history", "coverage", "errors"])
          .describe("Which audit view to return"),
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
      outputSchema: TOOL_OUTPUT_SCHEMA,
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

// ---------------------------------------------------------------------------
// cryptofolio_get_sync_history
// ---------------------------------------------------------------------------

export function registerGetSyncHistoryTool(server: McpServer): void {
  server.registerTool(
    "cryptofolio_get_sync_history",
    {
      description: "List recent wallet sync operations from the audit log, newest first, showing the outcome, records pulled, and duration, optionally filtered by wallet and chain. " +
        outputEnvelopeNote(
          "{items, total_fetched, offset, limit, has_more, next_offset?}"
        ),
      inputSchema: {
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
      outputSchema: TOOL_OUTPUT_SCHEMA,
    },
    async ({ wallet, chain, limit, offset }) => {
      try {
        const fetchLimit = offset + limit;
        const args = ["audit", "sync", "--limit", String(fetchLimit)];
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
                ? ` ${errorCount} with errors — use cryptofolio_get_audit_log view='errors' for details.`
                : "")
          )
        );
      } catch (err) {
        return toContent(
          handleCliError(err, "cryptofolio_get_sync_history")
        );
      }
    }
  );
}
