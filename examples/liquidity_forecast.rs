use stratum_dtl::{
    AccountId, Amount, AssetConfig, AssetId, Bps, Commitment, FeePolicy, FeeSchedule, FeeSplit,
    LiquidityForecaster, StratumLedger, StressScenario, StressShock, TrancheKind, TranchePolicy,
    VaultConfig, VaultId,
};

fn main() -> stratum_dtl::StratumResult<()> {
    let mut ledger = StratumLedger::with_standard_config();
    let asset = AssetId::new("asset:unit")?;
    let vault = VaultId::new("vault:primary")?;
    ledger.register_asset(AssetConfig::new(
        asset.clone(),
        "UNIT",
        6,
        Amount::new(1)?,
        Amount::new(20_000_000)?,
    ));
    let fees = FeeSchedule::new(
        FeePolicy::new(Bps::zero(), Bps::zero(), Bps::zero()),
        FeeSplit::standard(),
    );
    ledger.open_vault(VaultConfig::new(
        vault.clone(),
        asset,
        AccountId::new("account:operator")?,
        TranchePolicy::balanced(),
        fees,
    ))?;
    for (name, tranche, amount) in [
        ("senior", TrancheKind::Senior, 5_000_000),
        ("junior", TrancheKind::Junior, 1_000_000),
        ("insurance", TrancheKind::Insurance, 750_000),
        ("overflow", TrancheKind::Overflow, 250_000),
    ] {
        ledger.deposit(
            AccountId::new(format!("account:{name}"))?,
            vault.clone(),
            tranche,
            Amount::new(amount)?,
        )?;
    }
    let snapshot = ledger.vault_snapshot(&vault)?;
    let scenario = StressScenario::new("junior-40pct")
        .with_shock(StressShock::tranche(TrancheKind::Junior, Bps::new(4_000)?));
    let commitments = [
        Commitment {
            tranche: TrancheKind::Junior,
            due: Amount::new(500_000)?,
        },
        Commitment {
            tranche: TrancheKind::Senior,
            due: Amount::new(1_000_000)?,
        },
    ];
    let forecast = LiquidityForecaster.evaluate(
        &snapshot,
        &TranchePolicy::balanced(),
        &scenario,
        &commitments,
    )?;
    println!(
        "{}",
        serde_json::to_string_pretty(&forecast).expect("serializable forecast")
    );
    Ok(())
}
