# NodeMangler

A node-based visual programming tool for image and color manipulation, built in Rust.

NodeMangler lets you construct processing pipelines by connecting nodes in a visual graph editor. Each node performs a single operation — load an image, adjust contrast, convert color spaces, generate noise — and data flows through connections between them. The result is a non-destructive, composable workflow for image and color work.

## Repository Structure

The repository is organized as a monorepo:

- `app/` — Rust application (Cargo workspace)
- `website/` — Website (future)

### App Crates

| Crate | Path | Purpose |
|-------|------|---------|
| **mangler_core** | `app/crates/mangler_core/` | Core library — value system, node graph engine, operations, color spaces |
| **mangler_gui** | `app/crates/mangler_gui/` | Desktop GUI application built with egui/eframe |
| **mangler_cli** | `app/crates/mangler_cli/` | Headless CLI for running graphs without the GUI |

See each crate's README for details:
- [mangler_core README](app/crates/mangler_core/README.md) — the engine and operation library
- [mangler_gui README](app/crates/mangler_gui/README.md) — the desktop application

## Requirements

- **Rust nightly** toolchain (configured in `app/rust-toolchain.toml`)

## Build & Run

```bash
cd app

# Build everything
cargo build

# Run the GUI application
cargo run -p mangler_gui

# Run a graph headless from the CLI
cargo run -p mangler_cli

# Run tests
cargo test
```

## How It Works

1. **Values** flow between nodes. The type system includes: Bool, Integer, Decimal, Text, Color, Image, Path, FilterType, ImageType, ColorFormat, ColorSpace, BlendMode, NoiseWorleyDistanceFunction, TextHAlign, TextVAlign, VideoContainer, VideoCodec, Video, and Trigger. Values auto-convert where possible (e.g. Integer to Decimal, Bool to Color). Images are stored internally as `FloatImage` — 1–4 channel `f32` data — and only converted at I/O boundaries. A `Video` value is a lightweight handle (path + cached metadata) produced by the loader node and consumed by extract-frame ops; decoded frames are cached in a shared per-file ring buffer.

2. **Nodes** are created from operations. Each operation defines its inputs, outputs, and processing logic. Operations are registered via the `operations!` macro which generates the `Operation` enum and dispatch code.

3. **The graph engine** runs asynchronously on a tokio runtime. When an input changes, the engine determines which nodes are dirty and re-executes them. Results flow through connections to downstream nodes.

4. **The GUI** communicates with the engine through mpsc channels. `ChangeGraphMessage` and `ChangeNodeMessage` go from the UI to the engine; `GraphChangedMessage` and `NodeChangedMessage` come back with results, thumbnails, and status updates.

## Available Operations

### Numbers
- **Input:** Integer, Decimal, Pi, Tau, E
- **Arithmetic:** Add, Subtract, Multiply, Divide, Increment, Decrement, Max, Min, Clamp, Modulus, Round, Sign, Negate, Reciprocal, Average, Ceil, Floor, Trunc, Frac
- **Interpolation:** Step, Smoothstep, Lerp, Map Range
- **Trigonometry:** Sin, Cos, Tan, Asin, Acos, Atan, Atan2, Sinh, Cosh, Tanh
- **Algebra:** Abs, Sqrt, Cbrt, Nth Root, Pow, Factorial, GCD, LCM
- **Logarithmic:** Log, Ln, Exp, Log2, Log10
- **Random:** Random Integer, Random Decimal
- **Cast:** To Integer, To Decimal
- **Bitwise:** And, Or, Xor, Not, Left Shift, Right Shift

