# mangler_gui

The desktop application for [NodeMangler](../../../README.md) — a visual, node-based
editor for image and color manipulation, built with
[egui](https://github.com/emilk/egui) and
[eframe](https://github.com/emilk/egui/tree/master/crates/eframe).

It drives the [mangler_core](../mangler_core/) engine and reads/writes the same graph
JSON as the [mangler_cli](../mangler_cli/), so files round-trip between the two. Licensed
**MIT OR Apache-2.0**.

## Running

```bash
cargo run -p mangler_gui
```

The app launches maximized into a node-graph editor. On Windows the console window is
hidden in release builds.

## Features

### Graph editor

The central canvas where you build pipelines. Nodes sit on an infinite, pannable,
zoomable grid, and the graph re-evaluates automatically whenever an input changes.

- **Pan** by dragging the background
- **Zoom** with the scroll wheel
- **Add nodes** by dragging them out of the node menu
- **Connect** by dragging from an output dot to an input dot
- **Select** by clicking; selected nodes show a highlight border
- **Delete** via keyboard or context menu

### Node menu

A categorized, searchable panel on the left listing every available operation, grouped
into Numbers, Colors, Images, Logic, and Text (with subcategories like input,
transform, adjustments, filter, noise, …). Drag an operation onto the canvas to create a
node.

### Settings panel

Selecting a node shows its input parameters. The widget shown depends on the input type:

- **DragValue** — numeric scrubber
- **Slider** — bounded numeric slider
- **Checkbox** — boolean toggle
- **TextEdit** — string input
- **ComboBox** — enum/dropdown selection

### Image viewer & thumbnails

Click a node's output to inspect its result in the viewer panel — images, colors, and
text all render as visual previews. Compact thumbnails also appear directly on nodes for
quick reference. Image thumbnails are produced asynchronously by the engine's
thumbnail service, so the UI stays responsive while large frames resize; a node shows its
preview as soon as the deferred `ThumbnailReady` message lands.

### Themes

Four built-in themes, switchable from the menu bar:

- **Dark** — dark background, neutral tones
- **Dark Green** — dark background, green accents *(default)*
- **Light** — light background
- **Light Blue** — light background, blue accents

### Multiple programs

Several independent graphs can be open at once, accessible via tabs in the menu bar. Each
program owns its own engine instance, graph, editor state, and view panel.

### Save / load

Graphs serialize to JSON. Use the menu bar to create, open, and save graphs; the format
stores all nodes, their positions, input values, and connections. There is no backwards
compatibility for older files — they re-wire or re-export.

## Architecture

### Module overview

| Module | Purpose |
|--------|---------|
| `main.rs` | Entry point — configures the eframe window and launches the app |
| `app.rs` | Top-level `App` (`eframe::App`) — manages programs, themes, the menu bar |
| `program.rs` | `Program` — owns one engine instance plus all UI panels for one graph |
| `graph/` | Editor canvas: node rendering, input/output dots, connection drawing |
| `node_menu/` | Categorized, searchable operation list with drag-to-create |
| `settings/` | Node and graph settings panels |
| `view_window/` | Image viewer and view panel |
| `themes/` | The four theme definitions and switching |
| `title_bar/` | Window title bar |

### Communication with the engine

The GUI talks to the [mangler_core](../mangler_core/) engine over tokio mpsc channels:

```
UI ──ChangeGraphMessage──▶ Engine   (add/remove nodes & connections, save path)
UI ──ChangeNodeMessage───▶ Engine   (set input values, positions, expose in/outputs)
Engine ──GraphChangedMessage──▶ UI  (node/connection added, removed, loaded)
Engine ──NodeChangedMessage───▶ UI  (output values, thumbnails, timing, errors)
```

The engine runs on a separate tokio task. Each frame, the UI drains incoming messages and
updates its visual state accordingly.

## Dependencies

- `eframe` / `epaint` — egui framework for native desktop apps
- `egui_extras` — additional egui widgets
- `egui-phosphor` — icon font
- `mangler_core` — the engine and operation library
- `tokio` — async runtime
- `image` — icon loading
- `rfd` — native file dialogs (open/save)
- `puffin` — profiling (opt-in via the `PROFILE` constant)
- `sanitize-filename` — safe file naming
- `time` — time utilities
- `glam` — vector math
- `fastrand` — random number generation
- `winapi` (Windows only) — native window APIs
