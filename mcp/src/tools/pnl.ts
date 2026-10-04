/**
 * cryptofolio_get_pnl_summary      — overall realized + unrealized P&L
 * cryptofolio_get_realized_pnl     — closed positions with gain/loss per disposal
 * cryptofolio_get_unrealized_pnl   — open position P&L (mark-to-market)
 * cryptofolio_analyze_asset        — deep dive on one asset: holdings + P&L
 * cryptofolio_pnl_backfill         — recompute tax lots + realized P&L (WRITE)
 */

import { z } from "zod";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { runCli, runCliRaw } from "../cli.js";
import { addDecimalStrings, roundDecimalString } from "../decimal.js";
import {
  buildSuccess,
  toContent,
  handleCliError,
  paginate,
  outputEnvelopeNote,
} from "../formatters/response.js";
import type {
  CliPnlSummary,
  CliRealizedEntry,
  CliUnrealizedEntry,
} from "../types.js";

// ---------------------------------------------------------------------------
// cryptofolio_get_pnl_summary
// ---------------------------------------------------------------------------

export function registerGetPnlSummaryTool(server: McpServer): void {
  server.tool(
    "cryptofolio_get_pnl_summary",
    "Return total realized, unrealized, and net P&L, optionally filtered by account and date range. " +
      outputEnvelopeNote(
        "{total_realized, total_unrealized, net_pnl, account?, from?, to?}"
      ),
    {
      account: z
        .string()
        .optional()
        .describe("Filter to a specific account"),
      from_date: z
        .string()
        .optional()
        .describe("Start date in YYYY-MM-DD format"),
      to_date: z
        .string()
        .optional()
        .describe("End date in YYYY-MM-DD format"),
    },
    async ({ account, from_date, to_date }) => {
      try {
        const args = ["pnl", "summary"];
        if (account) args.push("--account", account);
        if (from_date) args.push("--from", from_date);
        if (to_date) args.push("--to", to_date);

        const raw = await runCli(args);
        const summary = raw as CliPnlSummary;

        return toContent(
          buildSuccess(
            summary,
            `Realized: $${summary.total_realized} | Unrealized: $${summary.total_unrealized} | Net: $${summary.net_pnl}`
          )
        );
      } catch (err) {
        return toContent(handleCliError(err, "cryptofolio_get_pnl_summary"));
      }
    }
  );
}

// ---------------------------------------------------------------------------
// cryptofolio_get_realized_pnl
// ---------------------------------------------------------------------------

export function registerGetRealizedPnlTool(server: McpServer): void {
  server.tool(
    "cryptofolio_get_realized_pnl",
    "List closed positions with proceeds, cost basis, and gain or loss per disposal event, optionally filtered by account, asset, and date range, using offset/limit pagination. " +
      outputEnvelopeNote(
        "{items, total_fetched, offset, limit, has_more, next_offset?}"
      ),
    {
      account: z
        .string()
        .optional()
        .describe("Filter to a specific account"),
      asset: z
        .string()
        .optional()
        .describe("Filter to a specific asset symbol"),
      from_date: z
        .string()
        .optional()
        .describe("Start date (e.g. 2024-01-01)"),
      to_date: z
        .string()
        .optional()
        .describe("End date (e.g. 2024-12-31)"),
      limit: z
        .number()
        .int()
        .min(1)
        .max(500)
        .default(50)
        .describe("Number of entries to return (default: 50)"),
      offset: z
        .number()
        .int()
        .min(0)
        .default(0)
        .describe("Entries to skip for pagination (default: 0)"),
    },
    async ({ account, asset, from_date, to_date, limit, offset }) => {
      try {
        const fetchLimit = offset + limit;
        const args = ["pnl", "realized", "--limit", String(fetchLimit)];
        if (account) args.push("--account", account);
        if (asset) args.push("--asset", asset);
        if (from_date) args.push("--from", from_date);
        if (to_date) args.push("--to", to_date);

        const raw = await runCli(args);
        const entries = Array.isArray(raw) ? (raw as CliRealizedEntry[]) : [];
        const page = paginate(entries, offset, limit);

        return toContent(
          buildSuccess(
            page,
            `${page.items.length} realized P&L entry(ies).`
          )
        );
      } catch (err) {
        return toContent(
          handleCliError(err, "cryptofolio_get_realized_pnl")
        );
      }
    }
  );
}

