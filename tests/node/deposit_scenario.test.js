import test from "node:test";
import assert from "node:assert/strict";
import { runScenario } from "../helpers/cargoScenario.js";

test("deposit scenario reports all tranche reserves", () => {
  const report = runScenario("deposits");
  assert.equal(report.kind, "Deposits");
  assert.equal(report.metrics.total_reserve, 7_000_000);
  assert.equal(report.metrics.senior_reserve, 5_000_000);
  assert.equal(report.metrics.junior_reserve, 1_000_000);
  assert.equal(report.metrics.insurance_reserve, 750_000);
  assert.equal(report.metrics.overflow_reserve, 250_000);
  assert.equal(report.accounts.length, 4);
});
