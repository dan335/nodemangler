# Mangler

![NodeMangler screenshot](screenshot.jpg)

A node-based tool for image and color manipulation.  Comparable to
Substance Designer, Blender's compositor, or TouchDesigner.  Written in Rust.

I wrote the framework for this project back in 2023 intending for it to be a replacement for Substance Designer but lost interest.  Picked it up again in 2026 with the help of Claude.

Includes a desktop GUI and headless CLI.  Create graphs in the GUI, save them as JSON, then run them from the CLI or vice versa.  CLI is intended to be easy for LLMs to create and edit graphs.

## Features

- Hundreds of operations across numbers, colors, images, logic, and text.
- Color spaces with lossless conversion between them: sRGB, Linear RGB, HSL, HSV,
  HWB, Lab, LCH, Oklab, Oklch, CMYK, XYZ, xyY, YUV, YCbCr — plus color analysis nodes.
- Procedural generation: noise types, patterns, shapes, and PBR
  (normal/height/AO/curvature) nodes.
- Images are processed as floating-point internally (1–4 channel `f32`) and only
  converted at I/O.

## Repository structure

This is a monorepo:

- `app/` — Rust application (Cargo workspace)
- `website/` — Website (future)

### Crates

| Crate | Path | Purpose |
|-------|------|---------|
| **mangler_core** | `app/crates/mangler_core/` | The engine — value system, node graph, operation library, color spaces |
| **mangler_gui** | `app/crates/mangler_gui/` | Desktop GUI app built with egui/eframe |
| **mangler_cli** | `app/crates/mangler_cli/` | Headless CLI for building and running graphs |

Each crate has its own README with the full details:

- [mangler_core README](app/crates/mangler_core/README.md) — engine internals and operation category overview
- [mangler_gui README](app/crates/mangler_gui/README.md) — the desktop application
- [mangler_cli README](app/crates/mangler_cli/README.md) — the command-line interface

## Requirements

- **Rust stable** toolchain (pinned in `app/rust-toolchain.toml`)

## Build & run

```bash
cd app

cargo build                 # build everything
cargo run -p mangler_gui    # launch the desktop app
cargo run -p mangler_cli    # run graphs headless (see the CLI README)
cargo test                  # run the test suite
```

## How it works

1. **Values flow between nodes.** The type system covers Bool, Integer, Decimal, Text,
   Color, Image, Path, Trigger, and a set of enum types (FilterType, ImageType,
   ColorFormat, ColorSpace, BlendMode, and more). Values auto-convert where it makes
   sense (Integer → Decimal, Bool → Color, …). Images are stored internally as
   `FloatImage` — 1–4 channel `f32` data — and only converted at I/O boundaries, so
   precision is preserved through the whole pipeline.

2. **Nodes are instances of operations.** Each operation declares its inputs, outputs,
   and async processing logic. Operations are registered through the `operations!` macro,
   which generates the `Operation` enum and all dispatch code.

3. **The graph engine runs asynchronously** on a tokio runtime. When an input changes,
   the engine marks the affected nodes dirty and re-executes them in dependency order;
   results propagate downstream through connections.

4. **The GUI and engine talk over channels.** `ChangeGraphMessage` and
   `ChangeNodeMessage` go UI → engine; `GraphChangedMessage` and `NodeChangedMessage`
   come back with outputs, thumbnails, timing, and status.

5. **Subgraphs** let a single node contain an entire nested graph, so you can package and
   reuse whole pipelines.

## Operations

