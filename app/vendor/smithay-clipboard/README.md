[![crates.io](https://img.shields.io/crates/v/smithay-clipboard.svg)](https://crates.io/crates/smithay-clipboard)


# Smithay Clipboard

This crate provides access to the Wayland clipboard for applications
already using some sort of GUI toolkit or a windowing library, like
[winit](https://github.com/rust-windowing/winit), since you should
have some surface around to receive keyboard/pointer events.

If you want to access clipboard from the CLI or to write clipboard manager,
this is not what you're looking for.

## Documentation

The documentation for the master branch is [available online](https://smithay.github.io/smithay-clipboard/).

The documentation for the releases can be found on [docs.rs](https://docs.rs/smithay-clipboard).

## Contact Us

If you have questions or want to discuss the project with us, join our chatroom on matrix:
[#sctk:matrix.org](https://matrix.to/#/#sctk:matrix.org).

## NodeMangler patch

This is a vendored copy of smithay-clipboard 0.7.3, wired in through
`[patch.crates-io]` in `app/Cargo.toml`. It adds one thing: accepting
`text/uri-list` drag-and-drop on the clipboard's existing `wl_data_device`
and handing the payload to a callback installed with `set_drop_handler`
(`src/dnd.rs`, plus the `enter`/`drop_performed` handlers in `src/state.rs`).

It lives here because winit 0.30 has no Wayland drag-and-drop, and a second
data device of our own would not help: Hyprland routes drag events (and the
clipboard selection) only to a client's *first* `wl_data_device`, which is
this crate's, created by egui-winit before the app gets control. When egui
moves to a winit with Wayland DnD, delete this directory and the patch entry.
