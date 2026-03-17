# Plan: Bring NodeMangler Closer to Substance Designer

## Context

NodeMangler has a solid async graph engine, 14 noise generators, 9 color spaces, and subgraph support, but lacks the deep filter library, PBR pipeline, and advanced pattern generators that make Substance Designer powerful. This plan adds features in priority order — each phase delivers standalone value and unlocks the next.

---

## Phase 1: Foundation — Blend Modes & Channel Operations ✅ COMPLETE

**Status:** Implemented and tested. 334 tests pass (22 new).

### 1A. Blend Modes ✅
- **File:** `crates/mangler/src/color/blend.rs`
- Expanded `BlendMode` from 2 → 17 modes: Normal, Lerp, Multiply, Screen, Overlay, SoftLight, HardLight, ColorDodge, ColorBurn, Darken, Lighten, Difference, Exclusion, LinearBurn, LinearDodge, Divide, Subtract
- `apply_blend_mode()` helper for per-channel sRGB formulas
- All 9 color space blend methods updated (non-sRGB modes delegate to `blend_srgb`)

### 1B. Channel Split / Merge / Shuffle ✅
- `crates/mangler/src/operations/images/channels/split.rs` — 1 image → 4 grayscale (R, G, B, A)
- `crates/mangler/src/operations/images/channels/merge.rs` — 4 grayscale → 1 RGBA
- `crates/mangler/src/operations/images/channels/shuffle.rs` — remap channels via 4 integer selectors

### 1C. Levels & Curves ✅
- `crates/mangler/src/operations/images/adjustments/levels.rs` — black point, white point, gamma on Rgba32F
- `crates/mangler/src/operations/images/adjustments/curves.rs` — contrast curve with strength + midpoint

### 1D. Gradient Map ✅
- `crates/mangler/src/operations/images/adjustments/gradient_map.rs` — luminance → 2 or 3 color stops (Rec. 709)

All ops registered in `operations!` macro and `operation_list()` (under "adjustments" and "channels" categories).

---

## Phase 2: Distortion & Tiling ✅ COMPLETE

**Status:** Implemented and tested. 385 tests pass (51 new).

### 2A. Warp / Displacement Node ✅
- **File:** `crates/mangler/src/operations/images/transform/warp.rs`
- Inputs: source image, displacement map (R=X offset, G=Y offset), intensity (0-200)
- Bilinear interpolation for sub-pixel sampling
- Reusable `bilinear_sample_rgba()` helper used by other Phase 2 nodes

### 2B. Directional Warp ✅
- **File:** `crates/mangler/src/operations/images/transform/directional_warp.rs`
- Displacement along a single configurable angle, intensity driven by grayscale map
- Uses Rec. 601 luminance weighting

### 2C. Safe Transform ✅
- **File:** `crates/mangler/src/operations/images/transform/safe_transform.rs`
- Translate (normalized -1..1), rotate (degrees), scale with edge wrapping
- Near-zero scale clamped to prevent division by zero

### 2D. Make It Tile ✅
- **File:** `crates/mangler/src/operations/images/transform/make_tile.rs`
- Cross-fade horizontal then vertical edges with configurable blend size (1-50%)
- Handles edge cases: 1x1 images, zero blend regions

### 2E. Mirror / Symmetry ✅
- **File:** `crates/mangler/src/operations/images/transform/mirror.rs`
- Mirror across X, Y, or both axes with configurable offset (0-1)

All 5 ops registered in `operations!` macro and `operation_list()` under "transform" category.

---

## Phase 3: Shapes & Pattern Generation ✅ COMPLETE

**Status:** Implemented and tested. 445 tests pass (60 new).

### 3A. Shape Nodes ✅
- **New files** in `crates/mangler/src/operations/images/shapes/`:
  - `rectangle.rs` — width, height, corner radius, rotation (rounded-box SDF)
  - `polygon.rs` — n-sided regular polygon (sector-based SDF)
  - `star.rs` — n-pointed star with inner/outer radius
  - `line.rs` — start/end points, thickness (line segment SDF)
  - `ellipse.rs` — width, height, rotation (gradient-corrected ellipse SDF)
- All render as grayscale SDF with smoothstep anti-aliasing

### 3B. Brick / Tile Patterns ✅
- **New files** in `crates/mangler/src/operations/images/patterns/`:
  - `brick.rs` — configurable columns, rows, offset, gap
  - `hexagonal.rs` — hexagonal tile grid with axial coordinate rounding
  - `weave.rs` — basket weave with two-tone horizontal/vertical strands
- Render as grayscale patterns

### 3C. Tile Sampler ✅
- **New file:** `crates/mangler/src/operations/images/patterns/tile_sampler.rs`
- Inputs: pattern image, width/height, count X/Y, scale, scale random, rotation random, offset random, seed
- Seeded LCG PRNG for deterministic randomization, inverse-transform sampling, max compositing

All 9 ops registered in `operations!` macro and `operation_list()` under "shapes" and "patterns" categories.

---

## Phase 4: Advanced Filters ✅ COMPLETE

**Status:** Implemented and tested. 567 tests pass (40 new).

