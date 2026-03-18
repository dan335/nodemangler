# Logic Nodes Implementation Plan

## Overview

Add a new top-level `logic` operation category for boolean logic, comparison, and flow control.

## Menu Structure

```
logic                    ← root category (peer of numbers, colors, images)
  ├─ input               (bool)
  ├─ comparison           (equal, not_equal, less_than, less_equal, greater_than, greater_equal)
  ├─ boolean              (and, or, not, xor, nand, nor)
  └─ flow                 (select)
```

## Phase 1 — Pure logic ops (no engine changes) ✅ COMPLETE

All operations follow the existing pattern. No changes to graph execution. All 14 operations implemented and tested (969 tests passing).

| Operation | Inputs | Output | Status |
|-----------|--------|--------|--------|
| bool input | input: Bool | Bool | ✅ |
| equal | a: Decimal, b: Decimal | Bool (handles Integer/Decimal/Bool/String) | ✅ |
| not_equal | a: Decimal, b: Decimal | Bool (handles Integer/Decimal/Bool/String) | ✅ |
| less_than | a: Decimal, b: Decimal | Bool (handles Integer/Decimal/mixed) | ✅ |
| less_equal | a: Decimal, b: Decimal | Bool (handles Integer/Decimal/mixed) | ✅ |
| greater_than | a: Decimal, b: Decimal | Bool (handles Integer/Decimal/mixed) | ✅ |
| greater_equal | a: Decimal, b: Decimal | Bool (handles Integer/Decimal/mixed) | ✅ |
| and | a: Bool, b: Bool | Bool | ✅ |
| or | a: Bool, b: Bool | Bool | ✅ |
| not | input: Bool | Bool | ✅ |
| xor | a: Bool, b: Bool | Bool | ✅ |
| nand | a: Bool, b: Bool | Bool | ✅ |
| nor | a: Bool, b: Bool | Bool | ✅ |
| select | condition: Bool, if_true: any, if_false: any | any (passthrough) | ✅ |

### Files created

```
crates/mangler/src/operations/logic/
  ├─ mod.rs
  ├─ inputs/
  │   ├─ mod.rs
  │   └─ bool_input.rs
  ├─ comparison/
  │   ├─ mod.rs
  │   ├─ equal.rs
  │   ├─ not_equal.rs
  │   ├─ less_than.rs
  │   ├─ less_equal.rs
  │   ├─ greater_than.rs
  │   └─ greater_equal.rs
  ├─ boolean/
  │   ├─ mod.rs
  │   ├─ and.rs
  │   ├─ or.rs
  │   ├─ not.rs
  │   ├─ xor.rs
  │   ├─ nand.rs
  │   └─ nor.rs
  └─ flow/
      ├─ mod.rs
      └─ select.rs
```

### Existing files modified

- `crates/mangler/src/operations/mod.rs` — added `pub mod logic;`, registered 14 ops in `operations!` macro, added `logic` section to `operation_list()`
- `CLAUDE.md` — documented logic operation categories

### Engine changes: None

### Design notes

- Bool inputs use `None` for InputSettings (no `Checkbox` variant exists in `InputSettings` — the UI renders Bool values as checkboxes automatically based on the `Value::Bool` type)
- Boolean ops (and, or, not, xor, nand, nor) use `convert_input()` to coerce Integer/Decimal inputs to Bool before operating, matching the existing conversion rules (non-zero = true, zero = false)
- Comparison ops do direct pattern matching on value pairs to handle Integer/Integer, Decimal/Decimal, and mixed Integer/Decimal comparisons
- Equal/NotEqual also support Bool/Bool and String/String comparisons
- Select passes through values of any type — it clones whichever input the condition selects

## Phase 2 — Conditional execution (future)

Would require changes to `Graph::run()` in `graph.rs` to support skipping downstream nodes based on a Gate/condition node.

### Possible approach

The current execution model in `Graph::run()` processes all nodes in topological order unconditionally. To add conditional execution:

1. **Gate node** — Takes a Bool condition input. When false, marks all downstream nodes as "gated" so the graph runner skips them.
2. **Implementation** — During topological traversal in `Graph::run()`, before executing each node, check if any upstream Gate node has a false condition. If so, skip execution and propagate a "gated" state downstream.
3. **Scope** — ~50-100 lines of changes in `graph.rs`, plus the Gate operation itself.
4. **Consideration** — Need to decide what happens to gated node outputs (hold last value? reset to default?). Holding last value is probably more useful for interactive workflows.

### Other potential Phase 2 additions

- **Switch** — Like select but with an Integer index and N inputs instead of a Bool condition
- **Counter/Accumulator** — Stateful node that increments on each evaluation (would require node-level persistent state)
- **String comparison** — contains, starts_with, ends_with, regex_match
