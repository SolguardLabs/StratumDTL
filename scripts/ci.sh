#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "$0")/toolchain.sh"

cargo fmt --all -- --check
cargo build --all-targets --locked
cargo test --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
if [[ -n "${NODE_BIN:-}" ]]; then
  "$NODE_BIN" --test "tests/node/*.test.js"
elif [[ "${CI:-}" == "true" ]]; then
  echo "node is required for JavaScript tests" >&2
  exit 127
else
  echo "node not available in this bash environment; run node --test \"tests/node/*.test.js\" from a shell with Node." >&2
fi
