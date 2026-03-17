# NodeMangler

Node-based visual programming tool for image and color manipulation.

## Project Structure

- `crates/mangler/` — Core library: value system, node graph engine, 200+ operations, color spaces
- `crates/nodemangler/` — GUI application built with egui/eframe

## Build & Test

```bash
cargo build          # Build all crates
cargo test           # Run all tests
cargo run -p nodemangler  # Run the GUI app
```

## Key Conventions

- **Nightly Rust** toolchain (see `rust-toolchain.toml`)
- **Async-first**: tokio multi-threaded runtime; graph execution is async
- **Message-driven UI**: changes flow through mpsc channels (`ChangeGraphMessage`, `ChangeNodeMessage`, etc.)
- Operations are generated via the `operations!` macro in `crates/mangler/src/operations/mod.rs`
- Value types: Bool, Integer, Decimal, String, Color, DynamicImage, Path, FilterType, ImageType, ColorFormat, Trigger, NoiseWorleyDistanceFunction, ColorSpace, BlendMode
- Color is stored as sRGBA floats with conversions to many color spaces (HSL, HSV, Lab, LCH, CMYK, XYZ, YUV, etc.)
- Subgraph support: nodes can contain entire graphs for composition