// ---------------------------------------------------------------------------
// cryptofolio_get_unrealized_pnl
// ---------------------------------------------------------------------------

export function registerGetUnrealizedPnlTool(server: McpServer): void {
  server.tool(
    "cryptofolio_get_unrealized_pnl",
    "Return each open position with average cost basis, current price, and unrealized gain, plus the summed total unrealized P&L. " +
      outputEnvelopeNote("{entries, total_unrealized_pnl}"),
    {
      account: z
        .string()
        .optional()
        .describe("Filter to a specific account"),
      asset: z
        .string()
        .optional()
        .describe("Filter to a specific asset symbol"),
    },
    async ({ account, asset }) => {
      try {
        const args = ["pnl", "unrealized"];
        if (account) args.push("--account", account);
        if (asset) args.push("--asset", asset);

        const raw = await runCli(args);
        // `pnl unrealized --json` emits a bare array (the CLI only prints a
        // total on its human-readable path), so the total is summed here.
        // Fail closed on an unexpected payload instead of misreporting zero.
        if (!Array.isArray(raw)) {
          throw new Error(
            "cryptofolio pnl unrealized returned an unexpected non-list payload"
          );
        }
        const entries = raw as CliUnrealizedEntry[];

        // Exact decimal-string addition — never parseFloat on money. The
        // existing message renders a 2dp USD total, so the exact sum is
        // rounded half-up to 2dp via integer math rather than truncated.
        const totalUnrealized = roundDecimalString(
          addDecimalStrings(entries.map((e) => e.unrealized_pnl)),
          2
        );

        return toContent(
          buildSuccess(
            { entries, total_unrealized_pnl: totalUnrealized },
            `${entries.length} open position(s). Total unrealized P&L: $${totalUnrealized}`
          )
        );
      } catch (err) {
        return toContent(
          handleCliError(err, "cryptofolio_get_unrealized_pnl")
        );
      }
    }
  );
}

// ---------------------------------------------------------------------------
// cryptofolio_analyze_asset
// ---------------------------------------------------------------------------

export function registerAnalyzeAssetTool(server: McpServer): void {
  server.tool(
    "cryptofolio_analyze_asset",
    "Return a combined P&L breakdown for one asset — realized and unrealized P&L — optionally scoped to a single account. " +
      outputEnvelopeNote(
        "{asset, account, realized_transactions, total_realized, total_unrealized, net_pnl} (all monetary values as decimal strings)"
      ),
    {
      asset: z
        .string()
        .describe("Asset symbol to analyze (e.g. BTC, ETH, SOL)"),
      account: z
        .string()
        .optional()
        .describe("Filter to a specific account"),
    },
    async ({ asset, account }) => {
      try {
        const args = ["pnl", "by-asset", asset];
        if (account) args.push("--account", account);

        const raw = await runCli(args);

        return toContent(
          buildSuccess(
            raw ?? {},
            `P&L analysis for ${asset}${account ? ` in ${account}` : ""}.`
          )
        );
      } catch (err) {
        return toContent(handleCliError(err, "cryptofolio_analyze_asset"));
      }
    }
  );
}

// ---------------------------------------------------------------------------
// cryptofolio_pnl_backfill
// ---------------------------------------------------------------------------

export function registerPnlBackfillTool(server: McpServer): void {
  server.tool(
    "cryptofolio_pnl_backfill",
    "Recompute tax lots and realized P&L by replaying every transaction oldest-first, clearing and rebuilding existing P&L data (destructive write); re-run cryptofolio_get_pnl_summary afterwards. " +
      outputEnvelopeNote("{output}"),
    {
      account: z
        .string()
        .optional()
        .describe("Only backfill transactions touching this account"),
    },
    async ({ account }) => {
      try {
        // Non-interactive: pass --yes to confirm the destructive recompute.
        // `pnl backfill` does not implement --json; use runCliRaw.
        const args = ["pnl", "backfill", "--yes"];
        if (account) args.push("--account", account);

        const output = await runCliRaw(args);

        return toContent(
          buildSuccess(
            { output: output || "Backfill completed." },
            "P&L backfill complete." +
              (account ? ` Scope: ${account}.` : " Scope: all accounts.")
          )
        );
      } catch (err) {
        return toContent(handleCliError(err, "cryptofolio_pnl_backfill"));
      }
    }
  );
}
