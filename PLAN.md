# Plan: Bring NodeMangler Closer to Substance Designer

## Context

NodeMangler has a solid async graph engine, 14 noise generators, 9 color spaces, and subgraph support. Phases 1-4 added blend modes, channel ops, distortion/tiling, shapes/patterns, and advanced filters. Phases 5-7 mostly complete (PBR pipeline, logic nodes, text rendering). This plan covers remaining work.

---

## Phase 5: PBR / Material Pipeline (IN PROGRESS)

**Status:** 4 of 5 nodes implemented (normal_from_height, ao_from_height, curvature, height_blend). Remaining:

### 5E. PBR Material Export
- **New file:** `crates/mangler/src/operations/images/outputs/pbr_export.rs`
- Package BaseColor + Normal + Roughness + Metallic + Height + AO into standard formats
- Export to folder with naming conventions (Unity, Unreal, glTF)

---

## Phase 6: Logic Nodes ✅ COMPLETE

14 logic operations implemented across 4 subcategories: input (bool), comparison (equal, not_equal, less_than, less_equal, greater_than, greater_equal), boolean (and, or, not, xor, nand, nor), flow (select). All tests passing.

---

## Phase 7: Text Rendering ✅ COMPLETE

Text node implemented in `crates/mangler/src/operations/images/inputs/text.rs`. Uses embedded Manrope-Regular font via `ab_glyph`. Inputs: text, font_size, image_width, image_height, x_position, y_position. 7 tests passing.

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
| 5     | 1 remaining | High     | In Progress |
| 6     | 14        | Medium     | ✅ Complete |
| 7     | 1         | Medium     | ✅ Complete |
| 8     | ~3+       | High       | |
