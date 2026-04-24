# Plan: Bring NodeMangler Closer to Substance Designer

## Context

NodeMangler has a solid async graph engine, 24 noise generators, 9 color spaces, and subgraph support. Phases 1-4 added blend modes, channel ops, distortion/tiling, shapes/patterns, and advanced filters. Phases 6, 7, and 9 are complete (logic nodes, text rendering, CLI polish). This plan covers remaining work.

---

## Phase 8: UI Improvements

**Scope:** All changes in `app/crates/mangler_gui/` (GUI crate).

### 8A. Frame / Comment Nodes
- Allow users to draw labeled rectangles around groups of nodes for organization
- Implementation: add a `FrameNode` type to the graph editor that renders as a colored, labeled background rectangle behind contained nodes. Frames are draggable and resize to fit their contents.

### 8B. Dot / Reroute Nodes
- Small passthrough nodes for cleaner wire routing
- Implementation: a minimal node with one input and one output of type `Value` (passthrough). Renders as a small dot rather than a full node box.

---

## Phase 10: New CLI Commands

**Scope:** `app/crates/mangler_cli/src/main.rs`

### 10A. `validate` command

Check a graph for problems without running it.

```rust
Commands::Validate {
    path: PathBuf,
}
```

Checks to perform:
- **Dangling connections**: input references a node ID that doesn't exist in the graph
- **Out-of-bounds slots**: connection references an output/input index beyond the node's slot count
- **Type mismatches**: connected output type doesn't match input's expected type (use `ValueType` comparison, accounting for `accepts_any_type`)
- **Disconnected inputs with no value**: inputs that have no connection and still hold default values (warning, not error)
- **Orphan nodes**: nodes with no connections to anything (warning)

Print results as: `OK: graph is valid` or list of `ERROR:`/`WARN:` lines with node IDs and slot indices.



---

## Phase 11: Simplified `set-input` Value Syntax

The JSON syntax `'{"Decimal":3.14}'` is painful (shell quoting, verbose). Add `--auto` flag or make auto-detection the default.

### Option A: Smart detection (default, breaking)

Change `set-input --value` to auto-detect type based on the target input's `ValueType`:

```bash
# Current (still works — any value starting with { is parsed as JSON)
mangler_cli set-input --node a1 --index 0 --value '{"Decimal":3.14}' graph.mangle.json

# New (auto-detects Decimal because input 0 expects Decimal)
mangler_cli set-input --node a1 --index 0 --value 3.14 graph.mangle.json

# Enum inputs auto-resolve variant names
mangler_cli set-input --node comp --index 4 --value Screen graph.mangle.json
```

Implementation in `cmd_set_input`:
1. If value starts with `{`, parse as JSON (backwards compatible)
2. Otherwise, load graph first, look up the target input's `value.value_type()`
3. Based on type: parse as Bool/Integer/Decimal/Text/enum variant
4. If ambiguous or parse fails, return error with expected type and examples

### Option B: Separate flag (non-breaking)

Add `--raw <value>` as alternative to `--value <json>`:

```bash
mangler_cli set-input --node a1 --index 0 --raw 3.14 graph.mangle.json
mangler_cli set-input --node comp --index 4 --raw Screen graph.mangle.json
```

**Recommended: Option A** — auto-detect with `{` prefix as JSON escape hatch. It's technically breaking but no existing valid input starts with anything other than `{`, so in practice nothing breaks.

---

## Phase 12: Subgraph Correctness

### 12C. Expose subgraphs in `mangler_cli` ✅ MOSTLY COMPLETE

**Shipped:** four new commands (`add-subgraph`, `set-subgraph-path`, `expose-input`, `expose-output`), subgraph state in `info` output (path + child-graph status + per-slot `[exposed]` tags + JSON fields), menu re-enable (one-liner in `operations/mod.rs`), 8 unit tests, and `cmd_subgraph_e2e_via_cli` that chains the full workflow end-to-end.

**Bug fix surfaced by the e2e test:** `Graph::set_subgraph_path` was clearing parent input values on reload, which broke save→set-input→load→run cycles. Now preserves exposed-input values by matching on name, so user-driven values survive a save/load.

