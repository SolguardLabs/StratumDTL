use crate::amount::Amount;
use crate::error::{StratumError, StratumResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rounding {
    Down,
    Up,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ratio {
    numerator: u128,
    denominator: u128,
}

impl Ratio {
    pub fn new(numerator: u128, denominator: u128) -> StratumResult<Self> {
        if denominator == 0 {
            return Err(StratumError::AmountOverflow);
        }
        Ok(Self {
            numerator,
            denominator,
        })
    }

    pub const fn one() -> Self {
        Self {
            numerator: 1,
            denominator: 1,
        }
    }

    pub const fn zero() -> Self {
        Self {
            numerator: 0,
            denominator: 1,
        }
    }

    pub const fn numerator(self) -> u128 {
        self.numerator
    }

    pub const fn denominator(self) -> u128 {
        self.denominator
    }

    pub fn checked_mul(self, other: Self) -> StratumResult<Self> {
        let numerator = self
            .numerator
            .checked_mul(other.numerator)
            .ok_or(StratumError::AmountOverflow)?;
        let denominator = self
            .denominator
            .checked_mul(other.denominator)
            .ok_or(StratumError::AmountOverflow)?;
        Self::new(numerator, denominator)
    }

    pub fn checked_div(self, other: Self) -> StratumResult<Self> {
        if other.numerator == 0 {
            return Err(StratumError::AmountOverflow);
        }
        let numerator = self
            .numerator
            .checked_mul(other.denominator)
            .ok_or(StratumError::AmountOverflow)?;
        let denominator = self
            .denominator
            .checked_mul(other.numerator)
            .ok_or(StratumError::AmountOverflow)?;
        Self::new(numerator, denominator)
    }

    pub fn apply_to_amount(self, amount: Amount, rounding: Rounding) -> StratumResult<Amount> {
        let product = amount
            .units()
            .checked_mul(self.numerator)
            .ok_or(StratumError::AmountOverflow)?;
        let quotient = product
            .checked_div(self.denominator)
            .ok_or(StratumError::AmountOverflow)?;
        let remainder = product
            .checked_rem(self.denominator)
            .ok_or(StratumError::AmountOverflow)?;
        let rounded = match rounding {
            Rounding::Down => quotient,
            Rounding::Up if remainder == 0 => quotient,
            Rounding::Up => quotient
                .checked_add(1)
                .ok_or(StratumError::AmountOverflow)?,
        };
        Ok(Amount::from_units(rounded))
    }

    pub fn invert(self) -> StratumResult<Self> {
        if self.numerator == 0 {
            return Err(StratumError::AmountOverflow);
        }
        Self::new(self.denominator, self.numerator)
    }

    pub fn min(self, other: Self) -> Self {
        let left = self.numerator.saturating_mul(other.denominator);
        let right = other.numerator.saturating_mul(self.denominator);
        if left <= right { self } else { other }
    }

    pub fn max(self, other: Self) -> Self {
        let left = self.numerator.saturating_mul(other.denominator);
        let right = other.numerator.saturating_mul(self.denominator);
        if left >= right { self } else { other }
    }
}

impl Default for Ratio {
    fn default() -> Self {
        Self::one()
    }
}
