use crate::ids::{AccountId, AssetId, VaultId};
use crate::tranche::TrancheKind;
use thiserror::Error;

pub type StratumResult<T> = Result<T, StratumError>;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StratumError {
    #[error("amount overflow")]
    AmountOverflow,
    #[error("amount must be greater than zero")]
    ZeroAmount,
    #[error("basis points must be in the range 0..=10000")]
    InvalidBasisPoints,
    #[error("share index must be greater than zero")]
    InvalidShareIndex,
    #[error("share balance is insufficient")]
    InsufficientShares,
    #[error("reserve balance is insufficient")]
    InsufficientReserve,
    #[error("loss exceeds available tranche reserves")]
    LossExceedsReserve,
    #[error("account not found: {0}")]
    AccountNotFound(AccountId),
    #[error("asset not found: {0}")]
    AssetNotFound(AssetId),
    #[error("vault not found: {0}")]
    VaultNotFound(VaultId),
    #[error("vault is paused: {0}")]
    VaultPaused(VaultId),
    #[error("asset is inactive: {0}")]
    AssetInactive(AssetId),
    #[error("tranche is inactive: {0}")]
    TrancheInactive(TrancheKind),
    #[error("tranche policy is invalid")]
    InvalidTranchePolicy,
    #[error("operation exceeds configured limit")]
    LimitExceeded,
    #[error("redemption would violate liquidity floor")]
    LiquidityFloor,
    #[error("fee split does not sum to 10000 bps")]
    InvalidFeeSplit,
    #[error("identifier is empty")]
    EmptyIdentifier,
    #[error("identifier contains unsupported characters")]
    InvalidIdentifier,
    #[error("snapshot serialization failed")]
    SnapshotEncoding,
    #[error("journal cursor is out of range")]
    JournalCursor,
    #[error("operation is not admitted by risk policy")]
    RiskRejected,
}
