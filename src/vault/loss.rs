use crate::amount::Amount;
use crate::ids::{Epoch, VaultId};
use crate::tranche::TrancheKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LossAllocation {
    pub tranche: TrancheKind,
    pub absorbed: Amount,
    pub reserve_after: Amount,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LossWindow {
    pub opened_epoch: Epoch,
    pub last_loss_epoch: Epoch,
    pub loss_total: Amount,
    pub absorbed_total: Amount,
    pub event_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LossReceipt {
    pub vault_id: VaultId,
    pub epoch: Epoch,
    pub requested_loss: Amount,
    pub absorbed_loss: Amount,
    pub reserve_before: Amount,
    pub reserve_after: Amount,
    pub allocations: Vec<LossAllocation>,
}

impl LossWindow {
    pub fn new(epoch: Epoch) -> Self {
        Self {
            opened_epoch: epoch,
            last_loss_epoch: epoch,
            loss_total: Amount::zero(),
            absorbed_total: Amount::zero(),
            event_count: 0,
        }
    }

    pub fn record(
        &mut self,
        epoch: Epoch,
        requested: Amount,
        absorbed: Amount,
    ) -> crate::StratumResult<()> {
        self.last_loss_epoch = epoch;
        self.loss_total = self.loss_total.checked_add(requested)?;
        self.absorbed_total = self.absorbed_total.checked_add(absorbed)?;
        self.event_count = self.event_count.saturating_add(1);
        Ok(())
    }

    pub fn clear(&mut self, epoch: Epoch) {
        *self = Self::new(epoch);
    }
}
