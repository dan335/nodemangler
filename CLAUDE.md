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

- **Nightly Rust** toolchain (see `app/rust-toolchain.toml`)
- **Async-first**: tokio multi-threaded runtime; graph execution is async on a separate tokio task
- **Message-driven UI**: the GUI and engine communicate through four mpsc channel message types:
  - `ChangeGraphMessage` — UI → engine: add/remove nodes and connections, set save path/name, start renders
  - `ChangeNodeMessage` — UI → engine: set input values, positions, expose inputs/outputs
  - `GraphChangedMessage` — engine → UI: node/connection added, removed, or loaded; render progress
  - `NodeChangedMessage` — engine → UI: output values changed, thumbnails, busy/error status, timing
- **Async thumbnails**: `Value::Image` and `Value::Video` thumbnails are computed off the engine thread by `ThumbnailService` (see `thumbnail_service.rs`). Engine emits `OutputChanged { thumbnail: None }` for image/video outputs; a follow-up `NodeChangedMessage::ThumbnailReady` arrives when the resize/decode finishes, with a stale-check keyed on `change_id` (image) or path (video). Scalar/enum thumbnails are still computed inline.
- Operations are generated via the `operations!` macro in `app/crates/mangler_core/src/operations/mod.rs`
- **Value types** (see `value.rs`): Bool, Integer, Decimal, Text, Color, Image, Path, FilterType, ImageType, ColorFormat, Trigger, NoiseWorleyDistanceFunction, ColorSpace, BlendMode, TextHAlign, TextVAlign, VideoContainer, VideoCodec, Video
- Images are `FloatImage` (1–4 channel `f32`, `Arc`-shared); `Value::Image { data, change_id }` carries a change id used by cache invalidation and stale-thumbnail rejection.
- `Value::Video(VideoRef)` is a lightweight handle (path + `VideoMeta`) produced by the `video from file` node; extract-frame ops consume it and share the process-global `VideoDecoderCache`.
- Color is stored as sRGBA floats with conversions to 9 color spaces: sRGB, Linear RGB, HSL, HSV, Lab, LCH, CMYK, XYZ, YUV
- BlendMode has 17 modes: Normal, Lerp, Multiply, Screen, Overlay, SoftLight, HardLight, ColorDodge, ColorBurn, Darken, Lighten, Difference, Exclusion, LinearBurn, LinearDodge, Divide, Subtract
- **Video container/codec are separate enums** with a static compatibility matrix (see `VideoContainer::supported_codecs`). Containers: Mp4, Mov, Mkv, WebM, Avi. Codecs: H264, H265, Vp8, Vp9, Av1, Mpeg4, ProRes.
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
- `app/crates/mangler_core/src/value.rs` — `Value` and `ValueType` enums, type conversions, fingerprinting, `VideoRef`/`VideoMeta`/`VideoContainer`/`VideoCodec`
- `app/crates/mangler_core/src/graph.rs` — `Graph` struct: node storage, dirty tracking, async execution, save/load
- `app/crates/mangler_core/src/node.rs` — `Node::run` dispatches to op, emits `OutputChanged` / `ThumbnailReady` / `Busy` / `Error`
- `app/crates/mangler_core/src/app.rs` — engine-side `App`: spawns tokio task, processes change messages
- `app/crates/mangler_core/src/operations/mod.rs` — `operations!` macro, `Operation` enum, operation list, `is_time_aware` / `apply_render_time` dispatch for the render loop
- `app/crates/mangler_core/src/thumbnail_service.rs` — async thumbnail worker with supersede-by-seq coalescing and video-first-frame decode
- `app/crates/mangler_core/src/video/` — decoder cache, encoder, metadata classifiers (feature-gated behind `video`)
- `app/crates/mangler_core/src/render.rs` — video render task: detached graph snapshot, driven by `apply_render_time` hooks (feature-gated)
- `app/crates/mangler_core/src/color/` — `Color` struct and color space conversions
- `app/crates/mangler_core/docs/video-setup.md` — how to install ffmpeg dev libs with `x264`/`x265`/`vpx`/`aom`
- `app/crates/mangler_gui/src/main.rs` — entry point, eframe window setup
- `app/crates/mangler_gui/src/app.rs` — GUI `App`: manages programs, themes, menu bar
- `app/crates/mangler_gui/src/program.rs` — `Program`: owns engine instance + all UI panels for one graph
- `app/crates/mangler_gui/src/graph/` — graph editor canvas, node rendering, connections
- `app/crates/mangler_gui/src/themes/` — 4 themes: Dark, DarkGreen (default), Light, LightBlue

## Video feature

Video ops live behind the `video` cargo feature on `mangler_core`. `mangler_gui` and `mangler_cli` enable it by default in their `Cargo.toml`. The feature pulls in `video-rs` + `ffmpeg-next`, which require FFmpeg development libraries built with **libx264, libx265, libvpx, libaom, and gpl**. See `app/crates/mangler_core/docs/video-setup.md` for vcpkg/prebuilt install instructions. vcpkg's default `ffmpeg` port omits the GPL codecs; builds using it will fail to encode with a cryptic "Invalid argument" — the setup doc explains how to reinstall with the needed feature set.

## Adding a New Operation

1. Create the operation struct in the appropriate `app/crates/mangler_core/src/operations/{category}/` directory
2. Implement `settings()`, `create_inputs()`, `create_outputs()`, and `async fn run()`
3. Register it in the `operations!` macro in `app/crates/mangler_core/src/operations/mod.rs`
4. Add it to the `operation_list()` function in the same file for it to appear in the node menu
5. Add `pub mod` in the parent category `mod.rs`
6. Add tests in a separate `{operation_name}_tests.rs` file, linked via `#[cfg(test)] #[path = "..."] mod tests;`

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
- `operations/text/` — inputs (text), manipulation (append, length, to_uppercase, to_lowercase, to_string)
- `operations/videos/inputs/` — file (`video from file` — loads a clip and emits a `Video` handle + metadata sockets)
- `operations/videos/transform/` — extract_frame_by_index, extract_frame_by_time (consume a `Video` handle, produce an `Image`)
- `operations/videos/outputs/` — file (`video to file` — encode the graph into a video file via the Render button)

## Known Issues

None currently.
