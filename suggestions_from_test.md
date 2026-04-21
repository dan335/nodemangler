# Suggestions from CLI Testing

These suggestions come from building an 800x400 "i mangled this" image (blue background, green text) entirely through the CLI.

---

## Bugs

### `image to file` saves 0-byte JPGs from RGBA sources
The gradient map and blend node outputs saved as 0-byte JPG files. Had to route through a `blit` node (which forces RGB) as a workaround. JPG doesn't support alpha — the save node should either auto-flatten alpha before encoding or return a clear error like "cannot save RGBA image as JPG, use PNG or flatten first."

### `blend` node produces 0-byte/1x1 output after bad input
When a `ColorSpace` input was set to an invalid enum value (`"SRGB"` instead of `"Srgb"`), the blend node produced a 1x1 pixel output. Even after correcting the value and re-running, the output remained broken. The node may be caching the error state and not recomputing properly when inputs are fixed.

### Relative path `"."` silently fails for save node
Setting the folder input to `{"Path":"."}` produced a 0-byte file with no error message. The output path showed `{"Path":""}` (empty), indicating the path wasn't resolved. Forward-slash full paths like `"D:/temp/mangler"` worked.

### Save node returns empty path on failure with no error
When the save node fails to write (due to path or format issues), it returns `{"Path":""}` and exit code 0. There is no indication anything went wrong — no stderr, no error flag, no non-zero exit code.

---

## CLI UX Improvements

### `list-ops` needs filtering
The command dumps 200+ operations with no way to narrow results. Suggestions:
- `list-ops --category images/input` — filter by category path
- `list-ops --search "text"` — fuzzy search by name
- `list-ops --type Decimal` — filter by input/output type
- `list-ops --categories` — just list the top-level categories

### No way to discover enum/type values before trial and error
Had to guess enum variants like `"Center"` vs `"Middle"` for `TextVAlign`, and `"SRGB"` vs `"Srgb"` for `ColorSpace`. The error messages do show valid values on failure, which is helpful, but you shouldn't have to fail first. Suggestions:
- `list-types TextVAlign` command to print accepted values
- Show accepted values in `info` output: `in[4] blend mode (BlendMode: Over|Screen|Multiply|...)`
- Show accepted values in `add-node` or `set-input --help` contextually

### `info` output could show more at a glance
- Show accepted enum values inline for enum-typed inputs
- Show which inputs have default values vs explicitly set values (maybe a `*` marker)
- Show the node's description alongside its name
- Optionally show only a single node: `info --node add1 graph.json`

### Flag naming inconsistencies
- `set-input` uses `--index` but `disconnect` uses `--input` for the same concept (zero-based input slot)
- `remove-node` requires `--id <ID>` but `add-node` uses `--id` as optional (auto-generated if omitted)
- `set-input` uses `--node` and `--index` as separate flags, but `connect`/`disconnect` use the `node:slot` colon syntax
- Consider unifying: either always use `--node <id> --slot <n>` or always use the `node:slot` compact form

### Error messages could include more context
- When `set-input` fails with a bad enum, include which node and input name in the error (not just the JSON parse error)
- When a node fails during `run`, show which node errored and which input caused it
- Consider a `--verbose` flag for `run` that shows execution order and per-node timing

---

## Feature Ideas

### `validate` command
Check a graph for issues before running:
- Type mismatches between connected slots
- Required inputs with no value and no connection
- Cycle detection
- Warn about disconnected subgraphs

### `run --node <id>` — partial execution
Execute only the subgraph needed for a specific node and print its outputs. Useful for debugging large graphs without running everything.

### `run --watch` — re-run on file change
Watch the graph JSON for external edits and re-execute automatically. Useful when editing the JSON directly or integrating with other tools.

### Simplified value syntax for `set-input`
The current JSON syntax `'{"Decimal":3.14}'` is verbose and error-prone (quoting issues on different shells). Consider accepting shorthand:
- `--value 3.14` (auto-detect Decimal)
- `--value true` (auto-detect Bool)
- `--value '"hello"'` (auto-detect Text)
- `--value '#FF0000'` (auto-detect Color from hex)
- Keep `--value-json '{...}'` for explicit/complex types

### `export` command for visualization
- `export --format dot graph.json` — output Graphviz DOT for rendering the node graph
- `export --format mermaid graph.json` — output Mermaid diagram syntax
- `export --format ascii graph.json` — render a simple ASCII graph in the terminal

### `duplicate-node` command
Copy a node with all its input values but no connections. Useful when building repetitive graph sections.

### `rename-node` command
Change a node's ID without rebuilding connections. Currently you'd need to remove and re-add.

### Templates / scaffolding
Pre-built graph fragments for common patterns:
- `mangler_cli template "text on background" --text "hello" --bg "#0044CC" --fg "#00FF44"`
- `mangler_cli template list` to show available templates

### `diff` command
Compare two graph files and show what changed — nodes added/removed, connections changed, values modified.

### `undo` support
Keep a history of graph states (maybe as a `.mangler_history` file alongside the graph). `mangler_cli undo graph.json` reverts the last mutation.

### Batch mode / script file
Accept a file of commands to execute in sequence:
```
$ mangler_cli batch commands.txt graph.json
```
This would make graph construction reproducible and scriptable without shell scripting overhead.

### `pin` / `freeze` a node
Mark a node's outputs as cached so `run` skips recomputation. Useful for expensive nodes (image processing) during iterative development.
