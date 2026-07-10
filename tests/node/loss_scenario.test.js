import test from "node:test";
import assert from "node:assert/strict";
import { runScenario } from "../helpers/cargoScenario.js";

test("loss scenario publishes adjusted junior reserves", () => {
  const report = runScenario("losses");
  assert.equal(report.kind, "Losses");
  assert.equal(report.metrics.loss_absorbed, 400_000);
  assert.equal(report.metrics.loss_allocations, 1);
  assert.equal(report.metrics.total_reserve, 6_600_000);
  assert.equal(report.metrics.junior_reserve, 600_000);
  assert.equal(report.vault.tranches.find((item) => item.kind === "Junior").share_index, 600_000_000_000);
});
