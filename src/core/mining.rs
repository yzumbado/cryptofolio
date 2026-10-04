//! DePIN mining accounting: capital-asset depreciation + earned-token income.
//!
//! See `docs/MINING_ASSET_ACCOUNTING.md` for the model. In short:
//!
//! * **Mining hardware** (the `MINER-*` pseudo-assets in the `DePIN Hardware`
//!   account) is a capital asset carried at **net book value** (NBV): its cost
//!   less straight-line depreciation over a useful life. It is NEVER priced by a
//!   market feed.
//! * **Earned tokens** (GEOD, WINGS, …) are **revenue at $0 cost basis** — every
//!   reward is income at the fair value on its receipt date.
//! * The two meet only in a **mining P&L**: `revenue − depreciation − opex`.
//!
//! This module is pure arithmetic over parameters the caller supplies (it reads
//! no DB and makes no network call), so it is trivially testable. The depreciation
//! parameters live in the ledger as a `correction` row whose `notes` carry a JSON
//! blob (`DEPRECIATION <miner>: {..}`); the caller parses that and hands the
//! fields here.

use chrono::{DateTime, Datelike, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Straight-line depreciation parameters for one mining rig.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepreciationParams {
    /// Acquisition cost in USD.
    pub cost_usd: Decimal,
    /// Useful life in months (e.g. 60 for 5-year straight-line).
    pub useful_life_months: u32,
    /// Date the asset was placed in service (depreciation starts here).
    pub in_service_date: DateTime<Utc>,
    /// Residual value at end of life (default $0).
    #[serde(default)]
    pub salvage_usd: Decimal,
}

/// A point-in-time depreciation reading for one rig.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepreciationState {
    /// Whole months the asset has been in service as of the valuation date.
    pub months_in_service: u32,
    /// Accumulated depreciation to date (never exceeds cost − salvage).
    pub accumulated: Decimal,
    /// Net book value = cost − accumulated depreciation (floored at salvage).
    pub net_book_value: Decimal,
}

/// Whole months between two instants, flooring partial months (an asset in
/// service for 29 days has depreciated 0 full months). Never negative.
fn months_between(from: DateTime<Utc>, to: DateTime<Utc>) -> u32 {
    if to <= from {
        return 0;
    }
    let mut months = (to.year() - from.year()) * 12 + (to.month() as i32 - from.month() as i32);
    // If the day-of-month hasn't been reached yet, the final month isn't complete.
    if to.day() < from.day() {
        months -= 1;
    }
    months.max(0) as u32
}

/// Compute straight-line depreciation for one rig as of `as_of`.
///
/// Monthly depreciation is `(cost − salvage) / useful_life_months`; accumulated
/// depreciation is capped at `cost − salvage` once the asset is fully
/// depreciated, so NBV never falls below salvage.
pub fn depreciation_at(params: &DepreciationParams, as_of: DateTime<Utc>) -> DepreciationState {
    let months = months_between(params.in_service_date, as_of);
    let depreciable = (params.cost_usd - params.salvage_usd).max(Decimal::ZERO);

    let accumulated = if params.useful_life_months == 0 {
        depreciable // degenerate: treat as immediately fully depreciated
    } else {
        let per_month = depreciable / Decimal::from(params.useful_life_months);
        (per_month * Decimal::from(months)).min(depreciable)
    };

    DepreciationState {
        months_in_service: months,
        accumulated,
        net_book_value: params.cost_usd - accumulated,
    }
}

/// A single dated reward credit (one on-chain daily distribution).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewardEvent {
    pub date: DateTime<Utc>,
    pub asset: String,
    /// Token amount received.
    pub quantity: Decimal,
    /// USD fair value of this reward on its receipt date (price × quantity).
    pub fmv_usd: Decimal,
}

/// How a mining P&L's revenue figure was derived.
///
/// Reported explicitly so an approximation is never silently substituted for
/// the dated stream: `DatedRewards` means the ledger had dated reward rows;
/// `CurrentFmv` means the current-value fallback was used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevenueBasis {
    /// Sum of dated reward FMVs read from the ledger.
    DatedRewards,
    /// Current held FMV — the fallback when no dated stream exists.
    CurrentFmv,
}

impl RevenueBasis {
    pub fn as_str(&self) -> &'static str {
        match self {
            RevenueBasis::DatedRewards => "dated_rewards",
            RevenueBasis::CurrentFmv => "current_fmv",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "dated_rewards" => Some(RevenueBasis::DatedRewards),
            "current_fmv" => Some(RevenueBasis::CurrentFmv),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            RevenueBasis::DatedRewards => "Dated on-chain rewards",
            RevenueBasis::CurrentFmv => "Current FMV approximation",
        }
    }
}

/// Total USD revenue from a dated reward stream (sum of each reward's FMV).
pub fn dated_reward_revenue(events: &[RewardEvent]) -> Decimal {
    events.iter().map(|e| e.fmv_usd).sum()
}

/// Pick mining revenue and report which basis was used.
///
/// The dated stream wins whenever it has any events; otherwise fall back to
/// `current_fmv` (the approximation that values currently-held earned tokens at
/// today's price). Returns `(revenue_usd, basis)`.
pub fn select_revenue(events: &[RewardEvent], current_fmv: Decimal) -> (Decimal, RevenueBasis) {
    if events.is_empty() {
        (current_fmv, RevenueBasis::CurrentFmv)
    } else {
        (dated_reward_revenue(events), RevenueBasis::DatedRewards)
    }
}

