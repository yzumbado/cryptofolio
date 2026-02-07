use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum CostBasisMethod {
    /// First In, First Out
    Fifo,
    /// Last In, First Out
    Lifo,
    /// Average cost of all purchases
    AverageCost,
}

impl Default for CostBasisMethod {
    fn default() -> Self {
        CostBasisMethod::Fifo
    }
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
