# Blender/Ghostty-Style Panel System for mangler_gui

## Context

The GUI currently hard-codes a fixed layout in `Program::show` (`program.rs:446-711`): node menu left (250px), graph editor center, settings right (300px), plus a floating viewer window (or separate OS viewport via the "viewer in separate window" checkbox). The user wants to keep the look/simplicity but replace this with a generic panel system like Blender's: a recursive tree of panels, each resizable and splittable horizontally/vertically, each switchable between **5 contexts: graph view, 2D preview, 3D preview, node list, settings** via an icon button in the panel's top-right corner.

**Confirmed decisions (from user Q&A):**
- Split/close commands live in the app settings menu and act on the **focused panel** (last hovered/clicked, with a subtle highlight).
- System default layout = today's 3 columns: node list | graph | settings.
- The floating embedded viewer window **goes away** — viewing a node output shows in 2D/3D preview panels. The "viewer in separate window" checkbox is **replaced** by "create separate window panel": a new OS window (egui viewport, draggable to other monitors) hosting a generic panel, itself splittable. Multiple windows allowed.
- Settings menu additions (in order, below theme): `create separate window panel`, `split horizontal`, `split vertical`, `close panel`, `set panel layout as default`, `reset panel layout to system default`.
- Layout is app-level (shared across program tabs); panel content renders the current Program's state.

All 5 panel contents already render into an arbitrary `&mut egui::Ui` (verified) — the work is the tree/splitter system, decomposing `Program::show`, dismantling `ViewPanel`, and persistence.

## Step 0 — Commit & push existing changes

User requested: before starting, commit and push the current working-tree changes (`value.rs`, `value_tests.rs`, `app.rs`, `graph_node_thumbnail.rs` — pre-existing modifications unrelated to this plan).

## Architecture

### New module `src/panels/` (add `mod panels;` in main.rs)

**`panel_kind.rs`**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PanelKind { Graph, Preview2D, Preview3D, NodeList, Settings }
// ALL: [PanelKind; 5], label() -> &str, icon() -> &str (egui-phosphor)
```

**`panel_tree.rs`** — pure logic (epaint Rect only, unit-testable without egui):
```rust
pub type LeafId = u64;
pub enum SplitDirection { Row, Column }  // Row = side-by-side, Column = stacked
pub enum PanelNode {
    Leaf { id: LeafId, kind: PanelKind },
    Split { direction: SplitDirection, fraction: f32, children: [Box<PanelNode>; 2] },
}
pub struct PanelTree { pub root: PanelNode }
```
Operations: `single()`, `system_default(work_width, &mut next_id)` (250 | flex | 300 as fractions), `reassign_ids()` (fresh ids on config load), `split(leaf_id, dir, new_id)` (replaces leaf with 50/50 split, new sibling same kind), `close(leaf_id)` (promote sibling; `Err(IsRoot)` if root), `set_kind`, `contains`, `leaves`, `first_leaf`, `set_fraction(&[usize] path, f32)`, `layout(rect) -> TreeLayout { leaves: Vec<(LeafId, PanelKind, Rect)>, splitters: Vec<Splitter> }`. Constants: `SPLITTER_WIDTH = 4.0`, `MIN_PANEL_SIZE = 80.0` (fractions clamped so children ≥ min). Fractions (not pixels) so panels scale proportionally on window resize, like Blender.

Tests in `panel_tree_tests.rs` (repo `#[path]` convention): split/close/promotion, layout tiles rect exactly with no overlap, min-size clamping, serde round-trip, reassign_ids uniqueness, system_default widths.

**`panel_view.rs`** — egui renderer:
```rust
pub enum PanelWindowId { Main, Secondary(u64) }
pub struct PanelFocus { pub window: PanelWindowId, pub leaf: LeafId }
pub fn render_tree(ui, tree: &mut PanelTree, work_rect, window: PanelWindowId,
    focused: &mut Option<PanelFocus>, program: &mut Program, theme) -> TreeRenderResponse
```
Per frame: `tree.layout(work_rect)`; splitters = `allocate_rect(Sense::drag())` with resize cursor, drag → pointer-relative fraction, clamped, `set_fraction`; each leaf rendered via `ui.push_id(leaf_id)` + `scope_builder(max_rect(rect))` + clip → `program.show_panel(ui, leaf_id, kind, theme)` (push_id is mandatory — duplicate settings/list panels would clash egui widget ids otherwise). Hover inside rect → update `focused` (sticky). Chrome: small frameless icon button at `rect.right_top() + (-24, 4)` placed with `ui.put` **after** content (wins pointer), opens popup menu of the 5 kinds. Focus highlight: 1px subtle stroke (theme accent, reduced alpha) on the focused leaf. Response carries `graph_rects: Vec<Rect>` for overlay hit-tests.

