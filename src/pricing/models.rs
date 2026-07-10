use crate::amount::{Amount, Bps, ShareIndex, Shares};
use crate::error::StratumResult;
use crate::snapshot::VaultSnapshot;
use crate::tranche::TrancheKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PricingMode {
    TrancheIndex,
    ReserveWeighted,
    Conservative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuoteInput {
    pub tranche: TrancheKind,
    pub shares: Shares,
    pub amount: Amount,
    pub fee_bps: Bps,
    pub mode: PricingMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuoteOutput {
    pub tranche: TrancheKind,
    pub gross: Amount,
    pub fee: Amount,
    pub net: Amount,
    pub index: ShareIndex,
    pub reserve_basis: Amount,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PricingBook {
    indexes: BTreeMap<TrancheKind, ShareIndex>,
    reserves: BTreeMap<TrancheKind, Amount>,
    total_reserve: Amount,
    total_supply: Shares,
}

impl QuoteInput {
    pub fn redemption(tranche: TrancheKind, shares: Shares, fee_bps: Bps) -> Self {
        Self {
            tranche,
            shares,
            amount: Amount::zero(),
            fee_bps,
            mode: PricingMode::TrancheIndex,
        }
    }

    pub fn deposit(tranche: TrancheKind, amount: Amount, fee_bps: Bps) -> Self {
        Self {
            tranche,
            shares: Shares::zero(),
            amount,
            fee_bps,
            mode: PricingMode::TrancheIndex,
        }
    }

    pub const fn with_mode(mut self, mode: PricingMode) -> Self {
        self.mode = mode;
        self
    }
}

impl PricingBook {
    pub fn from_snapshot(snapshot: &VaultSnapshot) -> Self {
        let mut indexes = BTreeMap::new();
        let mut reserves = BTreeMap::new();
        for tranche in &snapshot.tranches {
            indexes.insert(tranche.kind, tranche.share_index);
            reserves.insert(tranche.kind, tranche.reserve);
        }
        Self {
            indexes,
            reserves,
            total_reserve: snapshot.total_reserve,
            total_supply: snapshot.total_share_supply,
        }
    }

    pub fn index_for(&self, tranche: TrancheKind) -> ShareIndex {
        self.indexes
            .get(&tranche)
            .copied()
            .unwrap_or_else(ShareIndex::one)
    }

    pub fn reserve_for(&self, tranche: TrancheKind) -> Amount {
        self.reserves
            .get(&tranche)
            .copied()
            .unwrap_or_else(Amount::zero)
    }

    pub const fn total_reserve(&self) -> Amount {
        self.total_reserve
    }

    pub const fn total_supply(&self) -> Shares {
        self.total_supply
    }

    pub fn quote(&self, input: QuoteInput) -> StratumResult<QuoteOutput> {
        if input.shares.is_zero() {
            self.quote_deposit(input)
        } else {
            self.quote_redemption(input)
        }
    }

    pub fn quote_deposit(&self, input: QuoteInput) -> StratumResult<QuoteOutput> {
        let fee = input.amount.bps(input.fee_bps)?;
        let net = input.amount.checked_sub(fee)?;
        let index = self.index_for(input.tranche);
        Ok(QuoteOutput {
            tranche: input.tranche,
            gross: input.amount,
            fee,
            net,
            index,
            reserve_basis: self.reserve_for(input.tranche),
        })
    }

    pub fn quote_redemption(&self, input: QuoteInput) -> StratumResult<QuoteOutput> {
        let index = match input.mode {
            PricingMode::TrancheIndex => self.index_for(input.tranche),
            PricingMode::ReserveWeighted => self.blended_index()?,
            PricingMode::Conservative => self.conservative_index(input.tranche)?,
        };
        let gross = index.amount_from_shares(input.shares)?;
        let fee = gross.bps(input.fee_bps)?;
        let net = gross.checked_sub(fee)?;
        Ok(QuoteOutput {
            tranche: input.tranche,
            gross,
            fee,
            net,
            index,
            reserve_basis: self.reserve_for(input.tranche),
        })
    }

    pub fn blended_index(&self) -> StratumResult<ShareIndex> {
        ShareIndex::from_reserve_and_supply(self.total_reserve, self.total_supply)
    }

    pub fn conservative_index(&self, tranche: TrancheKind) -> StratumResult<ShareIndex> {
        let tranche_index = self.index_for(tranche);
        let blended = self.blended_index()?;
        ShareIndex::new(tranche_index.raw().min(blended.raw()))
    }

    pub fn reserve_weight_bps(&self, tranche: TrancheKind) -> StratumResult<Bps> {
        if self.total_reserve.is_zero() {
            return Ok(Bps::zero());
        }
        let raw =
            self.reserve_for(tranche).units().saturating_mul(10_000) / self.total_reserve.units();
        Bps::new(raw.min(10_000) as u16)
    }

    pub fn quote_all_redemptions(
        &self,
        shares_by_tranche: &BTreeMap<TrancheKind, Shares>,
        fee_bps: Bps,
        mode: PricingMode,
    ) -> StratumResult<Vec<QuoteOutput>> {
        let mut outputs = Vec::new();
        for kind in TrancheKind::public_order() {
            let shares = shares_by_tranche
                .get(&kind)
                .copied()
                .unwrap_or_else(Shares::zero);
            if shares.is_zero() {
                continue;
            }
            outputs.push(self.quote_redemption(QuoteInput {
                tranche: kind,
                shares,
                amount: Amount::zero(),
                fee_bps,
                mode,
            })?);
        }
        Ok(outputs)
    }

    pub fn quote_total_net(outputs: &[QuoteOutput]) -> StratumResult<Amount> {
        outputs
            .iter()
            .try_fold(Amount::zero(), |acc, output| acc.checked_add(output.net))
    }

    pub fn quote_total_fee(outputs: &[QuoteOutput]) -> StratumResult<Amount> {
        outputs
            .iter()
            .try_fold(Amount::zero(), |acc, output| acc.checked_add(output.fee))
    }
}
