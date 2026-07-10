use crate::StratumResult;
use crate::amount::{Amount, Bps, Shares};
use crate::asset::AssetConfig;
use crate::fees::{FeePolicy, FeeSchedule, FeeSplit};
use crate::ids::{AccountId, AssetId, VaultId};
use crate::ledger::StratumLedger;
use crate::risk::RiskReport;
use crate::snapshot::{AccountSnapshot, VaultSnapshot};
use crate::tranche::{TrancheKind, TranchePolicy};
use crate::vault::VaultConfig;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScenarioKind {
    Deposits,
    Losses,
    Redemptions,
    Fees,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioReport {
    pub kind: ScenarioKind,
    pub vault: VaultSnapshot,
    pub accounts: Vec<AccountSnapshot>,
    pub risk: RiskReport,
    pub metrics: BTreeMap<String, u128>,
    pub journal_entries: usize,
}

pub fn build_deposit_scenario() -> StratumResult<ScenarioReport> {
    let mut fixture = Fixture::new()?;
    fixture.seed_deposits()?;
    fixture.report(ScenarioKind::Deposits)
}

pub fn build_loss_scenario() -> StratumResult<ScenarioReport> {
    let mut fixture = Fixture::new()?;
    fixture.seed_deposits()?;
    let receipt = fixture
        .ledger
        .apply_loss(fixture.vault_id.clone(), Amount::new(400_000)?)?;
    fixture
        .ledger
        .publish_loss_indices(fixture.vault_id.clone())?;
    let mut report = fixture.report(ScenarioKind::Losses)?;
    report
        .metrics
        .insert("loss_absorbed".to_string(), receipt.absorbed_loss.units());
    report.metrics.insert(
        "loss_allocations".to_string(),
        receipt.allocations.len() as u128,
    );
    Ok(report)
}

pub fn build_redemption_scenario() -> StratumResult<ScenarioReport> {
    let mut fixture = Fixture::new()?;
    fixture.seed_deposits()?;
    fixture
        .ledger
        .apply_loss(fixture.vault_id.clone(), Amount::new(400_000)?)?;
    fixture
        .ledger
        .publish_loss_indices(fixture.vault_id.clone())?;
    let receipt = fixture.ledger.redeem(
        fixture.junior_lp.clone(),
        fixture.vault_id.clone(),
        TrancheKind::Junior,
        Shares::new(500_000)?,
    )?;
    let mut report = fixture.report(ScenarioKind::Redemptions)?;
    report
        .metrics
        .insert("redemption_gross".to_string(), receipt.gross_paid.units());
    report
        .metrics
        .insert("redemption_net".to_string(), receipt.net_paid.units());
    report
        .metrics
        .insert("redemption_fee".to_string(), receipt.fee.units());
    Ok(report)
}

pub fn build_fee_scenario() -> StratumResult<ScenarioReport> {
    let mut fixture = Fixture::new()?;
    fixture.seed_deposits()?;
    let distributed = fixture
        .ledger
        .distribute_fee_credit(fixture.vault_id.clone(), Amount::new(100_000)?)?;
    let mut report = fixture.report(ScenarioKind::Fees)?;
    for (kind, amount) in distributed {
        report
            .metrics
            .insert(format!("fee_credit_{}", kind.label()), amount.units());
    }
    Ok(report)
}

struct Fixture {
    ledger: StratumLedger,
    asset_id: AssetId,
    vault_id: VaultId,
    senior_lp: AccountId,
    junior_lp: AccountId,
    insurance_lp: AccountId,
    overflow_lp: AccountId,
}

impl Fixture {
    fn new() -> StratumResult<Self> {
        let asset_id = AssetId::new("asset:usdc")?;
        let vault_id = VaultId::new("vault:stratum-main")?;
        let controller = AccountId::new("account:controller")?;
        let senior_lp = AccountId::new("account:senior-lp")?;
        let junior_lp = AccountId::new("account:junior-lp")?;
        let insurance_lp = AccountId::new("account:insurance-lp")?;
        let overflow_lp = AccountId::new("account:overflow-lp")?;
        let mut ledger = StratumLedger::with_standard_config();
        let asset = AssetConfig::new(
            asset_id.clone(),
            "USDC",
            6,
            Amount::new(1)?,
            Amount::new(20_000_000)?,
        );
        ledger.register_asset(asset);

        let fee_schedule = FeeSchedule::new(
            FeePolicy::new(Bps::zero(), Bps::new(25).expect("valid bps"), Bps::zero()),
            FeeSplit::standard(),
        );
        let vault = VaultConfig::new(
            vault_id.clone(),
            asset_id.clone(),
            controller,
            TranchePolicy::balanced(),
            fee_schedule,
        );
        ledger.open_vault(vault)?;
        Ok(Self {
            ledger,
            asset_id,
            vault_id,
            senior_lp,
            junior_lp,
            insurance_lp,
            overflow_lp,
        })
    }

    fn seed_deposits(&mut self) -> StratumResult<()> {
        self.ledger.deposit(
            self.senior_lp.clone(),
            self.vault_id.clone(),
            TrancheKind::Senior,
            Amount::new(5_000_000)?,
        )?;
        self.ledger.deposit(
            self.junior_lp.clone(),
            self.vault_id.clone(),
            TrancheKind::Junior,
            Amount::new(1_000_000)?,
        )?;
        self.ledger.deposit(
            self.insurance_lp.clone(),
            self.vault_id.clone(),
            TrancheKind::Insurance,
            Amount::new(750_000)?,
        )?;
        self.ledger.deposit(
            self.overflow_lp.clone(),
            self.vault_id.clone(),
            TrancheKind::Overflow,
            Amount::new(250_000)?,
        )?;
        Ok(())
    }

    fn report(&self, kind: ScenarioKind) -> StratumResult<ScenarioReport> {
        let vault = self.ledger.vault_snapshot(&self.vault_id)?;
        let risk = self.ledger.risk_report(&self.vault_id)?;
        let mut metrics = BTreeMap::new();
        metrics.insert("total_reserve".to_string(), vault.total_reserve.units());
        metrics.insert(
            "pricing_reserve_basis".to_string(),
            vault.pricing_reserve_basis.units(),
        );
        metrics.insert(
            "senior_reserve".to_string(),
            vault
                .tranche(TrancheKind::Senior)
                .map(|snapshot| snapshot.reserve.units())
                .unwrap_or_default(),
        );
        metrics.insert(
            "junior_reserve".to_string(),
            vault
                .tranche(TrancheKind::Junior)
                .map(|snapshot| snapshot.reserve.units())
                .unwrap_or_default(),
        );
        metrics.insert(
            "insurance_reserve".to_string(),
            vault
                .tranche(TrancheKind::Insurance)
                .map(|snapshot| snapshot.reserve.units())
                .unwrap_or_default(),
        );
        metrics.insert(
            "overflow_reserve".to_string(),
            vault
                .tranche(TrancheKind::Overflow)
                .map(|snapshot| snapshot.reserve.units())
                .unwrap_or_default(),
        );
        metrics.insert(
            "fee_ledger_total".to_string(),
            self.ledger.fee_ledger().total_assessed().units(),
        );
        metrics.insert(
            "asset_registered".to_string(),
            u128::from(self.asset_id.as_str() == "asset:usdc"),
        );
        let accounts = vec![
            self.ledger.account_snapshot(&self.senior_lp)?,
            self.ledger.account_snapshot(&self.junior_lp)?,
            self.ledger.account_snapshot(&self.insurance_lp)?,
            self.ledger.account_snapshot(&self.overflow_lp)?,
        ];
        Ok(ScenarioReport {
            kind,
            vault,
            accounts,
            risk,
            metrics,
            journal_entries: self.ledger.journal().len(),
        })
    }
}
