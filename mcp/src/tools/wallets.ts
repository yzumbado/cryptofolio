/**
 * cryptofolio_manage_wallet — add, list, show, or remove blockchain wallets
 * cryptofolio_sync_wallet   — sync on-chain balance and transactions
 */

import { z } from "zod";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { runCli, runCliRaw, SYNC_TIMEOUT_MS } from "../cli.js";
import {
  buildSuccess,
  buildError,
  toContent,
  handleCliError,
} from "../formatters/response.js";
import type {
  CliWallet,
  CliWalletDetail,
  CliWalletAddResult,
} from "../types.js";

const BLOCKCHAINS = ["bitcoin", "ethereum", "solana", "cardano"] as const;

// ---------------------------------------------------------------------------
// cryptofolio_manage_wallet
// ---------------------------------------------------------------------------

export function registerManageWalletTool(server: McpServer): void {
  server.tool(
    "cryptofolio_manage_wallet",
    'Add, list, show details of, or remove a blockchain wallet. Use action "add" to register a new wallet address or xpub key, "list" to see all wallets, "show" to get details and holdings, or "remove" to delete a wallet.',
    {
      action: z
        .enum(["add", "list", "show", "remove"])
        .describe('Operation: "add", "list", "show", or "remove"'),
      name: z
        .string()
        .optional()
        .describe(
          "Wallet/account name. Required for add, show, remove. The account must already exist (create with cryptofolio_manage_account first)."
        ),
      blockchain: z
        .enum(BLOCKCHAINS)
        .optional()
        .describe(
          'Required for add. One of: bitcoin, ethereum, solana, cardano. For list, optionally filters by blockchain.'
        ),
      address: z
        .string()
        .optional()
        .describe("Blockchain address (conflicts with xpub)"),
      xpub: z
        .string()
        .optional()
        .describe("Extended public key for HD wallet (conflicts with address)"),
      derivation_path: z
        .string()
        .optional()
        .describe("Derivation path for xpub (e.g. m/84'/0'/0')"),
      address_type: z
        .enum(["legacy", "segwit", "native_segwit", "taproot", "erc20"])
        .optional()
        .describe(
          "Address type for xpub derivation. Use taproot for BIP-86 P2TR (bc1p) wallets."
        ),
      label: z.string().optional().describe("Optional label for this address"),
      account_type: z
        .enum([
          "exchange",
          "hardware_wallet",
          "software_wallet",
          "custodial_service",
          "bank",
        ])
        .optional()
        .describe(
          "If provided and the account does not exist, it will be created automatically with this type."
        ),
      category: z
        .string()
        .optional()
        .describe(
          'Category for auto-created account (e.g. "cold-storage"). Required when account_type is provided.'
        ),
    },
    async ({
      action,
      name,
      blockchain,
      address,
      xpub,
      derivation_path,
      address_type,
      label,
      account_type,
      category,
    }) => {
      try {
        switch (action) {
          case "add": {
            if (!name) {
              return toContent(
                buildError('name is required for action "add"', "MISSING_PARAM")
              );
            }
            if (!blockchain) {
              return toContent(
                buildError(
                  'blockchain is required for action "add"',
                  "MISSING_PARAM",
                  `One of: ${BLOCKCHAINS.join(", ")}`
                )
              );
            }
            if (!address && !xpub) {
              return toContent(
                buildError(
                  'Either address or xpub is required for action "add"',
                  "MISSING_PARAM"
                )
              );
            }

            const args = [
              "wallet",
              "add",
              name,
              "--blockchain",
              blockchain,
            ];
            if (address) args.push("--address", address);
            if (xpub) args.push("--xpub", xpub);
            if (derivation_path) args.push("--derivation-path", derivation_path);
            if (address_type) args.push("--address-type", address_type);
            if (label) args.push("--label", label);

            let raw: unknown;
            try {
              raw = await runCli(args);
            } catch (err: unknown) {
              // Auto-create the account if it doesn't exist and the caller
              // supplied account_type + category
              const msg = err instanceof Error ? err.message : String(err);
              const isNotFound =
                msg.toLowerCase().includes("account not found") ||
                msg.toLowerCase().includes("no such account");

              if (isNotFound && account_type && category) {
                // Create category (ignore error if it already exists)
                await runCliRaw(["category", "add", category]).catch(() => {});
                // Create account
                await runCli([
                  "account",
                  "add",
                  name,
                  "--type",
                  account_type,
                  "--category",
                  category,
                ]);
                // Retry wallet add
                raw = await runCli(args);
              } else if (isNotFound) {
                return toContent(
                  buildError(
                    `Account "${name}" not found. Provide account_type and category to create it automatically.`,
                    "ACCOUNT_NOT_FOUND",
                    'Example: { action: "add", name: "Ledger", blockchain: "bitcoin", address: "bc1q...", account_type: "hardware_wallet", category: "cold-storage" }'
                  )
                );
              } else {
                throw err;
              }
            }
            const result = raw as CliWalletAddResult;

            return toContent(
              buildSuccess(
                result,
                `Wallet "${name}" (${blockchain}) added successfully.` +
                  (result.derived_addresses
                    ? ` Derived ${result.derived_addresses} addresses from xpub.`
                    : "")
              )
            );
          }

          case "list": {
            const args = ["wallet", "list"];
            if (blockchain) args.push("--blockchain", blockchain);

            const raw = await runCli(args);
            const wallets = Array.isArray(raw) ? (raw as CliWallet[]) : [];

            return toContent(
              buildSuccess(
                { wallets },
                `${wallets.length} wallet(s) found.`
              )
            );
          }

          case "show": {
            if (!name) {
              return toContent(
                buildError('name is required for action "show"', "MISSING_PARAM")
              );
            }

            const raw = await runCli(["wallet", "show", name]);
            const wallet = raw as CliWalletDetail;

            return toContent(
              buildSuccess(
                wallet,
                `Wallet "${name}": ${wallet.addresses.length} address(es), ${wallet.holdings.length} asset(s).`
              )
            );
          }

          case "remove": {
            if (!name) {
              return toContent(
                buildError(
                  'name is required for action "remove"',
                  "MISSING_PARAM"
                )
              );
            }

            await runCliRaw(["wallet", "remove", name, "--yes"]);
            return toContent(
              buildSuccess(
                { name },
                `Wallet "${name}" removed successfully.`
              )
            );
          }
        }
      } catch (err) {
        return toContent(handleCliError(err, "cryptofolio_manage_wallet"));
      }
    }
  );
}

