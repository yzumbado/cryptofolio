// Shared TypeScript interfaces for the cryptofolio MCP server.

// ---------------------------------------------------------------------------
// Universal MCP response envelope
// ---------------------------------------------------------------------------

export interface SuccessResponse<T = unknown> {
  success: true;
  data: T;
  message?: string;
}

export interface ErrorResponse {
  success: false;
  error: string;
  code?: string;
  hint?: string;
}

export type ToolResponse = SuccessResponse | ErrorResponse;

// ---------------------------------------------------------------------------
// CLI JSON output schemas (mirrors Rust structs)
// ---------------------------------------------------------------------------

// config show --json
export interface CliConfigOutput {
  general: {
    default_account: string | null;
    use_testnet: boolean;
    currency: string;
  };
  binance: {
    api_key_configured: boolean;
    api_secret_configured: boolean;
  };
  blockfrost: {
    mainnet_api_key_configured: boolean;
    preprod_api_key_configured: boolean;
    preview_api_key_configured: boolean;
  };
  display: {
    color: boolean;
    decimals: number;
    price_decimals: number;
    thousands_separator: boolean;
  };
  paths: {
    config_dir: string;
    database: string;
  };
}

// account list --json
export interface CliAccount {
  name: string;
  account_type: string;
  category: string;
  sync_enabled: boolean;
  is_testnet: boolean;
}

// portfolio --json
export interface CliPortfolioHolding {
  asset: string;
  quantity: string;
  current_price?: string;
  current_value?: string;
  cost_basis?: string;
  unrealized_pnl?: string;
  unrealized_pnl_percent?: string;
}

export interface CliPortfolioEntry {
  account_name: string;
  category_name: string;
  holdings: CliPortfolioHolding[];
}

export interface CliPortfolio {
  total_value_usd: string;
  total_cost_basis: string;
  unrealized_pnl: string;
  unrealized_pnl_percent: string;
  entries: CliPortfolioEntry[];
}

// price <SYMBOL...> --json
export interface CliPrice {
  symbol: string;
  price: string;
}

// market <SYMBOL> --json
export interface CliMarketOutput {
  symbol: string;
  base_asset: string;
  quote_asset: string;
  price: string;
  ticker_24h?: {
    price_change: string;
    price_change_percent: string;
    high_24h: string;
    low_24h: string;
    volume: string;
    quote_volume: string;
  };
}

// tx list --json
export interface CliTransaction {
  id: number;
  timestamp: string;
  tx_type: string;
  from_account_id?: string;
  to_account_id?: string;
  from_asset?: string;
  from_quantity?: string;
  to_asset?: string;
  to_quantity?: string;
  price_usd?: string;
  fee?: string;
  fee_asset?: string;
  external_id?: string;
  notes?: string;
}

// pnl summary --json
export interface CliPnlSummary {
  total_realized: string;
  total_unrealized: string;
  net_pnl: string;
  account?: string;
  from?: string;
  to?: string;
}

// pnl realized --json
export interface CliRealizedEntry {
  id: number;
  date: string;
  asset: string;
  account: string;
  quantity: string;
  cost_basis: string;
  proceeds: string;
  gain_loss: string;
  holding_period_days?: number;
}

// pnl unrealized --json
export interface CliUnrealizedEntry {
  asset: string;
  account: string;
  quantity: string;
  avg_cost_basis: string;
  current_price: string;
  current_value: string;
  unrealized_pnl: string;
}

// wallet list --json
export interface CliWalletAddress {
  blockchain: string;
  address: string;
  label?: string;
  xpub?: string;
}

export interface CliWallet {
  name: string;
  id: string;
  type: string;
  addresses: CliWalletAddress[];
}

// wallet show --json
export interface CliWalletDetail {
  name: string;
  id: string;
  type: string;
  created_at: string;
  addresses: Array<{
    blockchain: string;
    address: string;
    network: string;
  }>;
  holdings: Array<{
    asset: string;
    quantity: string;
  }>;
}

// wallet add --json
export interface CliWalletAddResult {
  success: boolean;
  wallet: string;
  blockchain: string;
  address?: string;
  xpub?: string;
  network: string;
  derived_addresses?: number;
}

// audit sync|coverage|errors --json
export interface CliAuditSyncEntry {
  timestamp: string;
  wallet: string;
  address: string;
  chain: string;
  provider: string;
  action: string;
  records_in?: number;
  records_new?: number;
  error?: string;
  duration_ms?: number;
}

export interface CliAuditCoverageEntry {
  wallet: string;
  chain: string;
  address: string;
  last_block?: number;
  last_sync_at?: string;
}

export interface CliAuditErrorEntry {
  timestamp: string;
  wallet: string;
  address: string;
  chain: string;
  provider: string;
  error: string;
}
