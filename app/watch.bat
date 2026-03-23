set RUST_BACKTRACE=1
cargo watch --quiet --clear --exec clippy --exec "run -p mangler_gui"
