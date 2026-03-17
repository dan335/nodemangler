# Plan: Bring NodeMangler Closer to Substance Designer

## Context

NodeMangler has a solid async graph engine, 14 noise generators, 9 color spaces, and subgraph support. Phases 1-4 added blend modes, channel ops, distortion/tiling, shapes/patterns, and advanced filters. Phase 5 partially complete (3 of 5 PBR nodes). This plan covers remaining work.

---

## Phase 5: PBR / Material Pipeline (IN PROGRESS)

**Status:** 4 of 5 nodes implemented (normal_from_height, ao_from_height, curvature, height_blend).

### 5D. Height Blend ✅
- **File:** `crates/mangler/src/operations/images/pbr/height_blend.rs`
- Blends two materials using their height maps; overlay shows through where its height exceeds the base
- Inputs: base color, base height, overlay color, overlay height, blend amount (0-1), contrast (0-1)
- Outputs: blended color image + blended height image
- 8 tests passing

### 5E. PBR Material Export
- **New file:** `crates/mangler/src/operations/images/outputs/pbr_export.rs`
- Package BaseColor + Normal + Roughness + Metallic + Height + AO into standard formats
- Export to folder with naming conventions (Unity, Unreal, glTF)

---

## Phase 6: Logic Nodes

**Why:** Adds conditional/branching logic to the graph, enabling dynamic workflows where node behavior changes based on input values.

### 6A. Switch Node
- **New file:** `crates/mangler/src/operations/logic/switch.rs`
- Select between N inputs based on an integer index
- Inputs: index (integer), input_0 through input_N (any Value type)
- Output: the Value at the selected index (clamped to valid range)
- Implementation: accept `Value` type inputs so it works with images, colors, numbers, etc. Use `convert_input()` to get the index, then pass through the selected input unchanged.

### 6B. If/Else Node
- **New file:** `crates/mangler/src/operations/logic/if_else.rs`
- Conditional routing: if condition is true, output input A; otherwise output input B
- Inputs: condition (bool), if_true (any Value), if_false (any Value)
- Output: the selected Value
- Implementation: similar to switch but with a boolean selector. The condition input uses `ValueType::Bool`. Both branches are evaluated (since the graph is dataflow, not control flow), but only one is forwarded.

### 6C. Compare Node
- **New file:** `crates/mangler/src/operations/logic/compare.rs`
- Comparison operators returning a boolean
- Inputs: A (decimal), B (decimal), operator (enum: Equal, NotEqual, LessThan, LessEqual, GreaterThan, GreaterEqual)
- Output: Bool result
- Implementation: add a new `CompareOp` enum to `value.rs` (similar to how `BlendMode` works). The `run()` function converts both inputs to decimal, applies the selected comparison, outputs a `Value::Bool`.

### 6D. Boolean Logic Nodes
- **New files** in `crates/mangler/src/operations/logic/`:
  - `and.rs` — logical AND of two bool inputs
  - `or.rs` — logical OR of two bool inputs
  - `not.rs` — logical NOT of a single bool input
- Simple pass-through operations on `Value::Bool`

### Registration
- Create `crates/mangler/src/operations/logic/mod.rs` with `pub mod` for each node
- Add all logic nodes to the `operations!` macro in `crates/mangler/src/operations/mod.rs`
- Add a new "logic" category in `operation_list()` with subcategories for conditional and boolean ops

---

## Phase 7: Text Rendering

**Why:** Text-to-image is essential for labels, watermarks, and texture stamping. Enables generating text masks that feed into blend/composite workflows.

### 7A. Text Node
- **New file:** `crates/mangler/src/operations/images/inputs/text.rs`
- Render a text string to a grayscale image (white text on black background)
- Inputs:
  - text (String) — the text to render
  - font_size (Decimal, default 64.0) — size in pixels
  - image_width (Integer, default 512) — output image width
  - image_height (Integer, default 512) — output image height
  - x_position (Decimal, 0-1, default 0.5) — horizontal position (normalized)
  - y_position (Decimal, 0-1, default 0.5) — vertical position (normalized)
- Output: grayscale DynamicImage

### Implementation Details
- **Crate dependency:** Add `ab_glyph` to `crates/mangler/Cargo.toml` — it's a pure-Rust font rasterizer with no system dependencies
- **Font handling:** Embed a default font (e.g., `DejaVuSans.ttf` or `Roboto-Regular.ttf`) using `include_bytes!()` so the node works without external font files
- **Rendering pipeline:**
  1. Load font with `ab_glyph::FontArc::try_from_slice()`
  2. Scale glyphs to requested `font_size` using `font.as_scaled(font_size)`
  3. Layout glyphs: iterate chars, accumulate `h_advance` for x positions, use `height()` for line height
  4. Rasterize: for each glyph, call `font.outline_glyph()` then `draw()` to get per-pixel coverage values
  5. Write coverage values (0.0-1.0) into a `GrayImage`, then convert to `DynamicImage`
- **Positioning:** The x/y position inputs define where the text center lands on the image (0.5, 0.5 = centered). Calculate text bounding box first, then offset all glyphs so the bbox center aligns with the target position.
- Register under the "images > inputs" category in `operation_list()`

---

## Phase 8: UI Improvements

**Scope:** All changes in `crates/nodemangler/` (GUI crate).

### 8A. Frame / Comment Nodes
- Allow users to draw labeled rectangles around groups of nodes for organization
- Implementation: add a `FrameNode` type to the graph editor that renders as a colored, labeled background rectangle behind contained nodes. Frames are draggable and resize to fit their contents.

### 8B. Dot / Reroute Nodes
- Small passthrough nodes for cleaner wire routing
- Implementation: a minimal node with one input and one output of type `Value` (passthrough). Renders as a small dot rather than a full node box.

### 8C. Exposed Parameters UI on Subgraphs
- When a subgraph exposes inputs/outputs, show a clean parameter panel on the parent node
- Implementation: surface the exposed inputs as editable widgets on the subgraph node's inspector panel

### 8D. 3D Preview Panel (Stretch Goal)
- Display a mesh with the generated PBR material applied
- Requires wgpu integration alongside egui

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

---

## Verification

After each phase:
- `cargo build` — must compile cleanly
- `cargo test -p mangler` — all existing tests pass
- `cargo run -p nodemangler` — new nodes appear in menu, can be placed and connected
- Manual test: create a small graph exercising the new nodes, verify output images are correct

---

## Estimated Scope

| Phase | New Nodes | Complexity | Status |
|-------|-----------|------------|--------|
| 5     | 2 remaining | High     | In Progress |
| 6     | ~7        | Medium     | |
| 7     | 1         | Medium     | |
| 8     | ~3+       | High       | |
