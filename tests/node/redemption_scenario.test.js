import test from "node:test";
import assert from "node:assert/strict";
import { runScenario } from "../helpers/cargoScenario.js";

test("redemption scenario settles against the published tranche index", () => {
  const report = runScenario("redemptions");
  assert.equal(report.kind, "Redemptions");
  assert.equal(report.metrics.redemption_gross, 300_000);
  assert.equal(report.metrics.redemption_fee, 750);
  assert.equal(report.metrics.redemption_net, 299_250);
  assert.equal(report.metrics.junior_reserve, 300_000);
  assert.equal(report.metrics.fee_ledger_total, 750);
});
