use crate::amount::{Amount, Bps};
use crate::error::{StratumError, StratumResult};
use crate::tranche::TrancheKind;
use crate::vault::{RedemptionPlan, VaultState};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SolvencyBand {
    Healthy,
    Watch,
    Restricted,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskDecision {
    Admit,
    Review,
    Reject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiskLimits {
    pub min_total_reserve: Amount,
    pub min_senior_ratio_bps: Bps,
    pub max_loss_window_bps: Bps,
    pub max_single_redemption_bps: Bps,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiskReport {
    pub total_reserve: Amount,
    pub senior_reserve: Amount,
    pub pending_loss: Amount,
    pub senior_ratio_bps: Bps,
    pub pending_loss_bps: Bps,
    pub band: SolvencyBand,
    pub decision: RiskDecision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiskEngine {
    limits: RiskLimits,
}

impl RiskLimits {
    pub fn conservative() -> Self {
        Self {
            min_total_reserve: Amount::from_units(100),
            min_senior_ratio_bps: Bps::new(2_500).expect("valid bps"),
            max_loss_window_bps: Bps::new(4_000).expect("valid bps"),
            max_single_redemption_bps: Bps::new(5_000).expect("valid bps"),
        }
    }
}

impl Default for RiskLimits {
    fn default() -> Self {
        Self::conservative()
    }
}

impl RiskEngine {
    pub fn new(limits: RiskLimits) -> Self {
        Self { limits }
    }

    pub const fn limits(&self) -> RiskLimits {
        self.limits
    }

    pub fn evaluate(&self, vault: &VaultState) -> StratumResult<RiskReport> {
        let total_reserve = vault.total_reserve()?;
        let senior_reserve = vault.tranche(TrancheKind::Senior)?.reserve();
        let pending_loss = vault.loss_window().absorbed_total;
        let senior_ratio_bps = ratio_bps(senior_reserve, total_reserve)?;
        let pending_loss_bps = ratio_bps(pending_loss, total_reserve.checked_add(pending_loss)?)?;
        let band = self.band(total_reserve, senior_ratio_bps, pending_loss_bps);
        let decision = match band {
            SolvencyBand::Healthy => RiskDecision::Admit,
            SolvencyBand::Watch => RiskDecision::Review,
            SolvencyBand::Restricted | SolvencyBand::Closed => RiskDecision::Reject,
        };
        Ok(RiskReport {
            total_reserve,
            senior_reserve,
            pending_loss,
            senior_ratio_bps,
            pending_loss_bps,
            band,
            decision,
        })
    }

    pub fn admit_deposit(&self, vault: &VaultState, amount: Amount) -> StratumResult<RiskDecision> {
        let projected = vault.total_reserve()?.checked_add(amount)?;
        if projected < self.limits.min_total_reserve {
            Ok(RiskDecision::Review)
        } else {
            Ok(RiskDecision::Admit)
        }
    }

    pub fn admit_redemption(
        &self,
        vault: &VaultState,
        plan: &RedemptionPlan,
    ) -> StratumResult<RiskDecision> {
        let total = vault.pricing_view().total_reserve_basis();
        let limit = total.bps(self.limits.max_single_redemption_bps)?;
        if plan.quoted_gross > limit && !limit.is_zero() {
            return Ok(RiskDecision::Reject);
        }
        let remaining = vault.total_reserve()?.saturating_sub(plan.quoted_gross);
        if remaining < self.limits.min_total_reserve {
            return Ok(RiskDecision::Review);
        }
        Ok(self.evaluate(vault)?.decision)
    }

    fn band(
        &self,
        total_reserve: Amount,
        senior_ratio_bps: Bps,
        pending_loss_bps: Bps,
    ) -> SolvencyBand {
        if total_reserve < self.limits.min_total_reserve {
            return SolvencyBand::Closed;
        }
        if pending_loss_bps > self.limits.max_loss_window_bps {
            return SolvencyBand::Restricted;
        }
        if senior_ratio_bps < self.limits.min_senior_ratio_bps {
            return SolvencyBand::Watch;
        }
        SolvencyBand::Healthy
    }
}

impl Default for RiskEngine {
    fn default() -> Self {
        Self::new(RiskLimits::default())
    }
}

fn ratio_bps(part: Amount, total: Amount) -> StratumResult<Bps> {
    if total.is_zero() {
        return Ok(Bps::zero());
    }
    let raw = part
        .units()
        .checked_mul(10_000)
        .ok_or(StratumError::AmountOverflow)?
        .checked_div(total.units())
        .ok_or(StratumError::AmountOverflow)?;
    Bps::new(raw.min(10_000) as u16)
}
