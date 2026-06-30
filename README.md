# NodeMangler

A node-based tool for image, video and color manipulation.  Comparable to
Substance Designer, Blender's compositor, or TouchDesigner.  Written in Rust.

I wrote the framework for this project back in 2023 but lost interest.  Picked it up again in 2026 with the help of Claude.

Includes a desktop GUI and headless CLI.  Create graphs in the GUI, save them as JSON, then run them from the CLI or vice versa.  CLI is intended to be easy for LLMs to use.

## Features

- Hundreds of operations across numbers, colors, images, logic, text, and video.
- Color spaces with lossless conversion between them: sRGB, Linear RGB, HSL, HSV,
  HWB, Lab, LCH, Oklab, Oklch, CMYK, XYZ, xyY, YUV, YCbCr — plus color analysis nodes.
- Procedural generation: noise types, patterns, shapes, and PBR
  (normal/height/AO/curvature) nodes.
- Images are processed as floating-point internally (1–4 channel `f32`) and only
  converted at I/O.
- Optional video support (behind the `video` feature): load clips, extract/retime
  frames, encode a graph to a video file.

## Repository structure

This is a monorepo:

- `app/` — Rust application (Cargo workspace)
- `website/` — Website (future)

### Crates

| Crate | Path | Purpose |
|-------|------|---------|
| **mangler_core** | `app/crates/mangler_core/` | The engine — value system, node graph, operation library, color spaces, video pipeline |
| **mangler_gui** | `app/crates/mangler_gui/` | Desktop GUI app built with egui/eframe |
| **mangler_cli** | `app/crates/mangler_cli/` | Headless CLI for building and running graphs |

Each crate has its own README with the full details:

- [mangler_core README](app/crates/mangler_core/README.md) — engine internals and the complete operation reference
- [mangler_gui README](app/crates/mangler_gui/README.md) — the desktop application
- [mangler_cli README](app/crates/mangler_cli/README.md) — the command-line interface

## Requirements

- **Rust stable** toolchain (pinned in `app/rust-toolchain.toml`)
- *(Optional)* FFmpeg development libraries to build with video support — see
  [video-setup.md](app/crates/mangler_core/docs/video-setup.md)

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
   Color, Image, Path, Video, Trigger, and a set of enum types (FilterType, ImageType,
   ColorFormat, ColorSpace, BlendMode, and more). Values auto-convert where it makes
   sense (Integer → Decimal, Bool → Color, …). Images are stored internally as
   `FloatImage` — 1–4 channel `f32` data — and only converted at I/O boundaries, so
   precision is preserved through the whole pipeline. A `Video` value is a lightweight
   handle (path + cached metadata); frames are decoded lazily and cached.

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

An overview of the operation library. See the
[mangler_core README](app/crates/mangler_core/README.md) for the full list.

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
- **Video** *(behind the `video` feature)* — load from file/URL, extract or retime
  frames, and encode a graph to a video file.

## Video support

Video operations live behind the `video` Cargo feature (enabled by default in
`mangler_gui` and `mangler_cli`, off by default in `mangler_core`).

> **Building with video.** The feature requires FFmpeg development libraries compiled
> with `libx264`/`libx265`/`libvpx`/`libaom` and `gpl`. See
> [video-setup.md](app/crates/mangler_core/docs/video-setup.md) — vcpkg's default
> `ffmpeg` port omits these and renders fail with a cryptic "Invalid argument" until you
> reinstall with the GPL feature set.

> **Licensing.** A binary built with the `video` feature and linked against GPL FFmpeg
> (`libx264`/`libx265`) is subject to the GPL **when distributed**. Building locally, or
> distributing without the video feature (or against an LGPL-only FFmpeg), avoids this.
> See [video-setup.md](app/crates/mangler_core/docs/video-setup.md#licensing-read-before-distributing-builds)
> for the full breakdown and attribution.

## License

NodeMangler is split-licensed by crate:

- **`mangler_core`** — the reusable engine — is licensed under **MIT OR Apache-2.0**
  (at your option).
- **`mangler_gui`** and **`mangler_cli`** — the distributed applications — are licensed
  under **GPL-3.0-or-later**, because they link GPL FFmpeg (`libx264`/`libx265`) via the
  `video` feature.

See [LICENSE.md](LICENSE.md) for the rationale and your obligations when distributing
builds. Unless you state otherwise, a contribution to a crate is offered under that
crate's license.