**Deferred:** Step 3 (un-skip Subgraph in `show-ops`) — low value vs. scope cost. `flatten_ops` returns `Vec<(String, Operation)>` and can't structurally accommodate a non-Operation entry without restructuring. Users still discover subgraphs via `mangle <path> add-subgraph --help` and `mangle <path> info`. Revisit if users actually complain.

---

**Original plan below (for reference):**

The engine supports subgraphs end-to-end, but the CLI has zero surface area for them. `show-ops` explicitly skips `OperationListItem::Subgraph` (`flatten_ops_subgraph_items_are_skipped` test), there's no way to add a subgraph node from the CLI, no way to point one at a child file, and `info` doesn't render the subgraph state. Parity with the GUI requires:

1. **New command: `add-subgraph`** — adds a subgraph node to a graph file.

   ```rust
   Commands::AddSubgraph {
       path: PathBuf,
       /// Optional ID for the new node (auto-generated if omitted)
       #[arg(long)]
       id: Option<String>,
       /// Optional child .mangle.json to load immediately
       #[arg(long)]
       subgraph_file: Option<PathBuf>,
       /// Optional canvas position (x,y)
       #[arg(long)]
       position: Option<String>,
   }
   ```

   Implementation: call `graph.add_node(id, AddNodeType::Subgraph, pos, true, None)`, then if `--subgraph-file` is provided, call `graph.set_subgraph_path(node_id, path)` before saving.

2. **New command: `set-subgraph-path`** — points an existing subgraph node at a child file, triggering the load.

   ```rust
   Commands::SetSubgraphPath {
       path: PathBuf,
       #[arg(long)]
       node: String,
       #[arg(long, value_name = "FILE")]
       subgraph_file: PathBuf,
   }
   ```

   Implementation: `graph.set_subgraph_path(node, subgraph_file)`, then save. Validate the file exists and is readable before calling; return an error with context if the load fails.

3. **Un-skip Subgraph in `show-ops`.** Currently `flatten_ops` filters `OperationListItem::Subgraph` so `show-ops` can't mention subgraphs. Replace the skip with a single "subgraph" entry that shows how to add one (points at `add-subgraph`). Update `flatten_ops_subgraph_items_are_skipped` test → `flatten_ops_includes_subgraph`.

4. **Extend `info` output for subgraph nodes.** When a node's `node_type` is `NodeType::Subgraph`, show:
   - `path:` the child file path, or `(not set)` if empty
   - `child nodes:` count of nodes in the loaded child graph, or `(not loaded)` if the path doesn't point to a valid file
   - `exposed inputs: <name> (ValueType)` for each
   - `exposed outputs: <name> (ValueType)` for each

   Existing `set-input` should then Just Work on exposed inputs (index 0+) now that 12B removed the magic `"file path"` slot.

5. **Menu re-enable.** Uncomment `//OperationListItem::Subgraph,` at `operations/mod.rs:774` — required for step 3 and for the GUI work in 12E. This is one line.

6. **New commands: `expose-input` / `expose-output`.** Mark a node's input or output as exposed so a parent subgraph can surface it. Required to build a child graph end-to-end via the CLI; without these there's no CLI path from "empty graph" to "valid subgraph target."

   ```rust
   Commands::ExposeInput {
       path: PathBuf,
       #[arg(long)]
       node: String,
       #[arg(long)]
       input: usize,
       #[arg(long, default_value_t = true)]
       expose: bool,
   }
   // + symmetric ExposeOutput
   ```

   Implementation: look up node, set `input.is_exposed = expose` (the existing `ChangeNodeMessage::SetExposeInput` handler in `app.rs` does exactly this — we just need the CLI entry point).

7. **Unit tests.** In `mangler_cli/src/commands_tests.rs`:
   - `test_add_subgraph_without_file` — adds an empty subgraph node, verify save roundtrip
   - `test_add_subgraph_with_file` — adds a subgraph node pointing at a tempfile child, verify exposed I/O surfaces
   - `test_set_subgraph_path` — add subgraph, set path later, verify node updates
   - `test_expose_input_marks_flag` / `test_expose_output_marks_flag` — verify exposure commands set the correct flags
   - `test_info_shows_subgraph_state` — verify `info` output includes the new subgraph fields

