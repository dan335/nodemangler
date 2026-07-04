#!/usr/bin/env bash
# Build release executables (mangler_gui + mangler_cli) for THIS machine's OS.
# Cross-platform release builds happen in GitHub Actions — see scripts/release.sh.
set -euo pipefail
cd "$(dirname "$0")/../app"
cargo build --release -p mangler_gui -p mangler_cli
echo
echo "Binaries: app/target/release/mangler_gui and app/target/release/mangler_cli"
