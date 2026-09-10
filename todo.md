# TODO

Open items from wiring up AUR / Omarchy distribution (2026-08-26).

## Publish to the AUR

Our side is done and generated: `scripts/update_manifests.sh` writes
`packaging/aur/PKGBUILD` + `.SRCINFO`, and the `aur` job in
`.github/workflows/release.yml` pushes them. What is left needs your accounts.

- [ ] Register at <https://aur.archlinux.org/> and add an SSH **public** key
      under *My Account*.
- [ ] Add the matching **private** key to the repo as the
      `AUR_SSH_PRIVATE_KEY` secret (Settings > Secrets and variables >
      Actions). Until that exists the `aur` job skips itself, so releases keep
      working in the meantime.
- [x] Cut a new release carrying the desktop entry and icon in the Linux
      archive — done in v1.0.12, so any tag from there on is publishable.
      (An unclaimed name clones as an empty repo, so the first push the `aur`
      job makes is also what creates the package.)
- [ ] Confirm it landed: <https://aur.archlinux.org/packages/nodemangler-bin>

## Verify the package on an actual Arch box

The dependency list was read off the crate graph, not off `ldd` — winit and
glutin dlopen most of it, so none of it shows up as an ELF NEEDED entry. Worth
one real check before or just after the first push:

- [ ] `cd packaging/aur && makepkg -si` — does it install and run?
- [ ] `namcap PKGBUILD` and `namcap *.pkg.tar.zst` — over- or under-declared
      dependencies?
- [ ] `makepkg --printsrcinfo | diff - .SRCINFO` — the two files are generated
      independently (CI has no Arch box to run makepkg), so this is the only
      guard that they stay in agreement. Re-run it whenever the PKGBUILD gains
      a field.

## Omarchy repo (pkgs.omarchy.org)

- [ ] Post `packaging/omarchy-pkgs-issue.md` as an issue on
      `omacom-io/omarchy-pkgs` — but only *after* the AUR package exists, since
      the draft links to it.

Keep expectations low. Their ~115 packages are Omarchy's own tools, hardware
drivers, and the apps their install menus offer, nearly all sourced from the
AUR — and a `-bin` package gives them nothing to precompile. Being in the AUR
already reaches Omarchy users through *Install > AUR* / `omarchy pkg add`.

Not possible without an Arch Package Maintainer adopting us: the official
`extra` repo. Route there is AUR first, votes and popularity second, ask third.

## Follow-ups from the egui 0.36 / file-dialog update

egui 0.36 landed in v1.0.14, along with vendoring the Phosphor glyphs (which
is what unblocked it) and replacing the native rfd dialogs with an in-egui
picker. What is left:

- [ ] `libgtk-3-dev` and `libssl-dev` in `.github/workflows/release.yml` look
      vestigial — there is no `gtk-sys` and no `openssl-sys` anywhere in
      `Cargo.lock`, and now that rfd is gone nothing wants a portal at all
      (reqwest 0.13 uses rustls). Try dropping them; needs a real Linux build
      to confirm.
- [ ] Eyeball the vendored Phosphor icons once in the running GUI. Low risk —
      `icons_tests.rs` asserts every codepoint resolves to a real glyph — but
      they were never checked on screen.
- [ ] `glow` is still pinned to 0.17 because `egui_glow` 0.36 requires ^0.17.
      Revisit when egui moves to glow 0.18.

## Follow-ups from the dependency refresh (2026-09-10)

- [ ] `rawler` 0.8 added a Fuji-rotate step to its default develop pipeline and
      `steps_for` now emits it. It is a no-op on every non-Fuji sensor, so the
      guard tests cover the ordering but not the effect — worth one look at an
      actual Fuji SuperCCD/EXR raw if a sample turns up.