8. **End-to-end integration test** — `test_subgraph_e2e_via_cli` in `commands_tests.rs`. Proves the full CLI workflow: nothing → working subgraph → correct output. Steps:

   1. `cmd_new(child_path)` → empty child graph file
   2. `cmd_add_node(child_path, "numbers/inputs/decimal", id="val")` → adds decimal input node
   3. `cmd_expose_input(child_path, "val", 0, true)` → exposes the input
   4. `cmd_expose_output(child_path, "val", 0, true)` → exposes the output
   5. `cmd_new(parent_path)` → empty parent graph file
   6. `cmd_add_subgraph(parent_path, subgraph_file=Some(child_path), id=Some("sub"))` → adds subgraph node pointing at the child
   7. `cmd_set_input(parent_path, "sub", 0, "Decimal:42.0")` → drives the exposed input
   8. `cmd_run(parent_path)` → executes the graph
   9. `cmd_info(parent_path, node=Some("sub"))` → capture output; assert it shows the path, the single exposed input, and the output value `Decimal(42.0)`

   Cleanup both temp files on exit. Skip any step that requires functionality not yet implemented (test serves as a checklist during 12C development).

Estimated effort: ~half a day including tests. No GUI changes required. The e2e test is the single highest-leverage check — it will catch any integration gap between the new commands.

### 12D. Error-path coverage for subgraphs

The happy path is tested end-to-end. Currently untested:
- `set_subgraph_path` with a non-existent file
- `set_subgraph_path` with a malformed JSON file
- Child graph runtime error while parent is running
- Nested subgraphs (subgraph containing a subgraph)
- Channel saturation on the `rx_node_changed` path (many rapid runs)

Add targeted tests as time permits. Low priority until a real bug surfaces.

### 12E. Ship subgraphs as a user-visible GUI feature

The engine + inspector file picker shipped in 12B, but the menu entry is still commented out and no one has clicked through the feature in a running app.

**Do:**

1. Re-enable the menu entry (same one-line change as 12C step 5 — if 12C lands first, this is already done).
2. Manual smoke test: `cargo run -p mangler_gui`, add a subgraph node, pick a file, connect to exposed inputs, run, save, reopen, verify.
3. Wire a sensible empty state — when no file is picked, the node has zero I/O and looks broken on the canvas. Options: ghost/placeholder rendering on the node, auto-prompt file picker on creation, or leave as-is and rely on the inspector. Decide after the manual test reveals how bad the empty state actually feels.
4. **Exposed parameters UI polish** (was 8C). The exposed inputs already render in the inspector — minimum bar satisfied. Optional follow-on: group exposed inputs under a "parameters" sub-heading, show exposed-input names on the node body, or render a compact parameter panel on the node like Substance Designer.

Steps 1–3 are ~2 hours including the manual test. Step 4 is a separate day of polish, do later.

**Leaving for now:** `operations/sub_graph.rs` is dead code (older WIP attempt, doesn't compile against the current `OperationError` shape, isn't registered anywhere) but not worth touching right now per user direction.

### 12D. Error-path coverage for subgraphs

The happy path is tested end-to-end. Currently untested:
- `set_subgraph_path` with a non-existent file
- `set_subgraph_path` with a malformed JSON file
- Child graph runtime error while parent is running
- Nested subgraphs (subgraph containing a subgraph)
- Channel saturation on the `rx_node_changed` path (many rapid runs)

Add targeted tests as time permits. Low priority until a real bug surfaces.

---

## Implementation Pattern (for all new operations)

Every new operation follows the established pattern:

