//! NodeMangler project website — a tiny static-file HTTP server.
//!
//! Why Rust for a plain static site? Two reasons: (1) consistency — the whole
//! NodeMangler project is Rust, so the deploy story (build a binary, run it in
//! a slim container) matches `mangler_cli`/`mangler_gui`; (2) a future web
//! version of NodeMangler (a WASM node editor running in the browser) will
//! need a real Rust backend anyway, so this server is the seed of that —
//! today it just hands out files from `static/`, but it's already the right
//! shape to grow an API or a WASM asset route later.
//!
//! Deliberately minimal: no templating engine, no WASM, no framework beyond
//! axum + tower-http's `ServeDir`. Everything under `static/` is served
//! as-is; `ServeDir` serves `index.html` automatically for directory-style
//! requests (including `/`), so no extra routing is needed.

use std::net::SocketAddr;

use axum::Router;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    // `ServeDir` defaults to serving `index.html` when a request resolves to
    // a directory (e.g. `/` -> `static/index.html`), which is exactly the
    // single-page behavior this site needs.
    let app = Router::new().fallback_service(ServeDir::new("static"));

    // Port is configurable via `PORT` (matches typical container/PaaS
    // conventions) and defaults to 8080 for local runs.
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    // Bind on all interfaces so this works both bare-metal and in a container.
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind listener");

    println!("nodemangler site listening on http://{addr}");
    axum::serve(listener, app).await.expect("server error");
}
