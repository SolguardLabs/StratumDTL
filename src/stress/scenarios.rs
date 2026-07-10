use crate::amount::{Amount, Bps};
use crate::snapshot::VaultSnapshot;
use crate::tranche::TrancheKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StressShock {
    pub tranche: Option<TrancheKind>,
    pub loss_bps: Bps,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StressScenario {
    pub name: String,
    pub shocks: Vec<StressShock>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StressResult {
    pub scenario: String,
    pub pre_total_reserve: Amount,
    pub post_total_reserve: Amount,
    pub losses: BTreeMap<TrancheKind, Amount>,
    pub reserves_after: BTreeMap<TrancheKind, Amount>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StressEngine;

impl StressShock {
    pub fn global(loss_bps: Bps) -> Self {
        Self {
            tranche: None,
            loss_bps,
        }
    }

    pub fn tranche(tranche: TrancheKind, loss_bps: Bps) -> Self {
        Self {
            tranche: Some(tranche),
            loss_bps,
        }
    }
}

impl StressScenario {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            shocks: Vec::new(),
        }
    }

    pub fn with_shock(mut self, shock: StressShock) -> Self {
        self.shocks.push(shock);
        self
    }

    pub fn junior_drawdown() -> Self {
        Self::new("junior-drawdown")
            .with_shock(StressShock::tranche(
                TrancheKind::Junior,
                Bps::new(4_000).expect("valid bps"),
            ))
            .with_shock(StressShock::global(Bps::new(500).expect("valid bps")))
    }

    pub fn insurance_event() -> Self {
        Self::new("insurance-event")
            .with_shock(StressShock::tranche(
                TrancheKind::Junior,
                Bps::new(10_000).expect("valid bps"),
            ))
            .with_shock(StressShock::tranche(
                TrancheKind::Insurance,
                Bps::new(2_000).expect("valid bps"),
            ))
    }
}

impl StressResult {
    pub fn total_loss(&self) -> crate::StratumResult<Amount> {
        self.losses
            .values()
            .try_fold(Amount::zero(), |acc, amount| acc.checked_add(*amount))
    }

    pub fn loss_for(&self, tranche: TrancheKind) -> Amount {
        self.losses
            .get(&tranche)
            .copied()
            .unwrap_or_else(Amount::zero)
    }

    pub fn reserve_after(&self, tranche: TrancheKind) -> Amount {
        self.reserves_after
            .get(&tranche)
            .copied()
            .unwrap_or_else(Amount::zero)
    }

    pub fn loss_bps_of_start(&self) -> crate::StratumResult<Bps> {
        if self.pre_total_reserve.is_zero() {
            return Ok(Bps::zero());
        }
        let loss = self.total_loss()?;
        let raw = loss.units().saturating_mul(10_000) / self.pre_total_reserve.units();
        Bps::new(raw.min(10_000) as u16)
    }

    pub fn is_senior_touched(&self) -> bool {
        !self.loss_for(TrancheKind::Senior).is_zero()
    }

    pub fn exhausted_tranches(&self) -> Vec<TrancheKind> {
        TrancheKind::public_order()
            .into_iter()
            .filter(|kind| self.reserve_after(*kind).is_zero() && !self.loss_for(*kind).is_zero())
            .collect()
    }
}

impl StressEngine {
    pub fn run(
        &self,
        snapshot: &VaultSnapshot,
        scenario: &StressScenario,
    ) -> crate::StratumResult<StressResult> {
        let mut reserves = BTreeMap::new();
        for kind in TrancheKind::public_order() {
            reserves.insert(
                kind,
                snapshot
                    .tranche(kind)
                    .map(|tranche| tranche.reserve)
                    .unwrap_or_else(Amount::zero),
            );
        }
        let mut losses = BTreeMap::new();
        for shock in &scenario.shocks {
            self.apply_shock(&mut reserves, &mut losses, *shock)?;
        }
        let post_total_reserve = reserves
            .values()
            .try_fold(Amount::zero(), |acc, reserve| acc.checked_add(*reserve))?;
        Ok(StressResult {
            scenario: scenario.name.clone(),
            pre_total_reserve: snapshot.total_reserve,
            post_total_reserve,
            losses,
            reserves_after: reserves,
        })
    }

    pub fn run_many(
        &self,
        snapshot: &VaultSnapshot,
        scenarios: &[StressScenario],
    ) -> crate::StratumResult<Vec<StressResult>> {
        scenarios
            .iter()
            .map(|scenario| self.run(snapshot, scenario))
            .collect()
    }

    fn apply_shock(
        &self,
        reserves: &mut BTreeMap<TrancheKind, Amount>,
        losses: &mut BTreeMap<TrancheKind, Amount>,
        shock: StressShock,
    ) -> crate::StratumResult<()> {
        match shock.tranche {
            Some(tranche) => self.apply_direct_shock(reserves, losses, tranche, shock.loss_bps),
            None => self.apply_global_shock(reserves, losses, shock.loss_bps),
        }
    }

    fn apply_direct_shock(
        &self,
        reserves: &mut BTreeMap<TrancheKind, Amount>,
        losses: &mut BTreeMap<TrancheKind, Amount>,
        tranche: TrancheKind,
        loss_bps: Bps,
    ) -> crate::StratumResult<()> {
        let current = reserves.get(&tranche).copied().unwrap_or_else(Amount::zero);
        let loss = current.bps(loss_bps)?;
        reserves.insert(tranche, current.saturating_sub(loss));
        let previous = losses.get(&tranche).copied().unwrap_or_else(Amount::zero);
        losses.insert(tranche, previous.checked_add(loss)?);
        Ok(())
    }

    fn apply_global_shock(
        &self,
        reserves: &mut BTreeMap<TrancheKind, Amount>,
        losses: &mut BTreeMap<TrancheKind, Amount>,
        loss_bps: Bps,
    ) -> crate::StratumResult<()> {
        let total = reserves
            .values()
            .try_fold(Amount::zero(), |acc, reserve| acc.checked_add(*reserve))?;
        let mut remaining = total.bps(loss_bps)?;
        for tranche in TrancheKind::default_loss_order() {
            if remaining.is_zero() {
                break;
            }
            let current = reserves.get(&tranche).copied().unwrap_or_else(Amount::zero);
            let absorbed = current.min(remaining);
            reserves.insert(tranche, current.saturating_sub(absorbed));
            let previous = losses.get(&tranche).copied().unwrap_or_else(Amount::zero);
            losses.insert(tranche, previous.checked_add(absorbed)?);
            remaining = remaining.checked_sub(absorbed)?;
        }
        Ok(())
    }
}
