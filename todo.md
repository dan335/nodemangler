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
- [ ] Cut a new release. The AUR package installs a desktop entry and icon
      that only now ride along in the Linux archive, so it cannot point at
      v1.0.11 or earlier — the first tag after this change is the first
      publishable one. An unclaimed name clones as an empty repo, so that
      first push is also what creates the package.
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

## Follow-ups from the egui-phosphor update

- [ ] `libgtk-3-dev` and `libssl-dev` in `.github/workflows/release.yml` look
      vestigial — there is no `gtk-sys` and no `openssl-sys` anywhere in
      `Cargo.lock` (rfd 0.17 drives the XDG portal over zbus, reqwest 0.13
      uses rustls). Try dropping them; needs a real Linux build to confirm.
- [ ] Eyeball the phosphor icons once in the running GUI. Low risk — 0.13 still
      registers the font under the `"phosphor"` key that `app.rs:835` unwraps,
      and it compiles and tests clean — but it was never checked on screen.
- [ ] Optional: egui 0.36.1 is out. That is a separate upgrade across
      eframe / egui / egui_glow / epaint / egui_extras, and egui-phosphor 0.13
      pins egui ^0.35, so it needs a 0.36-compatible phosphor release first.
