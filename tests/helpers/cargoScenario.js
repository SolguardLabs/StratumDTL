import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");

export function runScenario(name) {
  const targetDir = path.join(root, "target", `node-${name.replaceAll(/[^a-z0-9_-]/gi, "_")}`);
  const result = spawnSync(
    "cargo",
    ["run", "--quiet", "--target-dir", targetDir, "--example", "scenario_vectors", "--", name],
    {
      cwd: root,
      encoding: "utf8",
    },
  );
  if (result.status !== 0) {
    throw new Error(result.stderr || `scenario ${name} failed`);
  }
  return JSON.parse(result.stdout);
}