// ---------------------------------------------------------------------------
// cryptofolio_sync_wallet
// ---------------------------------------------------------------------------

export function registerSyncWalletTool(server: McpServer): void {
  server.tool(
    "cryptofolio_sync_wallet",
    "Sync on-chain balance and transaction history for one or all blockchain wallets. This calls external blockchain APIs (Blockfrost, Etherscan, etc.) and may take up to 2 minutes. Use sync_all=true to sync every wallet at once.",
    {
      wallet_name: z
        .string()
        .optional()
        .describe("Name of the wallet to sync. Omit if using sync_all."),
      sync_all: z
        .boolean()
        .optional()
        .describe("Sync all wallets (default: false)"),
      import_history: z
        .boolean()
        .optional()
        .describe(
          "Import full transaction history (slower, use on first sync). Default: false."
        ),
      use_local_node: z
        .boolean()
        .optional()
        .describe("Use configured local node instead of public API. Default: false."),
    },
    async ({ wallet_name, sync_all, import_history, use_local_node }) => {
      try {
        if (!wallet_name && !sync_all) {
          return toContent(
            buildError(
              "Provide either wallet_name or set sync_all to true.",
              "MISSING_PARAM"
            )
          );
        }

        const args = ["wallet", "sync"];
        if (wallet_name) args.push(wallet_name);
        if (sync_all === true) args.push("--all");
        if (import_history === true) args.push("--import-history");
        if (use_local_node === true) args.push("--use-local-node");

        // Use longer timeout — blockchain sync can be slow
        const raw = await runCli(args, SYNC_TIMEOUT_MS);

        return toContent(
          buildSuccess(
            raw ?? { synced: true },
            wallet_name
              ? `Wallet "${wallet_name}" synced successfully.`
              : "All wallets synced successfully."
          )
        );
      } catch (err) {
        return toContent(handleCliError(err, "cryptofolio_sync_wallet"));
      }
    }
  );
}
