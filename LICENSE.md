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

No dependency restricts permissive licensing of the project, so it can be
embedded in other projects — including proprietary ones. (The only non-permissive
bits in the dependency tree are file-level: `option-ext` is MPL-2.0, and the
bundled fonts are under font licenses — see below.)

## Fonts

The bundled [Manrope](https://github.com/sharanda/manrope) font
(`app/crates/mangler_gui/assets/` and `app/crates/mangler_core/assets/`) is
copyright 2019 The Manrope Project Authors and licensed under the
[SIL Open Font License 1.1](app/crates/mangler_gui/assets/OFL.txt).

## Contributing

Unless you state otherwise, a contribution intentionally submitted for
inclusion in the project is offered under **MIT OR Apache-2.0**, with no
additional terms or conditions.
