use crate::amount::{Amount, ShareIndex, Shares};
use crate::snapshot::{AccountSnapshot, VaultSnapshot};
use crate::tranche::TrancheKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioEntry {
    pub tranche: TrancheKind,
    pub shares: Shares,
    pub estimated_value: Amount,
    pub principal: Amount,
    pub redeemed: Amount,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioBreakdown {
    pub entries: Vec<PortfolioEntry>,
    pub total_shares: Shares,
    pub total_estimated_value: Amount,
    pub total_principal: Amount,
    pub total_redeemed: Amount,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioReport {
    pub account: crate::ids::AccountId,
    pub by_vault: BTreeMap<crate::ids::VaultId, PortfolioBreakdown>,
}

impl PortfolioEntry {
    pub fn unrealized_pnl(&self) -> i128 {
        let basis = self.principal.saturating_sub(self.redeemed);
        self.estimated_value.units() as i128 - basis.units() as i128
    }
}

impl PortfolioBreakdown {
    pub fn new(entries: Vec<PortfolioEntry>) -> crate::StratumResult<Self> {
        let mut total_shares = Shares::zero();
        let mut total_estimated_value = Amount::zero();
        let mut total_principal = Amount::zero();
        let mut total_redeemed = Amount::zero();
        for entry in &entries {
            total_shares = total_shares.checked_add(entry.shares)?;
            total_estimated_value = total_estimated_value.checked_add(entry.estimated_value)?;
            total_principal = total_principal.checked_add(entry.principal)?;
            total_redeemed = total_redeemed.checked_add(entry.redeemed)?;
        }
        Ok(Self {
            entries,
            total_shares,
            total_estimated_value,
            total_principal,
            total_redeemed,
        })
    }

    pub fn entry(&self, tranche: TrancheKind) -> Option<&PortfolioEntry> {
        self.entries.iter().find(|entry| entry.tranche == tranche)
    }

    pub fn total_unrealized_pnl(&self) -> i128 {
        self.entries
            .iter()
            .map(PortfolioEntry::unrealized_pnl)
            .sum()
    }
}

impl PortfolioReport {
    pub fn build(
        account: &AccountSnapshot,
        vaults: &[VaultSnapshot],
    ) -> crate::StratumResult<Self> {
        let mut by_vault: BTreeMap<crate::ids::VaultId, Vec<PortfolioEntry>> = BTreeMap::new();
        for (vault_id, tranche, shares, principal, redeemed) in &account.positions {
            let index = index_for(vaults, vault_id, *tranche);
            let estimated_value = index.amount_from_shares(*shares)?;
            by_vault
                .entry(vault_id.clone())
                .or_default()
                .push(PortfolioEntry {
                    tranche: *tranche,
                    shares: *shares,
                    estimated_value,
                    principal: *principal,
                    redeemed: *redeemed,
                });
        }
        let mut breakdowns = BTreeMap::new();
        for (vault_id, entries) in by_vault {
            breakdowns.insert(vault_id, PortfolioBreakdown::new(entries)?);
        }
        Ok(Self {
            account: account.id.clone(),
            by_vault: breakdowns,
        })
    }

    pub fn total_value(&self) -> crate::StratumResult<Amount> {
        self.by_vault
            .values()
            .try_fold(Amount::zero(), |acc, item| {
                acc.checked_add(item.total_estimated_value)
            })
    }

    pub fn total_principal(&self) -> crate::StratumResult<Amount> {
        self.by_vault
            .values()
            .try_fold(Amount::zero(), |acc, item| {
                acc.checked_add(item.total_principal)
            })
    }

    pub fn total_redeemed(&self) -> crate::StratumResult<Amount> {
        self.by_vault
            .values()
            .try_fold(Amount::zero(), |acc, item| {
                acc.checked_add(item.total_redeemed)
            })
    }
}

fn index_for(
    vaults: &[VaultSnapshot],
    vault_id: &crate::ids::VaultId,
    tranche: TrancheKind,
) -> ShareIndex {
    vaults
        .iter()
        .find(|vault| &vault.id == vault_id)
        .and_then(|vault| vault.tranche(tranche))
        .map(|snapshot| snapshot.share_index)
        .unwrap_or_else(ShareIndex::one)
}
