use crate::amount::{Amount, ShareIndex, Shares};
use crate::ids::{AccountId, Epoch, VaultId};
use crate::tranche::TrancheKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RedemptionMode {
    StrictTranche,
    LayeredLiquidity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedemptionPlan {
    pub account: AccountId,
    pub vault_id: VaultId,
    pub tranche: TrancheKind,
    pub shares: Shares,
    pub quoted_gross: Amount,
    pub fee: Amount,
    pub quoted_net: Amount,
    pub epoch: Epoch,
    pub mode: RedemptionMode,
    pub reserve_basis: Amount,
    pub share_index_basis: ShareIndex,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedemptionReceipt {
    pub account: AccountId,
    pub vault_id: VaultId,
    pub tranche: TrancheKind,
    pub shares: Shares,
    pub gross_paid: Amount,
    pub net_paid: Amount,
    pub fee: Amount,
    pub paid_from: Vec<(TrancheKind, Amount)>,
    pub epoch: Epoch,
}

impl RedemptionReceipt {
    pub fn tranche_paid(&self, tranche: TrancheKind) -> Amount {
        self.paid_from
            .iter()
            .find_map(|(kind, amount)| (*kind == tranche).then_some(*amount))
            .unwrap_or_else(Amount::zero)
    }
}
