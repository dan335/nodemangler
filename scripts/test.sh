#!/usr/bin/env bash
# Run the full test suite. Extra args are passed to cargo test.
set -euo pipefail
cd "$(dirname "$0")/../app"
cargo test --workspace "$@"