**`panel_windows.rs`** — secondary OS windows:
```rust
pub struct SecondaryWindow { pub id: u64, pub tree: PanelTree, pub close_requested: bool }
```
`show_secondary_window(ctx, win, focused, program, theme)` uses `ctx.show_viewport_immediate(ViewportId::from_hash_of(("panel_window", win.id)), ...)` — same pattern as today's `ViewPanel::show_separate` (view_panel.rs:63-103), unique id per window. CentralPanel inside → `render_tree(...)` with `PanelWindowId::Secondary(id)`. Titlebar close → `close_requested`.

### Preview content extraction (dismantle `ViewPanel`)

- **`src/view_window/preview_2d.rs`** (new): free fn `show(ui, viewer: &mut ImageViewer, graph_node, output_index, cursor_position, theme)` — value dispatch moved verbatim from `ViewPanel::show_content` (Image → ImageViewer, Color → ColorViewer, else TextViewer), plus faint "{node} · {output}" header. `show_empty(ui, theme)` placeholder when nothing is being viewed.
- **`src/view_window/preview_3d.rs`** (new): `struct Preview3dPanel { viewer: Viewer3d, assignments: MaterialChannelAssignments }` — mesh combo + material channel UI moved from `ViewPanel::show_material_ui` + `viewer.show_material(...)`. Standalone (no longer gated on a viewed image; the 2D/3D `ViewTab` dies).
- **Delete** `src/view_window/view_panel.rs` entirely; update `view_window/mod.rs`.
- Remove `view_in_separate_window` everywhere: `App` field (app.rs:18,129), threading (app.rs:38,62), `AppMenu` params + checkbox (app_menu.rs:110), `Program::show` param, viewer block (program.rs:614-636), `is_mouse_over_viewer` (then remove the dead param from `GraphEditor::show` as cleanup).

### Program refactor (`src/program.rs`)

Split monolithic `Program::show` into:
- `update(&mut self, ctx, ui)` — once per frame: pointer tracking, copy/paste, `rx_graph_changed`/`rx_node_changed` pumps, dropped files, repaint policy.
- `show_panel(&mut self, ui, leaf_id, kind, theme)` — match on `PanelKind`, delegating to extracted private methods: `show_node_list_panel` (menu_panel block, drag capture), `show_settings_panel` (padding + ScrollArea + node/graph settings block), `show_graph_panel` (graph_editor.show + full `GraphEditorResponse` handling), `show_preview_2d_panel` / `show_preview_3d_panel` (per-leaf viewer instances).
- `show_overlays(&mut self, ctx, ui, theme, graph_rects: &[Rect], work_rect)` — Tab-to-search, delete keys, search popup, drag-from-menu release (node added when pointer inside **any** graph rect), ghost node, status message, timing, help text (anchor to `work_rect.left() + 20.0` instead of `NODE_MENU_WIDTH`). All `node_graph_rect.contains(p)` checks become `graph_rects.iter().any(|r| r.contains(p))`.

New fields: `viewers_2d: HashMap<LeafId, ImageViewer>`, `viewers_3d: HashMap<LeafId, Preview3dPanel>` (per-leaf pan/zoom & arcball state; resolve double-borrows by destructuring `let Self { viewers_3d, graph_editor, .. } = self;`). Add `prune_viewers(&mut self, live: &HashSet<LeafId>)` — 3D holds GL resources. **GraphEditor stays a single shared instance** (duplicate graph panels show the same pan/zoom; input already gated on `editor_rect.contains(cursor)`; per-leaf pan/zoom is a documented follow-up).

### App menu (`src/app_menu/app_menu.rs`)

Replace the checkbox with (inside `settings` menu, after `theme`):
```
──────────
create separate window panel
split horizontal
split vertical
close panel
──────────
set panel layout as default
reset panel layout to system default
```
`BarResponse` gains `panel_action: Option<PanelAction>`; `enum PanelAction { NewWindow, SplitHorizontal, SplitVertical, ClosePanel, SaveLayoutAsDefault, ResetLayout }`. "split horizontal" → `SplitDirection::Row` (side-by-side), "split vertical" → `Column` (stacked) — mapping isolated in one match, easy to flip.

### App integration (`src/app.rs`)

