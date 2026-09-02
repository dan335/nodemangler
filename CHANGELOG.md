# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

NodeMangler does not migrate saved graphs across breaking node-input changes.
Rewire or re-export affected graphs after those releases.

## [Unreleased]

### Added

- AUR package `nodemangler-bin`, generated alongside the other package
  manifests and pushed to the AUR on each release, so Arch and its derivatives
  (Omarchy, EndeavourOS, CachyOS) can install with `yay -S nodemangler-bin`.
  Linux archives now also carry the desktop entry and icon it installs.

### Changed

- egui-phosphor comes from crates.io (0.13) instead of a git fork, which leaves
  the dependency tree with no git sources at all.

## [1.0.11] - 2026-08-14

### Added

- Installed-font support on the from-text node: the font dropdown lists every
  face on the machine (plus the built-in Manrope), and graphs store the family
  name so a missing font falls back without losing the choice.
- Blend and composite place the foreground with scale and rotation, editable
  from a draggable box in the 2D preview.
- Raw develop controls on from-folder, so batch and watch shoots can develop
  camera raws with the same options as from-raw.
- Auto exposure adjustment node.
- Libraries list / thumbnails view with async, disk-cached image thumbs.
- Extra tone-map operators (Linear, Reinhard Luminance, Photographic Reinhard,
  Hejl, GT, AgX, Drago, PBR Neutral) with only the relevant controls shown.

### Changed

- Sample-pixel diameter can be dragged from the preview rim.
- From-clipboard captures are stored in the graph file (base64 PNG) so they
  survive reload instead of vanishing with the OS clipboard.

## [1.0.10] - 2026-08-08

### Added

- Spatial gizmos in the 2D preview: crop box, sample-pixel crosshair, and
  overlays for transform, perspective, swirl, spherize, vignette, drop shadow,
  mirror, circle, text, and gradient masks. Live drag commits on release.
- Mask nodes: hue-range, linear gradient, radial gradient, and mask combine.
- Skin tone color generator.
- Path output on from-file.

### Changed

- **Breaking:** crop is authored as 0–1 fractions of the source size (far edge
  rounded from origin + size) instead of raw pixels.
- Loading an older graph adopts newly added operation outputs instead of
  dropping them.

### Fixed

- Libraries panel scrollbar.
- Sampling and thumbnail quality around the new overlay toolkit.

## [1.0.9] - 2026-08-06

### Added

- Package-manager installs: Homebrew formula, Scoop bucket, and winget
  manifests (this repo is the tap/bucket). Linux releases include a
  single-file AppImage.

## [1.0.8] - 2026-08-01

Photo-editing suite. 443 nodes.

### Added

- Camera raw development via rawler (Canon CR3/CR2, Nikon NEF, Sony ARW,
  Fujifilm RAF, Adobe DNG, and ~20 more). New from-raw node; pickers and
  from-folder accept raw extensions automatically. Optional: build with
  `--no-default-features` to drop the LGPL rawler dependency.
- Watched folder / tethered shooting on from-folder: poll the camera download
  directory, wait for files to finish writing, then develop and export each
  new frame.
- White balance: Kelvin + tint on the Planckian locus (Bradford), plus a
  neutral-reference (eyedropper) color input. Per-channel curves on the
  curves node.
- Photo nodes: tone map, shadows/highlights, HSL mixer, color grade,
  negadoctor, defringe, texture, denoise, tone equalizer, bloom, lens
  distortion, chromatic aberration, and border.

## [1.0.7] - 2026-07-27

### Added

- HEIC/HEIF read (iPhone photos) via the pure-Rust `heif-oxide` crate.

### Changed

- Tone-curve editor: drag mirrored Bézier tangent knobs, including endpoints.

## [1.0.6] - 2026-07-14

Curve toolkit and the first photo-adjustment pass.

### Added

- From-folder input node, with batch runs that step through a directory and
  force-save outputs per file.
- Full curve category: generators (ellipse, polygon, star, arc, spiral,
  superellipse, wave, lissajous, random walk, fractal line), modifiers
  (transform, smooth, simplify, resample, jitter, offset, trim, round
  corners, mirror, reverse), combiners (join, morph), analysis (length,
  point count, bounds, centroid, area, sample point), trace contour,
  curve distance field, curve gradient, scatter on curve, and rasterize
  curve.
