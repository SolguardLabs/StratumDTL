use crate::error::{StratumError, StratumResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalSnapshot {
    pub namespace: String,
    pub version: u32,
    pub payload: serde_json::Value,
    pub labels: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SnapshotEncoder;

impl CanonicalSnapshot {
    pub fn new(namespace: impl Into<String>, version: u32, payload: serde_json::Value) -> Self {
        Self {
            namespace: namespace.into(),
            version,
            payload,
            labels: BTreeMap::new(),
        }
    }

    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }
}

impl SnapshotEncoder {
    pub fn encode<T: Serialize>(&self, namespace: &str, value: &T) -> StratumResult<Vec<u8>> {
        let payload = serde_json::to_value(value).map_err(|_| StratumError::SnapshotEncoding)?;
        let snapshot = CanonicalSnapshot::new(namespace, 1, payload);
        canonical_json_bytes(&snapshot)
    }
}

pub fn canonical_json_bytes<T: Serialize>(value: &T) -> StratumResult<Vec<u8>> {
    let mut bytes = serde_json::to_vec(value).map_err(|_| StratumError::SnapshotEncoding)?;
    bytes.push(b'\n');
    Ok(bytes)
}
