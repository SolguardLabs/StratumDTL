use crate::amount::{Amount, Bps};
use crate::error::{StratumError, StratumResult};
use crate::tranche::{LossPriority, TrancheKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranchePolicy {
    target_weights: BTreeMap<TrancheKind, Bps>,
    liquidity_floors: BTreeMap<TrancheKind, Bps>,
    max_redemption_bps: BTreeMap<TrancheKind, Bps>,
    loss_priority: LossPriority,
    auto_publish_loss_indices: bool,
}

impl TranchePolicy {
    pub fn new(
        target_weights: BTreeMap<TrancheKind, Bps>,
        liquidity_floors: BTreeMap<TrancheKind, Bps>,
        max_redemption_bps: BTreeMap<TrancheKind, Bps>,
        loss_priority: LossPriority,
    ) -> StratumResult<Self> {
        validate_weight_map(&target_weights)?;
        Ok(Self {
            target_weights,
            liquidity_floors,
            max_redemption_bps,
            loss_priority,
            auto_publish_loss_indices: false,
        })
    }

    pub fn balanced() -> Self {
        let mut target_weights = BTreeMap::new();
        target_weights.insert(TrancheKind::Senior, Bps::new(5_500).expect("valid bps"));
        target_weights.insert(TrancheKind::Junior, Bps::new(2_500).expect("valid bps"));
        target_weights.insert(TrancheKind::Insurance, Bps::new(1_500).expect("valid bps"));
        target_weights.insert(TrancheKind::Overflow, Bps::new(500).expect("valid bps"));

        let mut floors = BTreeMap::new();
        floors.insert(TrancheKind::Senior, Bps::new(3_500).expect("valid bps"));
        floors.insert(TrancheKind::Junior, Bps::new(500).expect("valid bps"));
        floors.insert(TrancheKind::Insurance, Bps::new(300).expect("valid bps"));
        floors.insert(TrancheKind::Overflow, Bps::zero());

        let mut max_redemption = BTreeMap::new();
        max_redemption.insert(TrancheKind::Senior, Bps::new(2_000).expect("valid bps"));
        max_redemption.insert(TrancheKind::Junior, Bps::new(4_000).expect("valid bps"));
        max_redemption.insert(TrancheKind::Insurance, Bps::new(2_500).expect("valid bps"));
        max_redemption.insert(TrancheKind::Overflow, Bps::new(7_500).expect("valid bps"));

        Self {
            target_weights,
            liquidity_floors: floors,
            max_redemption_bps: max_redemption,
            loss_priority: LossPriority::standard(),
            auto_publish_loss_indices: false,
        }
    }

    pub fn with_auto_publish_loss_indices(mut self, enabled: bool) -> Self {
        self.auto_publish_loss_indices = enabled;
        self
    }

    pub fn target_weight(&self, kind: TrancheKind) -> Bps {
        self.target_weights
            .get(&kind)
            .copied()
            .unwrap_or_else(Bps::zero)
    }

    pub fn liquidity_floor(&self, kind: TrancheKind) -> Bps {
        self.liquidity_floors
            .get(&kind)
            .copied()
            .unwrap_or_else(Bps::zero)
    }

    pub fn max_redemption_bps(&self, kind: TrancheKind) -> Bps {
        self.max_redemption_bps
            .get(&kind)
            .copied()
            .unwrap_or_else(Bps::full)
    }

    pub fn loss_priority(&self) -> &LossPriority {
        &self.loss_priority
    }

    pub const fn auto_publish_loss_indices(&self) -> bool {
        self.auto_publish_loss_indices
    }

    pub fn target_amounts(&self, total: Amount) -> StratumResult<BTreeMap<TrancheKind, Amount>> {
        let mut result = BTreeMap::new();
        let mut allocated = Amount::zero();
        for kind in TrancheKind::public_order() {
            let amount = total.bps(self.target_weight(kind))?;
            allocated = allocated.checked_add(amount)?;
            result.insert(kind, amount);
        }
        if allocated < total {
            let current = result
                .get(&TrancheKind::Overflow)
                .copied()
                .unwrap_or_else(Amount::zero);
            result.insert(
                TrancheKind::Overflow,
                current.checked_add(total.checked_sub(allocated)?)?,
            );
        }
        Ok(result)
    }

    pub fn required_floor(
        &self,
        kind: TrancheKind,
        total_reserve: Amount,
    ) -> StratumResult<Amount> {
        total_reserve.bps(self.liquidity_floor(kind))
    }

    pub fn redemption_limit(&self, kind: TrancheKind, reserve: Amount) -> StratumResult<Amount> {
        reserve.bps(self.max_redemption_bps(kind))
    }
}

impl Default for TranchePolicy {
    fn default() -> Self {
        Self::balanced()
    }
}

fn validate_weight_map(map: &BTreeMap<TrancheKind, Bps>) -> StratumResult<()> {
    let sum: u32 = TrancheKind::public_order()
        .iter()
        .map(|kind| u32::from(map.get(kind).copied().unwrap_or_else(Bps::zero).value()))
        .sum();
    if sum == 10_000 {
        Ok(())
    } else {
        Err(StratumError::InvalidTranchePolicy)
    }
}
