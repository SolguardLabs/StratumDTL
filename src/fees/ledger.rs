use crate::amount::Amount;
use crate::error::StratumResult;
use crate::fees::FeeAssessment;
use crate::ids::{AssetId, VaultId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeeLedger {
    protocol: BTreeMap<AssetId, Amount>,
    insurance: BTreeMap<AssetId, Amount>,
    operator: BTreeMap<VaultId, Amount>,
    total_assessed: Amount,
    assessment_count: u64,
}

impl FeeLedger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(
        &mut self,
        asset_id: AssetId,
        vault_id: VaultId,
        assessment: FeeAssessment,
    ) -> StratumResult<()> {
        let protocol = self
            .protocol
            .get(&asset_id)
            .copied()
            .unwrap_or_else(Amount::zero)
            .checked_add(assessment.protocol)?;
        self.protocol.insert(asset_id.clone(), protocol);

        let insurance = self
            .insurance
            .get(&asset_id)
            .copied()
            .unwrap_or_else(Amount::zero)
            .checked_add(assessment.insurance)?;
        self.insurance.insert(asset_id, insurance);

        let operator = self
            .operator
            .get(&vault_id)
            .copied()
            .unwrap_or_else(Amount::zero)
            .checked_add(assessment.operator)?;
        self.operator.insert(vault_id, operator);

        self.total_assessed = self.total_assessed.checked_add(assessment.fee)?;
        self.assessment_count = self.assessment_count.saturating_add(1);
        Ok(())
    }

    pub fn protocol_for(&self, asset_id: &AssetId) -> Amount {
        self.protocol
            .get(asset_id)
            .copied()
            .unwrap_or_else(Amount::zero)
    }

    pub fn insurance_for(&self, asset_id: &AssetId) -> Amount {
        self.insurance
            .get(asset_id)
            .copied()
            .unwrap_or_else(Amount::zero)
    }

    pub fn operator_for(&self, vault_id: &VaultId) -> Amount {
        self.operator
            .get(vault_id)
            .copied()
            .unwrap_or_else(Amount::zero)
    }

    pub const fn total_assessed(&self) -> Amount {
        self.total_assessed
    }

    pub const fn assessment_count(&self) -> u64 {
        self.assessment_count
    }
}
