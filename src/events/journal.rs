use crate::amount::{Amount, Shares};
use crate::error::{StratumError, StratumResult};
use crate::ids::{AccountId, Epoch, EventId, TxId, VaultId};
use crate::tranche::TrancheKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventKind {
    VaultOpened,
    DepositAccepted,
    LossRecorded,
    IndexPublished,
    RedemptionSettled,
    FeesDistributed,
    RiskEvaluated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JournalEvent {
    VaultOpened {
        vault_id: VaultId,
    },
    DepositAccepted {
        account: AccountId,
        vault_id: VaultId,
        tranche: TrancheKind,
        gross: Amount,
        net: Amount,
        shares: Shares,
    },
    LossRecorded {
        vault_id: VaultId,
        loss: Amount,
        absorbed: Amount,
    },
    IndexPublished {
        vault_id: VaultId,
        epoch: Epoch,
    },
    RedemptionSettled {
        account: AccountId,
        vault_id: VaultId,
        tranche: TrancheKind,
        shares: Shares,
        gross: Amount,
        net: Amount,
    },
    FeesDistributed {
        vault_id: VaultId,
        amount: Amount,
    },
    RiskEvaluated {
        vault_id: VaultId,
        admitted: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalEntry {
    id: EventId,
    tx_id: TxId,
    epoch: Epoch,
    kind: EventKind,
    event: JournalEvent,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JournalRecord {
    entries: Vec<JournalEntry>,
    next_event: u64,
    next_tx: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JournalFilter {
    pub kind: Option<EventKind>,
    pub from_epoch: Option<Epoch>,
    pub limit: usize,
}

impl JournalEvent {
    pub const fn kind(&self) -> EventKind {
        match self {
            Self::VaultOpened { .. } => EventKind::VaultOpened,
            Self::DepositAccepted { .. } => EventKind::DepositAccepted,
            Self::LossRecorded { .. } => EventKind::LossRecorded,
            Self::IndexPublished { .. } => EventKind::IndexPublished,
            Self::RedemptionSettled { .. } => EventKind::RedemptionSettled,
            Self::FeesDistributed { .. } => EventKind::FeesDistributed,
            Self::RiskEvaluated { .. } => EventKind::RiskEvaluated,
        }
    }
}

impl JournalEntry {
    pub fn id(&self) -> &EventId {
        &self.id
    }

    pub fn tx_id(&self) -> &TxId {
        &self.tx_id
    }

    pub const fn epoch(&self) -> Epoch {
        self.epoch
    }

    pub const fn kind(&self) -> EventKind {
        self.kind
    }

    pub const fn event(&self) -> &JournalEvent {
        &self.event
    }
}

impl JournalRecord {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, epoch: Epoch, event: JournalEvent) -> JournalEntry {
        let id = EventId::unchecked(format!("event:{}", self.next_event));
        let tx_id = TxId::unchecked(format!("tx:{}", self.next_tx));
        self.next_event = self.next_event.saturating_add(1);
        self.next_tx = self.next_tx.saturating_add(1);
        let entry = JournalEntry {
            id,
            tx_id,
            epoch,
            kind: event.kind(),
            event,
        };
        self.entries.push(entry.clone());
        entry
    }

    pub fn entries(&self) -> &[JournalEntry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, index: usize) -> StratumResult<&JournalEntry> {
        self.entries.get(index).ok_or(StratumError::JournalCursor)
    }

    pub fn query(&self, filter: JournalFilter) -> Vec<&JournalEntry> {
        self.entries
            .iter()
            .filter(|entry| {
                filter.kind.is_none_or(|kind| entry.kind == kind)
                    && filter.from_epoch.is_none_or(|epoch| entry.epoch >= epoch)
            })
            .take(filter.limit)
            .collect()
    }
}

impl Default for JournalFilter {
    fn default() -> Self {
        Self {
            kind: None,
            from_epoch: None,
            limit: usize::MAX,
        }
    }
}
