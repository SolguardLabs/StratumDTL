use crate::accounts::AccountState;
use crate::amount::{Amount, Shares};
use crate::asset::{AssetConfig, AssetRegistry};
use crate::error::{StratumError, StratumResult};
use crate::events::{JournalEntry, JournalEvent, JournalRecord};
use crate::fees::FeeLedger;
use crate::ids::{AccountId, Epoch, VaultId};
use crate::risk::{RiskDecision, RiskEngine, RiskLimits, RiskReport};
use crate::snapshot::{AccountSnapshot, VaultSnapshot};
use crate::tranche::TrancheKind;
use crate::vault::{DepositReceipt, LossReceipt, RedemptionReceipt, VaultConfig, VaultState};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerConfig {
    pub auto_create_accounts: bool,
    pub risk_limits: RiskLimits,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerReport {
    pub epoch: Epoch,
    pub vault_count: usize,
    pub account_count: usize,
    pub total_fee_assessed: Amount,
    pub journal_entries: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StratumLedger {
    config: LedgerConfig,
    assets: AssetRegistry,
    vaults: BTreeMap<VaultId, VaultState>,
    accounts: BTreeMap<AccountId, AccountState>,
    fee_ledger: FeeLedger,
    journal: JournalRecord,
    risk: RiskEngine,
    epoch: Epoch,
}

impl LedgerConfig {
    pub fn standard() -> Self {
        Self {
            auto_create_accounts: true,
            risk_limits: RiskLimits::default(),
        }
    }
}

impl Default for LedgerConfig {
    fn default() -> Self {
        Self::standard()
    }
}

impl StratumLedger {
    pub fn new(config: LedgerConfig) -> Self {
        Self {
            risk: RiskEngine::new(config.risk_limits),
            config,
            assets: AssetRegistry::new(),
            vaults: BTreeMap::new(),
            accounts: BTreeMap::new(),
            fee_ledger: FeeLedger::new(),
            journal: JournalRecord::new(),
            epoch: Epoch::genesis(),
        }
    }

    pub fn with_standard_config() -> Self {
        Self::new(LedgerConfig::standard())
    }

    pub const fn epoch(&self) -> Epoch {
        self.epoch
    }

    pub const fn fee_ledger(&self) -> &FeeLedger {
        &self.fee_ledger
    }

    pub const fn journal(&self) -> &JournalRecord {
        &self.journal
    }

    pub const fn risk_engine(&self) -> &RiskEngine {
        &self.risk
    }

    pub fn register_asset(&mut self, config: AssetConfig) {
        self.assets.register(config);
    }

    pub fn open_vault(&mut self, config: VaultConfig) -> StratumResult<JournalEntry> {
        self.assets.get(config.asset_id())?.ensure_active()?;
        let vault = VaultState::new(config)?;
        let vault_id = vault.id().clone();
        self.vaults.insert(vault_id.clone(), vault);
        Ok(self
            .journal
            .push(self.epoch, JournalEvent::VaultOpened { vault_id }))
    }

    pub fn ensure_account(&mut self, account: AccountId) {
        self.accounts
            .entry(account.clone())
            .or_insert_with(|| AccountState::new(account));
    }

    pub fn vault(&self, vault_id: &VaultId) -> StratumResult<&VaultState> {
        self.vaults
            .get(vault_id)
            .ok_or_else(|| StratumError::VaultNotFound(vault_id.clone()))
    }

    pub fn vault_mut(&mut self, vault_id: &VaultId) -> StratumResult<&mut VaultState> {
        self.vaults
            .get_mut(vault_id)
            .ok_or_else(|| StratumError::VaultNotFound(vault_id.clone()))
    }

    pub fn account(&self, account_id: &AccountId) -> StratumResult<&AccountState> {
        self.accounts
            .get(account_id)
            .ok_or_else(|| StratumError::AccountNotFound(account_id.clone()))
    }

    pub fn account_mut(&mut self, account_id: &AccountId) -> StratumResult<&mut AccountState> {
        self.accounts
            .get_mut(account_id)
            .ok_or_else(|| StratumError::AccountNotFound(account_id.clone()))
    }

    pub fn deposit(
        &mut self,
        account: AccountId,
        vault_id: VaultId,
        tranche: TrancheKind,
        amount: Amount,
    ) -> StratumResult<DepositReceipt> {
        if self.config.auto_create_accounts {
            self.ensure_account(account.clone());
        }
        let asset_id = self.vault(&vault_id)?.asset_id().clone();
        self.assets.get(&asset_id)?.validate_deposit(amount)?;
        let decision = self.risk.admit_deposit(self.vault(&vault_id)?, amount)?;
        if decision == RiskDecision::Reject {
            return Err(StratumError::RiskRejected);
        }
        let (receipt, assessment) =
            self.vault_mut(&vault_id)?
                .deposit(account.clone(), tranche, amount)?;
        self.fee_ledger
            .record(asset_id, vault_id.clone(), assessment)?;
        self.account_mut(&account)?.credit_deposit(
            vault_id.clone(),
            tranche,
            receipt.shares,
            receipt.net,
            receipt.fee,
        )?;
        self.journal.push(
            self.epoch,
            JournalEvent::DepositAccepted {
                account,
                vault_id,
                tranche,
                gross: receipt.gross,
                net: receipt.net,
                shares: receipt.shares,
            },
        );
        Ok(receipt)
    }

    pub fn apply_loss(&mut self, vault_id: VaultId, loss: Amount) -> StratumResult<LossReceipt> {
        let receipt = self.vault_mut(&vault_id)?.apply_loss(loss)?;
        self.journal.push(
            self.epoch,
            JournalEvent::LossRecorded {
                vault_id,
                loss,
                absorbed: receipt.absorbed_loss,
            },
        );
        Ok(receipt)
    }

    pub fn publish_loss_indices(&mut self, vault_id: VaultId) -> StratumResult<JournalEntry> {
        self.vault_mut(&vault_id)?.publish_loss_indices()?;
        self.epoch = self.epoch.next();
        Ok(self.journal.push(
            self.epoch,
            JournalEvent::IndexPublished {
                vault_id,
                epoch: self.epoch,
            },
        ))
    }

    pub fn redeem(
        &mut self,
        account: AccountId,
        vault_id: VaultId,
        tranche: TrancheKind,
        shares: Shares,
    ) -> StratumResult<RedemptionReceipt> {
        let owned = self.account(&account)?.total_shares_for(&vault_id, tranche);
        if owned < shares {
            return Err(StratumError::InsufficientShares);
        }
        let asset_id = self.vault(&vault_id)?.asset_id().clone();
        let plan = self
            .vault(&vault_id)?
            .quote_redemption(account.clone(), tranche, shares)?;
        let decision = self.risk.admit_redemption(self.vault(&vault_id)?, &plan)?;
        if decision == RiskDecision::Reject {
            return Err(StratumError::RiskRejected);
        }
        let assessment = self
            .vault(&vault_id)?
            .config()
            .fee_schedule()
            .assess_redemption(tranche, plan.quoted_gross)?;
        let receipt = self.vault_mut(&vault_id)?.redeem(plan)?;
        self.fee_ledger
            .record(asset_id, vault_id.clone(), assessment)?;
        self.account_mut(&account)?.debit_redemption(
            vault_id.clone(),
            tranche,
            shares,
            receipt.net_paid,
            receipt.fee,
        )?;
        self.journal.push(
            self.epoch,
            JournalEvent::RedemptionSettled {
                account,
                vault_id,
                tranche,
                shares,
                gross: receipt.gross_paid,
                net: receipt.net_paid,
            },
        );
        Ok(receipt)
    }

    pub fn distribute_fee_credit(
        &mut self,
        vault_id: VaultId,
        amount: Amount,
    ) -> StratumResult<BTreeMap<TrancheKind, Amount>> {
        let distributed = self.vault_mut(&vault_id)?.distribute_fee_credit(amount)?;
        self.journal.push(
            self.epoch,
            JournalEvent::FeesDistributed { vault_id, amount },
        );
        Ok(distributed)
    }

    pub fn risk_report(&self, vault_id: &VaultId) -> StratumResult<RiskReport> {
        self.risk.evaluate(self.vault(vault_id)?)
    }

    pub fn vault_snapshot(&self, vault_id: &VaultId) -> StratumResult<VaultSnapshot> {
        VaultSnapshot::from_state(self.vault(vault_id)?)
    }

    pub fn account_snapshot(&self, account_id: &AccountId) -> StratumResult<AccountSnapshot> {
        Ok(AccountSnapshot::from_state(self.account(account_id)?))
    }

    pub fn report(&self) -> LedgerReport {
        LedgerReport {
            epoch: self.epoch,
            vault_count: self.vaults.len(),
            account_count: self.accounts.len(),
            total_fee_assessed: self.fee_ledger.total_assessed(),
            journal_entries: self.journal.len(),
        }
    }
}

impl Default for StratumLedger {
    fn default() -> Self {
        Self::with_standard_config()
    }
}
