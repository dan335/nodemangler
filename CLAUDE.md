# NodeMangler

Node-based visual programming tool for image and color manipulation.

## Repository Layout

- `app/` — Rust application (Cargo workspace)
- `website/` — Website (future)

## Project Structure

- `app/crates/mangler_core/` — Core library: value system, node graph engine, operations, color spaces
- `app/crates/mangler_cli/` — CLI tool for running graphs headless
- `app/crates/mangler_gui/` — GUI application built with egui/eframe

## Build & Test

```bash
cd app
cargo build          # Build all crates
cargo test           # Run all tests
cargo run -p mangler_gui  # Run the GUI app
cargo run -p mangler_cli  # Run the CLI tool
```

## Key Conventions

- **Stable Rust** toolchain (see `app/rust-toolchain.toml`)
- **Async-first**: tokio multi-threaded runtime; graph execution is async on a separate tokio task
- **Message-driven UI**: the GUI and engine communicate through four mpsc channel message types:
  - `ChangeGraphMessage` — UI → engine: add/remove nodes and connections, set save path/name
  - `ChangeNodeMessage` — UI → engine: set input values, positions, expose inputs/outputs
  - `GraphChangedMessage` — engine → UI: node/connection added, removed, or loaded
  - `NodeChangedMessage` — engine → UI: output values changed, thumbnails, busy/error status, timing
- **Async thumbnails**: `Value::Image` thumbnails are computed off the engine thread by `ThumbnailService` (see `thumbnail_service.rs`). Engine emits `OutputChanged { thumbnail: None }` for image outputs; a follow-up `NodeChangedMessage::ThumbnailReady` arrives when the resize finishes, with a stale-check keyed on `change_id`. Scalar/enum thumbnails are still computed inline.
- Operations are generated via the `operations!` macro in `app/crates/mangler_core/src/operations/mod.rs`
- **Value types** (see `value.rs`): Bool, Integer, Decimal, Text, Color, Image, Path, FilterType, ImageType, ColorFormat, Trigger, NoiseWorleyDistanceFunction, ColorSpace, BlendMode, TextHAlign, TextVAlign
- Images are `FloatImage` (1–4 channel `f32`, `Arc`-shared); `Value::Image { data, change_id }` carries a change id used by cache invalidation and stale-thumbnail rejection.
- Color is stored as sRGBA floats with conversions to 9 color spaces: sRGB, Linear RGB, HSL, HSV, Lab, LCH, CMYK, XYZ, YUV
- BlendMode has 17 modes: Over, Lerp, Multiply, Screen, Overlay, SoftLight, HardLight, ColorDodge, ColorBurn, Darken, Lighten, Difference, Exclusion, LinearBurn, LinearDodge, Divide, Subtract
- Subgraph support: nodes can contain entire graphs for composition
- Graphs serialize to JSON via `GraphSaveData`
- **No backwards compatibility for saved graphs.** Field renames, value-type splits, and output-order changes land without migration paths; old graphs re-wire or re-export.
- **Tests go in a separate `_tests.rs` file**: for a file `foo.rs`, place tests in `foo_tests.rs` in the same directory. Link them from the source file using:
  ```rust
  #[cfg(test)]
  #[path = "foo_tests.rs"]
  mod tests;
  ```
  This keeps source files short for LLM context while preserving access to private functions.

## Key Files

- `app/crates/mangler_core/src/lib.rs` — message enums and public API
- `app/crates/mangler_core/src/value.rs` — `Value` and `ValueType` enums, type conversions, fingerprinting
- `app/crates/mangler_core/src/graph.rs` — `Graph` struct: node storage, dirty tracking, async execution, save/load
- `app/crates/mangler_core/src/node.rs` — `Node::run` dispatches to op, emits `OutputChanged` / `ThumbnailReady` / `Busy` / `Error`
- `app/crates/mangler_core/src/app.rs` — engine-side `App`: spawns tokio task, processes change messages
- `app/crates/mangler_core/src/operations/mod.rs` — `operations!` macro, `Operation` enum, operation list
- `app/crates/mangler_core/src/thumbnail_service.rs` — async thumbnail worker with supersede-by-seq coalescing
- `app/crates/mangler_core/src/color/` — `Color` struct and color space conversions
- `app/crates/mangler_gui/src/main.rs` — entry point, eframe window setup
- `app/crates/mangler_gui/src/app.rs` — GUI `App`: manages programs, themes, menu bar
- `app/crates/mangler_gui/src/program.rs` — `Program`: owns engine instance + all UI panels for one graph
- `app/crates/mangler_gui/src/graph/` — graph editor canvas, node rendering, connections
- `app/crates/mangler_gui/src/themes/` — 4 themes: Dark, DarkGreen (default), Light, LightBlue

## Adding a New Operation

1. Create the operation struct in the appropriate `app/crates/mangler_core/src/operations/{category}/` directory
2. Implement `settings()`, `create_inputs()`, `create_outputs()`, and `async fn run()`
3. Register it in the `operations!` macro in `app/crates/mangler_core/src/operations/mod.rs`
4. Add it to the `operation_list()` function in the same file for it to appear in the node menu
5. Add `pub mod` in the parent category `mod.rs`
6. Add tests in a separate `{operation_name}_tests.rs` file, linked via `#[cfg(test)] #[path = "..."] mod tests;`

## Operation Categories

