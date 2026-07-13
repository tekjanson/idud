#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "[quality] checking formatting"
cargo fmt --all -- --check

echo "[quality] running clippy"
cargo clippy --all-targets --all-features

echo "[quality] running repository hygiene tests"
cargo test --test repo_hygiene

echo "[quality] running functional pipeline tests"
cargo test --test e2e_contract_pipeline --test integration_dependency_analysis
