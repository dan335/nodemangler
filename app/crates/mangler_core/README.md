# mangler_core

The engine behind [NodeMangler](../../../README.md): the value type system, node graph
engine, operation library, color-space math, async thumbnail service, and (optionally)
the video decode/encode pipeline.

This crate has **no GUI**. It powers both the [mangler_gui](../mangler_gui/) desktop app
and the [mangler_cli](../mangler_cli/) headless tool, and can be embedded as a library in
your own program. It is licensed **MIT OR Apache-2.0**.

## Usage

```bash
cargo build -p mangler_core
cargo test  -p mangler_core

# with the optional video pipeline (needs FFmpeg dev libs — see docs/video-setup.md)
cargo build -p mangler_core --features video
```

## Architecture

### Value system (`value.rs`)

Everything that flows between nodes is a `Value`:

- `Bool`, `Integer`, `Decimal`, `Text` — primitives
- `Color` — an sRGBA float color with conversions to many color spaces (see below)
- `Image { data: Arc<FloatImage>, change_id: String }` — image data as an `Arc`-shared
  1–4 channel `f32` buffer; `change_id` drives cache invalidation and stale-thumbnail
  rejection
- `Path` — a filesystem path
- `Trigger` — a fire-once signal
- Enum-backed types: `FilterType` (Nearest, Triangle, CatmullRom, Gaussian, Lanczos3),
  `ColorFormat` (Rgba8, Rgb16, Rgba32F, …), `ImageType` (PNG, JPEG, WebP, TIFF, OpenEXR,
  …), `ColorSpace`, `BlendMode`, `NoiseWorleyDistanceFunction`, `TextHAlign`, `TextVAlign`
- `VideoContainer` (Mp4, Mov, Mkv, WebM, Avi) and `VideoCodec` (H264, H265, Vp8, Vp9,
  Av1, Mpeg4, ProRes) — orthogonal enums; a static compatibility matrix
  (`VideoContainer::supported_codecs`) encodes which pairs are legal
- `Video(VideoRef)` — a lightweight handle (path + cached `VideoMeta`) produced by the
  loader op; decoders live in the process-global `VideoDecoderCache`

Values convert between types where it makes sense via `Value::try_convert_to` — e.g. Bool
→ Integer (0/1), Decimal, Text, Color (black/white), and Image. Each value can produce a
`Thumbnail` for the UI and a `fingerprint()` hash for cheap change detection. `Image` and
`Video` thumbnails are computed asynchronously (see the thumbnail service) so the graph
run loop never blocks on resize or frame-decode work.

> **No backwards compatibility** for saved graphs: field renames, value-type splits, and
> output-order changes land without migration paths. Old graphs re-wire or re-export.

### Color system (`color/`)

Colors are stored as sRGBA floats (`Color { r, g, b, a }`) and convert to and from
**14 color spaces**:

| Space | Notes |
|-------|-------|
| **sRGB** | the storage format |
| **Linear RGB** | gamma-decoded |
| **HSL** | hue, saturation, lightness |
| **HSV** | hue, saturation, value |
| **HWB** | hue, whiteness, blackness |
| **Lab** | CIELAB perceptual |
| **LCH** | cylindrical Lab |
| **Oklab** | perceptual, modern |
| **Oklch** | cylindrical Oklab |
| **CMYK** | cyan, magenta, yellow, key |
| **XYZ** | CIE 1931 |
| **xyY** | CIE chromaticity + luminance |
| **YUV** | luma + chrominance (BT.601) |
| **YCbCr** | luma + chroma (BT.709) |

Lab and LCH share a unified white point. The `blend` submodule implements the 17 blend
modes used by the color and image blend ops.

### Graph engine (`graph.rs`)

The `Graph` struct owns a `HashMap<String, Node>` and drives execution:

- **Add/remove** nodes and connections
- **Dirty tracking** — changing an input marks downstream nodes dirty
- **Async execution** — the graph runs on a tokio task, draining change messages from
  mpsc channels
- **Topological evaluation** — nodes run in dependency order
- **Serialization** — graphs save/load as JSON via `GraphSaveData`
- **Subgraphs** — a node can contain an entire nested graph

`Node::run` (`node.rs`) dispatches to the operation and emits `OutputChanged`,
`ThumbnailReady`, `Busy`, and `Error` messages as it goes.

### Operations (`operations/`)

An operation defines what a node does. Each is a struct implementing:

- `settings()` → `NodeSettings` (name, category, color)
- `create_inputs()` → input sockets with names, defaults, accepted types, and UI widgets
- `create_outputs()` → output sockets
- `async fn run(inputs)` → `OperationResponse`

Operations are registered in the `operations!` macro, which generates the `Operation`
enum and every dispatch `match` arm — including the `is_time_aware` / `apply_render_time`
hooks the video render loop relies on. The categories:

