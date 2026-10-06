#!/bin/sh
# Rebuild and relaunch the GUI on source changes. Tests/clippy are run from
# the editing terminal, not here. Watches only sources + manifests so files
# written elsewhere while the app is running (saved graphs, exported images)
# don't restart the loop. --restart kills the running GUI before relaunching.
#
# Watch paths are absolute on purpose: watchexec resolves relative --watch
# paths against the project origin it discovers (the repo root, one level up),
# not the working directory, so `--watch crates` fails with "No such file or
# directory".
cd "$(dirname "$0")" || exit 1
export RUST_BACKTRACE=1
exec watchexec --restart --clear --quiet \
    --watch "$PWD/crates" --watch "$PWD/Cargo.toml" --watch "$PWD/Cargo.lock" \
    -- cargo run -p mangler_gui
