use crate::amount::{Amount, ShareIndex, Shares};
use crate::ids::{Epoch, VaultId};
use crate::snapshot::VaultSnapshot;
use crate::tranche::TrancheKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrancheFrame {
    pub tranche: TrancheKind,
    pub reserve: Amount,
    pub shares: Shares,
    pub index: ShareIndex,
    pub pending_loss: Amount,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReserveFrame {
    pub vault_id: VaultId,
    pub epoch: Epoch,
    pub total_reserve: Amount,
    pub total_share_supply: Shares,
    pub pricing_basis: Amount,
    pub tranches: Vec<TrancheFrame>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HistorySeries {
    frames: Vec<ReserveFrame>,
}

impl TrancheFrame {
    pub fn reserve_delta(self, previous: Self) -> i128 {
        self.reserve.units() as i128 - previous.reserve.units() as i128
    }

    pub fn index_delta(self, previous: Self) -> i128 {
        self.index.raw() as i128 - previous.index.raw() as i128
    }
}

impl ReserveFrame {
    pub fn from_snapshot(snapshot: &VaultSnapshot) -> Self {
        let tranches = snapshot
            .tranches
            .iter()
            .map(|tranche| TrancheFrame {
                tranche: tranche.kind,
                reserve: tranche.reserve,
                shares: tranche.share_supply,
                index: tranche.share_index,
                pending_loss: tranche.pending_loss,
            })
            .collect();
        Self {
            vault_id: snapshot.id.clone(),
            epoch: snapshot.epoch,
            total_reserve: snapshot.total_reserve,
            total_share_supply: snapshot.total_share_supply,
            pricing_basis: snapshot.pricing_reserve_basis,
            tranches,
        }
    }

    pub fn tranche(&self, tranche: TrancheKind) -> Option<TrancheFrame> {
        self.tranches
            .iter()
            .copied()
            .find(|frame| frame.tranche == tranche)
    }

    pub fn total_pending_loss(&self) -> crate::StratumResult<Amount> {
        self.tranches.iter().try_fold(Amount::zero(), |acc, frame| {
            acc.checked_add(frame.pending_loss)
        })
    }

    pub fn pricing_gap(&self) -> i128 {
        self.pricing_basis.units() as i128 - self.total_reserve.units() as i128
    }
}

impl HistorySeries {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_snapshots(snapshots: &[VaultSnapshot]) -> Self {
        Self {
            frames: snapshots.iter().map(ReserveFrame::from_snapshot).collect(),
        }
    }

    pub fn push_snapshot(&mut self, snapshot: &VaultSnapshot) {
        self.frames.push(ReserveFrame::from_snapshot(snapshot));
    }

    pub fn push_frame(&mut self, frame: ReserveFrame) {
        self.frames.push(frame);
    }

    pub fn frames(&self) -> &[ReserveFrame] {
        &self.frames
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    pub fn latest(&self) -> Option<&ReserveFrame> {
        self.frames.last()
    }

    pub fn by_epoch(&self, epoch: Epoch) -> Option<&ReserveFrame> {
        self.frames.iter().find(|frame| frame.epoch == epoch)
    }

    pub fn reserve_path(&self, tranche: TrancheKind) -> Vec<(Epoch, Amount)> {
        self.frames
            .iter()
            .filter_map(|frame| {
                frame
                    .tranche(tranche)
                    .map(|tranche_frame| (frame.epoch, tranche_frame.reserve))
            })
            .collect()
    }

    pub fn index_path(&self, tranche: TrancheKind) -> Vec<(Epoch, ShareIndex)> {
        self.frames
            .iter()
            .filter_map(|frame| {
                frame
                    .tranche(tranche)
                    .map(|tranche_frame| (frame.epoch, tranche_frame.index))
            })
            .collect()
    }

    pub fn max_drawdown(&self, tranche: TrancheKind) -> Amount {
        let mut peak = Amount::zero();
        let mut max_drawdown = Amount::zero();
        for (_, reserve) in self.reserve_path(tranche) {
            if reserve > peak {
                peak = reserve;
            }
            let drawdown = peak.saturating_sub(reserve);
            if drawdown > max_drawdown {
                max_drawdown = drawdown;
            }
        }
        max_drawdown
    }

    pub fn total_reserve_delta(&self) -> Option<i128> {
        let first = self.frames.first()?;
        let last = self.frames.last()?;
        Some(last.total_reserve.units() as i128 - first.total_reserve.units() as i128)
    }

    pub fn tranche_reserve_delta(&self, tranche: TrancheKind) -> Option<i128> {
        let first = self
            .frames
            .iter()
            .find_map(|frame| frame.tranche(tranche).map(|item| item.reserve))?;
        let last = self
            .frames
            .iter()
            .rev()
            .find_map(|frame| frame.tranche(tranche).map(|item| item.reserve))?;
        Some(last.units() as i128 - first.units() as i128)
    }

    pub fn pricing_gap_path(&self) -> Vec<(Epoch, i128)> {
        self.frames
            .iter()
            .map(|frame| (frame.epoch, frame.pricing_gap()))
            .collect()
    }

    pub fn max_pricing_gap(&self) -> Option<i128> {
        self.pricing_gap_path()
            .into_iter()
            .map(|(_, gap)| gap)
            .max()
    }

    pub fn min_pricing_gap(&self) -> Option<i128> {
        self.pricing_gap_path()
            .into_iter()
            .map(|(_, gap)| gap)
            .min()
    }

    pub fn epochs(&self) -> Vec<Epoch> {
        self.frames.iter().map(|frame| frame.epoch).collect()
    }

    pub fn compact_since(&self, epoch: Epoch) -> Self {
        Self {
            frames: self
                .frames
                .iter()
                .filter(|frame| frame.epoch >= epoch)
                .cloned()
                .collect(),
        }
    }

    pub fn first_for_vault(&self, vault_id: &VaultId) -> Option<&ReserveFrame> {
        self.frames.iter().find(|frame| &frame.vault_id == vault_id)
    }

    pub fn latest_for_vault(&self, vault_id: &VaultId) -> Option<&ReserveFrame> {
        self.frames
            .iter()
            .rev()
            .find(|frame| &frame.vault_id == vault_id)
    }

    pub fn frame_count_for_vault(&self, vault_id: &VaultId) -> usize {
        self.frames
            .iter()
            .filter(|frame| &frame.vault_id == vault_id)
            .count()
    }

    pub fn grouped_by_vault(&self) -> BTreeMap<VaultId, Vec<ReserveFrame>> {
        let mut grouped = BTreeMap::new();
        for frame in &self.frames {
            grouped
                .entry(frame.vault_id.clone())
                .or_insert_with(Vec::new)
                .push(frame.clone());
        }
        grouped
    }
}