1. Create struct in appropriate directory with `#[derive(Debug, Clone, Serialize, Deserialize)]`
2. Implement `settings()` → `NodeSettings { name, description }`
3. Implement `create_inputs()` → `Vec<Input>` with `InputSettings` (Slider, DragValue, etc.)
4. Implement `create_outputs()` → `Vec<Output>`
5. Implement `async fn run(inputs)` using `convert_input()` + the 5-step pattern
6. Register in `operations!` macro in `app/crates/mangler_core/src/operations/mod.rs`
7. Add to `operation_list()` in appropriate category
8. Add `pub mod` in parent `mod.rs` files

**Key files to modify for every operation:**
- `app/crates/mangler_core/src/operations/mod.rs` — macro registration + menu
- Parent category `mod.rs` — module declaration

---

## Verification

After each phase (from `app/` directory):
- `cargo build` — must compile cleanly
- `cargo test -p mangler_core` — all existing tests pass
- `cargo run -p mangler_gui` — new nodes appear in menu, can be placed and connected
- Manual test: create a small graph exercising the new nodes, verify output images are correct

---

## Estimated Scope

| Phase | Description | Complexity | Status |
|-------|-------------|------------|--------|
| 5     | PBR Pipeline (1 remaining) | High | In Progress |
| 8     | GUI Improvements (~3+) | High | |
| 10    | New CLI Commands (3 commands) | Medium | |
| 11    | Simplified Value Syntax | Low | |
| 12    | Subgraphs (12A ✅, 12B ✅, 12C ✅ mostly, 12D errors + 12E GUI ship remaining) | Low + Medium | In Progress |
| 13    | Edge-Preserving Filter Family | Low–High | In Progress |
| 14    | Substance Parity Tier 1 (flood fill, normal combine, histogram select, bevel, highpass, morphology) | Medium | |
| 15    | Substance Parity Tier 2 (glows, color match, splatter, 3D shape primitives, gradient dynamic, dither) | Medium–High | |

---

## Phase 14: Substance Parity — Tier 1 (high-leverage, low effort)

**Scope:** New ops under `app/crates/mangler_core/src/operations/images/`. No engine or GUI changes.

### 14A. Flood Fill + Flood Fill Mapper

**Category:** `operations/images/patterns/` (pairs with `tile_sampler`).

Two nodes. Together they let users drive per-cell color/rotation/scale from a mask — the core Substance workflow for bricks, tiles, scales, etc. Without this pair, `tile_sampler` output is monochrome per tile.

- **`flood_fill`** — 1 input (mask image), 1 output (data image). Labels 4-connected regions of the mask. Encodes per-cell data into RGBA channels of the output: suggested packing is `R = normalized cell index`, `G = random [0,1] per cell`, `B = bbox size x`, `A = bbox size y` (document whatever we pick). Implementation: two-pass union-find over mask pixels above a threshold input.
- **`flood_fill_mapper`** — 3 inputs (flood-fill data image, optional gradient image for color lookup, optional random-range settings), 1 output (mapped image). Samples the data image per-pixel, then emits color/value by looking the cell's random channel into the gradient.

Decisions:
- Threshold input on `flood_fill` (default 0.5) — cells are luminance above threshold.
- Minimum cell size input to discard specks (default 1 pixel).
- Max cells cap (default 65536) to keep output well-defined. Cells beyond the cap are marked invalid (index = 0 sentinel).

Tests: grid of rectangles → correct per-cell indices; one connected blob → single cell; disconnected dots → distinct cells; below-threshold ignored.

### 14B. Normal Combine / Blend / Invert

**Category:** new `operations/images/pbr/normal_ops/` or flat under `pbr/` (the three fit cleanly next to `normal_from_height`).

- **`normal_combine`** — 2 normal-map inputs + blend-mode enum (`Whiteout`, `RNM`, `PartialDerivative`, `Linear`), 1 output. Detail-over-base blending. Default `Whiteout` (most versatile).
- **`normal_blend`** — 2 normal maps + opacity [0,1], 1 output. Linear-interpolates normalized normals, renormalizes. Simpler than `combine`.
- **`normal_invert`** — 1 normal map input + two bool inputs (`invert_x`, `invert_y`). Flips components. Essential for OpenGL ↔ DirectX normal-map handoffs.

