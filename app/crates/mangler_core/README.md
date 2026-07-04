# mangler_core

The engine behind [NodeMangler](../../../README.md): the value type system, node graph
engine, operation library, color-space math, and async thumbnail service.

This crate has **no GUI**. It powers both the [mangler_gui](../mangler_gui/) desktop app
and the [mangler_cli](../mangler_cli/) headless tool, and can be embedded as a library in
your own program. It is licensed **MIT OR Apache-2.0**.

## Usage

```bash
cargo build -p mangler_core
cargo test  -p mangler_core
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

Values convert between types where it makes sense via `Value::try_convert_to` — e.g. Bool
→ Integer (0/1), Decimal, Text, Color (black/white), and Image. Each value can produce a
`Thumbnail` for the UI and a `fingerprint()` hash for cheap change detection. `Image`
thumbnails are computed asynchronously (see the thumbnail service) so the graph
run loop never blocks on resize work.

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
- **Fingerprint caching** — before a node runs, its input fingerprints are hashed; if
  the hash matches the previous run, the node is skipped. (`Trigger` values hash as a
  constant, so trigger-fired nodes are re-run explicitly.)
- **Serialization** — graphs save/load as JSON via `GraphSaveData` (no backwards
  compatibility for old files — they re-wire or re-export)
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
enum and every dispatch `match` arm. The categories:

| Category | Modules |
|----------|---------|
| **numbers** | `inputs` (decimal, integer, e, pi, tau), `arithmetic`, `algebra`, `trigonometry`, `interpolation`, `logarithmic`, `bitwise`, `random`, `cast` |
| **logic** | `inputs` (bool), `comparison`, `boolean`, `flow` (`select` mux) |
| **text** | `inputs` (text), `manipulation` (append, length, uppercase, lowercase, to_string) |
| **colors** | `inputs`/`outputs` (all 14 color spaces), `generation` (from/to hex, random), `manipulation` (adjust_hsv, clamp, grayscale, invert, set_alpha), `relationship` (harmony), `analysis`, `blend`, `cast`, `sample_image` |
| **images** | `inputs`, `outputs`, `transform`, `adjustments`, `blur`, `filter`, `fx`, `combine`, `channels`, `shapes`, `patterns`, `pbr`, `noise` (36 generators), `cast` |

The image file ops read PNG, JPEG, GIF, WebP, TIFF, TGA, BMP, ICO, PNM, QOI, Farbfeld,
Radiance HDR, and OpenEXR through the `image` crate, plus JPEG XL (`jxl-oxide`) and PSD
(`psd`, flattened composite) through dedicated pure-Rust decoders. Writing supports the
same `image`-crate formats plus AVIF (JPEG XL and PSD are read-only; AVIF is
write-only), with a shared quality input for JPEG/AVIF, a compression-level selector
for PNG, and 8/16/32-bit-float color formats validated per container (e.g. JPEG is
8-bit no-alpha, OpenEXR is float-only, HDR writes from `Rgb32F`).

Per-category shared helpers (e.g. `adjustments/common.rs`, `noise/mod.rs` hash/perm
tables, `erode.rs` separable morphology) keep the individual op files small. Heavy pixel
loops (`FloatImage` conversions and the larger image ops) are data-parallel via rayon,
and image buffers are `Arc`-shared so passing them between nodes never copies pixels.

Tests live in a sibling `{op}_tests.rs` file linked via
`#[cfg(test)] #[path = "..."] mod tests;`, which keeps source files short while still
reaching private functions.

### Async thumbnail service (`thumbnail_service.rs`)

`ThumbnailService` runs on a dedicated tokio task and keeps expensive thumbnails off the
engine thread:

- **Image sources** — resize + `to_rgba8` of a `FloatImage` (~15–50 ms on large frames)

Requests are coalesced per `(node_id, output_index)` with a monotonic sequence number, so
a burst of updates doesn't queue hundreds of wasted jobs. Stale jobs are dropped before
and after the `spawn_blocking` compute and again at the UI, so a late thumbnail for a
replaced value never overwrites the correct preview. Scalar/enum thumbnails stay on the
inline path.

### Message-driven API (`lib.rs`)

The engine communicates over four message types:

- `ChangeGraphMessage` — UI → engine: add/remove nodes and connections, set save
  path/name
- `ChangeNodeMessage` — UI → engine: set an input value or node position, expose
  inputs/outputs
- `GraphChangedMessage` — engine → UI: node/connection added, removed, or loaded
- `NodeChangedMessage` — engine → UI: `OutputChanged` (value changed), `ThumbnailReady`
  (deferred thumbnail), and busy/error/timing state

## Adding a new operation

1. Create the operation struct in the right `src/operations/{category}/` directory
2. Implement `settings()`, `create_inputs()`, `create_outputs()`, and `async fn run()`
3. Register it in the `operations!` macro in `src/operations/mod.rs`
4. Add it to `operation_list()` in the same file so it appears in the node menu
5. Add `pub mod` to the parent category `mod.rs`
6. Add tests in a sibling `{op}_tests.rs` file, linked with
   `#[cfg(test)] #[path = "..."] mod tests;`

A sync test in `operations_tests.rs` fails until the new node is also added to the
top-level README's Node Reference section — and catches stale entries when an operation
is renamed or removed.

## Dependencies

- `image` — image load/save/processing
- `jxl-oxide` — JPEG XL decoding (pure Rust)
- `psd` — Photoshop PSD decoding (flattened composite)
- `tokio` — async runtime for graph execution
- `serde` / `serde_json` — save/load serialization
- `glam` — 2D vector math (node positions)
- `noise` — procedural noise primitives
- `nanoid` — unique IDs
- `arboard` — clipboard access
- `reqwest` — HTTP (image from URL)
- `fastrand` — random number generation
- `dashmap` — lock-free per-key maps (thumbnail service)
- `rayon` — data-parallel pixel loops
- `ab_glyph` — text rendering for the `text` image-input node
