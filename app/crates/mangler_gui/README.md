# nodemangler

Desktop GUI application for NodeMangler — a visual node-based editor for image and color manipulation. Built with [egui](https://github.com/emilk/egui) and [eframe](https://github.com/emilk/egui/tree/master/crates/eframe).

## Running

```bash
cargo run -p nodemangler
```

The application launches maximized with a node graph editor. On Windows, the console window is hidden in release builds.

## Features

### Graph Editor
The central canvas where you build processing pipelines. Nodes are placed on an infinite, pannable, zoomable grid. Connect outputs to inputs by dragging between connection points. The graph re-evaluates automatically when inputs change.

- **Pan** the canvas by dragging the background
- **Zoom** with the scroll wheel
- **Add nodes** by dragging them from the node menu on the left
- **Connect nodes** by dragging from an output dot to an input dot
- **Select nodes** by clicking them; selected nodes show a highlight border
- **Delete nodes** with the keyboard or context menu

### Node Menu
A categorized, searchable panel on the left side listing all available operations. Nodes are organized into Numbers, Colors, and Images categories with subcategories (input, output, transform, adjustments, noise, etc.). Drag an operation from the menu onto the canvas to create a node.

### Settings Panel
When a node is selected, its input parameters appear in a settings panel. Depending on the input type, you'll see different widgets:
- **DragValue** — numeric scrubber
- **Slider** — bounded numeric slider
- **Checkbox** — boolean toggle
- **TextEdit** — string input
- **ComboBox** — dropdown selection

### Image Viewer
Click a node's output to view its result in the image viewer panel. Images, colors, and text values all render as visual previews. Thumbnails appear directly on nodes for quick reference.

### Themes
Four built-in themes:
- **Dark** — dark background with neutral tones
- **Dark Green** — dark background with green accents (default)
- **Light** — light background
- **Light Blue** — light background with blue accents

Switch themes from the application menu bar.

### Multiple Programs
The app supports multiple independent graph programs open simultaneously, accessible via tabs in the menu bar. Each program has its own graph, editor state, and view panel.

### Save / Load
Graphs serialize to JSON files. Use the menu bar to save, open, or create new graphs. The file format stores all nodes, their positions, input values, and connections.

## Architecture

### Module Overview

| Module | Purpose |
|--------|---------|
| `main.rs` | Entry point — configures eframe window and launches the app |
| `app.rs` | Top-level `App` struct implementing `eframe::App` — manages programs, themes, menu bar |
| `program.rs` | `Program` — owns a mangler engine instance and all UI panels for one graph |
| `graph/` | Graph editor canvas, node rendering, input/output dot rendering, connection drawing |
| `node_menu/` | Categorized operation list panel with drag-to-create |
| `settings/` | Node and graph settings panels |
| `view_window/` | Image viewer and view panel |
| `themes/` | Theme definitions and switching |
| `title_bar/` | Window title bar |

### Communication with the Engine

The GUI communicates with the [mangler_core](../mangler_core/) engine through tokio mpsc channels:

```
UI ──ChangeGraphMessage──> Engine (add/remove nodes, connections)
UI ──ChangeNodeMessage───> Engine (update input values, positions)
Engine ──NodeChangedMessage──> UI (output values, thumbnails, timing, errors)
Engine ──GraphChangedMessage─> UI (node added/removed/loaded, connections)
```

The engine runs on a separate tokio task. The UI polls for incoming messages each frame and updates the visual state accordingly.

## Dependencies

- `eframe` / `epaint` — egui framework for native desktop apps
- `egui_extras` — additional egui widgets
- `egui-phosphor` — icon font
- `mangler_core` — the core engine (workspace dependency)
- `tokio` — async runtime
- `image` — icon loading
- `rfd` — native file dialogs (open/save)
- `puffin` — profiling (opt-in via `PROFILE` constant)
- `sanitize-filename` — safe file naming
- `time` — time utilities
- `glam` — vector math
- `fastrand` — random number generation
- `winapi` (Windows only) — native window APIs