### Colors
- **Input:** from 9 color spaces — sRGB, Linear RGB, HSL, HSV, Lab, LCH, CMYK, XYZ, YUV
- **Output:** decompose to any of those same 9 color spaces
- **Generation:** From Hex, To Hex, Random Color
- **Manipulation:** Invert, Grayscale, Adjust HSV, Clamp, Set Alpha, Blend Mode (17 modes: Normal, Lerp, Multiply, Screen, Overlay, SoftLight, HardLight, ColorDodge, ColorBurn, Darken, Lighten, Difference, Exclusion, LinearBurn, LinearDodge, Divide, Subtract)
- **Analysis:** Most Common Colors (sampled from image), Distance, Luminance, Contrast Ratio, Color Temperature, Dominant Hue, Harmony Score, Mix Ratio
- **Harmony:** Complementary, Triadic, Analogous, Tetradic, Double Split Complementary, Monochromatic
- **Cast:** To Color

### Images
- **Input:** File, URL, Clipboard, Solid Color, Gradient, Text
- **Output:** File, Clipboard
- **Combine:** Blit, Blend (17 blend modes), Compare
- **Transform:** Crop, Resize, Resize Exact, Resize Fill, Flip H/V, Rotate 90/180/270, Rotate Around Center, Warp, Directional Warp, Safe Transform, Make Tile, Mirror, Seam Carve
- **Adjustments:** Contrast, Grayscale, Invert, Brighten, Hue Rotate, Levels, Auto Levels, Curves, Gradient Map, Posterize, Histogram Scan, Histogram Range, Distance
- **Blur:** Gaussian Blur, Directional Blur, Radial Blur, Slope Blur, Non-Uniform Blur
- **Filter:** Edge Detect, Emboss, Sharpen, Unsharpen
- **Channels:** Split, Merge, Shuffle
- **Shapes:** Rectangle, Ellipse, Polygon, Star, Line
- **Patterns:** Brick, Hexagonal, Weave, Tile Sampler
- **PBR:** Normal from Height, AO from Height, Curvature, Height Blend
- **Noise:** OpenSimplex, SuperSimplex, Perlin, Worley Distance, Worley Value, Billow, Cylinders, Domain Warp FBM, FBM, Heterogeneous Multifractal, Hybrid Multifractal, Ridged Multifractal, Value, Voronoi Crack, Voronoise, Reaction Diffusion, Erosion, Gabor, Gaussian, Crystal, Clouds, Plasma, Anisotropic, Dirt
- **Cast:** To Image

### Logic
- **Input:** Bool
- **Comparison:** Equal, Not Equal, Less Than, Less Equal, Greater Than, Greater Equal
- **Boolean:** And, Or, Not, Xor, Nand, Nor
- **Flow:** Select (mux — picks between two values based on a bool condition)

### Text
- **Input:** Text
- **Manipulation:** Append, Length, To Uppercase, To Lowercase, To String

### Videos
- **Input:** Video from File — opens a clip, emits a `Video` handle plus individual width/height/fps/duration/total_frames/container/codec sockets
- **Transform:** Extract Frame By Index, Extract Frame By Time — take a `Video` + frame-number/seconds, output the decoded frame as an `Image`
- **Output:** Video to File — renders the connected `image` stream to a video file via the Render button. Containers: MP4, MOV, MKV, WebM, AVI. Codecs: H.264 wired up today; H.265, VP8, VP9, AV1, MPEG-4, ProRes reserved in the compatibility matrix for future wiring.

> **Building with video support.** The `video` feature requires FFmpeg development libraries with `libx264`/`libx265`/`libvpx`/`libaom` compiled in. See `app/crates/mangler_core/docs/video-setup.md` — vcpkg's default `ffmpeg` port omits these and renders will fail with "Invalid argument" until you reinstall with the GPL feature set.

> **Video licensing.** The H.264/H.265 encoders (`libx264`/`libx265`) are GPL, so a binary built with the `video` feature and linked against a GPL FFmpeg is subject to the GPL when **distributed**. Building locally, or distributing without the video feature (or against an LGPL-only FFmpeg), avoids this. The `video` feature is off by default in `mangler_core`. See [video-setup.md](app/crates/mangler_core/docs/video-setup.md#licensing-read-before-distributing-builds) for the full breakdown and attribution.

## Subgraphs

Nodes can contain entire graphs, enabling composition and reuse of processing pipelines.
