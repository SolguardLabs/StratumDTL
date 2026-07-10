use crate::amount::{Amount, Shares};
use crate::error::{StratumError, StratumResult};
use crate::ids::{AccountId, VaultId};
use crate::tranche::TrancheKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PositionStatus {
    Active,
    Frozen,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PositionDelta {
    pub vault_id: VaultId,
    pub tranche: TrancheKind,
    pub shares_delta: i128,
    pub amount_delta: i128,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountPosition {
    vault_id: VaultId,
    tranche: TrancheKind,
    shares: Shares,
    principal: Amount,
    redeemed: Amount,
    status: PositionStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountState {
    id: AccountId,
    positions: BTreeMap<(VaultId, TrancheKind), AccountPosition>,
    received_amount: Amount,
    paid_fees: Amount,
    deposit_count: u64,
    redemption_count: u64,
}

impl AccountPosition {
    pub fn new(vault_id: VaultId, tranche: TrancheKind) -> Self {
        Self {
            vault_id,
            tranche,
            shares: Shares::zero(),
            principal: Amount::zero(),
            redeemed: Amount::zero(),
            status: PositionStatus::Active,
        }
    }

    pub fn vault_id(&self) -> &VaultId {
        &self.vault_id
    }

    pub const fn tranche(&self) -> TrancheKind {
        self.tranche
    }

    pub const fn shares(&self) -> Shares {
        self.shares
    }

    pub const fn principal(&self) -> Amount {
        self.principal
    }

    pub const fn redeemed(&self) -> Amount {
        self.redeemed
    }

    pub const fn status(&self) -> PositionStatus {
        self.status
    }

    pub fn credit(&mut self, shares: Shares, principal: Amount) -> StratumResult<()> {
        if self.status != PositionStatus::Active {
            return Err(StratumError::RiskRejected);
        }
        self.shares = self.shares.checked_add(shares)?;
        self.principal = self.principal.checked_add(principal)?;
        Ok(())
    }

    pub fn debit(&mut self, shares: Shares, received: Amount) -> StratumResult<()> {
        if self.status != PositionStatus::Active {
            return Err(StratumError::RiskRejected);
        }
        self.shares = self.shares.checked_sub(shares)?;
        self.redeemed = self.redeemed.checked_add(received)?;
        if self.shares.is_zero() {
            self.status = PositionStatus::Closed;
        }
        Ok(())
    }

    pub fn freeze(&mut self) {
        if self.status == PositionStatus::Active {
            self.status = PositionStatus::Frozen;
        }
    }

    pub fn reactivate(&mut self) {
        if self.status == PositionStatus::Frozen {
            self.status = PositionStatus::Active;
        }
    }
}

impl AccountState {
    pub fn new(id: AccountId) -> Self {
        Self {
            id,
            positions: BTreeMap::new(),
            received_amount: Amount::zero(),
            paid_fees: Amount::zero(),
            deposit_count: 0,
            redemption_count: 0,
        }
    }

    pub fn id(&self) -> &AccountId {
        &self.id
    }

    pub const fn received_amount(&self) -> Amount {
        self.received_amount
    }

    pub const fn paid_fees(&self) -> Amount {
        self.paid_fees
    }

    pub const fn deposit_count(&self) -> u64 {
        self.deposit_count
    }

    pub const fn redemption_count(&self) -> u64 {
        self.redemption_count
    }

    pub fn position(&self, vault_id: &VaultId, tranche: TrancheKind) -> Option<&AccountPosition> {
        self.positions.get(&(vault_id.clone(), tranche))
    }

    pub fn position_mut(
        &mut self,
        vault_id: &VaultId,
        tranche: TrancheKind,
    ) -> &mut AccountPosition {
        self.positions
            .entry((vault_id.clone(), tranche))
            .or_insert_with(|| AccountPosition::new(vault_id.clone(), tranche))
    }

    pub fn credit_deposit(
        &mut self,
        vault_id: VaultId,
        tranche: TrancheKind,
        shares: Shares,
        principal: Amount,
        fee: Amount,
    ) -> StratumResult<PositionDelta> {
        let position = self.position_mut(&vault_id, tranche);
        position.credit(shares, principal)?;
        self.paid_fees = self.paid_fees.checked_add(fee)?;
        self.deposit_count = self.deposit_count.saturating_add(1);
        Ok(PositionDelta {
            vault_id,
            tranche,
            shares_delta: shares.units() as i128,
            amount_delta: principal.units() as i128,
        })
    }

    pub fn debit_redemption(
        &mut self,
        vault_id: VaultId,
        tranche: TrancheKind,
        shares: Shares,
        received: Amount,
        fee: Amount,
    ) -> StratumResult<PositionDelta> {
        let position = self
            .positions
            .get_mut(&(vault_id.clone(), tranche))
            .ok_or(StratumError::InsufficientShares)?;
        position.debit(shares, received)?;
        self.received_amount = self.received_amount.checked_add(received)?;
        self.paid_fees = self.paid_fees.checked_add(fee)?;
        self.redemption_count = self.redemption_count.saturating_add(1);
        Ok(PositionDelta {
            vault_id,
            tranche,
            shares_delta: -(shares.units() as i128),
            amount_delta: received.units() as i128,
        })
    }

    pub fn total_shares_for(&self, vault_id: &VaultId, tranche: TrancheKind) -> Shares {
        self.position(vault_id, tranche)
            .map(AccountPosition::shares)
            .unwrap_or_else(Shares::zero)
    }

    pub fn positions(&self) -> impl Iterator<Item = &AccountPosition> {
        self.positions.values()
    }
}
