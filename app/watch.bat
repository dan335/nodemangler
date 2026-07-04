@echo off
rem Rebuild and relaunch the GUI on source changes. Tests/clippy are run from
rem the editing terminal, not here. Watches only sources + manifests so files
rem written elsewhere while the app is running (saved graphs, exported images)
rem don't restart the loop.
set RUST_BACKTRACE=1
cargo watch --quiet --clear ^
    --watch crates --watch Cargo.toml --watch Cargo.lock ^
    --exec "run -p mangler_gui"
