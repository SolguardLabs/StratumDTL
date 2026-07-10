use crate::amount::{Amount, Bps};
use crate::error::{StratumError, StratumResult};
use crate::tranche::TrancheKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeePolicy {
    pub deposit_bps: Bps,
    pub redemption_bps: Bps,
    pub performance_bps: Bps,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeeSplit {
    pub protocol_bps: Bps,
    pub insurance_bps: Bps,
    pub operator_bps: Bps,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeeSchedule {
    default_policy: FeePolicy,
    tranche_policy: BTreeMap<TrancheKind, FeePolicy>,
    split: FeeSplit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeeAssessment {
    pub gross: Amount,
    pub fee: Amount,
    pub net: Amount,
    pub protocol: Amount,
    pub insurance: Amount,
    pub operator: Amount,
}

impl FeePolicy {
    pub fn new(deposit_bps: Bps, redemption_bps: Bps, performance_bps: Bps) -> Self {
        Self {
            deposit_bps,
            redemption_bps,
            performance_bps,
        }
    }

    pub fn conservative() -> Self {
        Self {
            deposit_bps: Bps::new(15).expect("valid bps"),
            redemption_bps: Bps::new(20).expect("valid bps"),
            performance_bps: Bps::new(100).expect("valid bps"),
        }
    }
}

impl Default for FeePolicy {
    fn default() -> Self {
        Self::conservative()
    }
}

impl FeeSplit {
    pub fn new(protocol_bps: Bps, insurance_bps: Bps, operator_bps: Bps) -> StratumResult<Self> {
        let sum = u32::from(protocol_bps.value())
            + u32::from(insurance_bps.value())
            + u32::from(operator_bps.value());
        if sum != 10_000 {
            return Err(StratumError::InvalidFeeSplit);
        }
        Ok(Self {
            protocol_bps,
            insurance_bps,
            operator_bps,
        })
    }

    pub fn standard() -> Self {
        Self {
            protocol_bps: Bps::new(4_000).expect("valid bps"),
            insurance_bps: Bps::new(4_000).expect("valid bps"),
            operator_bps: Bps::new(2_000).expect("valid bps"),
        }
    }
}

impl Default for FeeSplit {
    fn default() -> Self {
        Self::standard()
    }
}

impl FeeSchedule {
    pub fn new(default_policy: FeePolicy, split: FeeSplit) -> Self {
        Self {
            default_policy,
            tranche_policy: BTreeMap::new(),
            split,
        }
    }

    pub fn with_tranche_policy(mut self, tranche: TrancheKind, policy: FeePolicy) -> Self {
        self.tranche_policy.insert(tranche, policy);
        self
    }

    pub fn policy_for(&self, tranche: TrancheKind) -> FeePolicy {
        self.tranche_policy
            .get(&tranche)
            .copied()
            .unwrap_or(self.default_policy)
    }

    pub const fn split(&self) -> FeeSplit {
        self.split
    }

    pub fn assess_deposit(
        &self,
        tranche: TrancheKind,
        gross: Amount,
    ) -> StratumResult<FeeAssessment> {
        let policy = self.policy_for(tranche);
        self.assess(gross, policy.deposit_bps)
    }

    pub fn assess_redemption(
        &self,
        tranche: TrancheKind,
        gross: Amount,
    ) -> StratumResult<FeeAssessment> {
        let policy = self.policy_for(tranche);
        self.assess(gross, policy.redemption_bps)
    }

    pub fn assess_performance(
        &self,
        tranche: TrancheKind,
        gross: Amount,
    ) -> StratumResult<FeeAssessment> {
        let policy = self.policy_for(tranche);
        self.assess(gross, policy.performance_bps)
    }

    fn assess(&self, gross: Amount, fee_bps: Bps) -> StratumResult<FeeAssessment> {
        let fee = gross.bps(fee_bps)?;
        let net = gross.checked_sub(fee)?;
        let protocol = fee.bps(self.split.protocol_bps)?;
        let insurance = fee.bps(self.split.insurance_bps)?;
        let operator = fee.checked_sub(protocol)?.checked_sub(insurance)?;
        Ok(FeeAssessment {
            gross,
            fee,
            net,
            protocol,
            insurance,
            operator,
        })
    }
}

impl Default for FeeSchedule {
    fn default() -> Self {
        Self::new(FeePolicy::default(), FeeSplit::default())
    }
}
