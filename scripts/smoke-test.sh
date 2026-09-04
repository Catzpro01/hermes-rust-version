#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
export HERMES_HOME=$(mktemp -d)
trap 'rm -rf "$HERMES_HOME"' EXIT
cd "$ROOT"
cargo build --release -p hermes-rs
printf 'hello\n/exit\n' | ./target/release/hermes-rs --provider fake
echo "✅ Smoke test passed"