### numbers/
- `inputs/` — decimal, integer, e, pi, tau
- `arithmetic/` — add, subtract, multiply, divide, modulus, negate, min, max, average, clamp, floor, ceil, round, trunc, frac, sign, reciprocal, increment, decrement
- `algebra/` — abs, sqrt, cbrt, nth_root, pow, factorial, gcd, lcm
- `trigonometry/` — sin, cos, tan, asin, acos, atan, atan2, sinh, cosh, tanh
- `interpolation/` — lerp, map_range, smoothstep, step
- `logarithmic/` — exp, ln, log, log2, log10
- `bitwise/` — bit_and, bit_or, bit_xor, bit_not, bit_shift_left, bit_shift_right
- `random/` — random_decimal, random_integer
- `cast/` — to_decimal, to_integer

### logic/
- `inputs/` — bool
- `comparison/` — equal, not_equal, less_than, less_equal, greater_than, greater_equal
- `boolean/` — and, or, not, xor, nand, nor
- `flow/` — select (mux: picks between two values based on a bool condition)

### text/
- `inputs/` — text
- `manipulation/` — append, length, to_uppercase, to_lowercase, to_string
- (`text/text_from_clipboard.rs` exists but is an unimplemented stub — not a registered node)

### colors/
- `inputs/` — srgb, rgb_linear, hsl, hsv, lab, lch, cmyk, xyz, yuv (construct a color from each of the 9 color spaces)
- `outputs/` — to_srgb, to_rgb_linear, to_hsl, to_hsv, to_lab, to_lch, to_cmyk, to_xyz, to_yuv (decompose a color into a space's components)
- `generation/` — from_hex, to_hex, random_color
- `manipulation/` — adjust_hsv, clamp, grayscale, invert, set_alpha
- `relationship/` — complementary, analogous, triadic, tetradic, monochromatic, double_split_complementary
- `analysis/` — luminance, contrast_ratio, distance, color_temperature, dominant_hue, harmony_score, mix_ratio
- `blend/` — blend_mode
- `cast/` — to_color
- `sample_image/` — most_common_colors

### images/
- `inputs/` — file, url, clipboard, color, gradient, text
- `outputs/` — file, clipboard
- `transform/` — crop, resize, resize_exact, resize_fill, flip_horizontal, flip_vertical, rotate_90, rotate_180, rotate_270, rotate_around_center, warp, directional_warp, safe_transform, make_tile, mirror, seam_carve, polar_coordinates, swirl, kaleidoscope, spherize, perspective
- `adjustments/` — brighten, contrast, levels, auto_levels, curves, grayscale, invert, posterize, saturation, hue_rotate, hsl, threshold, vignette, white_balance, color_balance, selective_color, color_to_mask, replace_color, frequency_split, dither, gradient_map, gradient_dynamic, color_match, distance, histogram_scan, histogram_range, histogram_select (shared `smoothstep`/HSL helpers live in `adjustments/common.rs`)
- `blur/` — blur, directional_blur, radial_blur, slope_blur, non_uniform_blur
- `filter/` — edge_detect, canny, emboss, sharpen, unsharpen, highpass, luminance_highpass, dog (difference of gaussians), median, bilateral, guided, non_local_means, anisotropic_diffusion, kuwahara, anisotropic_kuwahara, snn, oil_paint, toon, cross_hatch, halftone, ascii, convolution (custom 3x3 kernel), dilate, erode, open, close, morphological_gradient, top_hat, black_hat, outline, pixelate, vector_morphology, floyd_steinberg, ordered_dither (morphology ops share `separable_morphology` from `erode.rs`)
- `fx/` — drop_shadow, inner_glow, outer_glow
- `combine/` — blit, blend, compare
- `channels/` — split, merge, shuffle, select, mixer
- `shapes/` — rectangle, ellipse, circle, polygon, star, line, cone, pyramid, paraboloid
- `patterns/` — brick, hexagonal, weave, tile_sampler, tile_generator, splatter, flood_fill, flood_fill_mapper
- `pbr/` — normal_from_height, normal_to_height, normal_invert, normal_blend, normal_combine, ao_from_height, curvature, bevel, height_blend
- `noise/` — 28 generators: perlin, value, fbm, billow, ridged_multifractal, hybrid_multifractal, basic_multifractal, open_simplex, super_simplex, voronoise, voronoi_crack, worley_distance, worley_value, gabor, anisotropic, gaussian (white noise), blue_noise, curl (flow map, 3-channel), wave, clouds, plasma, crystal, dirt, cylinders, checkerboard, erosion, reaction_diffusion, domain_warp_fbm (`voronoi_common.rs` is a shared helper, not a node; `pixel_hash`/`periodic_value_2d`/`build_perm_tables` in `noise/mod.rs` are shared)

## Known Issues

None currently.


---

## Implementation Pattern (for all new operations)

Every new operation follows the established pattern:

1. Create struct in appropriate directory with `#[derive(Debug, Clone, Serialize, Deserialize)]`
2. Implement `settings()` → `NodeSettings { name, description }`
3. Implement `create_inputs()` → `Vec<Input>` with `InputSettings` (Slider, DragValue, etc.)
4. Implement `create_outputs()` → `Vec<Output>`
5. Implement `async fn run(inputs)` using `convert_input()` + the 5-step pattern
6. Register in `operations!` macro in `app/crates/mangler_core/src/operations/mod.rs`
7. Add to `operation_list()` in appropriate category
8. Add `pub mod` in parent `mod.rs` files

**Key files to modify for every operation:**
- `app/crates/mangler_core/src/operations/mod.rs` — macro registration + menu
- Parent category `mod.rs` — module declaration

---

## Verification

After each phase (from `app/` directory):
- `cargo build` — must compile cleanly
- `cargo test -p mangler_core` — all existing tests pass
- `cargo run -p mangler_gui` — new nodes appear in menu, can be placed and connected
- Manual test: create a small graph exercising the new nodes, verify output images are correct
