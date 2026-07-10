use crate::amount::{Amount, Bps};
use crate::error::{StratumError, StratumResult};
use crate::ids::AssetId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetStatus {
    Active,
    Paused,
    Retired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetConfig {
    id: AssetId,
    symbol: String,
    decimals: u8,
    status: AssetStatus,
    min_deposit: Amount,
    max_single_deposit: Amount,
    redemption_fee_bps: Bps,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssetRegistry {
    assets: BTreeMap<AssetId, AssetConfig>,
}

impl AssetConfig {
    pub fn new(
        id: AssetId,
        symbol: impl Into<String>,
        decimals: u8,
        min_deposit: Amount,
        max_single_deposit: Amount,
    ) -> Self {
        Self {
            id,
            symbol: symbol.into(),
            decimals,
            status: AssetStatus::Active,
            min_deposit,
            max_single_deposit,
            redemption_fee_bps: Bps::zero(),
        }
    }

    pub fn with_redemption_fee(mut self, fee: Bps) -> Self {
        self.redemption_fee_bps = fee;
        self
    }

    pub fn with_status(mut self, status: AssetStatus) -> Self {
        self.status = status;
        self
    }

    pub fn id(&self) -> &AssetId {
        &self.id
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub const fn decimals(&self) -> u8 {
        self.decimals
    }

    pub const fn status(&self) -> AssetStatus {
        self.status
    }

    pub const fn min_deposit(&self) -> Amount {
        self.min_deposit
    }

    pub const fn max_single_deposit(&self) -> Amount {
        self.max_single_deposit
    }

    pub const fn redemption_fee_bps(&self) -> Bps {
        self.redemption_fee_bps
    }

    pub fn ensure_active(&self) -> StratumResult<()> {
        if self.status == AssetStatus::Active {
            Ok(())
        } else {
            Err(StratumError::AssetInactive(self.id.clone()))
        }
    }

    pub fn validate_deposit(&self, amount: Amount) -> StratumResult<()> {
        self.ensure_active()?;
        if amount < self.min_deposit {
            return Err(StratumError::LimitExceeded);
        }
        if amount > self.max_single_deposit {
            return Err(StratumError::LimitExceeded);
        }
        Ok(())
    }
}

impl AssetRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, config: AssetConfig) -> Option<AssetConfig> {
        self.assets.insert(config.id().clone(), config)
    }

    pub fn get(&self, id: &AssetId) -> StratumResult<&AssetConfig> {
        self.assets
            .get(id)
            .ok_or_else(|| StratumError::AssetNotFound(id.clone()))
    }

    pub fn get_mut(&mut self, id: &AssetId) -> StratumResult<&mut AssetConfig> {
        self.assets
            .get_mut(id)
            .ok_or_else(|| StratumError::AssetNotFound(id.clone()))
    }

    pub fn contains(&self, id: &AssetId) -> bool {
        self.assets.contains_key(id)
    }

    pub fn len(&self) -> usize {
        self.assets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&AssetId, &AssetConfig)> {
        self.assets.iter()
    }
}
