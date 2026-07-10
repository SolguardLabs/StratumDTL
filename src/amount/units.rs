use crate::error::{StratumError, StratumResult};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

pub const INDEX_SCALE: u128 = 1_000_000_000_000;
pub const BPS_SCALE: u128 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
pub struct Amount(u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
pub struct Shares(u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ShareIndex(u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Bps(u16);

impl Amount {
    pub const fn zero() -> Self {
        Self(0)
    }

    pub fn new(units: u128) -> StratumResult<Self> {
        if units == 0 {
            return Err(StratumError::ZeroAmount);
        }
        Ok(Self(units))
    }

    pub const fn from_units(units: u128) -> Self {
        Self(units)
    }

    pub const fn units(self) -> u128 {
        self.0
    }

    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub fn checked_add(self, rhs: Self) -> StratumResult<Self> {
        self.0
            .checked_add(rhs.0)
            .map(Self)
            .ok_or(StratumError::AmountOverflow)
    }

    pub fn checked_sub(self, rhs: Self) -> StratumResult<Self> {
        self.0
            .checked_sub(rhs.0)
            .map(Self)
            .ok_or(StratumError::InsufficientReserve)
    }

    pub fn checked_mul(self, rhs: u128) -> StratumResult<Self> {
        self.0
            .checked_mul(rhs)
            .map(Self)
            .ok_or(StratumError::AmountOverflow)
    }

    pub fn checked_div(self, rhs: u128) -> StratumResult<Self> {
        if rhs == 0 {
            return Err(StratumError::AmountOverflow);
        }
        Ok(Self(self.0 / rhs))
    }

    pub fn min(self, rhs: Self) -> Self {
        if self.0 <= rhs.0 { self } else { rhs }
    }

    pub fn max(self, rhs: Self) -> Self {
        if self.0 >= rhs.0 { self } else { rhs }
    }

    pub fn saturating_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }

    pub fn bps(self, bps: Bps) -> StratumResult<Self> {
        let product = self
            .0
            .checked_mul(u128::from(bps.value()))
            .ok_or(StratumError::AmountOverflow)?;
        Ok(Self(product / BPS_SCALE))
    }

    pub fn split_floor(self, weights: &[Bps]) -> StratumResult<Vec<Self>> {
        let mut allocated = Vec::with_capacity(weights.len());
        let mut total = Self::zero();
        for weight in weights {
            let part = self.bps(*weight)?;
            total = total.checked_add(part)?;
            allocated.push(part);
        }
        if total > self {
            return Err(StratumError::AmountOverflow);
        }
        if let Some(last) = allocated.last_mut() {
            *last = last.checked_add(self.checked_sub(total)?)?;
        }
        Ok(allocated)
    }
}

impl Default for Amount {
    fn default() -> Self {
        Self::zero()
    }
}

impl Shares {
    pub const fn zero() -> Self {
        Self(0)
    }

    pub fn new(units: u128) -> StratumResult<Self> {
        if units == 0 {
            return Err(StratumError::ZeroAmount);
        }
        Ok(Self(units))
    }

    pub const fn from_units(units: u128) -> Self {
        Self(units)
    }

    pub const fn units(self) -> u128 {
        self.0
    }

    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub fn checked_add(self, rhs: Self) -> StratumResult<Self> {
        self.0
            .checked_add(rhs.0)
            .map(Self)
            .ok_or(StratumError::AmountOverflow)
    }

    pub fn checked_sub(self, rhs: Self) -> StratumResult<Self> {
        self.0
            .checked_sub(rhs.0)
            .map(Self)
            .ok_or(StratumError::InsufficientShares)
    }

    pub fn min(self, rhs: Self) -> Self {
        if self.0 <= rhs.0 { self } else { rhs }
    }
}

impl Default for Shares {
    fn default() -> Self {
        Self::zero()
    }
}

impl ShareIndex {
    pub fn new(value: u128) -> StratumResult<Self> {
        if value == 0 {
            return Err(StratumError::InvalidShareIndex);
        }
        Ok(Self(value))
    }

    pub const fn one() -> Self {
        Self(INDEX_SCALE)
    }

    pub const fn raw(self) -> u128 {
        self.0
    }

    pub fn amount_from_shares(self, shares: Shares) -> StratumResult<Amount> {
        let product = shares
            .units()
            .checked_mul(self.0)
            .ok_or(StratumError::AmountOverflow)?;
        Ok(Amount::from_units(product / INDEX_SCALE))
    }

    pub fn shares_from_amount(self, amount: Amount) -> StratumResult<Shares> {
        let product = amount
            .units()
            .checked_mul(INDEX_SCALE)
            .ok_or(StratumError::AmountOverflow)?;
        Ok(Shares::from_units(product / self.0))
    }

    pub fn from_reserve_and_supply(reserve: Amount, supply: Shares) -> StratumResult<Self> {
        if supply.is_zero() {
            return Ok(Self::one());
        }
        let product = reserve
            .units()
            .checked_mul(INDEX_SCALE)
            .ok_or(StratumError::AmountOverflow)?;
        Self::new(product / supply.units())
    }

    pub fn increase_by_bps(self, bps: Bps) -> StratumResult<Self> {
        let extra = self
            .0
            .checked_mul(u128::from(bps.value()))
            .ok_or(StratumError::AmountOverflow)?
            / BPS_SCALE;
        Self::new(
            self.0
                .checked_add(extra)
                .ok_or(StratumError::AmountOverflow)?,
        )
    }

    pub fn decrease_by_bps(self, bps: Bps) -> StratumResult<Self> {
        let cut = self
            .0
            .checked_mul(u128::from(bps.value()))
            .ok_or(StratumError::AmountOverflow)?
            / BPS_SCALE;
        Self::new(
            self.0
                .checked_sub(cut)
                .ok_or(StratumError::AmountOverflow)?,
        )
    }
}

impl Default for ShareIndex {
    fn default() -> Self {
        Self::one()
    }
}

impl Bps {
    pub fn new(value: u16) -> StratumResult<Self> {
        if u128::from(value) > BPS_SCALE {
            return Err(StratumError::InvalidBasisPoints);
        }
        Ok(Self(value))
    }

    pub const fn zero() -> Self {
        Self(0)
    }

    pub const fn full() -> Self {
        Self(10_000)
    }

    pub const fn value(self) -> u16 {
        self.0
    }
}

impl Default for Bps {
    fn default() -> Self {
        Self::zero()
    }
}

impl Display for Amount {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for Shares {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for ShareIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for Bps {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
