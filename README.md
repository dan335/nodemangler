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
| **mangler** | `app/crates/mangler/` | Core library — value system, node graph engine, operations, color spaces |
| **nodemangler** | `app/crates/nodemangler/` | GUI application built with egui/eframe |

See each crate's README for details:
- [mangler README](app/crates/mangler/README.md) — the engine and operation library
- [nodemangler README](app/crates/nodemangler/README.md) — the desktop application

## Requirements

- **Rust nightly** toolchain (configured in `app/rust-toolchain.toml`)

## Build & Run

```bash
cd app

# Build everything
cargo build

# Run the GUI application
cargo run -p nodemangler

# Run tests
cargo test
```

## How It Works

1. **Values** flow between nodes. The type system includes: Bool, Integer, Decimal, String, Color, Image, Path, FilterType, ColorFormat, ImageType, ColorSpace, BlendMode, NoiseWorleyDistanceFunction, and Trigger. Values auto-convert where possible (e.g. Integer to Decimal, Bool to Color).

2. **Nodes** are created from operations. Each operation defines its inputs, outputs, and processing logic. Operations are registered via the `operations!` macro which generates the `Operation` enum and dispatch code.

3. **The graph engine** runs asynchronously on a tokio runtime. When an input changes, the engine determines which nodes are dirty and re-executes them. Results flow through connections to downstream nodes.

4. **The GUI** communicates with the engine through mpsc channels. `ChangeGraphMessage` and `ChangeNodeMessage` go from the UI to the engine; `GraphChangedMessage` and `NodeChangedMessage` come back with results, thumbnails, and status updates.

## Available Operations

### Numbers
- **Input:** Integer, Decimal
- **Arithmetic:** Add, Subtract, Multiply, Divide, Modulo, Power, Abs, Negate
- **Interpolation:** Lerp, Smoothstep, Clamp, Remap
- **Algebra:** Floor, Ceil, Round, Fract, Sign, Min, Max
- **Trigonometry:** Sin, Cos, Tan, Asin, Acos, Atan, Atan2
- **Logarithmic:** Log, Log2, Log10, Exp
- **Random:** Random Integer, Random Decimal
- **Cast:** To Integer, To Decimal, To Bool, To String
- **Bitwise:** And, Or, Xor, Not, Left Shift, Right Shift

### Colors
- **Input:** from 9 color spaces — sRGB, Linear RGB, HSL, HSV, Lab, LCH, CMYK, XYZ, YUV
- **Output:** decompose to any of those same 9 color spaces
- **Blend:** 17 blend modes (Normal, Lerp, Multiply, Screen, Overlay, SoftLight, HardLight, ColorDodge, ColorBurn, Darken, Lighten, Difference, Exclusion, LinearBurn, LinearDodge, Divide, Subtract)
- **Analysis:** Sample Most Common Colors from an image
- **Cast:** Color from/to other value types

### Images
- **Input:** File, URL, Clipboard, Solid Color, Gradient, Text
- **Output:** File, Clipboard
- **Combine:** Blit, Blend (17 blend modes)
- **Transform:** Crop, Resize, Flip H/V, Rotate 90/180/270, Rotate Around Center, Warp, Directional Warp, Safe Transform, Make Tile, Mirror
- **Adjustments:** Contrast, Grayscale, Invert, Brighten, Hue Rotate, Posterize, Levels, Auto Levels, Curves, Gradient Map, Histogram Scan, Histogram Range, Distance
- **Blur:** Gaussian Blur, Directional Blur, Radial Blur, Slope Blur, Non-Uniform Blur
- **Filter:** Edge Detect, Emboss, Sharpen, Unsharpen
- **Channels:** Split, Merge, Shuffle
- **Shapes:** Rectangle, Ellipse, Polygon, Star, Line
- **Patterns:** Brick, Hexagonal, Weave, Tile Sampler
- **PBR:** Normal from Height, AO from Height, Curvature, Height Blend
- **Noise:** Perlin, Simplex, OpenSimplex, SuperSimplex, Perlin Surflet, Worley (Distance/Value), Billow, Cylinders, FBM, Heterogeneous Multifractal, Hybrid Multifractal, Ridged Multifractal, Value

### Logic
- **Input:** Bool
- **Comparison:** Equal, Not Equal, Less Than, Less Equal, Greater Than, Greater Equal
- **Boolean:** And, Or, Not, Xor, Nand, Nor
- **Flow:** Select (mux — picks between two values based on a bool condition)

## Subgraphs

Nodes can contain entire graphs, enabling composition and reuse of processing pipelines.