An overview of the operation library. See [Node Reference](#node-reference) below for
every node by name, or run `cargo run -p mangler_cli -- show-ops` for full descriptions
and I/O of each one.

- **Numbers** — arithmetic, trigonometry, algebra, logarithms, interpolation, bitwise,
  random, casts, and constants (π, τ, e).
- **Colors** — construct from / decompose into all 14 color spaces; hex conversion; HSV
  adjustment, grayscale, invert, alpha; 17 blend modes; harmony (complementary, triadic,
  analogous, tetradic, …); and analysis (luminance, contrast ratio, temperature, harmony
  score, dominant colors sampled from an image).
- **Images** — the largest category:
  - *Inputs/outputs:* file, URL, clipboard, solid color, gradient, text
  - *Transform:* crop, resize, rotate, flip, warp, mirror, make-tile, seam carve, swirl,
    kaleidoscope, polar, spherize, perspective
  - *Adjustments:* contrast, levels/curves, saturation, hue, white/color balance,
    selective color, threshold, posterize, gradient map, histograms, and more
  - *Blur & filter:* gaussian/directional/radial/slope blur; edge detect, Canny, emboss,
    sharpen, bilateral, Kuwahara, oil paint, halftone, ASCII, morphology, convolution…
  - *FX, combine, channels, shapes, patterns, PBR* (normal/height/AO/curvature/bevel)
  - *Noise:* 28 generators (Perlin, OpenSimplex, FBM family, Worley/Voronoi, Gabor,
    reaction-diffusion, erosion, curl, plasma, clouds, …)
- **Logic** — comparisons, boolean ops, and a `select` multiplexer.
- **Text** — append, length, case conversion, to-string.

## Node Reference

Every node in the graph editor's Add Node menu, by category and subcategory
(294 operation nodes, plus subgraph nodes for composing whole pipelines).

### Numbers (61)

- **Input:** Decimal, E, Integer, Pi, Tau
- **Algebraic:** Absolute Value, Cube Root, Factorial, GCD, LCM, Nth Root, Power, Square Root
- **Arithmetic:** Add, Average, Ceil, Clamp, Decrement, Divide, Floor, Fractional Part, Increment, Max, Min, Modulus, Multiply, Negate, Reciprocal, Round, Sign, Subtract, Truncate
- **Bitwise:** Bitwise And, Bitwise Not, Bitwise Or, Bitwise Xor, Shift Left, Shift Right
- **Cast:** To Decimal, To Integer
- **Interpolation:** Lerp, Map Range, Smoothstep, Step
- **Logarithmic:** Exp, Ln, Log, Log10, Log2
- **Random:** Random Decimal, Random Integer
- **Trigonometry:** Acos, Asin, Atan, Atan2, Cos, Cosh, Sin, Sinh, Tan, Tanh

### Colors (52)

- **Input:** CMYK, HSL, HSV, HWB, Lab, LCH, Oklab, Oklch, RGB, RGB Linear, xyY, XYZ, YCbCr, YUV
- **Output:** To CMYK, To HSL, To HSV, To HWB, To Lab, To LCH, To Oklab, To Oklch, To RGB, To RGB Linear, To xyY, To XYZ, To YCbCr, To YUV
- **Analysis:** Color Temperature, Contrast Ratio, Distance, Dominant Hue, Harmony Score, Luminance, Mix Ratio, Most Common Colors
- **Generation:** From Hex, Random Color, To Color, To Hex
- **Harmony:** Analogous, Complementary, Double Split Comp, Monochromatic, Tetradic, Triadic
- **Manipulation:** Adjust HSV, Blend, Clamp, Grayscale, Invert, Set Alpha

### Images (161)

- **Input:** From Clipboard, From Color, From File, From Gradient, From Text, From URL
- **Output:** To Clipboard, To File
- **Adjustments:** Auto Levels, Brighten, Color Balance, Color Match, Color To Mask, Contrast, Curves, Dither, Frequency Split, Gradient Dynamic, Gradient Map, Grayscale, Histogram Range, Histogram Scan, Histogram Select, HSL, Hue Shift, Invert, Levels, Posterize, Replace Color, Saturation, Selective Color, Threshold, Vignette, White Balance
- **Blur:** Blur, Directional Blur, Non-Uniform Blur, Radial Blur, Slope Blur
- **Cast:** To Image
- **Channels:** Channel Merge, Channel Mixer, Channel Select, Channel Shuffle, Channel Split
- **Combine:** Blend, Compare, Composite
- **Filter:** Anisotropic Diffusion, Anisotropic Kuwahara, ASCII, Bilateral, Black Hat, Canny, Close, Convolution, Cross Hatch, Difference Of Gaussians, Dilate, Distance Field, Edge Detect, Emboss, Erode, Floyd Steinberg, Guided Filter, Halftone, Highpass, Kuwahara, Luminance Highpass, Median, Morphological Gradient, Non Local Means, Oil Paint, Open, Ordered Dither, Outline, Pixelate, Sharpen, SNN, Toon, Top Hat, Unsharp Mask, Vector Morphology
- **FX:** Drop Shadow, Inner Glow, Outer Glow
- **Noise:** Anisotropic Noise, Billow Noise, Blue Noise, Checkerboard Noise, Cloud Noise, Concentric Rings, Crystal Noise, Curl Noise, Dirt Noise, Domain Warp, Erosion, FBM Noise, Gabor Noise, Hybrid Multifractal Noise, Multifractal Noise, Open Simplex Noise, Perlin Noise, Plasma Noise, Reaction Diffusion, Ridged Multifractal Noise, Super Simplex Noise, Value Noise, Voronoi Blend, Voronoi Crack Noise, Wave, White Noise, Worley Distance Noise, Worley Value Noise
- **Patterns:** Brick, Flood Fill, Flood Fill Mapper, Hexagonal, Splatter, Tile Generator, Tile Sampler, Weave
- **PBR:** AO From Height, Bevel, Curvature, Height Blend, Normal Blend, Normal Combine, Normal From Height, Normal Invert, Normal To Height
- **Shapes:** Circle, Cone, Ellipse, Line, Paraboloid, Polygon, Pyramid, Rectangle, Star
- **Transform:** Crop, Directional Warp, Flip Horizontal, Flip Vertical, Kaleidoscope, Make Tile, Mirror, Perspective, Polar Coordinates, Resize, Resize Exact, Resize Fill, Rotate, Rotate 180, Rotate 270, Rotate 90, Seam Carve, Spherize, Swirl, Tiling Transform, Warp

### Logic (14)

- **Input:** Boolean
- **Boolean:** And, Nand, Nor, Not, Or, Xor
- **Comparison:** Equal, Greater Or Equal, Greater Than, Less Or Equal, Less Than, Not Equal
- **Flow:** Select

### Text (6)

- **Input:** Text
- **Manipulation:** Append, Length, To Lowercase, To String, To Uppercase

## License

Every crate in NodeMangler — `mangler_core`, `mangler_gui`, and `mangler_cli` — is
licensed under **MIT OR Apache-2.0** (at your option).

See [LICENSE.md](LICENSE.md) for details. Unless you state otherwise, a contribution is
offered under the same terms, with no additional conditions.
