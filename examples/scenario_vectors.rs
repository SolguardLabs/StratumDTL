use stratum_dtl::{
    build_deposit_scenario, build_fee_scenario, build_loss_scenario, build_redemption_scenario,
};

fn main() {
    let scenario = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "deposits".to_string());
    let report = match scenario.as_str() {
        "deposits" => build_deposit_scenario(),
        "losses" => build_loss_scenario(),
        "redemptions" => build_redemption_scenario(),
        "fees" => build_fee_scenario(),
        other => {
            eprintln!("unknown scenario: {other}");
            std::process::exit(2);
        }
    };

    match report {
        Ok(report) => {
            let json = serde_json::to_string_pretty(&report).expect("scenario must serialize");
            println!("{json}");
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
