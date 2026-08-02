# Licensing

Every crate in NodeMangler is licensed **MIT OR Apache-2.0** (at your option):

| Crate | License |
|-------|---------|
| `app/crates/mangler_core` | **MIT OR Apache-2.0** (at your option) |
| `app/crates/mangler_gui` | **MIT OR Apache-2.0** (at your option) |
| `app/crates/mangler_cli` | **MIT OR Apache-2.0** (at your option) |

License texts:

- Core — [LICENSE-MIT](app/crates/mangler_core/LICENSE-MIT) and [LICENSE-APACHE](app/crates/mangler_core/LICENSE-APACHE)
- GUI — [LICENSE-MIT](app/crates/mangler_gui/LICENSE-MIT) and [LICENSE-APACHE](app/crates/mangler_gui/LICENSE-APACHE)
- CLI — [LICENSE-MIT](app/crates/mangler_cli/LICENSE-MIT) and [LICENSE-APACHE](app/crates/mangler_cli/LICENSE-APACHE)

## Camera RAW support and the LGPL

Camera RAW decoding is provided by [rawler](https://github.com/dnglab/dnglab),
which is **LGPL-2.1**. It is enabled by the `raw` cargo feature on
`mangler_core`, which is **on by default**.

This does not change NodeMangler's licence. The LGPL — unlike the GPL — does not
reach across the linking boundary, so every crate above remains MIT OR
Apache-2.0 and every line of NodeMangler's own source is still offered under
those terms.

What it does add is a condition on *distributed binaries* that contain rawler:
recipients must be able to swap in their own modified copy of the library and
relink. Rust links statically, so NodeMangler satisfies that the way LGPL-2.1
§6(a) permits — by publishing the complete source of the program, which lets
anyone point cargo at a patched rawler and rebuild. The full LGPL-2.1 text is in
[licenses/LGPL-2.1.txt](licenses/LGPL-2.1.txt).

**Embedding NodeMangler in a proprietary project:** build with
`default-features = false` to get a dependency tree with no LGPL code in it:

```toml
mangler_core = { version = "…", default-features = false }
```

RAW files are then simply not offered — the same way AVIF can be written but not
read, because decoding it needs a C library.

Apart from that opt-out-able feature, no dependency restricts permissive
licensing of the project. (The remaining non-permissive bits are file-level:
`option-ext` is MPL-2.0, and the bundled fonts are under font licenses — see
below.)

## Fonts

The bundled [Manrope](https://github.com/sharanda/manrope) font
(`app/crates/mangler_gui/assets/` and `app/crates/mangler_core/assets/`) is
copyright 2019 The Manrope Project Authors and licensed under the
[SIL Open Font License 1.1](app/crates/mangler_gui/assets/OFL.txt).

## Contributing

Unless you state otherwise, a contribution intentionally submitted for
inclusion in the project is offered under **MIT OR Apache-2.0**, with no
additional terms or conditions.
