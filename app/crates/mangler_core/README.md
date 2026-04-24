# mangler

Core library for NodeMangler. Provides the value type system, node graph engine, operation definitions, and color space conversions.

This crate has no GUI — it is the engine that powers the [nodemangler](../nodemangler/) application, and can be used as a library independently.

## Usage

```bash
cargo build -p mangler
cargo test -p mangler
```

## Architecture

### Value System (`value.rs`)

All data flowing between nodes is represented by the `Value` enum:

- `Bool`, `Integer`, `Decimal`, `Text` — primitives
- `Color` — sRGBA float color with conversions to many color spaces
- `Image { data: Arc<FloatImage>, change_id: String }` — image data as a 1–4 channel f32 buffer; `change_id` drives stale-thumbnail rejection and cache invalidation
- `Path` — filesystem path
- `FilterType` — image resampling filter (Nearest, Triangle, CatmullRom, Gaussian, Lanczos3)
- `ColorFormat` — pixel format (Rgba8, Rgb16, Rgba32F, etc.)
- `ImageType` — image file format (PNG, JPEG, WebP, TIFF, OpenEXR, etc.)
- `ColorSpace`, `BlendMode`, `Trigger`, `NoiseWorleyDistanceFunction`, `TextHAlign`, `TextVAlign`
- `VideoContainer` (Mp4, Mov, Mkv, WebM, Avi), `VideoCodec` (H264, H265, Vp8, Vp9, Av1, Mpeg4, ProRes) — orthogonal; a static compatibility matrix (`VideoContainer::supported_codecs`) encodes which pairs are valid
- `Video(VideoRef)` — lightweight handle (path + cached `VideoMeta`) produced by the loader op; decoders live in the process-global `VideoDecoderCache`

Values support type conversion where it makes sense (`Value::try_convert_to`). For example, Bool converts to Integer (0/1), Decimal, Text, Color (black/white), and Image.

Each value can produce a `Thumbnail` for display in the UI and a `fingerprint()` hash for efficient change detection. `Value::Image` and `Value::Video` thumbnails are computed asynchronously by `ThumbnailService` (see below) so the engine's graph-run loop isn't blocked by resize or frame-decode work.

### Color System (`color/`)

Colors are stored internally as sRGBA floats (`Color { r, g, b, a }`). The color module provides conversions to and from:

- **sRGB** (the storage format)
- **Linear RGB** (gamma-decoded)
- **HSL** (Hue, Saturation, Lightness)
- **HSV** (Hue, Saturation, Value)
- **Lab** (CIELAB perceptual)
- **LCH** (Cylindrical Lab)
- **CMYK** (Cyan, Magenta, Yellow, Key)
- **XYZ** (CIE 1931)
- **YUV** (Luma + Chrominance)

The `blend` module provides color blending modes.

### Graph Engine (`graph.rs`)

The `Graph` struct holds a `HashMap<String, Node>` and manages execution:

- **Adding/removing nodes** and connections
- **Dirty tracking** — when inputs change, downstream nodes are marked dirty
- **Async execution** — the graph runs on a tokio task, processing changes from mpsc channels
- **Topological evaluation** — nodes execute in dependency order
- **Serialization** — graphs save/load as JSON via `GraphSaveData`
- **Subgraph support** — a node can contain an entire nested graph

### Operations (`operations/`)

Operations define what nodes do. Each operation is a struct that implements:

- `settings()` — returns `NodeSettings` (name, category, color)
- `create_inputs()` — defines input connections with names, default values, valid types, and UI widgets
- `create_outputs()` — defines output connections
- `run(inputs)` — async function that processes inputs and returns `OperationResponse`

Operations are registered in the `operations!` macro, which generates the `Operation` enum and all dispatch `match` arms automatically.

Operations are organized into six categories:

