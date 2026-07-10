# StratumDTL

![banner](./assets/banner.png)

StratumDTL is a Rust library for liquidity-layered vaults used by distributed
transfer ledgers. A vault can split reserves into senior, junior, insurance and
overflow tranches, then expose consistent APIs for deposits, loss windows,
redemptions, fee distribution and solvency reporting.

The crate is designed as an audit target. The public surface models realistic
accounting flows without external services, chain RPCs or database processes.

## Components

- `amount`: fixed precision amounts, shares, basis points and share indexes.
- `tranche`: tranche classes, policy weights, status flags and reserve state.
- `vault`: vault lifecycle, deposits, redemptions, loss windows and snapshots.
- `accounts`: holder positions and redemption/deposit accounting.
- `fees`: schedule-based fee assessment and distribution ledgers.
- `risk`: solvency limits, liquidity reports and policy admission checks.
- `ledger`: orchestration layer over vaults, accounts, events and fees.
- `simulation`: deterministic scenario builders used by integration tests.

## Requirements

- Rust 1.96 or newer.
- Node.js 24 or newer for JavaScript integration checks.

## Commands

```bash
cargo build --all-targets --locked
cargo test --locked
node --test "tests/node/*.test.js"
```

For a full local CI pass:

```bash
bash scripts/ci.sh
```

## Example Scenarios

The example binary emits deterministic JSON vectors for downstream consumers:

```bash
cargo run --quiet --example scenario_vectors -- deposits
cargo run --quiet --example scenario_vectors -- losses
cargo run --quiet --example scenario_vectors -- redemptions
cargo run --quiet --example scenario_vectors -- fees
```

## Project Layout

```text
src/       Rust library modules
tests/     Rust integration tests and Node.js scenario checks
scripts/   Local CI and test entry points
examples/  JSON scenario vector emitter
```

## Security Model

StratumDTL assumes vault controllers configure tranche policy, fee schedules and
risk limits before user-facing operations are admitted. Account holders interact
through the ledger API, which records journal events and balance changes for
each operation.

The implementation favors deterministic integer math and explicit state
transitions so accounting can be reviewed without off-chain dependencies.
