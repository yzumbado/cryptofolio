/**
 * cryptofolio_list_transactions    — paginated transaction history
 * cryptofolio_record_transaction   — record buy/sell/transfer/swap
 * cryptofolio_track_conversion     — multi-step fiat-to-crypto flow
 * cryptofolio_export_transactions  — export CSV/JSON for tax reporting
 */

import { z } from "zod";
import { mkdirSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { runCli, runCliRaw } from "../cli.js";
import { multiplyDecimalStrings } from "../decimal.js";
import {
  buildSuccess,
  buildError,
  toContent,
  handleCliError,
  paginate,
  outputEnvelopeNote,
} from "../formatters/response.js";
import type { CliTransaction } from "../types.js";

// ---------------------------------------------------------------------------
// cryptofolio_list_transactions
// ---------------------------------------------------------------------------

export function registerListTransactionsTool(server: McpServer): void {
  server.tool(
    "cryptofolio_list_transactions",
    "List transaction history for an optional account and asset filter, newest first, using offset/limit pagination. " +
      outputEnvelopeNote(
        "{items, total_fetched, offset, limit, has_more, next_offset?}"
      ),
    {
      account: z
        .string()
        .optional()
        .describe("Filter to a specific account name"),
      asset: z
        .string()
        .optional()
        .describe('Filter to a specific asset symbol, e.g. "BTC"'),
      limit: z
        .number()
        .int()
        .min(1)
        .max(500)
        .default(50)
        .describe("Number of transactions to return (default: 50, max: 500)"),
      offset: z
        .number()
        .int()
        .min(0)
        .default(0)
        .describe("Number of transactions to skip for pagination (default: 0)"),
    },
    async ({ account, asset, limit, offset }) => {
      try {
        // `tx list` supports only --account and --limit, so asset filtering and
        // pagination happen here. Fetch a window of raw rows, filter it by asset,
        // and grow the window until it holds enough MATCHING rows. `offset` is
        // applied exactly once, by paginate() below, against the filtered set:
        // offset = number of matching transactions to skip.
        const MAX_FETCH_WINDOW = 5000;
        const needed = offset + limit;
        // Fetch one row beyond the requested page on both paths: paginate() can
        // then prove more rows exist (has_more) instead of assuming the window
        // boundary is the end of the data.
        const target = needed + 1;
        const sym = asset?.toUpperCase();

        let transactions: CliTransaction[] = [];
        let window = target;

        for (;;) {
          const args = ["tx", "list", "--limit", String(window)];
          if (account) args.push("--account", account);

          const rows = ((await runCli(args)) as CliTransaction[]) ?? [];
          transactions = sym
            ? rows.filter(
                (t) =>
                  t.from_asset?.toUpperCase() === sym ||
                  t.to_asset?.toUpperCase() === sym
              )
            : rows;

          // Stop when the window holds one more matching row than the page
          // needs (so has_more is provable), the CLI ran out of data (fewer
          // rows than requested), or the guard cap is reached. When the cap is
          // hit, has_more reflects only what was fetched.
          if (
            transactions.length >= target ||
            rows.length < window ||
            window >= MAX_FETCH_WINDOW
          ) {
            break;
          }
          window = Math.min(window * 2, MAX_FETCH_WINDOW);
        }

        const page = paginate(transactions, offset, limit);

        return toContent(
          buildSuccess(
            page,
            `Showing ${page.items.length} of ${page.total_fetched} transaction(s).` +
              (page.has_more
                ? ` Call again with offset: ${page.next_offset} for more.`
                : "")
          )
        );
      } catch (err) {
        return toContent(
          handleCliError(err, "cryptofolio_list_transactions")
        );
      }
    }
  );
}

// ---------------------------------------------------------------------------
// cryptofolio_record_transaction
// ---------------------------------------------------------------------------

export function registerRecordTransactionTool(server: McpServer): void {
  server.tool(
    "cryptofolio_record_transaction",
    "Record a buy, sell, transfer, or swap to the ledger (write), taking monetary values as decimal strings to preserve precision. " +
      outputEnvelopeNote("the CLI's transaction object, or {recorded: true}"),
    {
      type: z
        .enum(["buy", "sell", "transfer", "swap"])
        .describe("Transaction type"),
      asset: z
        .string()
        .optional()
        .describe(
          "Asset symbol for buy/sell/transfer (e.g. BTC). For swap, use from_asset and to_asset."
        ),
      quantity: z
        .string()
        .optional()
        .describe(
          "Quantity of asset for buy/sell/transfer (e.g. '0.1'). For swap, use from_quantity and to_quantity."
        ),
      account: z
        .string()
        .optional()
        .describe("Account name for buy/sell/swap"),
      from_account: z
        .string()
        .optional()
        .describe("Source account for transfer"),
      to_account: z
        .string()
        .optional()
        .describe("Destination account for transfer"),
      price_usd: z
        .string()
        .optional()
        .describe(
          "Price in USD per unit for buy/sell (e.g. '95000'). Required for buy and sell."
        ),
      from_asset: z
        .string()
        .optional()
        .describe("Source asset symbol for swap (e.g. USDT)"),
      from_quantity: z
        .string()
        .optional()
        .describe("Source quantity for swap (e.g. '1000')"),
      to_asset: z
        .string()
        .optional()
        .describe("Destination asset symbol for swap (e.g. BTC)"),
      to_quantity: z
        .string()
        .optional()
        .describe("Destination quantity for swap (e.g. '0.01049')"),
      fee: z
        .string()
        .optional()
        .describe("Transaction fee amount (optional)"),
      timestamp: z
        .string()
        .optional()
        .describe(
          "Transaction timestamp in ISO 8601 format (default: now)"
        ),
      notes: z.string().optional().describe("Optional notes or memo"),
      cost_basis_only: z
        .boolean()
        .optional()
        .describe(
          "Record a cost-basis tax lot without changing the holdings quantity (for buys on synced accounts)."
        ),
    },
    async ({
      type,
      asset,
      quantity,
      account,
      from_account,
      to_account,
      price_usd,
      from_asset,
      from_quantity,
      to_asset,
      to_quantity,
      fee,
      timestamp,
      notes,
      cost_basis_only,
    }) => {
      try {
        let args: string[];

        switch (type) {
          case "buy":
            if (!asset || !quantity || !price_usd || !account) {
              return toContent(
                buildError(
                  "buy requires: asset, quantity, price_usd, account",
                  "MISSING_PARAM"
                )
              );
            }
            args = [
              "tx",
              "buy",
              asset,
              quantity,
              "--account",
              account,
              "--price",
              price_usd,
            ];
            if (cost_basis_only === true) args.push("--cost-basis-only");
            break;

          case "sell":
            if (!asset || !quantity || !price_usd || !account) {
              return toContent(
                buildError(
                  "sell requires: asset, quantity, price_usd, account",
                  "MISSING_PARAM"
                )
              );
            }
            args = [
              "tx",
              "sell",
              asset,
              quantity,
              "--account",
              account,
              "--price",
              price_usd,
            ];
            break;

          case "transfer":
            if (!asset || !quantity || !from_account || !to_account) {
              return toContent(
                buildError(
                  "transfer requires: asset, quantity, from_account, to_account",
                  "MISSING_PARAM"
                )
              );
            }
            args = [
              "tx",
              "transfer",
              asset,
              quantity,
              "--from",
              from_account,
              "--to",
              to_account,
            ];
            if (fee) args.push("--fee", fee);
            break;

          case "swap":
            if (
              !from_asset ||
              !from_quantity ||
              !to_asset ||
              !to_quantity ||
              !account
            ) {
              return toContent(
                buildError(
                  "swap requires: from_asset, from_quantity, to_asset, to_quantity, account",
                  "MISSING_PARAM"
                )
              );
            }
            args = [
              "tx",
              "swap",
              "--from-asset",
              from_asset,
              "--from-quantity",
              from_quantity,
              "--to-asset",
              to_asset,
              "--to-quantity",
              to_quantity,
              "--account",
              account,
            ];
            break;
        }

        if (timestamp) args.push("--date", timestamp);
        if (notes) args.push("--notes", notes);

        const raw = await runCli(args);

        return toContent(
          buildSuccess(
            raw ?? { recorded: true },
            `Transaction recorded: ${type.toUpperCase()}` +
              (asset ? ` ${quantity} ${asset}` : "") +
              (account ? ` in ${account}` : "") +
              (from_account && to_account
                ? ` from ${from_account} → ${to_account}`
                : "")
          )
        );
      } catch (err) {
        return toContent(
          handleCliError(err, "cryptofolio_record_transaction")
        );
      }
    }
  );
}

// ---------------------------------------------------------------------------
// cryptofolio_track_conversion
// ---------------------------------------------------------------------------

interface ConversionStep {
  from: string;
  to: string;
  amount: string;
  rate?: string;
  account?: string;
  fee?: string;
  notes?: string;
}

export function registerTrackConversionTool(server: McpServer): void {
  server.tool(
    "cryptofolio_track_conversion",
    "Record a multi-step conversion such as CRC → USD → USDT → BTC as one swap transaction per step (write); failed steps are reported without rolling back earlier ones. " +
      outputEnvelopeNote("{description, steps_recorded, errors}"),
    {
      description: z
        .string()
        .describe(
          "Human-readable description of the overall conversion (e.g. 'Monthly DCA: CRC to BTC')"
        ),
      steps: z
        .array(
          z.object({
            from: z.string().describe("Source asset symbol (e.g. CRC)"),
            to: z.string().describe("Destination asset symbol (e.g. USD)"),
            amount: z.string().describe("Amount of source asset"),
            rate: z
              .string()
              .optional()
              .describe("Exchange rate (to/from ratio, optional)"),
            account: z
              .string()
              .optional()
              .describe("Account for this step"),
            fee: z.string().optional().describe("Fee for this step"),
            notes: z
              .string()
              .optional()
              .describe("Notes for this step"),
          })
        )
        .min(1)
        .describe("Ordered list of conversion steps"),
    },
    async ({ description, steps }) => {
      const recorded: string[] = [];
      const errors: string[] = [];

      for (const step of steps as ConversionStep[]) {
        try {
          // Compute to_quantity from rate if provided. Exact decimal-string
          // math — parseFloat here would inject binary-float noise into a
          // recorded quantity. Absent/empty rate keeps to_quantity = amount.
          const toAmt = step.rate
            ? multiplyDecimalStrings(step.amount, step.rate, 8)
            : step.amount;

          const args = [
            "tx",
            "swap",
            "--from-asset",
            step.from,
            "--from-quantity",
            step.amount,
            "--to-asset",
            step.to,
            "--to-quantity",
            toAmt,
          ];
          if (step.account) args.push("--account", step.account);
          if (step.fee) args.push("--fee", step.fee);
          if (step.notes) args.push("--notes", step.notes);
          else if (description)
            args.push("--notes", `${description}: ${step.from} → ${step.to}`);

          await runCli(args);
          recorded.push(`${step.from} → ${step.to} (${step.amount})`);
        } catch (err: unknown) {
          const msg = err instanceof Error ? err.message : String(err);
          errors.push(`Step ${step.from} → ${step.to}: ${msg}`);
        }
      }

      if (errors.length > 0 && recorded.length === 0) {
        return toContent(
          buildError(
            `All conversion steps failed: ${errors.join("; ")}`,
            "CONVERSION_FAILED"
          )
        );
      }

      return toContent(
        buildSuccess(
          { description, steps_recorded: recorded, errors },
          `Recorded ${recorded.length}/${(steps as ConversionStep[]).length} conversion step(s) for: ${description}`
        )
      );
    }
  );
}

// ---------------------------------------------------------------------------
// cryptofolio_export_transactions
// ---------------------------------------------------------------------------

export function registerExportTransactionsTool(server: McpServer): void {
  server.tool(
    "cryptofolio_export_transactions",
    "Write the filtered transaction history to a timestamped CSV or JSON file under ~/.config/cryptofolio/exports/ (write) and return its path. " +
      outputEnvelopeNote("{file_path, format, filters}"),
    {
      format: z
        .enum(["csv", "json"])
        .default("csv")
        .describe("Export format (default: csv)"),
      from_date: z
        .string()
        .optional()
        .describe("Start date filter in YYYY-MM-DD format"),
      to_date: z
        .string()
        .optional()
        .describe("End date filter in YYYY-MM-DD format"),
      account: z
        .string()
        .optional()
        .describe("Filter to a specific account"),
      asset: z
        .string()
        .optional()
        .describe("Filter to a specific asset symbol"),
    },
    async ({ format, from_date, to_date, account, asset }) => {
      try {
        // Create exports directory
        const exportsDir = join(
          homedir(),
          ".config",
          "cryptofolio",
          "exports"
        );
        mkdirSync(exportsDir, { recursive: true });

        // Generate timestamped filename
        const ts = new Date()
          .toISOString()
          .replace(/[:.]/g, "-")
          .slice(0, 19);
        const filename = `transactions-${ts}.${format}`;
        const outputPath = join(exportsDir, filename);

        const args = ["tx", "export", outputPath, "--format", format];
        if (from_date) args.push("--from", from_date);
        if (to_date) args.push("--to", to_date);
        if (account) args.push("--account", account);
        if (asset) args.push("--asset", asset);

        await runCliRaw(args);

        return toContent(
          buildSuccess(
            {
              file_path: outputPath,
              format,
              filters: {
                from_date: from_date ?? null,
                to_date: to_date ?? null,
                account: account ?? null,
                asset: asset ?? null,
              },
            },
            `Exported to: ${outputPath}`
          )
        );
      } catch (err) {
        return toContent(
          handleCliError(err, "cryptofolio_export_transactions")
        );
      }
    }
  );
}