- Photoshop-style spline tone curve (settings-panel editor with histogram)
  and tone-curve profile/falloff inputs on 14 nodes (glows, vignette,
  distance, gradient easing, shape height profiles, rolling-hills dome,
  river valley cross-sections).
- Photo adjustments: black and white, clarity, color lookup (.cube LUT),
  dehaze, exposure, grain, photo filter, vibrance.
- Terrain: rolling hills, guided rolling hills, spectral terrain,
  hillslope diffusion. Meander gains decoupled render width, oxbow
  handling, and faster iteration.
- Drag image rows from Libraries into the graph.

### Changed

- Resampling of images with alpha is premultiplied, so transparent edges no
  longer fringe.
- 2D view: wider zoom range and fit-on-view when a node output is shown.
- Typed number fields can exceed slider min/max; dragging stays bounded.
- Node description/help and dirty flags are no longer written into graph
  files (re-derived on load).
- Distance field uses a Felzenszwalb–Huttenlocher transform.

### Removed

- Rivers simulation node (replaced by meander + carve river).

## [1.0.5] - 2026-07-12

### Added

- `Curve` value type with a 2D-preview overlay editor (anchors, insert,
  delete, Bézier handles).
- Meander node (curvature-driven river evolution) and carve-river terrain
  conforming.
- Image outputs (to-file, clipboard, material) gain an auto-save checkbox
  and a momentary save button. To-file and material share folder + file
  name + format, pre-filled with the graph directory and a unique stem.
- Libraries: single-click an image to preview it in the 2D panel.
- Hidden, settings-panel-only inputs (used to tuck material Custom slots
  off the graph).
- `scripts/fix_tofile.py` to migrate pre-1.0.4 to-file nodes.

### Changed

- New graphs stay in memory until the user saves. Closing an unsaved graph
  with nodes (or quitting with such tabs) prompts save / discard / cancel.
  No more leftover `untitled N` files in the default library.
- **Breaking:** to-file / material destination model is folder + name +
  format again, with explicit save gating (see 1.0.4).

### Fixed

- Cleanup pass across core, operations, GUI, and CLI (40+ bugs).

## [1.0.4] - 2026-07-09

### Added

- Project website.
- Libraries panel lists image files next to graphs; double-click adds a
  from-file node. Path dialogs seed from the graph's folder.
- Forward-compatible graph load: unknown future nodes become placeholders
  and round-trip their raw JSON. Opening a newer-version file holds
  auto-save until you edit.
- Concurrent-edit detection when another process rewrites the graph file.

### Changed

- Filename is the source of truth for a graph's name. In-app rename
  renames the file; save-as is copy-forward. New graphs auto-save into
  `~/Documents/NodeMangler` as collision-free `untitled N`.
- Settings panels compacted with hairline sections.
- egui 0.34 → 0.35, Rust 1.97.
- CLI enum variant lists are derived from `mangler_core` instead of
  hand-copied.
- **Breaking:** to-file and material collapse to a single file-path input
  whose extension picks the format. Saved graphs using those nodes need
  rewiring. (1.0.5 restores a folder/name/format split.)

## [1.0.3] - 2026-07-07

### Added

- Blender-style panel system: split tree, detachable secondary OS windows,
  persisted layout, and a default Graph-over-2D/3D center column.
- Libraries panel: named links to folders of `.mangler.json` graphs.
- Material export node with Godot / Unity / Unreal / Custom channel-pack
  presets. Right-click a material node to bind it to the 3D preview.
