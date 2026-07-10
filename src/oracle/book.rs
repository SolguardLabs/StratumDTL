use crate::amount::{Amount, ShareIndex};
use crate::error::{StratumError, StratumResult};
use crate::ids::{Epoch, VaultId};
use crate::tranche::TrancheKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct OracleSource {
    name: String,
    priority: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexObservation {
    pub source: OracleSource,
    pub vault_id: VaultId,
    pub tranche: TrancheKind,
    pub epoch: Epoch,
    pub index: ShareIndex,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReserveObservation {
    pub source: OracleSource,
    pub vault_id: VaultId,
    pub tranche: TrancheKind,
    pub epoch: Epoch,
    pub reserve: Amount,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OracleBook {
    index_observations: BTreeMap<(VaultId, TrancheKind), Vec<IndexObservation>>,
    reserve_observations: BTreeMap<(VaultId, TrancheKind), Vec<ReserveObservation>>,
    max_depth: usize,
}

impl OracleSource {
    pub fn new(name: impl Into<String>, priority: u8) -> StratumResult<Self> {
        let name = name.into();
        if name.is_empty() {
            return Err(StratumError::EmptyIdentifier);
        }
        Ok(Self { name, priority })
    }

    pub fn canonical() -> Self {
        Self {
            name: "canonical".to_string(),
            priority: 0,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn priority(&self) -> u8 {
        self.priority
    }
}

impl IndexObservation {
    pub fn new(
        source: OracleSource,
        vault_id: VaultId,
        tranche: TrancheKind,
        epoch: Epoch,
        index: ShareIndex,
    ) -> Self {
        Self {
            source,
            vault_id,
            tranche,
            epoch,
            index,
        }
    }
}

impl ReserveObservation {
    pub fn new(
        source: OracleSource,
        vault_id: VaultId,
        tranche: TrancheKind,
        epoch: Epoch,
        reserve: Amount,
    ) -> Self {
        Self {
            source,
            vault_id,
            tranche,
            epoch,
            reserve,
        }
    }
}

impl OracleBook {
    pub fn new(max_depth: usize) -> Self {
        Self {
            index_observations: BTreeMap::new(),
            reserve_observations: BTreeMap::new(),
            max_depth: max_depth.max(1),
        }
    }

    pub fn with_default_depth() -> Self {
        Self::new(32)
    }

    pub fn push_index(&mut self, observation: IndexObservation) {
        let key = (observation.vault_id.clone(), observation.tranche);
        push_bounded(
            self.index_observations.entry(key).or_default(),
            observation,
            self.max_depth,
        );
    }

    pub fn push_reserve(&mut self, observation: ReserveObservation) {
        let key = (observation.vault_id.clone(), observation.tranche);
        push_bounded(
            self.reserve_observations.entry(key).or_default(),
            observation,
            self.max_depth,
        );
    }

    pub fn latest_index(
        &self,
        vault_id: &VaultId,
        tranche: TrancheKind,
    ) -> Option<&IndexObservation> {
        self.index_observations
            .get(&(vault_id.clone(), tranche))
            .and_then(|observations| observations.last())
    }

    pub fn latest_reserve(
        &self,
        vault_id: &VaultId,
        tranche: TrancheKind,
    ) -> Option<&ReserveObservation> {
        self.reserve_observations
            .get(&(vault_id.clone(), tranche))
            .and_then(|observations| observations.last())
    }

    pub fn index_history(&self, vault_id: &VaultId, tranche: TrancheKind) -> &[IndexObservation] {
        self.index_observations
            .get(&(vault_id.clone(), tranche))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn reserve_history(
        &self,
        vault_id: &VaultId,
        tranche: TrancheKind,
    ) -> &[ReserveObservation] {
        self.reserve_observations
            .get(&(vault_id.clone(), tranche))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn median_index(
        &self,
        vault_id: &VaultId,
        tranche: TrancheKind,
    ) -> StratumResult<Option<ShareIndex>> {
        let mut values: Vec<u128> = self
            .index_history(vault_id, tranche)
            .iter()
            .map(|observation| observation.index.raw())
            .collect();
        if values.is_empty() {
            return Ok(None);
        }
        values.sort_unstable();
        let middle = values.len() / 2;
        ShareIndex::new(values[middle]).map(Some)
    }

    pub fn reserve_delta(
        &self,
        vault_id: &VaultId,
        tranche: TrancheKind,
    ) -> StratumResult<Option<i128>> {
        let history = self.reserve_history(vault_id, tranche);
        if history.len() < 2 {
            return Ok(None);
        }
        let first = history.first().expect("length checked").reserve.units();
        let last = history.last().expect("length checked").reserve.units();
        Ok(Some(last as i128 - first as i128))
    }

    pub fn observed_tranches(&self, vault_id: &VaultId) -> Vec<TrancheKind> {
        TrancheKind::public_order()
            .into_iter()
            .filter(|kind| {
                self.index_observations
                    .contains_key(&(vault_id.clone(), *kind))
                    || self
                        .reserve_observations
                        .contains_key(&(vault_id.clone(), *kind))
            })
            .collect()
    }
}

impl Default for OracleSource {
    fn default() -> Self {
        Self::canonical()
    }
}

fn push_bounded<T>(items: &mut Vec<T>, value: T, max_depth: usize) {
    items.push(value);
    if items.len() > max_depth {
        let excess = items.len() - max_depth;
        items.drain(0..excess);
    }
}
