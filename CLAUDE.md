# NodeMangler

Node-based visual programming tool for image and color manipulation.

## Project Structure

- `crates/mangler/` — Core library: value system, node graph engine, operations, color spaces
- `crates/nodemangler/` — GUI application built with egui/eframe

## Build & Test

```bash
cargo build          # Build all crates
cargo test           # Run all tests
cargo run -p nodemangler  # Run the GUI app
```

## Key Conventions

- **Nightly Rust** toolchain (see `rust-toolchain.toml`)
- **Async-first**: tokio multi-threaded runtime; graph execution is async on a separate tokio task
- **Message-driven UI**: the GUI and engine communicate through four mpsc channel message types:
  - `ChangeGraphMessage` — UI → engine: add/remove nodes and connections, set save path/name
  - `ChangeNodeMessage` — UI → engine: set input values, positions, expose inputs/outputs
  - `GraphChangedMessage` — engine → UI: node/connection added, removed, or loaded
  - `NodeChangedMessage` — engine → UI: output values changed, thumbnails, busy/error status, timing
- Operations are generated via the `operations!` macro in `crates/mangler/src/operations/mod.rs`
- Value types: Bool, Integer, Decimal, String, Color, DynamicImage, Path, FilterType, ImageType, ColorFormat, Trigger, NoiseWorleyDistanceFunction, ColorSpace, BlendMode
- Color is stored as sRGBA floats with conversions to 9 color spaces: sRGB, Linear RGB, HSL, HSV, Lab, LCH, CMYK, XYZ, YUV
- BlendMode has 17 modes: Normal, Lerp, Multiply, Screen, Overlay, SoftLight, HardLight, ColorDodge, ColorBurn, Darken, Lighten, Difference, Exclusion, LinearBurn, LinearDodge, Divide, Subtract
- Subgraph support: nodes can contain entire graphs for composition
- Graphs serialize to JSON via `GraphSaveData`
- **Tests belong in the source file**: place unit tests as `#[cfg(test)] mod tests` at the bottom of the file being tested, not in a separate `tests/` directory

## Key Files

- `crates/mangler/src/lib.rs` — message enums and public API
- `crates/mangler/src/value.rs` — `Value` and `ValueType` enums, type conversions, fingerprinting
- `crates/mangler/src/graph.rs` — `Graph` struct: node storage, dirty tracking, async execution, save/load
- `crates/mangler/src/app.rs` — engine-side `App`: spawns tokio task, processes change messages
- `crates/mangler/src/operations/mod.rs` — `operations!` macro, `Operation` enum, operation list
- `crates/mangler/src/color/` — `Color` struct and color space conversions
- `crates/nodemangler/src/main.rs` — entry point, eframe window setup
- `crates/nodemangler/src/app.rs` — GUI `App`: manages programs, themes, menu bar
- `crates/nodemangler/src/program.rs` — `Program`: owns engine instance + all UI panels for one graph
- `crates/nodemangler/src/graph/` — graph editor canvas, node rendering, connections
- `crates/nodemangler/src/themes/` — 4 themes: Dark, DarkGreen (default), Light, LightBlue

## Adding a New Operation

1. Create the operation struct in the appropriate `crates/mangler/src/operations/{category}/` directory
2. Implement `settings()`, `create_inputs()`, `create_outputs()`, and `async fn run()`
3. Register it in the `operations!` macro in `crates/mangler/src/operations/mod.rs`
4. Add it to the `operation_list()` function in the same file for it to appear in the node menu
5. Add `pub mod` in the parent category `mod.rs`
6. Add tests as a `#[cfg(test)] mod tests` block at the bottom of the source file

## Operation Categories

- `operations/numbers/` — inputs, arithmetic, algebra, random
- `operations/colors/` — inputs, outputs, blend, sample_image
- `operations/images/inputs/` — file, url, clipboard, color, gradient
- `operations/images/outputs/` — file, clipboard
- `operations/images/combine/` — blit, blend
- `operations/images/transform/` — crop, resize, flip, rotate, warp, directional_warp, safe_transform, make_tile, mirror
- `operations/images/adjustments/` — blur, contrast, grayscale, invert, brighten, hue_rotate, unsharpen, levels, curves, gradient_map
- `operations/images/channels/` — split, merge, shuffle
- `operations/images/noise/` — 14 noise generators
- `operations/logic/inputs/` — bool
- `operations/logic/comparison/` — equal, not_equal, less_than, less_equal, greater_than, greater_equal
- `operations/logic/boolean/` — and, or, not, xor, nand, nor
- `operations/logic/flow/` — select (mux: picks between two values based on a bool condition)

## Known Issues

None currently.
