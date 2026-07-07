# NodeMangler

Node-based visual programming tool for image and color manipulation.

## Repository Layout

- `app/` — Rust application (Cargo workspace)
- `website/` — Website (future)
- `scripts/` — test/build/release scripts (`.sh` + `.bat`); see `scripts/README.md`

## Versioning & Releases

- The project version lives in **one place**: `[workspace.package] version` in
  `app/Cargo.toml`. All crates inherit it via `version.workspace = true`.
- `scripts/release.sh <version>` (or `release.bat`) runs tests, bumps the version,
  commits, tags `vX.Y.Z`, and pushes. The tag triggers
  `.github/workflows/release.yml`, which builds Windows/Linux/macOS executables on
  native runners and publishes them to GitHub Releases.

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
- **Menu hierarchy mirrors file hierarchy**: the node-menu category tree built in `operation_list()` must match the directory tree under `operations/`. When an operation moves to a different menu (sub)category, move its `.rs` and `_tests.rs` files into the matching directory and update its module path. (Known exception: the adjustments' distance node is listed under the filter/morphology menu.)
- **Value types** (see `value.rs`): Bool, Integer, Decimal, Text, Color, Image, Path, FilterType, ImageType, ColorFormat, Trigger, NoiseWorleyDistanceFunction, ColorSpace, BlendMode, EdgeMode (transform edge-fill: Fill/Wrap/Extend/Mirror), TextHAlign, TextVAlign, ExportPreset (material export target: Godot/Unity/Unreal/Custom)
- Images are `FloatImage` (1–4 channel `f32`, `Arc`-shared); `Value::Image { data, change_id }` carries a change id used by cache invalidation and stale-thumbnail rejection.
- **Resolution-independent spatial params**: any input that is a spatial size/radius/offset in pixels (blur sigma, morphology radius, glow/shadow size, warp displacement, cell size, etc.) is authored as **pixels at a 1024px reference** and scaled to the actual image in `run()` via `scale_to_resolution(value, w, h)` (= `value * max(w,h) / REFERENCE_RESOLUTION`), both in `operations/mod.rs`. This keeps the same value producing the same *relative* effect at any resolution (design at 512px, render at 4096px, don't touch the numbers). Integer radii use `.round().max(1.0) as <T>` (floor at 1); "0 = off" radii keep 0; per-axis offsets scale by `width`/`height` directly; pixel *areas* scale by the ratio squared. `transform`'s `offset x/y` is the exception — a plain fraction of image size (0.5 = half across). Output dimensions, resize targets, and crop coords stay in real pixels (not scaled). Tests that assert an effect on a small image either make the max dimension 1024 (scaling becomes identity) or pass a value bumped by `1024/dim`.
- Color is stored as sRGBA floats with conversions to 14 color spaces: sRGB, Linear RGB, HSL, HSV, HWB, Lab, LCH, Oklab, Oklch, CMYK, XYZ, xyY, YCbCr, YUV
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
- `app/crates/mangler_gui/src/app.rs` — GUI `App`: manages programs, themes, menu bar, panel tree + secondary windows
- `app/crates/mangler_gui/src/program.rs` — `Program`: owns engine instance + per-panel content (`update` / `show_panel` / `show_overlays`) for one graph
- `app/crates/mangler_gui/src/panels/` — Blender-style panel system: `panel_tree.rs` (recursive split tree, pure logic), `panel_view.rs` (splitters, kind-switcher chrome, focus), `panel_windows.rs` (secondary OS windows via immediate viewports). Each panel hosts one of 5 `PanelKind` contexts (Graph, Preview2D, Preview3D, NodeList, Settings); layout is app-level and persisted to config as `default_layout`; split/close in the settings menu act on the focused (last-hovered) panel
- `app/crates/mangler_gui/src/graph/` — graph editor canvas, node rendering, connections
- `app/crates/mangler_gui/src/view_window/` — panel content viewers: `preview_2d.rs` (image/color/text value dispatch), `preview_3d.rs` (PBR material view, GL renderer, arcball)
- `app/crates/mangler_gui/src/themes/` — 4 themes: Dark, DarkGreen (default), Light, LightBlue; all panel chrome must derive colors from `theme.get()` (no hardcoded colors)

## Adding a New Operation

1. Create the operation struct in the appropriate `app/crates/mangler_core/src/operations/{category}/` directory
2. Implement `settings()`, `create_inputs()`, `create_outputs()`, and `async fn run()`
3. Register it in the `operations!` macro in `app/crates/mangler_core/src/operations/mod.rs`
4. Add it to the `operation_list()` function in the same file for it to appear in the node menu
5. Add `pub mod` in the parent category `mod.rs`
6. Add tests in a separate `{operation_name}_tests.rs` file, linked via `#[cfg(test)] #[path = "..."] mod tests;`

## Operation Categories

### numbers/
- `inputs/` — decimal, integer, e, pi, tau, phi
- `arithmetic/` — add, subtract, multiply, divide, modulus, negate, min, max, average, clamp, floor, ceil, round, trunc, frac, sign, reciprocal, increment, decrement, snap (round to step), wrap (fold into [min,max)), ping_pong (triangle fold)
- `algebra/` — abs, sqrt, cbrt, nth_root, pow, factorial, gcd, lcm, hypot, distance_2d
- `trigonometry/` — sin, cos, tan, asin, acos, atan, atan2, sinh, cosh, tanh, to_degrees, to_radians, asinh, acosh, atanh
- `interpolation/` — lerp, map_range, smoothstep, step
- `logarithmic/` — exp, ln, log, log2, log10
- `bitwise/` — bit_and, bit_or, bit_xor, bit_not, bit_shift_left, bit_shift_right
- `random/` — random_decimal, random_integer, random_gaussian (Box–Muller normal)
- `cast/` — to_decimal, to_integer
- `image/` — image→number measurements (each takes an image, emits numbers; lives under numbers because it *produces* numbers). Shared `pixel_luma`/`pixel_rgba`/`luma_values` helpers in `numbers/image/mod.rs`. Nodes: dimensions, mean, min_max, median, percentile, std_dev, entropy, skewness, kurtosis, bounding_box, centroid, coverage, sharpness (variance of Laplacian), edge_density (Sobel), unique_colors, average_hue, image_difference (MSE/RMSE/MAE/PSNR), perceptual_hash (dHash Hamming). (The color-producing `sample pixel` node lives under `colors/sample_image/` instead.)
- `text/` — text→number (same output-type rule): parse_decimal, parse_integer, word_count, line_count, byte_length (UTF-8 bytes; distinct from `text/manipulation/length`'s char count), index_of, count_occurrences.

### logic/
- `inputs/` — bool
- `comparison/` — equal, not_equal, approx_equal (tolerance-based), in_range (inclusive min/max), less_than, less_equal, greater_than, greater_equal
- `boolean/` — and, or, not, xor, xnor, nand, nor
- `flow/` — select (mux: picks between two values based on a bool condition)
- `text/` — text→bool predicates (output-type rule): contains, starts_with, ends_with, is_empty, equals_ignore_case

### text/
- `inputs/` — text
- `manipulation/` — append, length, to_uppercase, to_lowercase, to_string, join, replace, substring, split, trim, pad, repeat, reverse, template ({}-placeholder substitution), title_case, format_number (number→text)
- `image/` — image→text (categorized under text because they *output* text): ascii_art, data_uri (base64 PNG data URI), image_info, palette_hex (dominant colors as hex), image_hash (average-hash). Reuse `pixel_luma`/`pixel_rgba` from `numbers/image/`.
- `encoding/` — base64_encode, base64_decode, url_encode, url_decode. Self-contained base64 codec (`base64_encode`/`base64_decode`) lives in `text/encoding/mod.rs` — no base64 crate dependency; also used by `text/image/data_uri`.
- (`text/text_from_clipboard.rs` exists but is an unimplemented stub — not a registered node)

### colors/
- `inputs/` — srgb, rgb_linear, hsl, hsv, hwb, lab, lch, oklab, oklch, cmyk, xyz, xyy, ycbcr, yuv (construct a color from each of the 14 color spaces)
- `outputs/` — to_srgb, to_rgb_linear, to_hsl, to_hsv, to_hwb, to_lab, to_lch, to_oklab, to_oklch, to_cmyk, to_xyz, to_xyy, to_ycbcr, to_yuv (decompose a color into a space's components)
- `generation/` — from_hex, to_hex, random_color
- `manipulation/` — adjust_hsv, clamp, grayscale, invert, set_alpha
- `relationship/` — complementary, analogous, triadic, tetradic, monochromatic, double_split_complementary
- `analysis/` — luminance, contrast_ratio, distance, color_temperature, dominant_hue, harmony_score, mix_ratio
- `blend/` — blend_mode
- `cast/` — to_color
- `sample_image/` — most_common_colors, sample_pixel (reads the color at a normalized x/y coordinate; menu-listed under colors→analysis alongside most_common_colors)

### images/
- `inputs/` — file, url, clipboard, color, gradient, text, constant (number-driven solid grayscale fill)
- `outputs/` — file, clipboard, material (channel-packed PBR texture export with engine presets)
- `transform/` — transform (combined affine: translate px + rotate + scale about centre, with fill/wrap/extend/mirror edge modes; replaced the old translate-only and wrap-only `safe_transform`/"tiling transform" nodes), crop, resize, resize_exact, resize_fill, flip_horizontal, flip_vertical, rotate_90, rotate_180, rotate_270, rotate_around_center, warp, directional_warp, make_tile, mirror, seam_carve, polar_coordinates, swirl, kaleidoscope, spherize, perspective
- `adjustments/` — brighten, contrast, levels, auto_levels, curves, grayscale, invert, posterize, saturation, hue_rotate, hsl, threshold, vignette, white_balance, color_balance, selective_color, color_to_mask, replace_color, frequency_split, dither, gradient_map, gradient_dynamic, color_match, distance, histogram_scan, histogram_range, histogram_select (shared `smoothstep`/HSL helpers live in `adjustments/common.rs`)
- `blur/` — blur, directional_blur, radial_blur, slope_blur, non_uniform_blur
- `filter/` — subdirectories mirror the node-menu subcategories; convolution (custom 3x3 kernel) sits at the filter root
  - `edges/` — edge_detect, canny, dog (difference of gaussians), sharpen, unsharpen, highpass, luminance_highpass
  - `smoothing/` — median, bilateral, guided, non_local_means, anisotropic_diffusion, snn
  - `morphology/` — erode, dilate, open, close, morphological_gradient, top_hat, black_hat, vector_morphology, outline (all share `separable_morphology` from `erode.rs`; the menu's morphology group also lists adjustments' distance node)
  - `stylize/` — emboss, kuwahara, anisotropic_kuwahara, toon, oil_paint, halftone, cross_hatch, ascii, pixelate
  - `dither/` — ordered_dither, floyd_steinberg
- `fx/` — drop_shadow, inner_glow, outer_glow
- `combine/` — blit, blend, compare
- `channels/` — split, merge, shuffle, select, mixer
- `shapes/` — rectangle, ellipse, circle, polygon, star, line, cone, pyramid, paraboloid
- `patterns/` — brick, hexagonal, weave, tile_sampler, tile_generator, splatter, flood_fill, flood_fill_mapper
- `pbr/` — normal_from_height, normal_to_height, normal_invert, normal_blend, normal_combine, ao_from_height, curvature, bevel, height_blend
- `simulation/` — physical-process simulation generators (more planned in `PLAN.md`). Category conventions: guidance-map image inputs (weakness, fuel, moisture, height) are optional and fall back to an internal seed-derived map, so every simulation node also works standalone (`is_unconnected`/`guidance_map_to_grid` helpers in `simulation/mod.rs`); input order is seed/width/height, guidance maps, then the main drivers (iteration-style counts like iterations/droplets/particles first — users step through them to watch the sim work), then fine-tuning params last. Nodes: hydraulic_erosion (faithful Beyer/Lague droplet sim; sequential single-threaded, erosion brush, single height output). (Frost DLA, drying-cracks, and sand-ripples nodes were built and deleted 2026-07 — not good enough; see PLAN.md. Each simulation node's `settings().help` should say what real model it's based on, or admit it's a heuristic — do not oversell heuristics as physical simulations.)
- `noise/` — 45 generators in subdirectories mirroring the node-menu subcategories (`voronoi_common.rs` at the noise root is a shared helper, not a node; `pixel_hash`/`periodic_perlin_2d`/`periodic_value_2d`/`build_perm_tables` in `noise/mod.rs` are shared)
  - `basic/` — perlin, value, open_simplex, super_simplex, gabor, phasor, anisotropic, gaussian (white noise), blue_noise
  - `fractal/` — fbm, billow, ridged_multifractal, basic_multifractal, hybrid_multifractal, domain_warp_fbm, flow (rotated-gradient fbm with advection), curl (flow map, 3-channel), clouds, plasma
  - `cellular/` — worley_distance, worley_value, voronoise, voronoi_crack, crystal, scales, craters
  - `structural/` — checkerboard, cylinders, wave, truchet (truchet tiles), warped_rings (fbm-warped concentric rings, non-tiling), veins (warped vein stripes)
  - `grunge/` — dirt, scratches, fibers, leaks (drip streaks, alignment control), stains (coffee-ring rims), peeling (flaking-paint mask), smear (soft directional streaks), growth (clustered organic patches)
  - `process/` — erosion, fault_terrain, reaction_diffusion, caustics (refraction simulation), lightning (branching filaments, non-tiling)

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
