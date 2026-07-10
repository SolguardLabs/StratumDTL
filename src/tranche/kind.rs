use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TrancheKind {
    Senior,
    Junior,
    Insurance,
    Overflow,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LossPriority {
    order: Vec<TrancheKind>,
}

impl TrancheKind {
    pub const fn all() -> [Self; 4] {
        [Self::Senior, Self::Junior, Self::Insurance, Self::Overflow]
    }

    pub const fn public_order() -> [Self; 4] {
        [Self::Senior, Self::Junior, Self::Insurance, Self::Overflow]
    }

    pub const fn default_loss_order() -> [Self; 4] {
        [Self::Junior, Self::Insurance, Self::Overflow, Self::Senior]
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Senior => "senior",
            Self::Junior => "junior",
            Self::Insurance => "insurance",
            Self::Overflow => "overflow",
        }
    }

    pub const fn risk_weight(self) -> u16 {
        match self {
            Self::Senior => 100,
            Self::Junior => 450,
            Self::Insurance => 275,
            Self::Overflow => 600,
        }
    }
}

impl Display for TrancheKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

impl LossPriority {
    pub fn new(order: Vec<TrancheKind>) -> Self {
        Self { order }
    }

    pub fn standard() -> Self {
        Self {
            order: TrancheKind::default_loss_order().to_vec(),
        }
    }

    pub fn senior_first() -> Self {
        Self {
            order: vec![
                TrancheKind::Senior,
                TrancheKind::Insurance,
                TrancheKind::Junior,
                TrancheKind::Overflow,
            ],
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = TrancheKind> + '_ {
        self.order.iter().copied()
    }

    pub fn as_slice(&self) -> &[TrancheKind] {
        &self.order
    }
}

impl Default for LossPriority {
    fn default() -> Self {
        Self::standard()
    }
}