Implementation notes: all three assume normals are stored in `[-1,1]` packed into RGB `[0,1]`. Add a shared helper `fn unpack_normal(pixel) -> [f32;3]` / `pack_normal(n) -> [f32;3]` in `pbr/mod.rs`. Renormalize after every op.

### 14C. Histogram Select

**Category:** `operations/images/adjustments/`.

Already have `histogram_scan` and `histogram_range`. Add `histogram_select` — picks a luminance band and outputs a soft-edged mask.

- Inputs: image, `position` [0,1], `range` [0,1], `contrast` [0,1].
- Output: single-channel mask where luminance is within `[position - range/2, position + range/2]`, softened by `contrast`.
- Implementation: `mask = smoothstep(range/2 - contrast*range, range/2, |lum - position|)` then invert.

### 14D. Bevel

**Category:** `operations/images/pbr/`.

From a binary-ish mask, produce a beveled height (optionally a normal map too).

- Inputs: mask image, `distance` (bevel width in px), `smoothing` [0,1], `corner_type` enum (`Round`, `Angular`), output-mode enum (`Height`, `Normal`).
- Output: one image (height or normal depending on mode).
- Implementation: run `distance` transform on the mask (you already have the distance op — factor its inner compute into a pub helper), clamp to `distance`, normalize to `[0,1]`, apply smoothstep based on `smoothing`. For normal output, chain through the existing `normal_from_height` helper.

### 14E. Highpass / Luminance Highpass

**Category:** `operations/images/filter/`.

Two small ops, ~30 lines each.

- **`highpass`** — 2 inputs (image, radius in px), 1 output. `out = 0.5 + (image - blur(image, radius))`. Use the existing blur helper if it's pub, else inline a gaussian.
- **`luminance_highpass`** — same as `highpass` but operates only on the luminance channel of the image, preserving chroma. Useful for sharpening without color ringing.

### 14F. Morphology (erode / dilate / open / close)

**Category:** `operations/images/filter/` (or a new `morphology/` if it grows).

Four ops on grayscale masks. Structuring element: square (default) or disk.

- **`erode`** — 2 inputs (image, radius px), 1 output. Per pixel, output = min over the structuring-element neighborhood.
- **`dilate`** — same with max.
- **`open`** — erode then dilate (removes specks).
- **`close`** — dilate then erode (fills holes).

Implementation: naive O(r²) per pixel is fine up to r≈15. For larger radii, use the van Herk / Gil–Werman O(1)-per-pixel algorithm — optional optimization, add only if real use hits the wall.

### 14G. Tests for Phase 14

Per-node `_tests.rs` alongside each source file (project convention). At minimum per op: round-trip via `Operation::run`, output dimensions match input, sentinel pixels hit expected values for a crafted input.

---

## Phase 15: Substance Parity — Tier 2 (medium effort, high-value FX)

**Scope:** New ops under `app/crates/mangler_core/src/operations/images/`. Requires the Phase 14 morphology + highpass helpers, so land 14 first.

### 15A. Shape Drop Shadow / Inner Glow / Outer Glow

**Category:** new `operations/images/fx/` directory.

Three related mask-FX ops.

- **`drop_shadow`** — inputs: mask, `offset_x` px, `offset_y` px, `blur_radius` px, `color` (Color), `opacity` [0,1]. Output: RGBA image of the shadow (composite onto the source separately using `blend`).
- **`outer_glow`** — inputs: mask, `radius` px, `intensity` [0,1], `color` (Color). Output: RGBA glow extending outside the mask. Implementation: `dilate(mask, radius) - mask`, blurred, tinted, alpha-scaled.
- **`inner_glow`** — inputs: mask, `radius` px, `intensity` [0,1], `color` (Color). Output: RGBA glow living inside the mask. Implementation: `mask - erode(mask, radius)`, blurred, tinted.

Document that composite (drop shadow under the source, glows over the source) is the user's responsibility — keeps each op single-output.

### 15B. Color Match (Histogram Transfer)

**Category:** `operations/images/adjustments/`.

