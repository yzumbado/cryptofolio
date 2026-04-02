/**
 * Cryptofolio MCP Server — entry point
 *
 * Registers all 18 tools and connects via stdio transport.
 * Run with: node dist/index.js
 * Or in dev: tsx src/index.ts
 *
 * Required env var:
 *   CRYPTOFOLIO_BIN  — path to the cryptofolio binary (falls back to PATH)
 */

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";

// Tool registrations
import { registerStatusTool } from "./tools/status.js";
import {
  registerListAccountsTool,
  registerManageAccountTool,
} from "./tools/accounts.js";
import { registerPortfolioTool } from "./tools/portfolio.js";
import {
  registerGetPricesTool,
  registerGetMarketDataTool,
} from "./tools/prices.js";
import {
  registerListTransactionsTool,
  registerRecordTransactionTool,
  registerTrackConversionTool,
  registerExportTransactionsTool,
} from "./tools/transactions.js";
import {
  registerManageWalletTool,
  registerSyncWalletTool,
} from "./tools/wallets.js";
import {
  registerGetPnlSummaryTool,
  registerGetRealizedPnlTool,
  registerGetUnrealizedPnlTool,
  registerAnalyzeAssetTool,
} from "./tools/pnl.js";
import { registerSyncExchangeTool } from "./tools/sync.js";
import { registerAuditLogTool } from "./tools/audit.js";

// ---------------------------------------------------------------------------
// Server setup
// ---------------------------------------------------------------------------

const server = new McpServer({
  name: "cryptofolio",
  version: "0.1.0",
});

// Phase 1 — MVP (query + basic write)
registerStatusTool(server);           // cryptofolio_get_system_status
registerListAccountsTool(server);     // cryptofolio_list_accounts
registerPortfolioTool(server);        // cryptofolio_get_portfolio
registerGetPricesTool(server);        // cryptofolio_get_prices
registerListTransactionsTool(server); // cryptofolio_list_transactions
registerRecordTransactionTool(server); // cryptofolio_record_transaction

// Phase 2 — Onboarding (full LLM-only setup)
registerManageAccountTool(server);   // cryptofolio_manage_account
registerManageWalletTool(server);    // cryptofolio_manage_wallet
registerSyncWalletTool(server);      // cryptofolio_sync_wallet
registerSyncExchangeTool(server);    // cryptofolio_sync_exchange
registerTrackConversionTool(server); // cryptofolio_track_conversion

// Phase 3 — P&L and tax reporting
registerGetPnlSummaryTool(server);   // cryptofolio_get_pnl_summary
registerGetRealizedPnlTool(server);  // cryptofolio_get_realized_pnl
registerGetUnrealizedPnlTool(server); // cryptofolio_get_unrealized_pnl
registerAnalyzeAssetTool(server);    // cryptofolio_analyze_asset
registerExportTransactionsTool(server); // cryptofolio_export_transactions

// Phase 4 — Audit and market data
registerAuditLogTool(server);        // cryptofolio_get_audit_log
registerGetMarketDataTool(server);   // cryptofolio_get_market_data

// ---------------------------------------------------------------------------
// Start
// ---------------------------------------------------------------------------

const transport = new StdioServerTransport();
await server.connect(transport);
