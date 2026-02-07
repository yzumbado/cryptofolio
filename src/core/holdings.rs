use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Holding {
    pub id: i64,
    pub account_id: String,
    pub asset: String,
    pub quantity: Decimal,
    pub avg_cost_basis: Option<Decimal>,
    pub updated_at: DateTime<Utc>,
}

impl Holding {
    pub fn cost_basis_total(&self) -> Option<Decimal> {
        self.avg_cost_basis.map(|cost| cost * self.quantity)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoldingWithPrice {
    pub holding: Holding,
    pub current_price: Option<Decimal>,
    pub current_value: Option<Decimal>,
    pub unrealized_pnl: Option<Decimal>,
    pub unrealized_pnl_percent: Option<Decimal>,
}

impl HoldingWithPrice {
    pub fn from_holding(holding: Holding, current_price: Option<Decimal>) -> Self {
        let current_value = current_price.map(|p| p * holding.quantity);

        let (unrealized_pnl, unrealized_pnl_percent) = match (current_value, holding.cost_basis_total()) {
            (Some(value), Some(cost)) if cost > Decimal::ZERO => {
                let pnl = value - cost;
                let pnl_percent = (pnl / cost) * Decimal::from(100);
                (Some(pnl), Some(pnl_percent))
            }
            _ => (None, None),
        };

        Self {
            holding,
            current_price,
            current_value,
            unrealized_pnl,
            unrealized_pnl_percent,
        }
    }
}
