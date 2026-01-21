#!/usr/bin/env bash
set -euo pipefail

workspace_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$workspace_root"

if ! command -v cargo-deny >/dev/null 2>&1; then
  echo "cargo-deny is required for supply-chain scanning (install with: cargo install cargo-deny)" >&2
  exit 1
fi

if ! command -v cargo-audit >/dev/null 2>&1; then
  echo "cargo-audit is required for vulnerability scanning (install with: cargo install cargo-audit)" >&2
  exit 1
fi

cargo deny check --workspace --all-features
cargo audit