### 4A. Additional Blur Types ✅
- **New files** in `crates/mangler/src/operations/images/adjustments/`:
  - `directional_blur.rs` — blur along a configurable angle with samples/intensity controls
  - `radial_blur.rs` — circular/spin blur around image center
  - `slope_blur.rs` — direction/intensity driven by a grayscale slope map (Sobel gradient)
  - `non_uniform_blur.rs` — per-pixel blur intensity from a grayscale map (Vogel disc sampling)

### 4B. Edge Detection & Effects ✅
- `edge_detect.rs` — Sobel operator edge detection with intensity control (outputs grayscale)
- `emboss.rs` — angle-based emboss effect with intensity control
- `sharpen.rs` — 3x3 convolution sharpen with intensity control
- `posterize.rs` — reduce color levels (2-256)

### 4C. Histogram Operations ✅
- `histogram_scan.rs` — isolate a luminance range with smoothstep boundaries
- `histogram_range.rs` — remap luminance to a target min/max range
- `auto_levels.rs` — auto white/black point detection via histogram clipping

### 4D. Distance Transform ✅
- `distance.rs` — compute distance field from binary image with threshold and spread controls

All 12 ops registered in `operations!` macro and `operation_list()` under "adjustments" category.

---

## Phase 5: PBR / Material Pipeline

**Why fifth:** Requires the filter foundation from Phase 4.

### 5A. Normal Map from Height
- **New file:** `crates/mangler/src/operations/images/pbr/normal_from_height.rs`
- Sobel-based normal computation from grayscale height map
- Output: RGB normal map in tangent space
- Inputs: height map, intensity/scale

### 5B. Ambient Occlusion from Height
- **New file:** `crates/mangler/src/operations/images/pbr/ao_from_height.rs`
- SSAO-style computation from height map

### 5C. Curvature from Normal
- **New file:** `crates/mangler/src/operations/images/pbr/curvature.rs`
- Detect convex/concave areas from normal map

### 5D. Height Blend
- **New file:** `crates/mangler/src/operations/images/pbr/height_blend.rs`
- Blend two materials using their height maps for realistic layering

### 5E. PBR Material Export
- **New file:** `crates/mangler/src/operations/images/outputs/pbr_export.rs`
- Package BaseColor + Normal + Roughness + Metallic + Height + AO into standard formats
- Export to folder with naming conventions (Unity, Unreal, glTF)

---

## Phase 6: Graph & UI Enhancements

### 6A. Logic Nodes
- **New files** in `crates/mangler/src/operations/logic/`:
  - `switch.rs` — select between inputs based on boolean/integer
  - `if_else.rs` — conditional routing
  - `compare.rs` — comparison operators returning bool

### 6B. Pixel Processor Node
- **New file:** `crates/mangler/src/operations/images/pixel_processor.rs`
- Per-pixel custom expression evaluation
- Mini expression language or subgraph-per-pixel
- Very powerful but complex — could start with a simple math expression evaluator

### 6C. UI Improvements (in `crates/nodemangler/`)
- Frame/Comment nodes for graph organization
- Dot/Reroute nodes for cleaner wiring
- Exposed parameters UI on subgraphs
- 3D preview panel (mesh + material, using wgpu) — stretch goal

### 6D. Text Rendering
- **New file:** `crates/mangler/src/operations/images/inputs/text.rs`
- Render text string to image with font, size, color inputs
- Use `rusttype` or `ab_glyph` crate

---

## Implementation Pattern (for all new operations)

Every new operation follows the established pattern:

1. Create struct in appropriate directory with `#[derive(Debug, Clone, Serialize, Deserialize)]`
2. Implement `settings()` → `NodeSettings { name, description }`
3. Implement `create_inputs()` → `Vec<Input>` with `InputSettings` (Slider, DragValue, etc.)
4. Implement `create_outputs()` → `Vec<Output>`
5. Implement `async fn run(inputs)` using `convert_input()` + the 5-step pattern
6. Register in `operations!` macro in `crates/mangler/src/operations/mod.rs`
7. Add to `operation_list()` in appropriate category
8. Add `pub mod` in parent `mod.rs` files

**Key files to modify for every operation:**
- `crates/mangler/src/operations/mod.rs` — macro registration + menu
- Parent category `mod.rs` — module declaration

**For new Value/enum variants (Phase 1A blend modes):**
- `crates/mangler/src/color/blend.rs` — BlendMode enum + formulas
- `crates/mangler/src/value.rs` — display names if needed

---

## Verification

After each phase:
- `cargo build` — must compile cleanly
- `cargo test` — all existing tests pass
- `cargo run -p nodemangler` — new nodes appear in menu, can be placed and connected
- Manual test: create a small graph exercising the new nodes, verify output images are correct

---

## Estimated Scope

| Phase | New Nodes | Complexity | Dependencies | Status |
|-------|-----------|------------|--------------|--------|
| 1     | 8         | Low-Medium | None         | ✅ Done |
| 2     | 5         | Medium     | Phase 1      | ✅ Done |
| 3     | 9         | Medium-High| Phase 2      | ✅ Done |
| 4     | 12        | Medium     | Phase 1      | ✅ Done |
| 5     | ~5        | High       | Phase 4      |        |
| 6     | ~6+       | High       | All above    |        |

Phases 1-4 are complete. Phase 5 (PBR / Material Pipeline) is next.
