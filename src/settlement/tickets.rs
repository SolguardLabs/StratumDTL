use crate::amount::{Amount, Shares};
use crate::error::{StratumError, StratumResult};
use crate::ids::{AccountId, Epoch, TxId, VaultId};
use crate::tranche::TrancheKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettlementStatus {
    Pending,
    Admitted,
    Settled,
    Rejected,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepositTicket {
    pub tx_id: TxId,
    pub account: AccountId,
    pub vault_id: VaultId,
    pub tranche: TrancheKind,
    pub amount: Amount,
    pub epoch: Epoch,
    pub status: SettlementStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedemptionTicket {
    pub tx_id: TxId,
    pub account: AccountId,
    pub vault_id: VaultId,
    pub tranche: TrancheKind,
    pub shares: Shares,
    pub epoch: Epoch,
    pub status: SettlementStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettlementInstruction {
    Deposit(DepositTicket),
    Redemption(RedemptionTicket),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TicketBook {
    tickets: BTreeMap<TxId, SettlementInstruction>,
    sequence: u64,
}

impl DepositTicket {
    pub fn new(
        tx_id: TxId,
        account: AccountId,
        vault_id: VaultId,
        tranche: TrancheKind,
        amount: Amount,
        epoch: Epoch,
    ) -> Self {
        Self {
            tx_id,
            account,
            vault_id,
            tranche,
            amount,
            epoch,
            status: SettlementStatus::Pending,
        }
    }

    pub fn with_status(mut self, status: SettlementStatus) -> Self {
        self.status = status;
        self
    }
}

impl RedemptionTicket {
    pub fn new(
        tx_id: TxId,
        account: AccountId,
        vault_id: VaultId,
        tranche: TrancheKind,
        shares: Shares,
        epoch: Epoch,
    ) -> Self {
        Self {
            tx_id,
            account,
            vault_id,
            tranche,
            shares,
            epoch,
            status: SettlementStatus::Pending,
        }
    }

    pub fn with_status(mut self, status: SettlementStatus) -> Self {
        self.status = status;
        self
    }
}

impl SettlementInstruction {
    pub fn tx_id(&self) -> &TxId {
        match self {
            Self::Deposit(ticket) => &ticket.tx_id,
            Self::Redemption(ticket) => &ticket.tx_id,
        }
    }

    pub const fn status(&self) -> SettlementStatus {
        match self {
            Self::Deposit(ticket) => ticket.status,
            Self::Redemption(ticket) => ticket.status,
        }
    }

    pub fn account(&self) -> &AccountId {
        match self {
            Self::Deposit(ticket) => &ticket.account,
            Self::Redemption(ticket) => &ticket.account,
        }
    }

    pub fn vault_id(&self) -> &VaultId {
        match self {
            Self::Deposit(ticket) => &ticket.vault_id,
            Self::Redemption(ticket) => &ticket.vault_id,
        }
    }

    pub const fn tranche(&self) -> TrancheKind {
        match self {
            Self::Deposit(ticket) => ticket.tranche,
            Self::Redemption(ticket) => ticket.tranche,
        }
    }

    pub fn set_status(&mut self, status: SettlementStatus) {
        match self {
            Self::Deposit(ticket) => ticket.status = status,
            Self::Redemption(ticket) => ticket.status = status,
        }
    }
}

impl TicketBook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn next_tx(&mut self) -> TxId {
        let tx = TxId::unchecked(format!("ticket:{}", self.sequence));
        self.sequence = self.sequence.saturating_add(1);
        tx
    }

    pub fn insert(&mut self, instruction: SettlementInstruction) -> Option<SettlementInstruction> {
        self.tickets
            .insert(instruction.tx_id().clone(), instruction)
    }

    pub fn get(&self, tx_id: &TxId) -> StratumResult<&SettlementInstruction> {
        self.tickets
            .get(tx_id)
            .ok_or(StratumError::InvalidIdentifier)
    }

    pub fn get_mut(&mut self, tx_id: &TxId) -> StratumResult<&mut SettlementInstruction> {
        self.tickets
            .get_mut(tx_id)
            .ok_or(StratumError::InvalidIdentifier)
    }

    pub fn set_status(&mut self, tx_id: &TxId, status: SettlementStatus) -> StratumResult<()> {
        self.get_mut(tx_id)?.set_status(status);
        Ok(())
    }

    pub fn pending_for_vault(&self, vault_id: &VaultId) -> Vec<&SettlementInstruction> {
        self.tickets
            .values()
            .filter(|instruction| {
                instruction.vault_id() == vault_id
                    && matches!(
                        instruction.status(),
                        SettlementStatus::Pending | SettlementStatus::Admitted
                    )
            })
            .collect()
    }

    pub fn by_account(&self, account: &AccountId) -> Vec<&SettlementInstruction> {
        self.tickets
            .values()
            .filter(|instruction| instruction.account() == account)
            .collect()
    }

    pub fn len(&self) -> usize {
        self.tickets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tickets.is_empty()
    }
}
