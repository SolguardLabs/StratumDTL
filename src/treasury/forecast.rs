use crate::amount::{Amount, Bps};
use crate::error::{StratumError, StratumResult};
use crate::snapshot::VaultSnapshot;
use crate::stress::{StressEngine, StressScenario};
use crate::tranche::{TrancheKind, TranchePolicy};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commitment {
    pub tranche: TrancheKind,
    pub due: Amount,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrancheCoverage {
    pub tranche: TrancheKind,
    pub stressed_reserve: Amount,
    pub policy_floor: Amount,
    pub available: Amount,
    pub committed: Amount,
    pub surplus: Amount,
    pub shortfall: Amount,
    pub coverage_bps: Bps,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiquidityForecast {
    pub scenario: String,
    pub starting_reserve: Amount,
    pub stressed_reserve: Amount,
    pub total_commitments: Amount,
    pub total_available: Amount,
    pub total_shortfall: Amount,
    pub tranches: Vec<TrancheCoverage>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LiquidityForecaster;

impl LiquidityForecaster {
    pub fn evaluate(
        &self,
        snapshot: &VaultSnapshot,
        policy: &TranchePolicy,
        scenario: &StressScenario,
        commitments: &[Commitment],
    ) -> StratumResult<LiquidityForecast> {
        let stress = StressEngine.run(snapshot, scenario)?;
        let mut due_by_tranche = BTreeMap::new();
        for item in commitments {
            let prior = due_by_tranche
                .get(&item.tranche)
                .copied()
                .unwrap_or_else(Amount::zero);
            due_by_tranche.insert(item.tranche, prior.checked_add(item.due)?);
        }

        let mut tranches = Vec::new();
        let mut total_commitments = Amount::zero();
        let mut total_available = Amount::zero();
        let mut total_shortfall = Amount::zero();
        for kind in TrancheKind::public_order() {
            let reserve = stress.reserve_after(kind);
            let floor = policy.required_floor(kind, stress.post_total_reserve)?;
            let available = reserve.saturating_sub(floor);
            let due = due_by_tranche
                .get(&kind)
                .copied()
                .unwrap_or_else(Amount::zero);
            let surplus = available.saturating_sub(due);
            let shortfall = due.saturating_sub(available);
            let coverage_bps = if due.is_zero() {
                Bps::full()
            } else {
                let raw = available
                    .units()
                    .checked_mul(10_000)
                    .ok_or(StratumError::AmountOverflow)?
                    / due.units();
                Bps::new(raw.min(10_000) as u16)?
            };
            total_commitments = total_commitments.checked_add(due)?;
            total_available = total_available.checked_add(available)?;
            total_shortfall = total_shortfall.checked_add(shortfall)?;
            tranches.push(TrancheCoverage {
                tranche: kind,
                stressed_reserve: reserve,
                policy_floor: floor,
                available,
                committed: due,
                surplus,
                shortfall,
                coverage_bps,
            });
        }
        Ok(LiquidityForecast {
            scenario: scenario.name.clone(),
            starting_reserve: stress.pre_total_reserve,
            stressed_reserve: stress.post_total_reserve,
            total_commitments,
            total_available,
            total_shortfall,
            tranches,
        })
    }
}

impl LiquidityForecast {
    pub fn fully_covered(&self) -> bool {
        self.total_shortfall.is_zero()
    }

    pub fn tranche(&self, kind: TrancheKind) -> Option<&TrancheCoverage> {
        self.tranches.iter().find(|line| line.tranche == kind)
    }
}
