Title: Package request: nodemangler (node-based image/texture editor)

Hi — requesting `nodemangler-bin` for the Omarchy repo, and happy to be told
it's out of scope.

**What it is:** NodeMangler is a node-based visual editor for image and colour
manipulation — procedural textures, PBR material export, camera-raw
development, terrain/erosion simulation. Think Substance Designer, but a small
native app. Rust + egui, native Wayland and X11, no Electron. MIT OR Apache-2.0.

- Site: https://nodemangler.com/
- Source: https://github.com/dan335/nodemangler
- AUR: https://aur.archlinux.org/packages/nodemangler-bin

**Why ask:** it's a graphics tool that suits the Omarchy audience, and it
starts fast on Hyprland with no toolkit baggage. Entirely your call whether it
clears the bar for the repo.

**Packaging:** `nodemangler-bin` repackages the `linux-x86_64` release archive,
so there's no build cost — `"source": "aur"` would just work. If you'd rather
track our releases directly, we publish a `SHA256SUMS.txt` on every GitHub
release, so the declarative upstream feed fits without a custom hook:

```json
{
  "source": "local",
  "upstream": {
    "github": "dan335/nodemangler",
    "checksums": "SHA256SUMS.txt",
    "assets": { "x86_64": "nodemangler-{tag}-linux-x86_64.tar.gz" }
  }
}
```

There's no GTK and no OpenSSL in the tree — file dialogs go through the XDG
portal over zbus and TLS is rustls — so the runtime deps are `libglvnd`,
`libxkbcommon`, `wayland`, the usual libx11/libxcursor/libxrandr/libxi set, and
`xdg-desktop-portal`. It installs the GUI as `nodemangler`, the CLI as
`mangle`, plus a desktop entry and icon. x86_64 only right now — there's no
aarch64 Linux build yet, so it'd need excluding from that arch.

Glad to make any packaging changes that make this easier.
