use stratum_dtl::{
    AccountId, Amount, AssetConfig, AssetId, Bps, Commitment, FeePolicy, FeeSchedule, FeeSplit,
    LiquidityForecaster, StratumLedger, StressScenario, StressShock, TrancheKind, TranchePolicy,
    VaultConfig, VaultId,
};

fn funded_vault() -> (StratumLedger, VaultId) {
    let mut ledger = StratumLedger::with_standard_config();
    let asset = AssetId::new("asset:unit").unwrap();
    let vault = VaultId::new("vault:forecast").unwrap();
    ledger.register_asset(AssetConfig::new(
        asset.clone(),
        "UNIT",
        6,
        Amount::new(1).unwrap(),
        Amount::new(10_000).unwrap(),
    ));
    let fees = FeeSchedule::new(
        FeePolicy::new(Bps::zero(), Bps::zero(), Bps::zero()),
        FeeSplit::standard(),
    );
    ledger
        .open_vault(VaultConfig::new(
            vault.clone(),
            asset,
            AccountId::new("account:controller").unwrap(),
            TranchePolicy::balanced(),
            fees,
        ))
        .unwrap();
    for (label, kind, amount) in [
        ("senior", TrancheKind::Senior, 5_000),
        ("junior", TrancheKind::Junior, 1_000),
        ("insurance", TrancheKind::Insurance, 750),
        ("overflow", TrancheKind::Overflow, 250),
    ] {
        ledger
            .deposit(
                AccountId::new(format!("account:{label}")).unwrap(),
                vault.clone(),
                kind,
                Amount::new(amount).unwrap(),
            )
            .unwrap();
    }
    (ledger, vault)
}

#[test]
fn forecast_aggregates_commitments_and_preserves_tranche_isolation() {
    let (ledger, vault) = funded_vault();
    let snapshot = ledger.vault_snapshot(&vault).unwrap();
    let scenario = StressScenario::new("junior-stress").with_shock(StressShock::tranche(
        TrancheKind::Junior,
        Bps::new(4_000).unwrap(),
    ));
    let forecast = LiquidityForecaster
        .evaluate(
            &snapshot,
            &TranchePolicy::balanced(),
            &scenario,
            &[
                Commitment {
                    tranche: TrancheKind::Junior,
                    due: Amount::new(300).unwrap(),
                },
                Commitment {
                    tranche: TrancheKind::Junior,
                    due: Amount::new(200).unwrap(),
                },
                Commitment {
                    tranche: TrancheKind::Senior,
                    due: Amount::new(1_000).unwrap(),
                },
            ],
        )
        .unwrap();
    assert_eq!(forecast.starting_reserve.units(), 7_000);
    assert_eq!(forecast.stressed_reserve.units(), 6_600);
    assert_eq!(forecast.total_commitments.units(), 1_500);
    let junior = forecast.tranche(TrancheKind::Junior).unwrap();
    assert_eq!(junior.policy_floor.units(), 330);
    assert_eq!(junior.available.units(), 270);
    assert_eq!(junior.shortfall.units(), 230);
    assert_eq!(junior.coverage_bps.value(), 5_400);
    assert_eq!(forecast.total_shortfall.units(), 230);
    assert!(!forecast.fully_covered());
}

#[test]
fn forecast_without_commitments_is_covered() {
    let (ledger, vault) = funded_vault();
    let snapshot = ledger.vault_snapshot(&vault).unwrap();
    let forecast = LiquidityForecaster
        .evaluate(
            &snapshot,
            &TranchePolicy::balanced(),
            &StressScenario::new("base"),
            &[],
        )
        .unwrap();
    assert!(forecast.fully_covered());
    assert_eq!(forecast.total_shortfall.units(), 0);
    assert_eq!(
        forecast
            .tranche(TrancheKind::Senior)
            .unwrap()
            .coverage_bps
            .value(),
        10_000
    );
}
