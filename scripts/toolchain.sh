#!/usr/bin/env bash

if ! command -v cargo >/dev/null 2>&1; then
  candidates=()
  if [[ -n "${HOME:-}" ]]; then
    candidates+=("$HOME/.cargo/bin")
  fi
  if [[ -n "${USERPROFILE:-}" ]] && command -v cygpath >/dev/null 2>&1; then
    candidates+=("$(cygpath "$USERPROFILE")/.cargo/bin")
  fi
  candidates+=("/c/Users/${USERNAME:-}/.cargo/bin")

  for candidate in "${candidates[@]}"; do
    if [[ -x "$candidate/cargo" || -x "$candidate/cargo.exe" ]]; then
      export PATH="$candidate:$PATH"
      break
    fi
  done
fi

if ! command -v node >/dev/null 2>&1; then
  node_candidates=(
    "/mnt/c/Program Files/nodejs"
    "/c/Program Files/nodejs"
  )
  if [[ -n "${ProgramFiles:-}" ]] && command -v cygpath >/dev/null 2>&1; then
    node_candidates+=("$(cygpath "$ProgramFiles")/nodejs")
  fi
  if [[ -n "${ProgramW6432:-}" ]] && command -v cygpath >/dev/null 2>&1; then
    node_candidates+=("$(cygpath "$ProgramW6432")/nodejs")
  fi

  for node_dir in "${node_candidates[@]}"; do
    if [[ -x "$node_dir/node" || -x "$node_dir/node.exe" ]]; then
      export PATH="$node_dir:$PATH"
      break
    fi
  done
fi

NODE_BIN="${NODE_BIN:-}"
if [[ -z "$NODE_BIN" ]] && command -v node >/dev/null 2>&1; then
  NODE_BIN="node"
elif [[ -z "$NODE_BIN" ]] && command -v node.exe >/dev/null 2>&1; then
  if node.exe --version >/dev/null 2>&1; then
    NODE_BIN="node.exe"
  fi
fi
export NODE_BIN
