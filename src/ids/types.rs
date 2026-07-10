use crate::error::{StratumError, StratumResult};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> StratumResult<Self> {
                let value = value.into();
                validate_identifier(&value)?;
                Ok(Self(value))
            }

            pub fn unchecked(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

id_type!(AccountId);
id_type!(AssetId);
id_type!(VaultId);
id_type!(TxId);
id_type!(EventId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
pub struct Epoch(u64);

impl Epoch {
    pub const fn genesis() -> Self {
        Self(0)
    }

    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }

    pub fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

impl Default for Epoch {
    fn default() -> Self {
        Self::genesis()
    }
}

impl Display for Epoch {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn validate_identifier(value: &str) -> StratumResult<()> {
    if value.is_empty() {
        return Err(StratumError::EmptyIdentifier);
    }
    let valid = value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ':' | '-' | '_' | '.'));
    if valid {
        Ok(())
    } else {
        Err(StratumError::InvalidIdentifier)
    }
}
