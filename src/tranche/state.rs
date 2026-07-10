use crate::amount::{Amount, Bps, ShareIndex, Shares};
use crate::error::{StratumError, StratumResult};
use crate::tranche::TrancheKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrancheConfig {
    kind: TrancheKind,
    enabled: bool,
    deposit_fee_bps: Bps,
    redemption_fee_bps: Bps,
    min_reserve: Amount,
    max_reserve: Amount,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrancheState {
    config: TrancheConfig,
    reserve: Amount,
    share_supply: Shares,
    share_index: ShareIndex,
    pending_loss: Amount,
    accrued_fee_credit: Amount,
    deposits_count: u64,
    redemptions_count: u64,
    loss_count: u64,
}

impl TrancheConfig {
    pub fn new(kind: TrancheKind) -> Self {
        Self {
            kind,
            enabled: true,
            deposit_fee_bps: Bps::zero(),
            redemption_fee_bps: Bps::zero(),
            min_reserve: Amount::zero(),
            max_reserve: Amount::from_units(u128::MAX),
        }
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn with_deposit_fee(mut self, fee: Bps) -> Self {
        self.deposit_fee_bps = fee;
        self
    }

    pub fn with_redemption_fee(mut self, fee: Bps) -> Self {
        self.redemption_fee_bps = fee;
        self
    }

    pub fn with_reserve_bounds(mut self, min_reserve: Amount, max_reserve: Amount) -> Self {
        self.min_reserve = min_reserve;
        self.max_reserve = max_reserve;
        self
    }

    pub const fn kind(&self) -> TrancheKind {
        self.kind
    }

    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    pub const fn deposit_fee_bps(&self) -> Bps {
        self.deposit_fee_bps
    }

    pub const fn redemption_fee_bps(&self) -> Bps {
        self.redemption_fee_bps
    }

    pub const fn min_reserve(&self) -> Amount {
        self.min_reserve
    }

    pub const fn max_reserve(&self) -> Amount {
        self.max_reserve
    }
}

impl TrancheState {
    pub fn new(config: TrancheConfig) -> Self {
        Self {
            config,
            reserve: Amount::zero(),
            share_supply: Shares::zero(),
            share_index: ShareIndex::one(),
            pending_loss: Amount::zero(),
            accrued_fee_credit: Amount::zero(),
            deposits_count: 0,
            redemptions_count: 0,
            loss_count: 0,
        }
    }

    pub fn kind(&self) -> TrancheKind {
        self.config.kind()
    }

    pub const fn config(&self) -> &TrancheConfig {
        &self.config
    }

    pub const fn reserve(&self) -> Amount {
        self.reserve
    }

    pub const fn share_supply(&self) -> Shares {
        self.share_supply
    }

    pub const fn share_index(&self) -> ShareIndex {
        self.share_index
    }

    pub const fn pending_loss(&self) -> Amount {
        self.pending_loss
    }

    pub const fn accrued_fee_credit(&self) -> Amount {
        self.accrued_fee_credit
    }

    pub const fn deposits_count(&self) -> u64 {
        self.deposits_count
    }

    pub const fn redemptions_count(&self) -> u64 {
        self.redemptions_count
    }

    pub const fn loss_count(&self) -> u64 {
        self.loss_count
    }

    pub fn ensure_enabled(&self) -> StratumResult<()> {
        if self.config.enabled() {
            Ok(())
        } else {
            Err(StratumError::TrancheInactive(self.kind()))
        }
    }

    pub fn deposit(&mut self, net_amount: Amount) -> StratumResult<Shares> {
        self.ensure_enabled()?;
        let new_reserve = self.reserve.checked_add(net_amount)?;
        if new_reserve > self.config.max_reserve() {
            return Err(StratumError::LimitExceeded);
        }
        let minted = if self.share_supply.is_zero() {
            Shares::from_units(net_amount.units())
        } else {
            self.share_index.shares_from_amount(net_amount)?
        };
        self.reserve = new_reserve;
        self.share_supply = self.share_supply.checked_add(minted)?;
        self.share_index = ShareIndex::from_reserve_and_supply(self.reserve, self.share_supply)?;
        self.deposits_count = self.deposits_count.saturating_add(1);
        Ok(minted)
    }

    pub fn credit_fees(&mut self, amount: Amount) -> StratumResult<()> {
        self.reserve = self.reserve.checked_add(amount)?;
        self.accrued_fee_credit = self.accrued_fee_credit.checked_add(amount)?;
        self.share_index = ShareIndex::from_reserve_and_supply(self.reserve, self.share_supply)?;
        Ok(())
    }

    pub fn debit_reserve(&mut self, amount: Amount) -> StratumResult<()> {
        self.reserve = self.reserve.checked_sub(amount)?;
        self.redemptions_count = self.redemptions_count.saturating_add(1);
        Ok(())
    }

    pub fn burn_shares(&mut self, shares: Shares) -> StratumResult<()> {
        self.share_supply = self.share_supply.checked_sub(shares)?;
        self.share_index = ShareIndex::from_reserve_and_supply(self.reserve, self.share_supply)?;
        Ok(())
    }

    pub fn absorb_loss(&mut self, amount: Amount) -> StratumResult<Amount> {
        if amount.is_zero() {
            return Ok(Amount::zero());
        }
        let absorbed = amount.min(self.reserve);
        self.reserve = self.reserve.checked_sub(absorbed)?;
        self.pending_loss = self.pending_loss.checked_add(absorbed)?;
        self.loss_count = self.loss_count.saturating_add(1);
        Ok(absorbed)
    }

    pub fn publish_index(&mut self) -> StratumResult<()> {
        self.share_index = ShareIndex::from_reserve_and_supply(self.reserve, self.share_supply)?;
        self.pending_loss = Amount::zero();
        Ok(())
    }

    pub fn expected_index(&self) -> StratumResult<ShareIndex> {
        ShareIndex::from_reserve_and_supply(self.reserve, self.share_supply)
    }

    pub fn redeemable_after_floor(&self) -> StratumResult<Amount> {
        let floor = self.config.min_reserve();
        if self.reserve <= floor {
            return Ok(Amount::zero());
        }
        self.reserve.checked_sub(floor)
    }

    pub fn utilization_bps(&self) -> StratumResult<Bps> {
        if self.share_supply.is_zero() || self.reserve.is_zero() {
            return Ok(Bps::zero());
        }
        let supply_value = self.share_index.amount_from_shares(self.share_supply)?;
        if supply_value.is_zero() {
            return Ok(Bps::zero());
        }
        let raw = self
            .reserve
            .units()
            .saturating_mul(10_000)
            .checked_div(supply_value.units())
            .ok_or(StratumError::AmountOverflow)?;
        Bps::new(raw.min(10_000) as u16)
    }
}
