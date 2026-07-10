use crate::amount::{Amount, Bps};
use crate::error::{StratumError, StratumResult};
use crate::tranche::{TrancheKind, TranchePolicy};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllocationIntent {
    pub total: Amount,
    pub weights: BTreeMap<TrancheKind, Bps>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllocationLine {
    pub tranche: TrancheKind,
    pub weight: Bps,
    pub amount: Amount,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllocationPlan {
    pub total: Amount,
    pub lines: Vec<AllocationLine>,
    pub remainder: Amount,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllocationVariance {
    pub tranche: TrancheKind,
    pub current: Amount,
    pub planned: Amount,
    pub delta: i128,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllocationPlanner {
    policy: TranchePolicy,
}

impl AllocationIntent {
    pub fn new(total: Amount, weights: BTreeMap<TrancheKind, Bps>) -> StratumResult<Self> {
        let sum: u32 = weights.values().map(|bps| u32::from(bps.value())).sum();
        if sum != 10_000 {
            return Err(StratumError::InvalidTranchePolicy);
        }
        Ok(Self { total, weights })
    }

    pub fn from_policy(total: Amount, policy: &TranchePolicy) -> Self {
        let mut weights = BTreeMap::new();
        for kind in TrancheKind::public_order() {
            weights.insert(kind, policy.target_weight(kind));
        }
        Self { total, weights }
    }

    pub fn weight(&self, tranche: TrancheKind) -> Bps {
        self.weights
            .get(&tranche)
            .copied()
            .unwrap_or_else(Bps::zero)
    }
}

impl AllocationPlan {
    pub fn from_lines(total: Amount, lines: Vec<AllocationLine>) -> StratumResult<Self> {
        let allocated = lines
            .iter()
            .try_fold(Amount::zero(), |acc, line| acc.checked_add(line.amount))?;
        let remainder = if allocated < total {
            total.checked_sub(allocated)?
        } else {
            Amount::zero()
        };
        Ok(Self {
            total,
            lines,
            remainder,
        })
    }

    pub fn amount_for(&self, tranche: TrancheKind) -> Amount {
        self.lines
            .iter()
            .find_map(|line| (line.tranche == tranche).then_some(line.amount))
            .unwrap_or_else(Amount::zero)
    }

    pub fn weight_for(&self, tranche: TrancheKind) -> Bps {
        self.lines
            .iter()
            .find_map(|line| (line.tranche == tranche).then_some(line.weight))
            .unwrap_or_else(Bps::zero)
    }

    pub fn allocated_total(&self) -> StratumResult<Amount> {
        self.lines
            .iter()
            .try_fold(Amount::zero(), |acc, line| acc.checked_add(line.amount))
    }

    pub fn is_fully_allocated(&self) -> bool {
        self.remainder.is_zero()
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn validate(&self) -> StratumResult<()> {
        let allocated = self.allocated_total()?;
        if allocated > self.total {
            return Err(StratumError::InvalidTranchePolicy);
        }
        let weight_sum: u32 = self
            .lines
            .iter()
            .map(|line| u32::from(line.weight.value()))
            .sum();
        if weight_sum != 10_000 {
            return Err(StratumError::InvalidTranchePolicy);
        }
        Ok(())
    }

    pub fn variances(&self, current: &BTreeMap<TrancheKind, Amount>) -> Vec<AllocationVariance> {
        self.lines
            .iter()
            .map(|line| {
                let current_amount = current
                    .get(&line.tranche)
                    .copied()
                    .unwrap_or_else(Amount::zero);
                AllocationVariance {
                    tranche: line.tranche,
                    current: current_amount,
                    planned: line.amount,
                    delta: current_amount.units() as i128 - line.amount.units() as i128,
                }
            })
            .collect()
    }

    pub fn scale(&self, new_total: Amount) -> StratumResult<Self> {
        let mut lines = Vec::with_capacity(self.lines.len());
        for line in &self.lines {
            lines.push(AllocationLine {
                tranche: line.tranche,
                weight: line.weight,
                amount: new_total.bps(line.weight)?,
            });
        }
        Self::from_lines(new_total, lines)
    }
}

impl AllocationPlanner {
    pub fn new(policy: TranchePolicy) -> Self {
        Self { policy }
    }

    pub fn policy(&self) -> &TranchePolicy {
        &self.policy
    }

    pub fn plan_policy_allocation(&self, total: Amount) -> StratumResult<AllocationPlan> {
        let intent = AllocationIntent::from_policy(total, &self.policy);
        self.plan(intent)
    }

    pub fn plan(&self, intent: AllocationIntent) -> StratumResult<AllocationPlan> {
        let mut lines = Vec::new();
        let mut allocated = Amount::zero();
        for kind in TrancheKind::public_order() {
            let weight = intent.weight(kind);
            let amount = intent.total.bps(weight)?;
            allocated = allocated.checked_add(amount)?;
            lines.push(AllocationLine {
                tranche: kind,
                weight,
                amount,
            });
        }
        let remainder = if allocated < intent.total {
            intent.total.checked_sub(allocated)?
        } else {
            Amount::zero()
        };
        if !remainder.is_zero()
            && let Some(line) = lines
                .iter_mut()
                .find(|line| line.tranche == TrancheKind::Overflow)
        {
            line.amount = line.amount.checked_add(remainder)?;
        }
        Ok(AllocationPlan {
            total: intent.total,
            lines,
            remainder,
        })
    }

    pub fn normalize_weights(
        &self,
        raw_weights: BTreeMap<TrancheKind, u128>,
    ) -> StratumResult<BTreeMap<TrancheKind, Bps>> {
        let denominator: u128 = raw_weights.values().copied().sum();
        if denominator == 0 {
            return Err(StratumError::InvalidTranchePolicy);
        }
        let mut normalized = BTreeMap::new();
        let mut used: u16 = 0;
        for kind in TrancheKind::public_order() {
            let raw = raw_weights.get(&kind).copied().unwrap_or_default();
            let value = ((raw.saturating_mul(10_000)) / denominator).min(10_000) as u16;
            normalized.insert(kind, Bps::new(value)?);
            used = used.saturating_add(value);
        }
        if used < 10_000 {
            let overflow = normalized
                .get(&TrancheKind::Overflow)
                .copied()
                .unwrap_or_else(Bps::zero);
            normalized.insert(
                TrancheKind::Overflow,
                Bps::new(overflow.value().saturating_add(10_000 - used))?,
            );
        }
        Ok(normalized)
    }

    pub fn plan_from_raw_weights(
        &self,
        total: Amount,
        raw_weights: BTreeMap<TrancheKind, u128>,
    ) -> StratumResult<AllocationPlan> {
        let weights = self.normalize_weights(raw_weights)?;
        self.plan(AllocationIntent::new(total, weights)?)
    }

    pub fn equal_weight_plan(&self, total: Amount) -> StratumResult<AllocationPlan> {
        let mut weights = BTreeMap::new();
        weights.insert(TrancheKind::Senior, Bps::new(2_500)?);
        weights.insert(TrancheKind::Junior, Bps::new(2_500)?);
        weights.insert(TrancheKind::Insurance, Bps::new(2_500)?);
        weights.insert(TrancheKind::Overflow, Bps::new(2_500)?);
        self.plan(AllocationIntent::new(total, weights)?)
    }
}

impl Default for AllocationPlanner {
    fn default() -> Self {
        Self::new(TranchePolicy::balanced())
    }
}
