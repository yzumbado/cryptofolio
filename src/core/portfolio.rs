use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::holdings::HoldingWithPrice;
use super::account::Category;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioEntry {
    pub account_id: String,
    pub account_name: String,
    pub category_id: String,
    pub category_name: String,
    pub holdings: Vec<HoldingWithPrice>,
}

impl PortfolioEntry {
    pub fn total_value(&self) -> Decimal {
        self.holdings
            .iter()
            .filter_map(|h| h.current_value)
            .sum()
    }

    pub fn total_cost_basis(&self) -> Decimal {
        self.holdings
            .iter()
            .filter_map(|h| h.holding.cost_basis_total())
            .sum()
    }

    pub fn total_unrealized_pnl(&self) -> Decimal {
        self.holdings
            .iter()
            .filter_map(|h| h.unrealized_pnl)
            .sum()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Portfolio {
    pub entries: Vec<PortfolioEntry>,
    pub total_value_usd: Decimal,
    pub total_cost_basis: Decimal,
    pub unrealized_pnl: Decimal,
    pub unrealized_pnl_percent: Decimal,
}

impl Portfolio {
    pub fn from_entries(entries: Vec<PortfolioEntry>) -> Self {
        let total_value_usd: Decimal = entries.iter().map(|e| e.total_value()).sum();
        let total_cost_basis: Decimal = entries.iter().map(|e| e.total_cost_basis()).sum();
        let unrealized_pnl = total_value_usd - total_cost_basis;
        let unrealized_pnl_percent = if total_cost_basis > Decimal::ZERO {
            (unrealized_pnl / total_cost_basis) * Decimal::from(100)
        } else {
            Decimal::ZERO
        };

        Self {
            entries,
            total_value_usd,
            total_cost_basis,
            unrealized_pnl,
            unrealized_pnl_percent,
        }
    }

    pub fn by_category(&self) -> Vec<CategorySummary> {
        use std::collections::HashMap;

        let mut categories: HashMap<String, CategorySummary> = HashMap::new();

        for entry in &self.entries {
            let summary = categories
                .entry(entry.category_id.clone())
                .or_insert_with(|| CategorySummary {
                    category_id: entry.category_id.clone(),
                    category_name: entry.category_name.clone(),
                    accounts: Vec::new(),
                    total_value: Decimal::ZERO,
                    total_cost_basis: Decimal::ZERO,
                });

            summary.accounts.push(entry.clone());
            summary.total_value += entry.total_value();
            summary.total_cost_basis += entry.total_cost_basis();
        }

        let mut result: Vec<_> = categories.into_values().collect();
        result.sort_by(|a, b| b.total_value.cmp(&a.total_value));
        result
    }

    pub fn asset_totals(&self) -> Vec<AssetTotal> {
        use std::collections::HashMap;

        let mut assets: HashMap<String, AssetTotal> = HashMap::new();

        for entry in &self.entries {
            for h in &entry.holdings {
                let total = assets
                    .entry(h.holding.asset.clone())
                    .or_insert_with(|| AssetTotal {
                        asset: h.holding.asset.clone(),
                        quantity: Decimal::ZERO,
                        value: Decimal::ZERO,
                        cost_basis: Decimal::ZERO,
                    });

                total.quantity += h.holding.quantity;
                if let Some(value) = h.current_value {
                    total.value += value;
                }
                if let Some(cost) = h.holding.cost_basis_total() {
                    total.cost_basis += cost;
                }
            }
        }

        let mut result: Vec<_> = assets.into_values().collect();
        result.sort_by(|a, b| b.value.cmp(&a.value));
        result
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategorySummary {
    pub category_id: String,
    pub category_name: String,
    pub accounts: Vec<PortfolioEntry>,
    pub total_value: Decimal,
    pub total_cost_basis: Decimal,
}

impl CategorySummary {
    pub fn unrealized_pnl(&self) -> Decimal {
        self.total_value - self.total_cost_basis
    }

    pub fn unrealized_pnl_percent(&self) -> Decimal {
        if self.total_cost_basis > Decimal::ZERO {
            (self.unrealized_pnl() / self.total_cost_basis) * Decimal::from(100)
        } else {
            Decimal::ZERO
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetTotal {
    pub asset: String,
    pub quantity: Decimal,
    pub value: Decimal,
    pub cost_basis: Decimal,
}

impl AssetTotal {
    pub fn unrealized_pnl(&self) -> Decimal {
        self.value - self.cost_basis
    }
}