| Category | Modules |
|----------|---------|
| **numbers** | `inputs`, `arithmetic`, `interpolation`, `algebra`, `trigonometry`, `random`, `cast`, `logarithmic`, `bitwise` |
| **colors** | `inputs` (9 color spaces), `outputs` (9 color spaces), `blend`, `analysis` (`sample_image`), `cast` |
| **images** | `inputs`, `outputs`, `transform`, `adjustments`, `combine`, `blur`, `filter`, `noise` (14+ types), `channels`, `shapes`, `patterns`, `pbr` |
| **logic** | `inputs`, `comparison`, `boolean`, `flow` (`select`) |
| **text** | `inputs` (text), `manipulation` (append, length, uppercase, lowercase, to_string) |
| **videos** | `inputs` (`video from file`), `transform` (`extract_frame_by_index`, `extract_frame_by_time`), `outputs` (`video to file`) — feature-gated behind `video`; see `docs/video-setup.md` |

### Async Thumbnail Service (`thumbnail_service.rs`)

`ThumbnailService` runs on a dedicated tokio task and handles expensive thumbnails off the engine thread:

- **Image sources** — resize + to_rgba8 of a `FloatImage` (~15–50ms on large frames)
- **Video sources** — decode frame 0 via `VideoDecoderCache` then resize

Requests are coalesced by `(node_id, output_index)` using a monotonic sequence number, so scrubbing through a video doesn't queue hundreds of wasted thumbnail jobs. Stale jobs are dropped both before and after the spawn_blocking compute, and once more at the UI so that late thumbnails for a replaced value never overwrite the correct preview. Scalar/enum thumbnails (text) stay on the inline path.

### Video Pipeline (`video/`, feature-gated behind `video`)

- `VideoDecoderCache` — process-global ring-buffer cache keyed on path. Size 64 frames per clip with a 60-frame seek threshold tuned for scrub-friendly interactive use. Reclaims pixel buffers (`Arc<FloatImage>` → `Vec<f32>`) on ring eviction so successive decodes on the same clip share one allocation.
- `VideoEncoder` — wraps `video-rs`'s encoder; takes an explicit `(VideoContainer, VideoCodec)` pair, validates the combo against the static matrix, then dispatches to an `EncoderPreset`. H.264 MP4/MOV/MKV are wired up today; other legal matrix pairs return a distinct "not yet implemented" error so encoder coverage can expand independently.
- Container/codec classification reads FFmpeg's demuxer short-name + codec id via a parallel `ffmpeg::format::input(&path)`; strict — unknown containers/codecs return errors rather than silently degrading.

### Render Task (`render.rs`, feature-gated behind `video`)

The Video Output node's Render button sends `ChangeGraphMessage::StartRender` to the engine, which spawns `render::run_render` on a detached graph snapshot. The render loop discovers every time-aware node (`Operation::is_time_aware`), drives each frame by calling the op's own `apply_render_time` hook, then `graph.run().await`s and pulls the encoded frame from the output node's `image` input. Progress is streamed back to the UI as `GraphChangedMessage::RenderProgress`; success and failure both reset any "starting…" UI state.

### Message-Driven Communication (`lib.rs`)

The engine uses four message types for communication:

- `ChangeGraphMessage` — UI tells engine to add/remove nodes or connections, save path, start a render
- `ChangeNodeMessage` — UI tells engine to update a node's input value or position
- `GraphChangedMessage` — engine tells UI a node/connection was added, removed, or loaded; render progress
- `NodeChangedMessage` — engine tells UI an output value changed (`OutputChanged`), a deferred thumbnail arrived (`ThumbnailReady`), or busy/error/timing state changed

## Dependencies

- `image` / `imageproc` — image loading, saving, and processing
- `tokio` — async runtime for graph execution
- `serde` / `serde_json` — serialization for save/load
- `glam` — 2D vector math (node positions)
- `noise` — procedural noise generation
- `nanoid` — unique ID generation
- `arboard` — clipboard access
- `reqwest` — HTTP requests (image from URL)
- `fastrand` — random number generation
- `dashmap` — lock-free per-key maps (decoder cache, thumbnail service)
- `rayon` — data-parallel pixel-loop primitives
- `ab_glyph` — text rendering for the `text` image-input node

### Feature-gated (`video` feature)

- `video-rs` — high-level ffmpeg wrapper for decode/encode
- `ffmpeg-next` — direct ffmpeg bindings for metadata introspection
- `ndarray` — required by `video-rs` for frame buffers

Enabling the `video` feature requires FFmpeg development libraries built with `libx264`, `libx265`, `libvpx`, `libaom`, and `gpl`. See `docs/video-setup.md` for platform-specific install instructions.
