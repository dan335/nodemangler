# NodeMangler

Node-based visual programming tool for image and color manipulation.

## Project Structure

- `crates/mangler_core/` — Core library: value system, node graph engine, operations, color spaces
- `crates/mangler_tui/` — CLI tool for running graphs headless
- `crates/mangler_gui/` — GUI application built with egui/eframe

## Build & Test

```bash
cargo build          # Build all crates
cargo test           # Run all tests
cargo run -p mangler_gui  # Run the GUI app
cargo run -p mangler_tui  # Run the CLI tool
```

## Key Conventions

- **Nightly Rust** toolchain (see `rust-toolchain.toml`)
- **Async-first**: tokio multi-threaded runtime; graph execution is async on a separate tokio task
- **Message-driven UI**: the GUI and engine communicate through four mpsc channel message types:
  - `ChangeGraphMessage` — UI → engine: add/remove nodes and connections, set save path/name
  - `ChangeNodeMessage` — UI → engine: set input values, positions, expose inputs/outputs
  - `GraphChangedMessage` — engine → UI: node/connection added, removed, or loaded
  - `NodeChangedMessage` — engine → UI: output values changed, thumbnails, busy/error status, timing
- Operations are generated via the `operations!` macro in `crates/mangler_core/src/operations/mod.rs`
- Value types: Bool, Integer, Decimal, String, Color, DynamicImage, Path, FilterType, ImageType, ColorFormat, Trigger, NoiseWorleyDistanceFunction, ColorSpace, BlendMode
- Color is stored as sRGBA floats with conversions to 9 color spaces: sRGB, Linear RGB, HSL, HSV, Lab, LCH, CMYK, XYZ, YUV
- BlendMode has 17 modes: Normal, Lerp, Multiply, Screen, Overlay, SoftLight, HardLight, ColorDodge, ColorBurn, Darken, Lighten, Difference, Exclusion, LinearBurn, LinearDodge, Divide, Subtract
- Subgraph support: nodes can contain entire graphs for composition
- Graphs serialize to JSON via `GraphSaveData`
- **Tests belong in the source file**: place unit tests as `#[cfg(test)] mod tests` at the bottom of the file being tested, not in a separate `tests/` directory

## Key Files

- `crates/mangler_core/src/lib.rs` — message enums and public API
- `crates/mangler_core/src/value.rs` — `Value` and `ValueType` enums, type conversions, fingerprinting
- `crates/mangler_core/src/graph.rs` — `Graph` struct: node storage, dirty tracking, async execution, save/load
- `crates/mangler_core/src/app.rs` — engine-side `App`: spawns tokio task, processes change messages
- `crates/mangler_core/src/operations/mod.rs` — `operations!` macro, `Operation` enum, operation list
- `crates/mangler_core/src/color/` — `Color` struct and color space conversions
- `crates/mangler_gui/src/main.rs` — entry point, eframe window setup
- `crates/mangler_gui/src/app.rs` — GUI `App`: manages programs, themes, menu bar
- `crates/mangler_gui/src/program.rs` — `Program`: owns engine instance + all UI panels for one graph
- `crates/mangler_gui/src/graph/` — graph editor canvas, node rendering, connections
- `crates/mangler_gui/src/themes/` — 4 themes: Dark, DarkGreen (default), Light, LightBlue

## Adding a New Operation

1. Create the operation struct in the appropriate `crates/mangler_core/src/operations/{category}/` directory
2. Implement `settings()`, `create_inputs()`, `create_outputs()`, and `async fn run()`
3. Register it in the `operations!` macro in `crates/mangler_core/src/operations/mod.rs`
4. Add it to the `operation_list()` function in the same file for it to appear in the node menu
5. Add `pub mod` in the parent category `mod.rs`
6. Add tests as a `#[cfg(test)] mod tests` block at the bottom of the source file

## Operation Categories

- `operations/numbers/` — inputs, arithmetic, interpolation, algebra, trigonometry, random, cast, logarithmic, bitwise
- `operations/colors/` — inputs, outputs, blend, analysis (sample_image), cast
- `operations/images/inputs/` — file, url, clipboard, color, gradient
- `operations/images/outputs/` — file, clipboard
- `operations/images/combine/` — blit, blend
- `operations/images/transform/` — crop, resize, flip, rotate, warp, directional_warp, safe_transform, make_tile, mirror
- `operations/images/adjustments/` — contrast, grayscale, invert, brighten, hue_rotate, posterize, levels, auto_levels, curves, gradient_map, histogram_scan, histogram_range, distance
- `operations/images/blur/` — blur, directional_blur, radial_blur, slope_blur, non_uniform_blur
- `operations/images/filter/` — edge_detect, emboss, sharpen, unsharpen
- `operations/images/channels/` — split, merge, shuffle
- `operations/images/shapes/` — rectangle, ellipse, polygon, star, line
- `operations/images/patterns/` — brick, hexagonal, weave, tile_sampler
- `operations/images/pbr/` — normal_from_height, ao_from_height, curvature, height_blend
- `operations/images/noise/` — 14 noise generators
- `operations/logic/inputs/` — bool
- `operations/logic/comparison/` — equal, not_equal, less_than, less_equal, greater_than, greater_equal
- `operations/logic/boolean/` — and, or, not, xor, nand, nor
- `operations/logic/flow/` — select (mux: picks between two values based on a bool condition)
- `operations/text/` — text_from_clipboard (disabled/WIP)

## Known Issues

None currently.
