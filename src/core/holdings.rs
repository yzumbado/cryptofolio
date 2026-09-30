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
    pub cost_basis_currency: Option<String>, // Currency for avg_cost_basis
    pub avg_cost_basis_base: Option<Decimal>, // Cost basis in base currency (USD)
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
    /// DeFi classification of the holding's symbol (Plain / Supply / Debt).
    /// Debt positions carry a negative `current_value`.
    #[serde(default = "default_defi_kind")]
    pub defi_kind: crate::core::defi::DefiKind,
}

fn default_defi_kind() -> crate::core::defi::DefiKind {
    crate::core::defi::DefiKind::Plain
}

impl HoldingWithPrice {
    pub fn from_holding(holding: Holding, current_price: Option<Decimal>) -> Self {
        let current_value = current_price.map(|p| p * holding.quantity);

        let (unrealized_pnl, unrealized_pnl_percent) =
            match (current_value, holding.cost_basis_total()) {
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
            defi_kind: crate::core::defi::DefiKind::Plain,
        }
    }

    /// Build a holding valued through its DeFi classification. `underlying_price`
    /// is the USD price of the *underlying* asset (e.g. USDT for `aEthUSDT`,
    /// GHO for `variableDebtEthGHO`). Debt positions get a negative value so
    /// they subtract from net worth. aTokens track the underlying ~1:1 in token
    /// terms, so quantity × underlying price is the correct supplied value.
    pub fn from_holding_defi(
        holding: Holding,
        classified: &crate::core::defi::DefiAsset,
        underlying_price: Option<Decimal>,
    ) -> Self {
        use crate::core::defi::DefiKind;

        if classified.kind == DefiKind::Plain {
            return Self::from_holding(holding, underlying_price);
        }

        let sign = Decimal::from(classified.sign() as i64);
        let current_value = underlying_price.map(|p| sign * p * holding.quantity);

        // Cost basis / P&L are not meaningful for protocol receipt tokens in the
        // current model (they carry no acquisition cost), so leave them None and
        // report the position by value only.
        Self {
            holding,
            current_price: underlying_price,
            current_value,
            unrealized_pnl: None,
            unrealized_pnl_percent: None,
            defi_kind: classified.kind,
        }
    }
}