/// The mining P&L for a reporting window (or cumulative).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MiningPnl {
    /// Token income at fair value on receipt (sum of reward FMVs).
    pub revenue_usd: Decimal,
    /// Accumulated hardware depreciation over all rigs.
    pub depreciation_usd: Decimal,
    /// Operating expenses (power/internet). $0 when no incremental cost.
    pub opex_usd: Decimal,
    /// revenue − depreciation − opex.
    pub operating_profit_usd: Decimal,
    /// Hardware cost across all rigs.
    pub hardware_cost_usd: Decimal,
    /// Net book value across all rigs.
    pub net_book_value_usd: Decimal,
    /// Fraction of hardware cost recovered by token income to date (0..1+).
    pub capital_recovered_fraction: Decimal,
}

/// Assemble a cumulative mining P&L from reward income, per-rig depreciation and
/// optional opex. `revenue_usd` is the sum of reward FMVs; pass the current held
/// FMV when a dated reward stream is unavailable.
pub fn mining_pnl(
    revenue_usd: Decimal,
    depreciation_usd: Decimal,
    hardware_cost_usd: Decimal,
    opex_usd: Decimal,
) -> MiningPnl {
    let net_book_value_usd = (hardware_cost_usd - depreciation_usd).max(Decimal::ZERO);
    let capital_recovered_fraction = if hardware_cost_usd.is_zero() {
        Decimal::ZERO
    } else {
        revenue_usd / hardware_cost_usd
    };
    MiningPnl {
        revenue_usd,
        depreciation_usd,
        opex_usd,
        operating_profit_usd: revenue_usd - depreciation_usd - opex_usd,
        hardware_cost_usd,
        net_book_value_usd,
        capital_recovered_fraction,
    }
}

/// Whether a holding symbol is a mining-hardware pseudo-asset (`MINER-*`), which
/// must be carried at NBV and never market-priced.
pub fn is_mining_hardware(symbol: &str) -> bool {
    symbol.to_uppercase().starts_with("MINER-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::str::FromStr;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn d(y: i32, m: u32, day: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, day, 0, 0, 0).unwrap()
    }

    #[test]
    fn months_between_floors_partial_months() {
        assert_eq!(months_between(d(2026, 5, 2), d(2026, 10, 3)), 5);
        assert_eq!(months_between(d(2026, 5, 2), d(2026, 5, 30)), 0); // <1 month
        assert_eq!(months_between(d(2026, 5, 2), d(2026, 6, 1)), 0); // day not reached
        assert_eq!(months_between(d(2026, 5, 2), d(2026, 6, 2)), 1);
        assert_eq!(months_between(d(2026, 10, 3), d(2026, 5, 2)), 0); // future in-service
    }

    #[test]
    fn straight_line_depreciation_5yr() {
        let p = DepreciationParams {
            cost_usd: dec("1300"),
            useful_life_months: 60,
            in_service_date: d(2026, 5, 2),
            salvage_usd: dec("0"),
        };
        let s = depreciation_at(&p, d(2026, 10, 3));
        assert_eq!(s.months_in_service, 5);
        // 1300/60 * 5 = 108.333...
        assert!((s.accumulated - dec("108.333333")).abs() < dec("0.01"));
        assert!((s.net_book_value - dec("1191.67")).abs() < dec("0.01"));
    }

    #[test]
    fn depreciation_caps_at_cost_after_useful_life() {
        let p = DepreciationParams {
            cost_usd: dec("1300"),
            useful_life_months: 60,
            in_service_date: d(2020, 1, 1),
            salvage_usd: dec("0"),
        };
        let s = depreciation_at(&p, d(2030, 1, 1)); // 10 years > 5-year life
        assert_eq!(s.accumulated, dec("1300"));
        assert_eq!(s.net_book_value, dec("0"));
    }

    #[test]
    fn mining_pnl_profitable_when_revenue_exceeds_depreciation() {
        let pnl = mining_pnl(dec("671.59"), dec("216.67"), dec("2600"), dec("0"));
        assert_eq!(pnl.operating_profit_usd, dec("454.92"));
        assert_eq!(pnl.net_book_value_usd, dec("2383.33"));
        // 671.59 / 2600 = 0.2583...
        assert!((pnl.capital_recovered_fraction - dec("0.2583")).abs() < dec("0.001"));
    }

    #[test]
    fn miner_symbols_are_hardware() {
        assert!(is_mining_hardware("MINER-GEODNET-LOC1"));
        assert!(is_mining_hardware("miner-wingbits-loc2"));
        assert!(!is_mining_hardware("GEOD"));
        assert!(!is_mining_hardware("BTC"));
    }

    fn reward(asset: &str, quantity: &str, fmv: &str) -> RewardEvent {
        RewardEvent {
            date: d(2026, 9, 1),
            asset: asset.to_string(),
            quantity: dec(quantity),
            fmv_usd: dec(fmv),
        }
    }

    #[test]
    fn revenue_prefers_the_dated_stream_when_present() {
        let events = vec![
            reward("GEOD", "10", "0.50"),
            reward("GEOD", "10", "0.75"),
            reward("WINGS", "5", "1.25"),
        ];
        let (revenue, basis) = select_revenue(&events, dec("999.99"));
        assert_eq!(basis, RevenueBasis::DatedRewards);
        assert_eq!(revenue, dec("2.50"));
        assert_eq!(dated_reward_revenue(&events), dec("2.50"));
    }

    #[test]
    fn revenue_falls_back_to_current_fmv_without_dated_events() {
        let (revenue, basis) = select_revenue(&[], dec("671.59"));
        assert_eq!(basis, RevenueBasis::CurrentFmv);
        assert_eq!(revenue, dec("671.59"));
    }

    #[test]
    fn revenue_basis_round_trips_through_str() {
        for basis in [RevenueBasis::DatedRewards, RevenueBasis::CurrentFmv] {
            assert_eq!(RevenueBasis::from_str(basis.as_str()), Some(basis));
        }
        assert_eq!(RevenueBasis::from_str("nonsense"), None);
    }
}
