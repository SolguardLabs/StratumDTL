use crate::accounts::AccountState;
use crate::amount::{Amount, ShareIndex, Shares};
use crate::ids::{AccountId, AssetId, Epoch, VaultId};
use crate::tranche::TrancheKind;
use crate::vault::VaultState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrancheSnapshot {
    pub kind: TrancheKind,
    pub reserve: Amount,
    pub share_supply: Shares,
    pub share_index: ShareIndex,
    pub pending_loss: Amount,
    pub accrued_fee_credit: Amount,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultSnapshot {
    pub id: VaultId,
    pub asset_id: AssetId,
    pub epoch: Epoch,
    pub total_reserve: Amount,
    pub total_share_supply: Shares,
    pub total_deposited: Amount,
    pub total_redeemed: Amount,
    pub total_losses: Amount,
    pub total_fees: Amount,
    pub pricing_reserve_basis: Amount,
    pub tranches: Vec<TrancheSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountSnapshot {
    pub id: AccountId,
    pub received_amount: Amount,
    pub paid_fees: Amount,
    pub deposit_count: u64,
    pub redemption_count: u64,
    pub positions: Vec<(VaultId, TrancheKind, Shares, Amount, Amount)>,
}

impl TrancheSnapshot {
    pub fn from_state(state: &crate::tranche::TrancheState) -> Self {
        Self {
            kind: state.kind(),
            reserve: state.reserve(),
            share_supply: state.share_supply(),
            share_index: state.share_index(),
            pending_loss: state.pending_loss(),
            accrued_fee_credit: state.accrued_fee_credit(),
        }
    }
}

impl VaultSnapshot {
    pub fn from_state(vault: &VaultState) -> crate::StratumResult<Self> {
        let tranches = vault.tranches().map(TrancheSnapshot::from_state).collect();
        Ok(Self {
            id: vault.id().clone(),
            asset_id: vault.asset_id().clone(),
            epoch: vault.epoch(),
            total_reserve: vault.total_reserve()?,
            total_share_supply: vault.total_share_supply()?,
            total_deposited: vault.total_deposited(),
            total_redeemed: vault.total_redeemed(),
            total_losses: vault.total_losses(),
            total_fees: vault.total_fees(),
            pricing_reserve_basis: vault.pricing_view().total_reserve_basis(),
            tranches,
        })
    }

    pub fn tranche(&self, kind: TrancheKind) -> Option<&TrancheSnapshot> {
        self.tranches.iter().find(|snapshot| snapshot.kind == kind)
    }
}

impl AccountSnapshot {
    pub fn from_state(account: &AccountState) -> Self {
        let positions = account
            .positions()
            .map(|position| {
                (
                    position.vault_id().clone(),
                    position.tranche(),
                    position.shares(),
                    position.principal(),
                    position.redeemed(),
                )
            })
            .collect();
        Self {
            id: account.id().clone(),
            received_amount: account.received_amount(),
            paid_fees: account.paid_fees(),
            deposit_count: account.deposit_count(),
            redemption_count: account.redemption_count(),
            positions,
        }
    }
}
