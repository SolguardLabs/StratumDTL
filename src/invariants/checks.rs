use crate::amount::{Amount, ShareIndex, Shares};
use crate::snapshot::{AccountSnapshot, VaultSnapshot};
use crate::tranche::TrancheKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvariantViolation {
    pub code: String,
    pub context: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvariantReport {
    pub violations: Vec<InvariantViolation>,
    pub checked_accounts: usize,
    pub checked_positions: usize,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct InvariantSuite;

impl InvariantViolation {
    pub fn new(code: impl Into<String>, context: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            context: context.into(),
        }
    }
}

impl InvariantReport {
    pub fn is_valid(&self) -> bool {
        self.violations.is_empty()
    }

    pub fn count_code(&self, code: &str) -> usize {
        self.violations
            .iter()
            .filter(|violation| violation.code == code)
            .count()
    }
}

impl InvariantSuite {
    pub fn check(&self, vault: &VaultSnapshot, accounts: &[AccountSnapshot]) -> InvariantReport {
        let mut violations = Vec::new();
        self.check_total_reserve(vault, &mut violations);
        self.check_total_supply(vault, &mut violations);
        self.check_share_indexes(vault, &mut violations);
        self.check_account_positions(vault, accounts, &mut violations);
        let checked_positions = accounts.iter().map(|account| account.positions.len()).sum();
        InvariantReport {
            violations,
            checked_accounts: accounts.len(),
            checked_positions,
        }
    }

    fn check_total_reserve(&self, vault: &VaultSnapshot, violations: &mut Vec<InvariantViolation>) {
        let mut total = Amount::zero();
        for tranche in &vault.tranches {
            match total.checked_add(tranche.reserve) {
                Ok(next) => total = next,
                Err(_) => {
                    violations.push(InvariantViolation::new(
                        "reserve_overflow",
                        tranche.kind.label(),
                    ));
                    return;
                }
            }
        }
        if total != vault.total_reserve {
            violations.push(InvariantViolation::new(
                "reserve_total",
                format!("expected {} got {}", vault.total_reserve, total),
            ));
        }
    }

    fn check_total_supply(&self, vault: &VaultSnapshot, violations: &mut Vec<InvariantViolation>) {
        let mut total = Shares::zero();
        for tranche in &vault.tranches {
            match total.checked_add(tranche.share_supply) {
                Ok(next) => total = next,
                Err(_) => {
                    violations.push(InvariantViolation::new(
                        "share_supply_overflow",
                        tranche.kind.label(),
                    ));
                    return;
                }
            }
        }
        if total != vault.total_share_supply {
            violations.push(InvariantViolation::new(
                "share_supply_total",
                format!("expected {} got {}", vault.total_share_supply, total),
            ));
        }
    }

    fn check_share_indexes(&self, vault: &VaultSnapshot, violations: &mut Vec<InvariantViolation>) {
        for tranche in &vault.tranches {
            if tranche.share_supply.is_zero() {
                continue;
            }
            let expected =
                ShareIndex::from_reserve_and_supply(tranche.reserve, tranche.share_supply);
            match expected {
                Ok(index) if index == tranche.share_index => {}
                Ok(index) => {
                    if tranche.pending_loss.is_zero() {
                        violations.push(InvariantViolation::new(
                            "share_index",
                            format!(
                                "{} expected {} got {}",
                                tranche.kind.label(),
                                index,
                                tranche.share_index
                            ),
                        ));
                    }
                }
                Err(_) => violations.push(InvariantViolation::new(
                    "share_index_overflow",
                    tranche.kind.label(),
                )),
            }
        }
    }

    fn check_account_positions(
        &self,
        vault: &VaultSnapshot,
        accounts: &[AccountSnapshot],
        violations: &mut Vec<InvariantViolation>,
    ) {
        for kind in TrancheKind::public_order() {
            let mut total = Shares::zero();
            for account in accounts {
                for (position_vault, position_tranche, shares, _, _) in &account.positions {
                    if position_vault == &vault.id && *position_tranche == kind {
                        match total.checked_add(*shares) {
                            Ok(next) => total = next,
                            Err(_) => {
                                violations.push(InvariantViolation::new(
                                    "account_share_overflow",
                                    kind.label(),
                                ));
                                return;
                            }
                        }
                    }
                }
            }
            let vault_supply = vault
                .tranche(kind)
                .map(|tranche| tranche.share_supply)
                .unwrap_or_else(Shares::zero);
            if total > vault_supply {
                violations.push(InvariantViolation::new(
                    "account_supply",
                    format!("{} account {} vault {}", kind.label(), total, vault_supply),
                ));
            }
        }
    }
}
