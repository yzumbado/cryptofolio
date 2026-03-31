#![allow(dead_code)]

pub mod calculator;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

pub use calculator::PnLCalculator;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[derive(Default)]
pub enum CostBasisMethod {
    /// First In, First Out
    #[default]
    Fifo,
    /// Last In, First Out
    Lifo,
    /// Average cost of all purchases
    AverageCost,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PnLSummary {
    pub total_realized: Decimal,
    pub total_unrealized: Decimal,
    pub total_fees: Decimal,
    pub net_pnl: Decimal,
}

impl PnLSummary {
    pub fn new() -> Self {
        Self {
            total_realized: Decimal::ZERO,
            total_unrealized: Decimal::ZERO,
            total_fees: Decimal::ZERO,
            net_pnl: Decimal::ZERO,
        }
    }

    pub fn calculate_net(&mut self) {
        self.net_pnl = self.total_realized + self.total_unrealized - self.total_fees;
    }
}

impl Default for PnLSummary {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a tax lot (cost basis) for a specific purchase of an asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxLot {
    pub id: i64,
    pub account_id: String,
    pub asset: String,
    pub quantity: Decimal,
    pub remaining_quantity: Decimal,
    pub acquisition_price: Decimal,
    pub acquisition_date: DateTime<Utc>,
    pub acquisition_tx_id: Option<i64>,
    pub cost_basis_method: CostBasisMethod,
    pub fully_disposed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Represents a realized profit or loss from disposing (selling) an asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealizedPnL {
    pub id: i64,
    pub account_id: String,
    pub asset: String,
    pub disposal_date: DateTime<Utc>,
    pub disposal_tx_id: Option<i64>,
    pub quantity: Decimal,
    pub proceeds: Decimal,
    pub cost_basis: Decimal,
    pub realized_gain: Decimal, // proceeds - cost_basis
    pub holding_period_days: Option<i64>,
    pub tax_lot_id: Option<i64>,
    pub cost_basis_method: CostBasisMethod,
    pub created_at: DateTime<Utc>,
}

/// Internal result from matching a disposal to tax lots
#[derive(Debug, Clone)]
pub struct DisposalMatch {
    pub tax_lot_id: i64,
    pub quantity: Decimal,
    pub cost_basis: Decimal,
    pub acquisition_date: DateTime<Utc>,
}
