# NodeMangler website

The project site at [nodemangler.com](https://nodemangler.com): a standalone
Rust crate (`mangler_site`, intentionally **not** part of the `app/` Cargo
workspace) that serves the static files in `static/` with axum + tower-http.
The server adds gzip/brotli compression, per-type `Cache-Control`, a couple of
security headers, and a styled 404 page; everything else is plain files.

## Contents

- `static/index.html` — the whole site (one page), plus `404.html`
- `static/robots.txt`, `static/sitemap.xml`, `static/llms.txt` — crawl files
  for search engines and LLM agents; `llms.txt` links to the repo README as
  the always-current node reference instead of duplicating it
- `static/style.css`, `static/favicon.svg`, `static/screenshot.jpg`
- `src/main.rs` — the server (port from `PORT`, default 8080)

## Run locally

```bash
cd website
cargo run
# → http://localhost:8080
```

`ServeDir` resolves `static/` relative to the working directory, so run from
`website/` (the Dockerfile sets the equivalent workdir in the container).

## Deploy

`deploy.sh` (or `deploy.bat`) builds the Docker image for linux/x86_64, pushes
it to GHCR (`ghcr.io/dan335/nodemangler-site`), then restarts the compose
stack on the host over SSH. TLS and routing are handled by Traefik on the
host; this container just speaks plain HTTP on 8080.

## Keeping it fresh

The page deliberately quotes **no version numbers or node counts** — those
live in the repo README, which `llms.txt` links to. When facts change, the
files to check are `static/llms.txt` (counts and version stamp),
`static/sitemap.xml` (`lastmod`), and the JSON-LD `softwareVersion` in
`static/index.html`.