- 3D view: directional shadows and SSAO, compact toolbar.
- Releases also publish to [itch.io](https://danp.itch.io/nodemangler).

### Changed

- Spatial sizes (blur, glow, warp, cell size, …) are authored in pixels at
  a 1024px reference and scale with image resolution.
- Dropping a connection picks the nearest compatible port.
- Themes use a dedicated control-surface fill for checkboxes and sliders.

### Fixed

- Delete / Backspace deleting nodes unexpectedly.
- Splitter drag only resizes the adjacent panels.
- Long text thumbnails under nodes wrap and truncate.

## [1.0.2] - 2026-07-05

381 nodes.

### Added

- 64 analysis and utility nodes: image measurements (dimensions, mean,
  median, percentile, entropy, sharpness, perceptual hash, …),
  image→text (ASCII art, data URI, image info, palette hex, image hash),
  text predicates and encoding (base64, URL), extra number/trig helpers
  (phi, hypot, distance 2D, wrap, ping-pong, snap, inverse hyperbolics),
  sample pixel, constant image, and hydraulic erosion.
- `examples/video`: run a graph over every frame of a video.

### Changed

- Noise and filter operations reorganized into menu subcategories.
- Tiling transform replaced by the combined transform node.

### Fixed

- Checkerboard produced a solid image (now draws a real grid).
- Curvature panicked on grayscale inputs.

### Removed

- Creased noise.

## [1.0.1] - 2026-07-04

315 nodes. First post-release hotfix — published binaries actually launch.

### Added

- 18 noise generators: craters, fault terrain, fibers, flow, phasor,
  scales, scratches, truchet tiles, leaks, stains, peeling, smear,
  growth, caustics, veins, warped rings, creased, lightning.
- JPEG XL and PSD read; AVIF write.
- Logic: XNOR, approx-equal, in-range.
- PNG compression setting on to-file.

### Changed

- Blend allows negative positions so the foreground can slide past the
  top-left edge.

### Fixed

- **Release builds panicked on startup** because the window icon was loaded
  from a path that only exists on the build machine. The icon is now
  embedded in the binary.
- To-file no longer strips suffixes from dotted names like `render.v2`,
  and rejects empty names / folder-as-file paths.

## [1.0.0] - 2026-07-03

First public release. 294 nodes.

A node-based image editor and procedural texture generator in Rust — GUI
to author graphs, CLI (`mangle`) to run them headless. Same JSON graph
format in both.

### Added

- Visual graph editor (egui): pan/zoom canvas, searchable node menu,
  settings panel, async thumbnails, 2D image/color/text preview, 3D PBR
  material view, four themes, subgraphs.
- Headless CLI for scripts and automation.
- Floating-point pipeline: 1–4 channel `f32` from input to output.
- 14 color spaces with construct/decompose nodes, 17 blend modes, harmony
  and analysis.
- Image I/O (file, URL, clipboard, gradient, text), transforms (crop,
  resize, warp, kaleidoscope, seam carve, …), adjustments, blurs,
  filters (edges, morphology, stylize, dither), FX (shadow, glows),
  channels, shapes, patterns, and PBR maps (normal, AO, curvature, bevel).
- 28 noise generators (Perlin, OpenSimplex, Worley, Gabor, FBM, reaction
  diffusion, …).
- Numbers, logic, and text node libraries.
- Dual MIT OR Apache-2.0 license.
- Multi-OS release builds (Windows, Linux, macOS Apple Silicon + Intel).

[Unreleased]: https://github.com/dan335/nodemangler/compare/v1.0.11...HEAD
[1.0.11]: https://github.com/dan335/nodemangler/compare/v1.0.10...v1.0.11
[1.0.10]: https://github.com/dan335/nodemangler/compare/v1.0.9...v1.0.10
[1.0.9]: https://github.com/dan335/nodemangler/compare/v1.0.8...v1.0.9
[1.0.8]: https://github.com/dan335/nodemangler/compare/v1.0.7...v1.0.8
[1.0.7]: https://github.com/dan335/nodemangler/compare/v1.0.6...v1.0.7
[1.0.6]: https://github.com/dan335/nodemangler/compare/v1.0.5...v1.0.6
[1.0.5]: https://github.com/dan335/nodemangler/compare/v1.0.4...v1.0.5
[1.0.4]: https://github.com/dan335/nodemangler/compare/v1.0.3...v1.0.4
[1.0.3]: https://github.com/dan335/nodemangler/compare/v1.0.2...v1.0.3
[1.0.2]: https://github.com/dan335/nodemangler/compare/v1.0.1...v1.0.2
[1.0.1]: https://github.com/dan335/nodemangler/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/dan335/nodemangler/releases/tag/v1.0.0
