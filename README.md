# NodeMangler

A node-based visual programming tool for image and color manipulation, built in Rust.

NodeMangler lets you construct processing pipelines by connecting nodes in a visual graph editor. Each node performs a single operation — load an image, adjust contrast, convert color spaces, generate noise — and data flows through connections between them. The result is a non-destructive, composable workflow for image and color work.

## Project Structure

This is a Cargo workspace with two crates:

| Crate | Path | Purpose |
|-------|------|---------|
| **mangler** | `crates/mangler/` | Core library — value system, node graph engine, operations, color spaces |
| **nodemangler** | `crates/nodemangler/` | GUI application built with egui/eframe |

See each crate's README for details:
- [mangler README](crates/mangler/README.md) — the engine and operation library
- [nodemangler README](crates/nodemangler/README.md) — the desktop application

## Requirements

- **Rust nightly** toolchain (configured in `rust-toolchain.toml`)

## Build & Run

```bash
# Build everything
cargo build

# Run the GUI application
cargo run -p nodemangler

# Run tests
cargo test
```

## How It Works

1. **Values** flow between nodes. The type system includes: Bool, Integer, Decimal, String, Color, Image, Path, FilterType, ColorFormat, ImageType, ColorSpace, BlendMode, and more. Values auto-convert where possible (e.g. Integer to Decimal, Bool to Color).

2. **Nodes** are created from operations. Each operation defines its inputs, outputs, and processing logic. Operations are registered via the `operations!` macro which generates the `Operation` enum and dispatch code.

3. **The graph engine** runs asynchronously on a tokio runtime. When an input changes, the engine determines which nodes are dirty and re-executes them. Results flow through connections to downstream nodes.

4. **The GUI** communicates with the engine through mpsc channels. `ChangeGraphMessage` and `ChangeNodeMessage` go from the UI to the engine; `GraphChangedMessage` and `NodeChangedMessage` come back with results, thumbnails, and status updates.

## Available Operations

### Numbers
- Input: Integer, Decimal
- Arithmetic: Add
- Random: Random Integer, Random Decimal

### Colors
- Input from 9 color spaces: sRGB, Linear RGB, HSL, HSV, Lab, LCH, CMYK, XYZ, YUV
- Output/decompose to any of those same 9 color spaces
- Blend: Lerp
- Sample: Most Common Colors from an image

### Images
- Input: File, URL, Clipboard, Solid Color, Gradient
- Output: File, Clipboard
- Combine: Blit, Blend
- Transform: Crop, Resize, Resize Exact, Resize Fill, Flip H/V, Rotate 90/180/270, Rotate Around Center
- Adjustments: Blur, Contrast, Grayscale, Invert, Brighten, Hue Rotate, Unsharpen
- Noise: Perlin, Simplex, OpenSimplex, SuperSimplex, Perlin Surflet, Worley (Distance/Value), Billow, Cylinders, FBM, Heterogenous Multifractal, Hybrid Multifractal, Ridged Multifractal, Value

## Subgraphs

Nodes can contain entire graphs, enabling composition and reuse of processing pipelines.
