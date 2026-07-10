use crate::amount::Amount;
use crate::snapshot::{AccountSnapshot, VaultSnapshot};
use crate::tranche::TrancheKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AuditSeverity {
    Info,
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditFinding {
    pub severity: AuditSeverity,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditReport {
    pub findings: Vec<AuditFinding>,
    pub vault_total_reserve: Amount,
    pub account_position_count: usize,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AuditEngine;

impl AuditFinding {
    pub fn new(
        severity: AuditSeverity,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            code: code.into(),
            message: message.into(),
        }
    }
}

impl AuditReport {
    pub fn has_high_findings(&self) -> bool {
        self.findings
            .iter()
            .any(|finding| finding.severity >= AuditSeverity::High)
    }

    pub fn finding_count(&self, severity: AuditSeverity) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.severity == severity)
            .count()
    }

    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }
}

impl AuditEngine {
    pub fn review(&self, vault: &VaultSnapshot, accounts: &[AccountSnapshot]) -> AuditReport {
        let mut findings = Vec::new();
        self.check_tranche_presence(vault, &mut findings);
        self.check_reserve_sum(vault, &mut findings);
        self.check_account_supply(vault, accounts, &mut findings);
        self.check_negative_drift(vault, &mut findings);
        AuditReport {
            findings,
            vault_total_reserve: vault.total_reserve,
            account_position_count: accounts.iter().map(|account| account.positions.len()).sum(),
        }
    }

    fn check_tranche_presence(&self, vault: &VaultSnapshot, findings: &mut Vec<AuditFinding>) {
        for kind in TrancheKind::public_order() {
            if vault.tranche(kind).is_none() {
                findings.push(AuditFinding::new(
                    AuditSeverity::High,
                    "missing_tranche",
                    format!("missing tranche snapshot: {}", kind.label()),
                ));
            }
        }
    }

    fn check_reserve_sum(&self, vault: &VaultSnapshot, findings: &mut Vec<AuditFinding>) {
        let summed = vault
            .tranches
            .iter()
            .try_fold(Amount::zero(), |acc, tranche| {
                acc.checked_add(tranche.reserve)
            });
        match summed {
            Ok(total) if total == vault.total_reserve => {}
            Ok(_) => findings.push(AuditFinding::new(
                AuditSeverity::High,
                "reserve_sum_mismatch",
                "sum of tranche reserves differs from vault total",
            )),
            Err(_) => findings.push(AuditFinding::new(
                AuditSeverity::High,
                "reserve_sum_overflow",
                "sum of tranche reserves overflowed",
            )),
        }
    }

    fn check_account_supply(
        &self,
        vault: &VaultSnapshot,
        accounts: &[AccountSnapshot],
        findings: &mut Vec<AuditFinding>,
    ) {
        for kind in TrancheKind::public_order() {
            let account_supply: u128 = accounts
                .iter()
                .flat_map(|account| account.positions.iter())
                .filter(|(vault_id, tranche, _, _, _)| vault_id == &vault.id && *tranche == kind)
                .map(|(_, _, shares, _, _)| shares.units())
                .sum();
            let vault_supply = vault
                .tranche(kind)
                .map(|tranche| tranche.share_supply.units())
                .unwrap_or_default();
            if account_supply > vault_supply {
                findings.push(AuditFinding::new(
                    AuditSeverity::High,
                    "account_supply_exceeds_vault",
                    format!("account supply exceeds vault supply for {}", kind.label()),
                ));
            }
        }
    }

    fn check_negative_drift(&self, vault: &VaultSnapshot, findings: &mut Vec<AuditFinding>) {
        if vault.total_redeemed
            > vault
                .total_deposited
                .checked_add(vault.total_fees)
                .unwrap_or(vault.total_deposited)
        {
            findings.push(AuditFinding::new(
                AuditSeverity::Medium,
                "redemption_drift",
                "redeemed amount is above deposited amount plus fee credit",
            ));
        }
        if vault.pricing_reserve_basis > vault.total_reserve && vault.total_losses.is_zero() {
            findings.push(AuditFinding::new(
                AuditSeverity::Low,
                "pricing_basis_above_reserve",
                "pricing reserve basis is above live reserve without recorded losses",
            ));
        }
    }
}
