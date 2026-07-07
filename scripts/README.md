# Scripts

Each script has a `.sh` (macOS/Linux) and `.bat` (Windows) version.

| Script | What it does |
|--------|--------------|
| `test` | Run the full test suite (`cargo test --workspace`) |
| `build` | Build release executables for **this machine's OS** into `app/target/release/` |
| `release` | Cut a release: run tests, bump the version, commit, tag `vX.Y.Z`, and push |

## Releasing

```bash
./scripts/release.sh 1.1.0      # macOS/Linux
scripts\release.bat 1.1.0       # Windows
```

Pushing the `v1.1.0` tag triggers `.github/workflows/release.yml`, which builds
executables natively on Windows, Linux, and macOS runners (GUI apps can't be
reliably cross-compiled from one machine) and publishes them to GitHub Releases
as:

- `nodemangler-v1.1.0-windows-x86_64.zip`
- `nodemangler-v1.1.0-linux-x86_64.tar.gz`
- `nodemangler-v1.1.0-macos-aarch64.tar.gz` (Apple Silicon)
- `nodemangler-v1.1.0-macos-x86_64.tar.gz` (Intel)

The workflow refuses to run if the tag doesn't match the Cargo.toml version.

## itch.io

The same tag push also uploads the four builds to itch.io (via
[butler](https://itch.io/docs/butler/)) as channels `windows`, `linux`,
`mac-arm64`, and `mac-x86_64`, with the version number attached. The job is
skipped until these are configured in the GitHub repo
(**Settings → Secrets and variables → Actions**):

1. Create the project page on itch.io first (a draft is fine) — butler can't
   push to a game that doesn't exist.
2. Repository **variable** `ITCH_TARGET`: the itch.io target as `user/game`,
   e.g. `danphi/nodemangler` (the game part is the page's URL slug).
3. Repository **secret** `BUTLER_API_KEY`: an API key generated at
   <https://itch.io/user/settings/api-keys>.

## Version number

The single source of truth is `[workspace.package] version` in `app/Cargo.toml`;
all three crates inherit it via `version.workspace = true`. The release scripts
update it for you, but you can also edit that one line by hand. In code it's
available as `env!("CARGO_PKG_VERSION")`.
