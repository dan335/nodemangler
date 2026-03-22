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

- `Bool`, `Integer`, `Decimal`, `String` — primitives
- `Color` — sRGBA float color with conversions to many color spaces
- `DynamicImage` — image data (backed by the `image` crate, wrapped in `Arc`)
- `Path` — filesystem path
- `FilterType` — image resampling filter (Nearest, Triangle, CatmullRom, Gaussian, Lanczos3)
- `ColorFormat` — pixel format (Rgba8, Rgb16, Rgba32F, etc.)
- `ImageType` — image file format (PNG, JPEG, WebP, TIFF, OpenEXR, etc.)
- `ColorSpace`, `BlendMode`, `Trigger`, `NoiseWorleyDistanceFunction`

Values support type conversion where it makes sense (`Value::try_convert_to`). For example, Bool converts to Integer (0/1), Decimal, String, Color (black/white), and Image.

Each value can produce a `Thumbnail` for display in the UI and a `fingerprint()` hash for efficient change detection.

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

Operations are organized into four categories:

| Category | Modules |
|----------|---------|
| **numbers** | `inputs`, `arithmetic`, `interpolation`, `algebra`, `trigonometry`, `random`, `cast`, `logarithmic`, `bitwise` |
| **colors** | `inputs` (9 color spaces), `outputs` (9 color spaces), `blend`, `analysis` (`sample_image`), `cast` |
| **images** | `inputs`, `outputs`, `transform`, `adjustments`, `combine`, `blur`, `filter`, `noise` (14 types), `channels`, `shapes`, `patterns`, `pbr` |
| **logic** | `inputs`, `comparison`, `boolean`, `flow` (`select`) |

### Message-Driven Communication (`lib.rs`)

The engine uses four message types for communication:

- `ChangeGraphMessage` — UI tells engine to add/remove nodes or connections
- `ChangeNodeMessage` — UI tells engine to update a node's input value or position
- `GraphChangedMessage` — engine tells UI a node/connection was added, removed, or loaded
- `NodeChangedMessage` — engine tells UI an output value changed, with optional thumbnail

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