| Category | Modules |
|----------|---------|
| **numbers** | `inputs` (decimal, integer, e, pi, tau), `arithmetic`, `algebra`, `trigonometry`, `interpolation`, `logarithmic`, `bitwise`, `random`, `cast` |
| **logic** | `inputs` (bool), `comparison`, `boolean`, `flow` (`select` mux) |
| **text** | `inputs` (text), `manipulation` (append, length, uppercase, lowercase, to_string) |
| **colors** | `inputs`/`outputs` (all 14 color spaces), `generation` (from/to hex, random), `manipulation` (adjust_hsv, clamp, grayscale, invert, set_alpha), `relationship` (harmony), `analysis`, `blend`, `cast`, `sample_image` |
| **images** | `inputs`, `outputs`, `transform`, `adjustments`, `blur`, `filter`, `fx`, `combine`, `channels`, `shapes`, `patterns`, `pbr`, `noise` (28 generators), `cast` |
| **videos** | `inputs` (from file/url), `transform` (extract/trim/speed/reverse/loop), `outputs` (to file) — feature-gated behind `video` |

Per-category shared helpers (e.g. `adjustments/common.rs`, `noise/mod.rs` hash/perm
tables, `erode.rs` separable morphology) keep the individual op files small.

Tests live in a sibling `{op}_tests.rs` file linked via
`#[cfg(test)] #[path = "..."] mod tests;`, which keeps source files short while still
reaching private functions.

### Async thumbnail service (`thumbnail_service.rs`)

`ThumbnailService` runs on a dedicated tokio task and keeps expensive thumbnails off the
engine thread:

- **Image sources** — resize + `to_rgba8` of a `FloatImage` (~15–50 ms on large frames)
- **Video sources** — decode frame 0 via `VideoDecoderCache`, then resize

Requests are coalesced per `(node_id, output_index)` with a monotonic sequence number, so
scrubbing a video doesn't queue hundreds of wasted jobs. Stale jobs are dropped before
and after the `spawn_blocking` compute and again at the UI, so a late thumbnail for a
replaced value never overwrites the correct preview. Scalar/enum thumbnails stay on the
inline path.

### Video pipeline (`video/`, feature `video`)

- **`VideoDecoderCache`** — process-global ring-buffer cache keyed on path (64 frames per
  clip, 60-frame seek threshold tuned for scrub-friendly playback). Reclaims pixel
  buffers (`Arc<FloatImage>` → `Vec<f32>`) on eviction so repeat decodes share one
  allocation.
- **`VideoEncoder`** — wraps `video-rs`'s encoder; takes an explicit
  `(VideoContainer, VideoCodec)` pair, validates it against the static matrix, then
  dispatches to an `EncoderPreset`. H.264 MP4/MOV/MKV are wired up today; other legal
  pairs return a distinct "not yet implemented" error so coverage can grow independently.
- **Metadata classification** reads FFmpeg's demuxer short-name and codec id via a
  parallel `ffmpeg::format::input(&path)`; it's strict — unknown containers/codecs error
  rather than silently degrading.

### Render task (`render.rs`, feature `video`)

The Video Output node's Render button sends `ChangeGraphMessage::StartRender`. The engine
spawns `render::run_render` on a detached graph snapshot, discovers every time-aware node
(`Operation::is_time_aware`), drives each frame through the op's `apply_render_time` hook,
`graph.run().await`s, and pulls the encoded frame from the output node's `image` input.
Progress streams back as `GraphChangedMessage::RenderProgress`; success and failure both
reset any "starting…" UI state.

### Message-driven API (`lib.rs`)

The engine communicates over four message types:

- `ChangeGraphMessage` — UI → engine: add/remove nodes and connections, set save
  path/name, start a render
- `ChangeNodeMessage` — UI → engine: set an input value or node position, expose
  inputs/outputs
- `GraphChangedMessage` — engine → UI: node/connection added, removed, or loaded; render
  progress
- `NodeChangedMessage` — engine → UI: `OutputChanged` (value changed), `ThumbnailReady`
  (deferred thumbnail), and busy/error/timing state

## Dependencies

- `image` / `imageproc` — image load/save/processing
- `tokio` — async runtime for graph execution
- `serde` / `serde_json` — save/load serialization
- `glam` — 2D vector math (node positions)
- `noise` — procedural noise primitives
- `nanoid` — unique IDs
- `arboard` — clipboard access
- `reqwest` — HTTP (image/video from URL)
- `fastrand` — random number generation
- `dashmap` — lock-free per-key maps (decoder cache, thumbnail service)
- `rayon` — data-parallel pixel loops
- `ab_glyph` — text rendering for the `text` image-input node

### Feature `video`

- `video-rs` — high-level FFmpeg wrapper for decode/encode
- `ffmpeg-next` — direct FFmpeg bindings for metadata introspection
- `ndarray` — frame buffers required by `video-rs`

Enabling `video` requires FFmpeg development libraries built with `libx264`, `libx265`,
`libvpx`, `libaom`, and `gpl`. See [`docs/video-setup.md`](docs/video-setup.md) for
platform-specific install instructions and the licensing implications of distributing a
GPL-linked build.
