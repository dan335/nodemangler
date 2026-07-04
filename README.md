# NodeMangler

An open-source, node-based image editor and procedural texture generator written in
Rust — in the spirit of Substance Designer, Blender's compositor, or TouchDesigner.
Build image and color pipelines by connecting nodes; the graph re-runs automatically as
you tweak it.

![NodeMangler screenshot](screenshot.jpg)

I wrote the framework for this back in 2023, intending it to be a replacement for
Substance Designer, but lost interest. Picked it up again in 2026 with the help of
Claude.

## Features

- **315 nodes** across images, colors, numbers, logic, and text — see the full
  [Node Reference](#node-reference) below.
- **Procedural generation** — 46 noise generators (Perlin, OpenSimplex, Worley, Gabor,
  phasor, reaction-diffusion, …) including a grunge set (leaks, stains, peeling, smear,
  growth) plus patterns, shapes, and PBR nodes (normal, height, ambient occlusion,
  curvature, bevel). A simulation category is scaffolded for physical-process
  generators (crack propagation, hydraulic erosion, frost growth, …) — see `plan.md`.
- **14 color spaces** with conversion between them: sRGB, Linear RGB, HSL, HSV, HWB,
  Lab, LCH, Oklab, Oklch, CMYK, XYZ, xyY, YUV, YCbCr — plus color analysis and harmony
  nodes.
- **Floating-point pipeline** — images stay 1–4 channel `f32` from input to output;
  conversion only happens at I/O.
- **16 file formats** — PNG, JPEG, GIF, WebP, TIFF, TGA, BMP, ICO, PNM, QOI, Farbfeld,
  Radiance HDR, OpenEXR, JPEG XL, PSD, and AVIF (JPEG XL and PSD are read-only, AVIF
  write-only), with quality and compression settings where the encoder supports them
  and 8/16/32-bit-float output color formats.
- **Subgraphs** — package an entire pipeline into a single reusable node.
- **GUI and CLI** — build graphs visually in the desktop app or headless from the
  command line. Both share the same JSON graph format, and the CLI is designed to be
  easy for scripts and LLMs to drive.

## Build & run

Requires a stable Rust toolchain (pinned in `app/rust-toolchain.toml`).

```bash
cd app
cargo run -p mangler_gui    # launch the desktop app
cargo run -p mangler_cli    # run graphs headless (see the CLI README)
cargo build                 # build everything
cargo test                  # run the test suite
```

## How it works

Nodes are instances of *operations* — each declares its inputs, outputs, and async
processing logic. Values (numbers, colors, images, text, …) flow along connections and
auto-convert where it makes sense. The engine runs on a tokio runtime: when an input
changes, affected nodes are marked dirty and re-execute in dependency order, with
results propagating downstream. Graphs save as JSON and round-trip freely between the
GUI and the CLI.

See the [mangler_core README](app/crates/mangler_core/README.md) for the engine
internals.

## Repository structure

- `app/` — Rust application (Cargo workspace)
- `website/` — Website (future)

| Crate | Path | Purpose |
|-------|------|---------|
| [**mangler_core**](app/crates/mangler_core/README.md) | `app/crates/mangler_core/` | The engine — value system, node graph, operation library, color spaces |
| [**mangler_gui**](app/crates/mangler_gui/README.md) | `app/crates/mangler_gui/` | Desktop app built with egui/eframe |
| [**mangler_cli**](app/crates/mangler_cli/README.md) | `app/crates/mangler_cli/` | Headless CLI for building and running graphs |

Each crate README goes into detail on that component.

## Operations

A quick tour of the operation library. The [Node Reference](#node-reference) below lists
every node by name, and `cargo run -p mangler_cli -- show-ops` prints full descriptions
and I/O for each.

- **Numbers** — arithmetic, trigonometry, algebra, logarithms, interpolation, bitwise,
  random, casts, and constants (π, τ, e).
- **Colors** — construct from / decompose into all 14 color spaces; hex conversion; 17
  blend modes; harmony (complementary, triadic, analogous, …); and analysis (luminance,
  contrast ratio, temperature, dominant colors sampled from an image).
- **Images** — the largest category: file/URL/clipboard/gradient/text I/O; transforms
  from crop and resize to warp, kaleidoscope, and seam carving; levels, curves, and
  color adjustments; blurs and filters from Gaussian to Kuwahara to halftone; shadows
  and glows; channel ops; shapes; patterns; PBR maps; and 46 noise generators.
- **Logic** — comparisons, boolean ops, and a `select` multiplexer.
- **Text** — append, length, case conversion, to-string.

## Node Reference

Every node in the graph editor's Add Node menu, by category and subcategory
(305 operation nodes, plus subgraph nodes for composing whole pipelines).

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

### Images (169)

- **Input:** From Clipboard, From Color, From File, From Gradient, From Text, From URL
- **Output:** To Clipboard, To File
- **Adjustments:** Auto Levels, Brighten, Color Balance, Color Match, Color To Mask, Contrast, Curves, Dither, Frequency Split, Gradient Dynamic, Gradient Map, Grayscale, Histogram Range, Histogram Scan, Histogram Select, HSL, Hue Shift, Invert, Levels, Posterize, Replace Color, Saturation, Selective Color, Threshold, Vignette, White Balance
- **Blur:** Blur, Directional Blur, Non-Uniform Blur, Radial Blur, Slope Blur
- **Cast:** To Image
- **Channels:** Channel Merge, Channel Mixer, Channel Select, Channel Shuffle, Channel Split
- **Combine:** Blend, Compare, Composite
- **Filter / Edges:** Canny, Difference Of Gaussians, Edge Detect, Highpass, Luminance Highpass, Sharpen, Unsharp Mask
- **Filter / Smoothing:** Anisotropic Diffusion, Bilateral, Guided Filter, Median, Non Local Means, SNN
- **Filter / Morphology:** Black Hat, Close, Dilate, Distance Field, Erode, Morphological Gradient, Open, Outline, Top Hat, Vector Morphology
- **Filter / Stylize:** Anisotropic Kuwahara, ASCII, Cross Hatch, Emboss, Halftone, Kuwahara, Oil Paint, Pixelate, Toon
- **Filter / Dither:** Floyd Steinberg, Ordered Dither
- **Filter:** Convolution
- **FX:** Drop Shadow, Inner Glow, Outer Glow
- **Noise:** Anisotropic Noise, Billow Noise, Blue Noise, Caustics Noise, Checkerboard Noise, Cloud Noise, Concentric Rings, Craters, Crystal Noise, Curl Noise, Dirt Noise, Domain Warp, Erosion, Fault Terrain, FBM Noise, Fibers, Flow Noise, Gabor Noise, Growth Noise, Hybrid Multifractal Noise, Leaks Noise, Lightning Noise, Multifractal Noise, Open Simplex Noise, Peeling Noise, Perlin Noise, Phasor Noise, Plasma Noise, Reaction Diffusion, Ridged Multifractal Noise, Scales, Scratches, Smear Noise, Stains Noise, Super Simplex Noise, Truchet Tiles, Value Noise, Veins Noise, Voronoi Blend, Voronoi Crack Noise, Warped Rings Noise, Wave, White Noise, Worley Distance Noise, Worley Value Noise
- **Patterns:** Brick, Flood Fill, Flood Fill Mapper, Hexagonal, Splatter, Tile Generator, Tile Sampler, Weave
- **PBR:** AO From Height, Bevel, Curvature, Height Blend, Normal Blend, Normal Combine, Normal From Height, Normal Invert, Normal To Height
- **Shapes:** Circle, Cone, Ellipse, Line, Paraboloid, Polygon, Pyramid, Rectangle, Star
- **Transform:** Crop, Directional Warp, Flip Horizontal, Flip Vertical, Kaleidoscope, Make Tile, Mirror, Perspective, Polar Coordinates, Resize, Resize Exact, Resize Fill, Rotate, Rotate 180, Rotate 270, Rotate 90, Seam Carve, Spherize, Swirl, Tiling Transform, Warp

### Logic (17)

- **Input:** Boolean
- **Boolean:** And, Nand, Nor, Not, Or, Xnor, Xor
- **Comparison:** Approx Equal, Equal, Greater Or Equal, Greater Than, In Range, Less Or Equal, Less Than, Not Equal
- **Flow:** Select

### Text (6)

- **Input:** Text
- **Manipulation:** Append, Length, To Lowercase, To String, To Uppercase

## License

MIT OR Apache-2.0, at your option — see [LICENSE.md](LICENSE.md).
