# Plan: Bring NodeMangler Closer to Substance Designer

## Context

NodeMangler has a solid async graph engine, 24 noise generators, 9 color spaces, and subgraph support. Phases 1-4 added blend modes, channel ops, distortion/tiling, shapes/patterns, and advanced filters. Phases 6, 7, and 9 are complete (logic nodes, text rendering, CLI polish). This plan covers remaining work.

---

## Phase 5: PBR / Material Pipeline (IN PROGRESS)

**Status:** 4 of 5 nodes implemented (normal_from_height, ao_from_height, curvature, height_blend). Remaining:

### 5E. PBR Material Export
- **New file:** `app/crates/mangler_core/src/operations/images/outputs/pbr_export.rs`
- Package BaseColor + Normal + Roughness + Metallic + Height + AO into standard formats
- Export to folder with naming conventions (Unity, Unreal, glTF)

---

## Phase 8: UI Improvements

**Scope:** All changes in `app/crates/mangler_gui/` (GUI crate).

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

### 10B. `duplicate-node` command

Copy a node with all its input values but no connections.

```rust
Commands::DuplicateNode {
    path: PathBuf,
    /// ID of the node to copy
    #[arg(long)]
    source: String,
    /// ID for the new copy (auto-generated if omitted)
    #[arg(long)]
    id: Option<String>,
}
```

Implementation:
1. Load graph, find source node
2. Get the source node's operation type
3. Add a new node with that operation via `graph.add_node()`
4. Copy all input values from source to new node via `graph.set_input()` (skip connections)
5. Save and print the new node ID

### 10C. `rename-node` command

Change a node's ID without losing connections.

```rust
Commands::RenameNode {
    path: PathBuf,
    /// Current node ID
    #[arg(long)]
    from: String,
    /// New node ID
    #[arg(long)]
    to: String,
}
```

Implementation:
1. Load graph, verify `from` exists and `to` doesn't
2. Remove node entry from `graph.nodes` HashMap
3. Reinsert with new key, updating `node.id`
4. Walk all other nodes: update any `input.connection` that references the old ID
5. Walk all other nodes: update any `output.connection` entries that reference the old ID
6. Save graph

This operates on the serialized `GraphSaveData` level (not via `Graph` methods) since the graph engine doesn't have a rename API. Load as `GraphSaveData` via `serde_json::from_str`, mutate, save.

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




## todo

- make_tile needs fixing - the seam artifacts make it unusable in its current state.
- make all noise generators tiling
- Add JPG quality control. The image_to_file node has no quality parameter for JPG output.
