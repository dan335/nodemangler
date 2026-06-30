# Licensing

NodeMangler is **split-licensed by crate**:

| Crate | License |
|-------|---------|
| `app/crates/mangler_core` | **MIT OR Apache-2.0** (at your option) |
| `app/crates/mangler_gui` | **GPL-3.0-or-later** |
| `app/crates/mangler_cli` | **GPL-3.0-or-later** |

License texts:

- Core — [LICENSE-MIT](app/crates/mangler_core/LICENSE-MIT) and [LICENSE-APACHE](app/crates/mangler_core/LICENSE-APACHE)
- GUI — [app/crates/mangler_gui/LICENSE](app/crates/mangler_gui/LICENSE)
- CLI — [app/crates/mangler_cli/LICENSE](app/crates/mangler_cli/LICENSE)

## Why the split

`mangler_core` is the reusable engine (value system, node-graph executor,
image/color/noise operations). In its **default build it has no copyleft
dependencies**, so it is licensed permissively and can be embedded in other
projects — including proprietary ones.

`mangler_gui` and `mangler_cli` are the distributed applications. They enable
the `video` feature, which links FFmpeg built with the GPL encoders
`libx264`/`libx265`. A distributed binary that links those is a combined work
governed by the GPL, so the applications are licensed **GPL-3.0-or-later**.
This mirrors [Blender](https://www.blender.org) — a GPL application — and its
separately Apache-2.0-licensed Cycles engine.

## Distributing builds

A binary of `mangler_gui` / `mangler_cli` built with the `video` feature is
GPL. When you distribute one you must:

1. Convey it under the GPL.
2. Provide the complete corresponding source — your code **plus** the exact
   FFmpeg / x264 / x265 sources you built against (or a written offer valid for
   three years).
3. Include the GPL license text and the FFmpeg/codec attributions.
4. Not add restrictions on top of the GPL (no proprietary EULA).

Building **without** the `video` feature, or against an **LGPL-only** FFmpeg
(no `libx264`/`libx265`, e.g. using hardware encoders), avoids the copyleft
obligation. See
[app/crates/mangler_core/docs/video-setup.md](app/crates/mangler_core/docs/video-setup.md#licensing-read-before-distributing-builds)
for the full FFmpeg/GPL breakdown and attribution.

## Contributing

Unless you state otherwise, a contribution intentionally submitted for
inclusion in a crate is offered under **that crate's license** (MIT OR
Apache-2.0 for `mangler_core`; GPL-3.0-or-later for `mangler_gui` and
`mangler_cli`).
