use crate::amount::Amount;
use crate::events::{JournalEvent, JournalRecord};
use crate::ids::VaultId;
use crate::snapshot::VaultSnapshot;
use crate::tranche::TrancheKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReconciliationStatus {
    Matched,
    Drift,
    Incomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReserveMovement {
    pub tranche: TrancheKind,
    pub deposits: Amount,
    pub losses: Amount,
    pub redemptions: Amount,
    pub fee_credit: Amount,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconciliationReport {
    pub vault_id: VaultId,
    pub status: ReconciliationStatus,
    pub movements: Vec<ReserveMovement>,
    pub expected_total: Amount,
    pub snapshot_total: Amount,
    pub drift: i128,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ReconciliationEngine;

impl ReserveMovement {
    pub fn new(tranche: TrancheKind) -> Self {
        Self {
            tranche,
            deposits: Amount::zero(),
            losses: Amount::zero(),
            redemptions: Amount::zero(),
            fee_credit: Amount::zero(),
        }
    }

    pub fn net(self) -> crate::StratumResult<Amount> {
        let positive = self.deposits.checked_add(self.fee_credit)?;
        let negative = self.losses.checked_add(self.redemptions)?;
        Ok(positive.saturating_sub(negative))
    }

    pub fn record_deposit(&mut self, amount: Amount) -> crate::StratumResult<()> {
        self.deposits = self.deposits.checked_add(amount)?;
        Ok(())
    }

    pub fn record_loss(&mut self, amount: Amount) -> crate::StratumResult<()> {
        self.losses = self.losses.checked_add(amount)?;
        Ok(())
    }

    pub fn record_redemption(&mut self, amount: Amount) -> crate::StratumResult<()> {
        self.redemptions = self.redemptions.checked_add(amount)?;
        Ok(())
    }

    pub fn record_fee_credit(&mut self, amount: Amount) -> crate::StratumResult<()> {
        self.fee_credit = self.fee_credit.checked_add(amount)?;
        Ok(())
    }
}

impl ReconciliationReport {
    pub fn movement(&self, tranche: TrancheKind) -> Option<&ReserveMovement> {
        self.movements
            .iter()
            .find(|movement| movement.tranche == tranche)
    }

    pub fn is_matched(&self) -> bool {
        self.status == ReconciliationStatus::Matched
    }
}

impl ReconciliationEngine {
    pub fn reconcile(
        &self,
        snapshot: &VaultSnapshot,
        journal: &JournalRecord,
    ) -> crate::StratumResult<ReconciliationReport> {
        let mut buckets = self.empty_buckets();
        let mut saw_vault_open = false;
        for entry in journal.entries() {
            match entry.event() {
                JournalEvent::VaultOpened { vault_id } if vault_id == &snapshot.id => {
                    saw_vault_open = true;
                }
                JournalEvent::DepositAccepted {
                    vault_id,
                    tranche,
                    net,
                    ..
                } if vault_id == &snapshot.id => {
                    bucket_mut(&mut buckets, *tranche)?.record_deposit(*net)?;
                }
                JournalEvent::LossRecorded {
                    vault_id, absorbed, ..
                } if vault_id == &snapshot.id => {
                    self.assign_loss(snapshot, &mut buckets, *absorbed)?;
                }
                JournalEvent::RedemptionSettled {
                    vault_id,
                    tranche,
                    gross,
                    ..
                } if vault_id == &snapshot.id => {
                    bucket_mut(&mut buckets, *tranche)?.record_redemption(*gross)?;
                }
                JournalEvent::FeesDistributed { vault_id, amount } if vault_id == &snapshot.id => {
                    self.assign_fee_credit(snapshot, &mut buckets, *amount)?;
                }
                _ => {}
            }
        }
        let expected_total = buckets.values().try_fold(Amount::zero(), |acc, movement| {
            acc.checked_add(movement.net()?)
        })?;
        let drift = snapshot.total_reserve.units() as i128 - expected_total.units() as i128;
        let status = if !saw_vault_open {
            ReconciliationStatus::Incomplete
        } else if drift == 0 {
            ReconciliationStatus::Matched
        } else {
            ReconciliationStatus::Drift
        };
        Ok(ReconciliationReport {
            vault_id: snapshot.id.clone(),
            status,
            movements: buckets.values().copied().collect(),
            expected_total,
            snapshot_total: snapshot.total_reserve,
            drift,
        })
    }

    fn empty_buckets(&self) -> BTreeMap<TrancheKind, ReserveMovement> {
        TrancheKind::public_order()
            .into_iter()
            .map(|kind| (kind, ReserveMovement::new(kind)))
            .collect()
    }

    fn assign_loss(
        &self,
        snapshot: &VaultSnapshot,
        buckets: &mut BTreeMap<TrancheKind, ReserveMovement>,
        amount: Amount,
    ) -> crate::StratumResult<()> {
        let mut remaining = amount;
        for kind in TrancheKind::default_loss_order() {
            if remaining.is_zero() {
                break;
            }
            let reserve = snapshot
                .tranche(kind)
                .map(|tranche| tranche.reserve)
                .unwrap_or_else(Amount::zero);
            let assigned = reserve.min(remaining);
            if !assigned.is_zero() {
                bucket_mut(buckets, kind)?.record_loss(assigned)?;
                remaining = remaining.checked_sub(assigned)?;
            }
        }
        Ok(())
    }

    fn assign_fee_credit(
        &self,
        snapshot: &VaultSnapshot,
        buckets: &mut BTreeMap<TrancheKind, ReserveMovement>,
        amount: Amount,
    ) -> crate::StratumResult<()> {
        if snapshot.total_reserve.is_zero() {
            return Ok(());
        }
        let mut assigned_total = Amount::zero();
        for kind in TrancheKind::public_order() {
            let reserve = snapshot
                .tranche(kind)
                .map(|tranche| tranche.reserve)
                .unwrap_or_else(Amount::zero);
            let raw =
                amount.units().saturating_mul(reserve.units()) / snapshot.total_reserve.units();
            let credit = Amount::from_units(raw);
            assigned_total = assigned_total.checked_add(credit)?;
            bucket_mut(buckets, kind)?.record_fee_credit(credit)?;
        }
        if assigned_total < amount {
            let remainder = amount.checked_sub(assigned_total)?;
            bucket_mut(buckets, TrancheKind::Overflow)?.record_fee_credit(remainder)?;
        }
        Ok(())
    }
}

fn bucket_mut(
    buckets: &mut BTreeMap<TrancheKind, ReserveMovement>,
    tranche: TrancheKind,
) -> crate::StratumResult<&mut ReserveMovement> {
    Ok(buckets
        .entry(tranche)
        .or_insert_with(|| ReserveMovement::new(tranche)))
}
