# NodeMangler

Node-based visual programming tool for image and color manipulation.

## Repository Layout

- `app/` — Rust application (Cargo workspace)
- `website/` — Project marketing site: standalone Rust crate (`mangler_site`, not in the `app/` workspace) serving static files with axum + tower-http; deployed via Docker to a host running Traefik
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
- **Value types** (see `value.rs`): Bool, Integer, Decimal, Text, Color, Image, Path, FilterType, ImageType, ColorFormat, Trigger, NoiseWorleyDistanceFunction, ColorSpace, BlendMode, EdgeMode (transform edge-fill: Fill/Wrap/Extend/Mirror), TextHAlign, TextVAlign, ExportPreset (material export target: Godot/Unity/Unreal/Custom), Curve (user-drawn path/shape)
- **Curve** (`mangler_core/src/curve.rs`): `Value::Curve(Curve)` — a user-drawn 2D path in normalized [0,1]² y-down coords: `points: Vec<[f32;2]>` + `closed: bool` + `interpolation` (Linear | Smooth = centripetal Catmull-Rom through the points | Bezier = cubic spans with one **mirrored** tangent handle per anchor) + `handles: Vec<[f32;2]>` (`#[serde(default)]`; per-anchor tangent *offsets* — out-handle = anchor+h, in-handle = anchor−h, so anchors are C¹ by construction and moving an anchor carries its handles; entries missing/misaligned fall back to `auto_handle` = uniform Catmull-Rom tangent `(next−prev)/6`, making Smooth→Bezier shape-preserving). Plain derive-serde (points persist in graph saves, unlike images). Single subpath — use multiple curve nodes for multiple paths. **Edited as a Preview2D overlay, not in the settings panel**: when the settings-panel node (`Program.editing_node_id`) has an unconnected Curve input, every 2D preview panel overlays the curve on whatever image it shows (`mangler_gui/src/view_window/curve_overlay.rs`) — drag anchors to move, click to insert/append, double/right-click to delete (floor 2 points), in Bezier mode drag either mirrored tangent knob (registered before anchors so anchors win overlapping hits), closed/interpolation controls in a corner strip; empty-space drag still pans (egui resolves click and drag hits independently; handles are topmost drag targets). Edits mutate the local input value per-frame but send `ChangeNodeMessage::SetInput` only on gesture end, so heavy downstream nodes re-run once per drag. Settings panel/`preview_2d` show a summary via `curve_summary`; no image displayed → letterboxed square fallback canvas.
- Images are `FloatImage` (1–4 channel `f32`, `Arc`-shared); `Value::Image { data, change_id }` carries a change id used by cache invalidation and stale-thumbnail rejection.
- **Hidden (settings-panel-only) inputs**: `Input::hidden_in_graph()` sets `Input.hide_in_graph` — the graph editor draws no connection dot/row for that input (rows compact over visible inputs; hidden ones can't be connected or auto-connected), but the node settings panel still shows it. `#[serde(skip)]`, re-derived from `create_inputs()` on load like `default_value`. Used for config-only inputs that would bloat a node, e.g. the material node's 20 Custom-slot inputs (12..=31).
- **Resolution-independent spatial params**: any input that is a spatial size/radius/offset in pixels (blur sigma, morphology radius, glow/shadow size, warp displacement, cell size, etc.) is authored as **pixels at a 1024px reference** and scaled to the actual image in `run()` via `scale_to_resolution(value, w, h)` (= `value * max(w,h) / REFERENCE_RESOLUTION`), both in `operations/mod.rs`. This keeps the same value producing the same *relative* effect at any resolution (design at 512px, render at 4096px, don't touch the numbers). Integer radii use `.round().max(1.0) as <T>` (floor at 1); "0 = off" radii keep 0; per-axis offsets scale by `width`/`height` directly; pixel *areas* scale by the ratio squared. `transform`'s `offset x/y` is the exception — a plain fraction of image size (0.5 = half across). Output dimensions, resize targets, and crop coords stay in real pixels (not scaled). Tests that assert an effect on a small image either make the max dimension 1024 (scaling becomes identity) or pass a value bumped by `1024/dim`.
- Color is stored as sRGBA floats with conversions to 14 color spaces: sRGB, Linear RGB, HSL, HSV, HWB, Lab, LCH, Oklab, Oklch, CMYK, XYZ, xyY, YCbCr, YUV
- BlendMode has 17 modes: Over, Lerp, Multiply, Screen, Overlay, SoftLight, HardLight, ColorDodge, ColorBurn, Darken, Lighten, Difference, Exclusion, LinearBurn, LinearDodge, Divide, Subtract
- Subgraph support: nodes can contain entire graphs for composition
- **Filename is the source of truth for a graph's name**: the tab/display name always derives from the file stem (helpers in `mangler_core/src/naming.rs`: `GRAPH_EXTENSION`, `graph_display_name`, `sanitize_name` — spaces allowed, `graph_file_name`); the embedded `GraphSaveData.name` is a write-only mirror ignored on load. In-app rename sends `ChangeGraphMessage::RenameFile { new_stem }` → `Graph::rename_file` physically renames the file (collision-guarded, re-stats `last_synced_mtime` from the new path) and replies `GraphChangedMessage::FileRenamed { new_path }`; save failures surface as `GraphChangedMessage::SaveError`. Save-as (`SetSavePath`) is copy-forward — the old file remains; the engine saves **synchronously** on `SetSavePath` (not on the ~1s debounce) and acks with `GraphChangedMessage::SavedTo { path }`, which the GUI's deferred-close flow waits on before aborting the engine task
- **New graphs are in-memory and unsaved until the user saves them**: a new tab (startup or menu "new") has `save_path = None` — no file on disk, no auto-save; the settings panel's single "graph" section (name field + location line + save button in `graph_settings_panel.rs`) performs the first save via an rfd dialog seeded from the default library, and auto-save takes over from then on. While unsaved, the name field edits `Program::fallback_name` (a GUI-side pending name that seeds the dialog's file stem; nothing is sent to the engine). Closing an unsaved tab with nodes — or quitting with any such tabs — prompts save/discard/cancel (`PendingClose` state machine in GUI `app.rs`: the quit intercept cancels `close_requested` and resolves tabs one at a time; "save" waits for the engine's `SavedTo` ack before closing). The Libraries panel's "create graph" is the one flow that still creates a file immediately. First run still creates/links `~/Documents/NodeMangler` (`AppConfig::ensure_default_library`, persisted as `AppConfig::default_library`). Opening a graph (menu or library or file-drop) goes through `App::open_or_focus`, which focuses an already-open tab instead of spawning a second engine on the same file
- Graphs serialize to JSON via `GraphSaveData`; every save is stamped with the app version (`version` field, from `APP_VERSION` = workspace version). Loading tolerates a missing field (empty string = pre-versioning). `save_to_file` serializes through a borrowing mirror `GraphSaveRef` — keep its fields in sync with `GraphSaveData`, including the `#[serde(with = "saved_nodes")]` attribute on `nodes`. `NodeSettings.description`/`help` and `Node.is_dirty` are not saved: description/help are re-derived in `Graph::load` from `operation.settings()` (subgraph nodes get `Node::subgraph_settings()`, keeping the saved name), and every loaded node is dirty
- **Graceful forward compat** (not migration): nodes that fail to parse (e.g. saved by a newer NodeMangler) become `NodeType::Unknown { raw }` placeholders instead of failing the whole load — the tolerant per-node (de)serializer is the `saved_nodes.rs` serde-with module, which writes `raw` back verbatim on save (patching only position + connections), so unknown future fields round-trip. `Graph::load` fills `Graph.load_report` and emits `GraphChangedMessage::LoadWarnings` (before the `LoadedNode` stream); a newer-version file holds auto-save (`hold_saves` in engine `app.rs`) until the user edits, so opening alone never downgrades the file. Version comparison is hand-rolled in `version.rs`
- **Concurrent-edit safety**: `Graph.last_synced_mtime` (set on load/save) + `disk_is_newer()` make auto-save detect an external rewrite instead of clobbering it — the engine pauses saving, sends `GraphChangedMessage::FileConflict`, and waits for `ChangeGraphMessage::ResolveFileConflict { keep_ours }` (true = overwrite; false = `GraphCleared` + reload from disk). Subgraph child load failures / unknown child nodes surface as `NodeChangedMessage::Error` on the parent subgraph node
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
- `app/crates/mangler_core/src/version.rs` — `parse_version` / `is_newer_than_app` (strict `X.Y.Z`, no semver crate)
- `app/crates/mangler_core/src/saved_nodes.rs` — tolerant serde-with module for `GraphSaveData.nodes`: unknown nodes → placeholders on load, verbatim raw JSON write-back on save
- `app/crates/mangler_core/src/color/` — `Color` struct and color space conversions
- `app/crates/mangler_gui/src/main.rs` — entry point, eframe window setup
- `app/crates/mangler_gui/src/app.rs` — GUI `App`: manages programs, themes, menu bar, panel tree + secondary windows
- `app/crates/mangler_gui/src/program.rs` — `Program`: owns engine instance + per-panel content (`update` / `show_panel` / `show_overlays` / per-window `show_menu_drag`) for one graph. Graph timing + help text draw inside each graph panel; the status message and Tab-search are main-window overlays. Each Graph panel has its own `GraphCamera` (per-leaf pan/zoom, `graph_cameras: HashMap<LeafId, GraphCamera>`), the same per-leaf pattern used for the 2D/3D viewers, so panning/zooming one graph panel doesn't affect others
- `app/crates/mangler_gui/src/pan_zoom.rs` — shared drag-to-pan / zoom-about-cursor input handling used by both the graph editor and the 2D preview. All canvas pointer input is read from the rendering `Ui`'s own context (per-viewport), never from a cached main-window pointer — that's what makes panels in secondary OS windows work
- `app/crates/mangler_gui/src/panels/` — Blender-style panel system: `panel_tree.rs` (recursive split tree, pure logic), `panel_view.rs` (splitters, kind-switcher chrome), `panel_windows.rs` (secondary OS windows via immediate viewports). Each panel hosts one of 6 `PanelKind` contexts (Graph, Preview2D, Preview3D, NodeList, Libraries, Settings); layout is app-level and persisted to config as `default_layout` (default: left column = NodeList over Libraries); split/close live in each panel's own corner-button menu and act on that panel, while the settings menu keeps new-window/save-layout/reset-layout
- `app/crates/mangler_gui/src/libraries/` — Libraries panel: named links to on-disk folders of `.mangler.json` graphs **and image files** (local or network share; links persisted as `AppConfig::libraries`, content rescanned each session). `library_scanner.rs` polls all roots on a 2s background thread (no file-watcher — polling works on network shares) and classifies entries into `FolderScan.graphs` / `FolderScan.images` (image extensions from `ValueType::file_extensions`); single-clicking an image previews it in the focused program's 2D panel (loaded off the graph into `Program::library_image_preview`, which takes precedence over a viewed node output; the row highlights while shown; `view_node` clears it so a node output reclaims the panel — last action wins), and double-clicking an image adds an image-from-file node to the current graph. `libraries_state.rs` is app-global state on the GUI `App`, threaded through `render_tree` into `Program::show_panel`; panel actions that need the programs map (open/create graph → new tab, add image node, preview image) queue as `LibraryAction`s drained by `App::handle_library_action`. Image decoding is shared: `mangler_core::operations::images::inputs::file::load_image_from_path` (png/jpg/… via the `image` crate, plus jxl/psd) backs both the image-from-file node and the library preview. Deletes go to the OS recycle bin via the `trash` crate; removing a library only unlinks it. Node `Value::Path` file dialogs seed their start directory from the current graph's folder unless the input sets an explicit `set_directory`
- `app/crates/mangler_gui/src/graph/` — graph editor canvas, node rendering, connections
- `app/crates/mangler_gui/src/view_window/` — panel content viewers: `preview_2d.rs` (image/color/text value dispatch), `preview_3d.rs` (PBR material view, GL renderer, arcball), `curve_overlay.rs` (interactive Curve editor drawn over the 2D preview; pure widget — `Program::show_preview_2d_panel` maps norm↔screen via `ImageViewer::get_rect` and commits via `tx_change_node`)
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

### curves/
- `inputs/` — curve (emits a `Value::Curve`; drawn via the Preview2D overlay — see the Curve bullet in Key Conventions)
- `simulation/` — curve-space simulations (primary output is a Curve, so they live here per the output-type rule, but they follow the images/simulation conventions: seed-first input order, optional guidance maps, iteration count as the main driver). Nodes: meander (Howard & Knutson 1984 curvature-driven bank migration, meanderpy-style: signed curvature non-dimensionalized by the *local* `meander scale` + tanh-saturated, upstream-EMA lag sets the meander wavelength (exposed directly as `bend wavelength` in meander-scale widths; lag = wavelength/8.3 internally), neck cutoffs splice loops into oxbow lakes; the initial symmetry-breaking undulation (`initial wobble`) and the continuous per-iteration bank noise (`bank roughness`) are separate inputs — the roughness re-seeds bends because the instability is convective; without it the pinned upstream end relaminarizes straight. **Two decoupled width-like scales**: `meander scale` is the simulation's physical length (drives every local scale — wavelength, cutoff necks, migration step, bend tightness), while `channel width` is render-only (the rasterized river stroke). Both default to 10px@1024 so they start equal, but separating them lets a thin drawn river carry broad meanders or vice versa. Neither is constant: sqrt-of-discharge growth from `upstream fraction` at the source to full value downstream (upstream forms small tight bends, downstream big loops); `bend widening` (curvature-keyed) and `width noise` (seeded sinusoids) vary the rendered masks only. Per-iteration hot loops are allocation-free: the cutoff spatial hash uses a trivial multiply hasher with buckets cleared in place, resample is double-buffered, corridor stamping caps at ~512 age samples; all oxbows rasterize into one shared field (smoothstep is monotonic, so max-composing fields = max-composing masks). Outputs: evolved centerline Curve (RDP-decimated, feeds rasterize_curve/carve_river; width lives only in the rasters) + river mask / oxbows (rendered at the widths they were abandoned with) / age-graded migration-map images via an internal variable-width polyline rasterizer; masks are raw linear like rasterize_curve, not sRGB-encoded)

### images/
- `inputs/` — file, url, clipboard, color, gradient, text, constant (number-driven solid grayscale fill)
- `outputs/` — file, clipboard, material (channel-packed PBR texture export with engine presets). **Save gating**: all three take an `auto save` checkbox (off by default) + a momentary `save` button (a `Value::Bool` with `InputSettings::Button`; the click sends a one-shot `Bool(true)` pulse that `run` consumes/resets). A write happens only when auto-save is on, the button was clicked, or the run is *forced* — headless CLI `graph.run()` (`mangle run`/`show-output`) sets `Graph::force_save_outputs`, threaded to ops as `RunContext::force_save`. Shared helpers `should_save_and_consume`/`save_gate_inputs` live in `outputs/mod.rs`. The `to file` and `material` nodes share the same destination model: `folder` (relative to the graph's save dir unless absolute; empty = graph dir; auto-created) + `file name` (empty = graph name) + a `format` dropdown (`Value::ImageType`; `to file` defaults jpg, `material` defaults png) that picks the extension — resolved by the shared `resolve_output_dir_and_stem` helper in `outputs/mod.rs` via `RunContext { graph_dir, graph_name }` (a thread-local set around each op's `run`; see `run_context.rs`), since ops otherwise see only their inputs. `material` writes one file per preset texture as `{stem}_{suffix}.{ext}` and outputs the folder; `to file` writes `{stem}.{ext}` and outputs the full path. On creation, `Graph::add_node` pre-fills a fresh output node's `folder` with the graph's own directory (absolute) and its `file name` with a unique `{graph name}_{N}` stem (`Graph::next_unique_output_stem` picks the smallest N not taken by a sibling output node's name or an existing file in the folder; `output_node_path_inputs` maps each op to its `folder`/`file name` indices — `to file` 1/2, `material` 9/10), so multiple output nodes don't collide; both are only filled when still empty so recreated/pasted nodes keep their explicit values. (`material`'s input order is otherwise a frozen positional contract: 0..7 maps, 8 preset, 9/10/11 folder/file name/format, 12..31 four Custom slots, 32/33 save-gate.)
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
- `shapes/` — rectangle, ellipse, circle, polygon, star, line, cone, pyramid, paraboloid, rasterize_curve (Curve → grayscale mask: AA stroke with px@1024 `stroke width`/`feather`, even-odd fill when closed; feeds e.g. carve_river's river mask)
- `patterns/` — brick, hexagonal, weave, tile_sampler, tile_generator, splatter, flood_fill, flood_fill_mapper
- `pbr/` — normal_from_height, normal_to_height, normal_invert, normal_blend, normal_combine, ao_from_height, curvature, bevel, height_blend
- `simulation/` — physical-process simulation generators (more planned in `PLAN.md`). Category conventions: guidance-map image inputs (weakness, fuel, moisture, height) are optional and fall back to an internal seed-derived map, so every simulation node also works standalone (`is_unconnected`/`guidance_map_to_grid` helpers in `simulation/mod.rs`); input order is seed/width/height, guidance maps, then the main drivers (iteration-style counts like iterations/droplets/particles first — users step through them to watch the sim work), then fine-tuning params last. Nodes: hydraulic_erosion (faithful Beyer/Lague droplet sim; sequential single-threaded, erosion brush, single height output; tiles), carve_river (conforms terrain to a user path mask: labeled distance-field valley profile + monotonic downstream water line propagated from outlets, so paths over ridges become gorges instead of uphill rivers; does NOT tile), hillslope_diffusion (Roering et al. 1999 nonlinear soil creep: explicit mass-conserving flux update with the multiplier clamped at M=10 — the update stays a convex combination so any creep rate is stable; rounds crests into rolling hills; torus wrap, tiles when input tiles), guided_rolling_hills (rolling hills parted around a river mask — **dark = river** on a light background, so meander's white-on-black mask needs an invert node first; the old distance-field mode is gone. Flat bed plus a **convex** valley wall `1-(1-d)^q` (steepest at the bank, rounding into the hilltops); instead of a per-pixel fade, **each splatted hill's amplitude is scaled by the wall height at its own center** (per-cell factor table indexed by *unwrapped* cell so seam-wrapped hill instances get their actual location's factor) — hills in the channel vanish, wall hills shrink but keep full dome shapes, so the river flows *between* hills; a narrow smoothstep bank cut (`max(2px, 10% of valley width)`) trims skirts spilling into the channel; the modulated splat is normalized by the **unmodulated** field's min/max so past-the-rim pixels and the unconnected/empty-mask fallback stay pixel-identical to plain rolling hills; optional levee bump; heuristic, not physical; tiles only when unconnected — the distance transform is not toroidal once a mask drives it). Shared helpers in `simulation/mod.rs`: `fallback_terrain` (torus fBm) and `distance_field_labeled` (Felzenszwalb-Huttenlocher EDT that also returns each pixel's nearest-site index). (Frost DLA, drying-cracks, and sand-ripples nodes were built and deleted 2026-07 — not good enough; see PLAN.md. The rivers drainage-network node (priority-flood → D8 → stream-power) was deleted 2026-07-12; replacement river nodes to be designed later. Each simulation node's `settings().help` should say what real model it's based on, or admit it's a heuristic — do not oversell heuristics as physical simulations.)
- `noise/` — 47 generators in subdirectories mirroring the node-menu subcategories (`voronoi_common.rs` at the noise root is a shared helper, not a node; `pixel_hash`/`periodic_perlin_2d`/`periodic_value_2d`/`build_perm_tables` in `noise/mod.rs` are shared)
  - `basic/` — perlin, value, open_simplex, super_simplex, gabor, phasor, anisotropic, gaussian (white noise), blue_noise
  - `fractal/` — fbm, billow, ridged_multifractal, basic_multifractal, hybrid_multifractal, domain_warp_fbm, flow (rotated-gradient fbm with advection), curl (flow map, 3-channel), clouds, plasma
  - `cellular/` — worley_distance, worley_value, voronoise, voronoi_crack, crystal, scales, craters, rolling_hills (Hann-kernel hill splatting; peakiness exponent reshapes the profile, merge slider blends tallest-wins→sum overlap composition)
  - `structural/` — checkerboard, cylinders, wave, truchet (truchet tiles), warped_rings (fbm-warped concentric rings, non-tiling), veins (warped vein stripes)
  - `grunge/` — dirt, scratches, fibers, leaks (drip streaks, alignment control), stains (coffee-ring rims), peeling (flaking-paint mask), smear (soft directional streaks), growth (clustered organic patches)
  - `process/` — erosion, fault_terrain, reaction_diffusion, caustics (refraction simulation), lightning (branching filaments, non-tiling), spectral_terrain (Voss random-phase spectral synthesis, integer wavevectors so it tiles exactly; row rotation recurrence instead of per-pixel cos)

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
