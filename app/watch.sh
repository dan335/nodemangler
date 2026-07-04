#!/bin/sh
# Rebuild and relaunch the GUI on source changes. Tests/clippy are run from
# the editing terminal, not here. Watches only sources + manifests so files
# written elsewhere while the app is running (saved graphs, exported images)
# don't restart the loop.
export RUST_BACKTRACE=1
exec cargo watch --quiet --clear \
    --watch crates --watch Cargo.toml --watch Cargo.lock \
    --exec "run -p mangler_gui"
