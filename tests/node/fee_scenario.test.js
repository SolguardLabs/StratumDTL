import test from "node:test";
import assert from "node:assert/strict";
import { runScenario } from "../helpers/cargoScenario.js";

test("fee scenario distributes credit by target tranche weights", () => {
  const report = runScenario("fees");
  assert.equal(report.kind, "Fees");
  assert.equal(report.metrics.fee_credit_senior, 55_000);
  assert.equal(report.metrics.fee_credit_junior, 25_000);
  assert.equal(report.metrics.fee_credit_insurance, 15_000);
  assert.equal(report.metrics.fee_credit_overflow, 5_000);
  assert.equal(report.metrics.total_reserve, 7_100_000);
});
