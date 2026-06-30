# mangler_cli

The headless command-line interface for [NodeMangler](../../../README.md) — create, edit,
and run node graphs without the GUI.

It drives the same [mangler_core](../mangler_core/) engine and reads/writes the same graph
JSON as the [desktop app](../mangler_gui/), so files round-trip freely between the two.
That makes it handy for automation, scripting, CI, batch processing, and driving the
engine from agents/LLMs. Licensed **MIT OR Apache-2.0**.

## Running

```bash
# from app/
cargo run -p mangler_cli -- <PATH> <COMMAND> [OPTIONS]
```

The binary is `mangler_cli` (its built-in help refers to the command as `mangle`). Most
commands take the graph JSON path **first**, then a subcommand:

```bash
cargo run -p mangler_cli -- graph.json new
cargo run -p mangler_cli -- graph.json add-node --type images/combine/blend
cargo run -p mangler_cli -- graph.json run
```

Pass the global `--json` flag for machine-readable output instead of human-readable text.

## Commands

### Discovery (no graph file needed)

- `show-ops [--group <category>] [--search <keyword>] [--compact]` — list, browse, or
  search every operation
- `show-op <op>` — detailed info for one operation (e.g. `show-op images/combine/blend`)
- `show-types [<TypeName>]` — enum types and their valid variants
  (e.g. `show-types blendmode`)
- `show-values` — the value-format reference used by `set-input --value`

### Building a graph

- `new` — create a new empty graph JSON file
- `info [--node <id>] [--compact]` — inspect nodes, inputs, outputs, and connections
- `add-node --type <op> [--id <id>] [--name <name>]`
- `remove-node --id <id>`
- `connect --from <node>:<output> --to <node>:<input>`
- `disconnect --node <id> --input <index>`
- `set-input --node <id> --input <i> --value <Type:value> [--input <j> --value <…> …]`
  (batch-capable)
- `set-name --node <id> --name <name>`
- `set-enabled --node <id> --enabled <true|false>`
  (disabled nodes pass inputs through unchanged)

### Subgraphs

- `add-subgraph [--id <id>] [--subgraph-file <child.mangle.json>]`
- `set-subgraph-path --node <id> --subgraph-file <file>`
- `expose-input --node <id> --input <index> [--expose <true|false>]`
- `expose-output --node <id> --output <index> [--expose <true|false>]`

### Running & output

- `run` — execute the graph and print every node's output values
- `show-output --node <id> [--output <i>] [--stats] [--sample x,y] [--save out.png]` —
  run, then inspect one node's output: per-channel image statistics, pixel sampling
  (`x,y` or named positions like `center`), and saving images (or JSON for non-image
  values) to a file

## Value format

Inputs are set with `--value Type:value`:

```
decimal:3.14
integer:5
bool:true
text:hello
color:1.0,0.0,0.0,1.0          # r,g,b,a
path:out.png
blendmode:Multiply             # enum variant by name
```

Run `show-values` for the full reference (including the JSON form), and
`show-types <Type>` to list a given enum's valid variants.

## Example

```bash
# create a graph, add an "add" node, set its two inputs, run it
cargo run -p mangler_cli -- g.json new
cargo run -p mangler_cli -- g.json add-node --type numbers/arithmetic/add --id sum
cargo run -p mangler_cli -- g.json set-input --node sum --input 0 --value decimal:2 --input 1 --value decimal:3
cargo run -p mangler_cli -- g.json show-output --node sum     # -> 5
```

## Architecture

| Module | Purpose |
|--------|---------|
| `main.rs` | Entry point — parses args (clap) and dispatches to a command |
| `cli.rs` | clap `Cli` / `Commands` definitions and help text |
| `commands.rs` | Command implementations against the mangler_core engine |
| `value_parse.rs` | Parses `Type:value` strings into engine `Value`s |
| `image_stats.rs` | Per-channel image statistics and pixel sampling |
| `format.rs` | Human-readable and `--json` output formatting |
| `helpers.rs` | Shared utilities (graph load/save, node lookup) |

## Dependencies

- `mangler_core` — the engine and operation library
- `clap` — argument parsing (derive)
- `tokio` — async runtime for graph execution
- `serde_json` — graph (de)serialization
- `image` — saving image outputs
- `glam` — vector math

The `mangler_cli` binary is licensed **MIT OR Apache-2.0**.
