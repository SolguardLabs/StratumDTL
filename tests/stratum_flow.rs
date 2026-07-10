use stratum_dtl::{
    AccountId, Amount, AssetConfig, AuditEngine, Bps, FeePolicy, FeeSchedule, FeeSplit,
    InvariantSuite, Shares, StratumLedger, TrancheKind, TranchePolicy, VaultConfig, VaultId,
};

fn fixture() -> (
    StratumLedger,
    VaultId,
    AccountId,
    AccountId,
    AccountId,
    AccountId,
) {
    let asset_id = stratum_dtl::AssetId::new("asset:usdc").unwrap();
    let vault_id = VaultId::new("vault:main").unwrap();
    let controller = AccountId::new("account:controller").unwrap();
    let senior = AccountId::new("account:senior").unwrap();
    let junior = AccountId::new("account:junior").unwrap();
    let insurance = AccountId::new("account:insurance").unwrap();
    let overflow = AccountId::new("account:overflow").unwrap();
    let mut ledger = StratumLedger::with_standard_config();
    ledger.register_asset(AssetConfig::new(
        asset_id.clone(),
        "USDC",
        6,
        Amount::new(1).unwrap(),
        Amount::new(20_000_000).unwrap(),
    ));
    let schedule = FeeSchedule::new(
        FeePolicy::new(Bps::zero(), Bps::new(25).unwrap(), Bps::zero()),
        FeeSplit::standard(),
    );
    ledger
        .open_vault(VaultConfig::new(
            vault_id.clone(),
            asset_id,
            controller,
            TranchePolicy::balanced(),
            schedule,
        ))
        .unwrap();
    (ledger, vault_id, senior, junior, insurance, overflow)
}

fn seed(ledger: &mut StratumLedger, vault: &VaultId, accounts: &[AccountId; 4]) {
    ledger
        .deposit(
            accounts[0].clone(),
            vault.clone(),
            TrancheKind::Senior,
            Amount::new(5_000_000).unwrap(),
        )
        .unwrap();
    ledger
        .deposit(
            accounts[1].clone(),
            vault.clone(),
            TrancheKind::Junior,
            Amount::new(1_000_000).unwrap(),
        )
        .unwrap();
    ledger
        .deposit(
            accounts[2].clone(),
            vault.clone(),
            TrancheKind::Insurance,
            Amount::new(750_000).unwrap(),
        )
        .unwrap();
    ledger
        .deposit(
            accounts[3].clone(),
            vault.clone(),
            TrancheKind::Overflow,
            Amount::new(250_000).unwrap(),
        )
        .unwrap();
}

#[test]
fn deposits_mint_expected_tranche_shares() {
    let (mut ledger, vault, senior, junior, insurance, overflow) = fixture();
    seed(
        &mut ledger,
        &vault,
        &[
            senior.clone(),
            junior.clone(),
            insurance.clone(),
            overflow.clone(),
        ],
    );
    let snapshot = ledger.vault_snapshot(&vault).unwrap();
    assert_eq!(snapshot.total_reserve.units(), 7_000_000);
    assert_eq!(
        snapshot
            .tranche(TrancheKind::Senior)
            .unwrap()
            .share_supply
            .units(),
        5_000_000,
    );
    assert_eq!(
        snapshot
            .tranche(TrancheKind::Junior)
            .unwrap()
            .share_supply
            .units(),
        1_000_000,
    );
    assert_eq!(ledger.account_snapshot(&senior).unwrap().deposit_count, 1);
}

#[test]
fn loss_window_reduces_junior_and_publication_updates_index() {
    let (mut ledger, vault, senior, junior, insurance, overflow) = fixture();
    seed(&mut ledger, &vault, &[senior, junior, insurance, overflow]);
    let receipt = ledger
        .apply_loss(vault.clone(), Amount::new(400_000).unwrap())
        .unwrap();
    assert_eq!(receipt.absorbed_loss.units(), 400_000);
    assert_eq!(receipt.allocations[0].tranche, TrancheKind::Junior);
    assert_eq!(receipt.allocations[0].reserve_after.units(), 600_000);

    ledger.publish_loss_indices(vault.clone()).unwrap();
    let snapshot = ledger.vault_snapshot(&vault).unwrap();
    assert_eq!(snapshot.total_reserve.units(), 6_600_000);
    assert_eq!(
        snapshot
            .tranche(TrancheKind::Junior)
            .unwrap()
            .share_index
            .raw(),
        600_000_000_000,
    );
}

#[test]
fn redemption_after_published_loss_uses_adjusted_index() {
    let (mut ledger, vault, senior, junior, insurance, overflow) = fixture();
    seed(
        &mut ledger,
        &vault,
        &[
            senior.clone(),
            junior.clone(),
            insurance.clone(),
            overflow.clone(),
        ],
    );
    ledger
        .apply_loss(vault.clone(), Amount::new(400_000).unwrap())
        .unwrap();
    ledger.publish_loss_indices(vault.clone()).unwrap();
    let receipt = ledger
        .redeem(
            junior.clone(),
            vault.clone(),
            TrancheKind::Junior,
            Shares::new(500_000).unwrap(),
        )
        .unwrap();
    assert_eq!(receipt.gross_paid.units(), 300_000);
    assert_eq!(receipt.fee.units(), 750);
    assert_eq!(receipt.net_paid.units(), 299_250);
    assert_eq!(receipt.tranche_paid(TrancheKind::Junior).units(), 300_000);
}

#[test]
fn fee_credit_distribution_follows_policy_targets() {
    let (mut ledger, vault, senior, junior, insurance, overflow) = fixture();
    seed(&mut ledger, &vault, &[senior, junior, insurance, overflow]);
    let distributed = ledger
        .distribute_fee_credit(vault.clone(), Amount::new(100_000).unwrap())
        .unwrap();
    assert_eq!(distributed[&TrancheKind::Senior].units(), 55_000);
    assert_eq!(distributed[&TrancheKind::Junior].units(), 25_000);
    assert_eq!(distributed[&TrancheKind::Insurance].units(), 15_000);
    assert_eq!(distributed[&TrancheKind::Overflow].units(), 5_000);
}

#[test]
fn snapshots_pass_public_invariant_and_audit_checks() {
    let (mut ledger, vault, senior, junior, insurance, overflow) = fixture();
    seed(
        &mut ledger,
        &vault,
        &[
            senior.clone(),
            junior.clone(),
            insurance.clone(),
            overflow.clone(),
        ],
    );
    let vault_snapshot = ledger.vault_snapshot(&vault).unwrap();
    let accounts = vec![
        ledger.account_snapshot(&senior).unwrap(),
        ledger.account_snapshot(&junior).unwrap(),
        ledger.account_snapshot(&insurance).unwrap(),
        ledger.account_snapshot(&overflow).unwrap(),
    ];
    let invariant_report = InvariantSuite.check(&vault_snapshot, &accounts);
    assert!(invariant_report.is_valid());
    let audit_report = AuditEngine.review(&vault_snapshot, &accounts);
    assert!(audit_report.is_clean());
}