New fields: `main_tree: Option<PanelTree>` (lazy — needs work rect), `secondary_windows: Vec<SecondaryWindow>`, `next_window_id: u64`, `next_leaf_id: LeafId`, `focused: Option<PanelFocus>`.

`App::ui` flow: bg + app_menu → program tab handling (unchanged) → compute `work_rect` (below `APP_MENU_HEIGHT`) → lazy tree init (config `default_layout` + `reassign_ids`, else `system_default`) → `handle_panel_action(...)`:
- Target = `focused` if still valid, else main tree's first leaf.
- Split → `tree.split(leaf, dir, new_id)`, focus new leaf. Close → `tree.close`; `Err(IsRoot)`: main = no-op + status toast, secondary = close window; on success prune viewers.
- NewWindow → push `SecondaryWindow` with single-leaf tree (focused panel's kind, else Preview2D).
- SaveLayoutAsDefault → load config, `config.default_layout = Some(main_tree.root.clone())`, save (same pattern as theme persistence, app.rs:55-57). Main-window tree only; secondary windows are session-only (v1).
- ResetLayout → `main_tree = system_default(...)`, clear focus, prune viewers (in-memory; user can re-save as default).

Then: `program.update(ctx, ui)` → `render_tree(main window)` → `program.show_overlays(..., &resp.graph_rects, work_rect)` → each secondary window via `show_secondary_window` → retain non-closed, fix dangling focus.

Behavior notes: viewing a node with no Preview2D leaf open anywhere → set `viewing_node_id_index` + status toast "no 2D preview panel open — use a panel's corner menu". Menu actions apply to last-hovered panel even in a secondary window.

### Config (`src/config.rs`)

```rust
pub struct AppConfig {
    #[serde(default)] pub theme: Option<String>,
    #[serde(default)] pub default_layout: Option<PanelNode>,
}
```
Existing theme-only config.json keeps loading (`serde(default)` + existing `unwrap_or_default()`). Extend `config_tests.rs`: round-trip + missing-field.

## Implementation Phases (each compiles & runs)

1. **Tree core** — `panels/{mod,panel_kind,panel_tree,panel_tree_tests}.rs`; no UI change; tests green.
2. **Program decomposition (behavior-preserving)** — `preview_2d.rs`/`preview_3d.rs` (copied; ViewPanel still alive); refactor `Program::show` into `update`/`show_panel`/`show_overlays` behind a temporary `show` wrapper that computes today's 3 fixed rects (dummy leaf ids). App unchanged; manual check: identical behavior.
3. **Tree rendering in main window; ViewPanel dies** — `panel_view.rs`; App gains tree/focus/leaf-counter; delete wrapper, old rect code, `view_panel.rs`, `view_in_separate_window`; menu gets split/close (`PanelAction`). Manual check: default layout matches old app; split/resize/close/switch/focus/2D-3D previews work.
4. **Persistence** — config field, `reassign_ids`, "set panel layout as default" / "reset panel layout to system default" menu items; startup prefers user default.
5. **Secondary windows** — `panel_windows.rs`, "create separate window panel", focus routing across windows, last-leaf-close closes window.
6. **Cleanup** — remove `is_mouse_over_viewer` from `GraphEditor::show`, prune_viewers wiring, cursor polish, clippy.

## Verification

- After each phase (from `app/`): `cargo build && cargo test -p mangler_gui`; `cargo clippy` at end.
- Unit: tree ops, layout tiling/min-size, serde, id reassignment, config round-trip.
- Manual GUI checklist: first launch = 250|flex|300; splitter drag + min size; corner menu switches all 5 kinds; hover moves focus highlight, split H/V/close act on it; close last main panel = no-op; view node → 2D preview panel; 3D preview arcball; two 2D previews have independent pan/zoom; save-as-default → restart restores; reset → system default; old theme-only config still loads; secondary window: create/switch kind/split/move to monitor/close both ways; drag node from list onto graph panel at any position; Tab search only over graph panels; program tabs share layout, keep per-program state.

## Critical Files

- `src/panels/panel_tree.rs`, `panel_view.rs`, `panel_windows.rs`, `panel_kind.rs` (new)
- `src/view_window/preview_2d.rs`, `preview_3d.rs` (new); `view_panel.rs` (delete)
- `src/program.rs` (decompose show; viewer maps)
- `src/app.rs` (tree ownership, focus, actions, secondary windows)
- `src/app_menu/app_menu.rs` (menu items, PanelAction)
- `src/config.rs`, `src/main.rs`

---

# Backlog

- [ ] Frame / Comment Nodes
- [ ] Dot / Reroute Nodes
