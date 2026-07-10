#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "$0")/toolchain.sh"

cargo test --locked
if [[ -n "${NODE_BIN:-}" ]]; then
  "$NODE_BIN" --test "tests/node/*.test.js"
elif [[ "${CI:-}" == "true" ]]; then
  echo "node is required for JavaScript tests" >&2
  exit 127
else
  echo "node not available in this bash environment; run node --test \"tests/node/*.test.js\" from a shell with Node." >&2
fi