- Inputs: `source` image, `reference` image, optional `strength` [0,1] (default 1.0).
- Output: `source` remapped so its per-channel cumulative histogram matches `reference`.
- Implementation: compute 256-bin CDF per channel on each image, build LUT `source_cdf[v] → reference value with closest CDF`, apply per pixel. Work in luminance + chroma if we want to preserve color relationships — but start with per-channel; document limitation.

### 15C. Splatter

**Category:** `operations/images/patterns/`.

Free-placement variant of `tile_sampler`. Stamps an input image at N random positions with per-stamp random rotation / scale / color tint.

- Inputs: pattern image, `count`, `seed`, `rotation_range`, `scale_range`, `color_variation`, `position_jitter`. (Ranges are `[min, max]` pairs — match the existing tile_sampler signatures.)
- Output: composed image.
- Implementation: seeded RNG, loop `count` times, sample a random transform, blit the pattern image with the current alpha blend. Reuses existing `blit` logic — consider factoring it into a shared helper.

Variant: **`splatter_circular`** — arranges stamps around a circle (center, radius, count). Optional, add if splatter lands cleanly.

### 15D. 3D Shape Primitives — Paraboloid / Pyramid / Cone

**Category:** `operations/images/shapes/`.

You have flat 2D shapes (rectangle, ellipse, polygon, star, line). Add parametric height shapes for PBR use.

- **`paraboloid`** — inputs: size, `falloff`, `smoothness`. Output: grayscale height `1 - (d/size)²` clamped.
- **`pyramid`** — inputs: size, `steps` (int, 0 = smooth), `rotation`. Output: height image of an N-sided pyramid (or axis-aligned square pyramid if rotation-only is easier).
- **`cone`** — inputs: size, `angle`, `truncate`. Output: height image of a cone with optional flat top.

All three produce a single grayscale height image centered in the output. Size inputs in normalized `[0,1]` coords relative to image dimensions.

### 15E. Gradient Dynamic

**Category:** `operations/images/adjustments/` (next to `gradient_map`).

Samples a gradient along a user-supplied direction field (RGB normal-map-style input interpreted as 2D vectors, R/G used).

- Inputs: `input` image (drives sample position via luminance or direction), `gradient` image, `vector_field` image (optional; if absent, sample uniformly by `input` luminance like `gradient_map`).
- Output: gradient-colored image where the sample coordinate along the gradient is modulated by the vector field.
- Use case: flow-aligned gradient painting, curvature-driven coloration.

Implementation: per pixel, compute `t = lum(input) + dot(vector_field_xy, grad_direction) * strength`, clamp to `[0,1]`, sample `gradient[t]`.

### 15F. Ordered Dither / Blue Noise Dither

**Category:** `operations/images/adjustments/`.

Two ops (or one op with an enum — prefer the enum since they share inputs).

- **`dither`** — inputs: image, `levels` (int, default 4), `pattern` enum (`Bayer4`, `Bayer8`, `BlueNoise`), `strength` [0,1].
- Output: quantized image with dither noise added before quantization.
- Implementation: embed a fixed Bayer 4×4 and 8×8 matrix and a 64×64 blue-noise texture as const arrays. Per pixel: `q = floor((v + threshold - 0.5/levels) * levels) / levels`. Tile the threshold texture across the output.

### 15G. Tests for Phase 15

Per-node `_tests.rs` alongside each source file. Splatter determinism: same seed → identical output. Dither: `levels=1` → binary output. Drop shadow: zero offset and zero blur → scaled mask.

---

## Phase 14/15 Ordering Notes

- **Do 14 first.** 15A (glows) depends on 14F (morphology). 15B (color match) shares the CDF helper with anything future in the histogram family.
- **Skipped intentionally:** Pixel Processor (weeks-scale), Bake from Mesh (needs 3D view), Transformation 2D unified node (incremental over existing rotate/transform ops — revisit if users ask).
- **Prereqs:** factor existing `distance` inner compute and `blur` gaussian into `pub(crate)` helpers before starting — 14D, 14E, 15A all want them.

---






## todo

- make_tile needs fixing - the seam artifacts make it unusable in its current state.
- make all noise generators tiling
- Add JPG quality control. The image_to_file node has no quality parameter for JPG output.
