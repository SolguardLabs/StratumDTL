use crate::amount::{Amount, ShareIndex, Shares};
use crate::error::{StratumError, StratumResult};
use crate::fees::{FeeAssessment, FeeSchedule};
use crate::ids::{AccountId, AssetId, Epoch, VaultId};
use crate::tranche::{TrancheConfig, TrancheKind, TranchePolicy, TrancheState};
use crate::vault::{
    LossAllocation, LossReceipt, LossWindow, RedemptionMode, RedemptionPlan, RedemptionReceipt,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultConfig {
    id: VaultId,
    asset_id: AssetId,
    controller: AccountId,
    policy: TranchePolicy,
    fee_schedule: FeeSchedule,
    redemption_mode: RedemptionMode,
    paused: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReservePricingView {
    epoch: Epoch,
    total_reserve_basis: Amount,
    total_share_basis: Shares,
    tranche_reserve_basis: BTreeMap<TrancheKind, Amount>,
    tranche_index_basis: BTreeMap<TrancheKind, ShareIndex>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepositReceipt {
    pub account: AccountId,
    pub vault_id: VaultId,
    pub tranche: TrancheKind,
    pub gross: Amount,
    pub fee: Amount,
    pub net: Amount,
    pub shares: Shares,
    pub epoch: Epoch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VaultState {
    config: VaultConfig,
    tranches: BTreeMap<TrancheKind, TrancheState>,
    epoch: Epoch,
    pricing_view: ReservePricingView,
    loss_window: LossWindow,
    total_deposited: Amount,
    total_redeemed: Amount,
    total_losses: Amount,
    total_fees: Amount,
}

impl VaultConfig {
    pub fn new(
        id: VaultId,
        asset_id: AssetId,
        controller: AccountId,
        policy: TranchePolicy,
        fee_schedule: FeeSchedule,
    ) -> Self {
        Self {
            id,
            asset_id,
            controller,
            policy,
            fee_schedule,
            redemption_mode: RedemptionMode::LayeredLiquidity,
            paused: false,
        }
    }

    pub fn with_redemption_mode(mut self, mode: RedemptionMode) -> Self {
        self.redemption_mode = mode;
        self
    }

    pub fn with_paused(mut self, paused: bool) -> Self {
        self.paused = paused;
        self
    }

    pub fn id(&self) -> &VaultId {
        &self.id
    }

    pub fn asset_id(&self) -> &AssetId {
        &self.asset_id
    }

    pub fn controller(&self) -> &AccountId {
        &self.controller
    }

    pub const fn policy(&self) -> &TranchePolicy {
        &self.policy
    }

    pub const fn fee_schedule(&self) -> &FeeSchedule {
        &self.fee_schedule
    }

    pub const fn redemption_mode(&self) -> RedemptionMode {
        self.redemption_mode
    }

    pub const fn paused(&self) -> bool {
        self.paused
    }
}

impl ReservePricingView {
    pub fn from_tranches(
        epoch: Epoch,
        tranches: &BTreeMap<TrancheKind, TrancheState>,
    ) -> StratumResult<Self> {
        let mut total_reserve = Amount::zero();
        let mut total_shares = Shares::zero();
        let mut reserve_basis = BTreeMap::new();
        let mut index_basis = BTreeMap::new();
        for kind in TrancheKind::public_order() {
            let tranche = tranches
                .get(&kind)
                .ok_or(StratumError::InvalidTranchePolicy)?;
            total_reserve = total_reserve.checked_add(tranche.reserve())?;
            total_shares = total_shares.checked_add(tranche.share_supply())?;
            reserve_basis.insert(kind, tranche.reserve());
            index_basis.insert(kind, tranche.share_index());
        }
        Ok(Self {
            epoch,
            total_reserve_basis: total_reserve,
            total_share_basis: total_shares,
            tranche_reserve_basis: reserve_basis,
            tranche_index_basis: index_basis,
        })
    }

    pub const fn epoch(&self) -> Epoch {
        self.epoch
    }

    pub const fn total_reserve_basis(&self) -> Amount {
        self.total_reserve_basis
    }

    pub const fn total_share_basis(&self) -> Shares {
        self.total_share_basis
    }

    pub fn tranche_reserve_basis(&self, kind: TrancheKind) -> Amount {
        self.tranche_reserve_basis
            .get(&kind)
            .copied()
            .unwrap_or_else(Amount::zero)
    }

    pub fn tranche_index_basis(&self, kind: TrancheKind) -> ShareIndex {
        self.tranche_index_basis
            .get(&kind)
            .copied()
            .unwrap_or_else(ShareIndex::one)
    }
}

impl VaultState {
    pub fn new(config: VaultConfig) -> StratumResult<Self> {
        let mut tranches = BTreeMap::new();
        for kind in TrancheKind::public_order() {
            tranches.insert(kind, TrancheState::new(TrancheConfig::new(kind)));
        }
        let pricing_view = ReservePricingView::from_tranches(Epoch::genesis(), &tranches)?;
        Ok(Self {
            config,
            tranches,
            epoch: Epoch::genesis(),
            pricing_view,
            loss_window: LossWindow::new(Epoch::genesis()),
            total_deposited: Amount::zero(),
            total_redeemed: Amount::zero(),
            total_losses: Amount::zero(),
            total_fees: Amount::zero(),
        })
    }

    pub fn id(&self) -> &VaultId {
        self.config.id()
    }

    pub fn asset_id(&self) -> &AssetId {
        self.config.asset_id()
    }

    pub fn controller(&self) -> &AccountId {
        self.config.controller()
    }

    pub const fn config(&self) -> &VaultConfig {
        &self.config
    }

    pub const fn epoch(&self) -> Epoch {
        self.epoch
    }

    pub const fn pricing_view(&self) -> &ReservePricingView {
        &self.pricing_view
    }

    pub const fn loss_window(&self) -> &LossWindow {
        &self.loss_window
    }

    pub const fn total_deposited(&self) -> Amount {
        self.total_deposited
    }

    pub const fn total_redeemed(&self) -> Amount {
        self.total_redeemed
    }

    pub const fn total_losses(&self) -> Amount {
        self.total_losses
    }

    pub const fn total_fees(&self) -> Amount {
        self.total_fees
    }

    pub fn tranche(&self, kind: TrancheKind) -> StratumResult<&TrancheState> {
        self.tranches
            .get(&kind)
            .ok_or(StratumError::InvalidTranchePolicy)
    }

    pub fn tranche_mut(&mut self, kind: TrancheKind) -> StratumResult<&mut TrancheState> {
        self.tranches
            .get_mut(&kind)
            .ok_or(StratumError::InvalidTranchePolicy)
    }

    pub fn tranches(&self) -> impl Iterator<Item = &TrancheState> {
        self.tranches.values()
    }

    pub fn total_reserve(&self) -> StratumResult<Amount> {
        self.tranches
            .values()
            .try_fold(Amount::zero(), |acc, tranche| {
                acc.checked_add(tranche.reserve())
            })
    }

    pub fn total_share_supply(&self) -> StratumResult<Shares> {
        self.tranches
            .values()
            .try_fold(Shares::zero(), |acc, tranche| {
                acc.checked_add(tranche.share_supply())
            })
    }

    pub fn deposit(
        &mut self,
        account: AccountId,
        tranche: TrancheKind,
        amount: Amount,
    ) -> StratumResult<(DepositReceipt, FeeAssessment)> {
        self.ensure_open()?;
        let assessment = self.config.fee_schedule().assess_deposit(tranche, amount)?;
        let minted = self.tranche_mut(tranche)?.deposit(assessment.net)?;
        self.total_deposited = self.total_deposited.checked_add(amount)?;
        self.total_fees = self.total_fees.checked_add(assessment.fee)?;
        self.refresh_pricing_view()?;
        Ok((
            DepositReceipt {
                account,
                vault_id: self.id().clone(),
                tranche,
                gross: amount,
                fee: assessment.fee,
                net: assessment.net,
                shares: minted,
                epoch: self.epoch,
            },
            assessment,
        ))
    }

    pub fn apply_loss(&mut self, loss: Amount) -> StratumResult<LossReceipt> {
        self.ensure_open()?;
        let reserve_before = self.total_reserve()?;
        if loss > reserve_before {
            return Err(StratumError::LossExceedsReserve);
        }

        let mut remaining = loss;
        let mut allocations = Vec::new();
        let loss_order = self.config.policy().loss_priority().as_slice().to_vec();
        for kind in loss_order {
            if remaining.is_zero() {
                break;
            }
            let tranche = self.tranche_mut(kind)?;
            let absorbed = tranche.absorb_loss(remaining)?;
            if !absorbed.is_zero() {
                allocations.push(LossAllocation {
                    tranche: kind,
                    absorbed,
                    reserve_after: tranche.reserve(),
                });
                remaining = remaining.checked_sub(absorbed)?;
            }
        }

        if !remaining.is_zero() {
            return Err(StratumError::LossExceedsReserve);
        }

        let reserve_after = self.total_reserve()?;
        let absorbed = reserve_before.checked_sub(reserve_after)?;
        self.loss_window.record(self.epoch, loss, absorbed)?;
        self.total_losses = self.total_losses.checked_add(absorbed)?;
        if self.config.policy().auto_publish_loss_indices() {
            self.publish_loss_indices()?;
        }
        Ok(LossReceipt {
            vault_id: self.id().clone(),
            epoch: self.epoch,
            requested_loss: loss,
            absorbed_loss: absorbed,
            reserve_before,
            reserve_after,
            allocations,
        })
    }

    pub fn quote_redemption(
        &self,
        account: AccountId,
        tranche: TrancheKind,
        shares: Shares,
    ) -> StratumResult<RedemptionPlan> {
        self.ensure_open()?;
        self.tranche(tranche)?.ensure_enabled()?;
        if shares.is_zero() {
            return Err(StratumError::ZeroAmount);
        }
        let share_index = self.pricing_view.tranche_index_basis(tranche);
        let gross = share_index.amount_from_shares(shares)?;
        let assessment = self
            .config
            .fee_schedule()
            .assess_redemption(tranche, gross)?;
        let limit = self
            .config
            .policy()
            .redemption_limit(tranche, self.pricing_view.total_reserve_basis())?;
        if gross > limit && !limit.is_zero() {
            return Err(StratumError::LimitExceeded);
        }
        Ok(RedemptionPlan {
            account,
            vault_id: self.id().clone(),
            tranche,
            shares,
            quoted_gross: gross,
            fee: assessment.fee,
            quoted_net: assessment.net,
            epoch: self.epoch,
            mode: self.config.redemption_mode(),
            reserve_basis: self.pricing_view.total_reserve_basis(),
            share_index_basis: share_index,
        })
    }

    pub fn redeem(&mut self, plan: RedemptionPlan) -> StratumResult<RedemptionReceipt> {
        self.ensure_open()?;
        if plan.vault_id != *self.id() {
            return Err(StratumError::VaultNotFound(plan.vault_id));
        }
        if plan.epoch != self.epoch {
            return Err(StratumError::RiskRejected);
        }
        self.tranche_mut(plan.tranche)?.burn_shares(plan.shares)?;
        let paid_from = self.withdraw_for_redemption(plan.tranche, plan.quoted_gross, plan.mode)?;
        self.total_redeemed = self.total_redeemed.checked_add(plan.quoted_gross)?;
        self.total_fees = self.total_fees.checked_add(plan.fee)?;
        self.refresh_touched_indices(&paid_from)?;
        self.refresh_pricing_view()?;
        Ok(RedemptionReceipt {
            account: plan.account,
            vault_id: self.id().clone(),
            tranche: plan.tranche,
            shares: plan.shares,
            gross_paid: plan.quoted_gross,
            net_paid: plan.quoted_net,
            fee: plan.fee,
            paid_from,
            epoch: self.epoch,
        })
    }

    pub fn publish_loss_indices(&mut self) -> StratumResult<()> {
        for tranche in self.tranches.values_mut() {
            tranche.publish_index()?;
        }
        self.epoch = self.epoch.next();
        self.loss_window.clear(self.epoch);
        self.refresh_pricing_view()
    }

    pub fn distribute_fee_credit(
        &mut self,
        amount: Amount,
    ) -> StratumResult<BTreeMap<TrancheKind, Amount>> {
        let targets = self.config.policy().target_amounts(amount)?;
        let mut distributed = BTreeMap::new();
        for (kind, amount) in targets {
            if amount.is_zero() {
                continue;
            }
            self.tranche_mut(kind)?.credit_fees(amount)?;
            distributed.insert(kind, amount);
        }
        self.total_fees = self.total_fees.checked_add(amount)?;
        self.refresh_pricing_view()?;
        Ok(distributed)
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.config.paused = paused;
    }

    fn ensure_open(&self) -> StratumResult<()> {
        if self.config.paused() {
            Err(StratumError::VaultPaused(self.id().clone()))
        } else {
            Ok(())
        }
    }

    fn refresh_pricing_view(&mut self) -> StratumResult<()> {
        self.pricing_view = ReservePricingView::from_tranches(self.epoch, &self.tranches)?;
        Ok(())
    }

    fn refresh_touched_indices(
        &mut self,
        paid_from: &[(TrancheKind, Amount)],
    ) -> StratumResult<()> {
        let mut touched = BTreeSet::new();
        for (kind, _) in paid_from {
            touched.insert(*kind);
        }
        for kind in touched {
            self.tranche_mut(kind)?.publish_index()?;
        }
        Ok(())
    }

    fn withdraw_for_redemption(
        &mut self,
        requested_tranche: TrancheKind,
        amount: Amount,
        mode: RedemptionMode,
    ) -> StratumResult<Vec<(TrancheKind, Amount)>> {
        match mode {
            RedemptionMode::StrictTranche => self.withdraw_ordered(amount, &[requested_tranche]),
            RedemptionMode::LayeredLiquidity => {
                let order = layered_withdrawal_order(requested_tranche);
                self.withdraw_ordered(amount, &order)
            }
        }
    }

    fn withdraw_ordered(
        &mut self,
        amount: Amount,
        order: &[TrancheKind],
    ) -> StratumResult<Vec<(TrancheKind, Amount)>> {
        let mut remaining = amount;
        let mut paid_from = Vec::new();
        for kind in order {
            if remaining.is_zero() {
                break;
            }
            let available = self.tranche(*kind)?.reserve();
            let payment = available.min(remaining);
            if payment.is_zero() {
                continue;
            }
            self.tranche_mut(*kind)?.debit_reserve(payment)?;
            paid_from.push((*kind, payment));
            remaining = remaining.checked_sub(payment)?;
        }
        if remaining.is_zero() {
            Ok(paid_from)
        } else {
            Err(StratumError::InsufficientReserve)
        }
    }
}

fn layered_withdrawal_order(requested_tranche: TrancheKind) -> Vec<TrancheKind> {
    let mut order = Vec::with_capacity(4);
    order.push(requested_tranche);
    for kind in [
        TrancheKind::Senior,
        TrancheKind::Insurance,
        TrancheKind::Overflow,
        TrancheKind::Junior,
    ] {
        if !order.contains(&kind) {
            order.push(kind);
        }
    }
    order
}
