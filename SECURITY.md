# Security Policy

## Scope

This repository contains a local Rust library for vault accounting, tranche
liquidity and deterministic integration tests. The intended review scope is the
crate source, tests, scripts and documented library behavior.

## Expected Invariants

- Vault reserves must remain non-negative after deposits, losses, redemptions
  and fee distribution.
- Account share balances must not exceed the tranche supply recorded by the
  vault.
- Redemptions must respect tranche status, risk limits and available reserve
  policy.
- Fees must be recorded in the fee ledger before they are distributed.
- Loss windows must be journaled and reflected in solvency reports.
- Snapshot serialization must be deterministic for audit tooling.

## Automated Validation

Local validation is handled by:

```bash
bash scripts/ci.sh
```

The CI script runs formatting checks, compilation, Rust tests, clippy and
JavaScript integration checks.

## Dependency Management

Rust dependencies are pinned through `Cargo.lock`. JavaScript tests use the
built-in Node.js test runner and do not require runtime dependencies.

## Reporting Format

Internal findings should include:

- affected module and function;
- exact input sequence;
- observed state transition;
- expected invariant;
- economic impact;
- proposed remediation and regression coverage.
