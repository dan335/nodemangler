set RUST_BACKTRACE=1
cargo watch --quiet --clear --exec test --exec clippy --exec "run -p mangler_gui"
