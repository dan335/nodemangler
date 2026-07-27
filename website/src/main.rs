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
//! axum + tower-http. Everything under `static/` is served as-is; `ServeDir`
//! serves `index.html` automatically for directory-style requests (including
//! `/`), so no extra routing is needed. On top of the bare file service this
//! adds three things a public site wants:
//!
//! - gzip/brotli compression (`CompressionLayer`, negotiated per request;
//!   its default predicate already skips image content types)
//! - `Cache-Control` by file type, plus a couple of no-cost security headers
//! - a styled `404.html` instead of `ServeDir`'s empty-body 404

use std::net::SocketAddr;

use axum::{
    Router,
    extract::Request,
    http::{HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
};
use tower_http::{compression::CompressionLayer, services::ServeDir};

/// Post-processes every response from the static-file service: swaps
/// `ServeDir`'s bare 404 for the styled page and stamps caching/security
/// headers. One middleware instead of separate layers because the
/// `Cache-Control` choice needs the request path, which plain response
/// layers don't see.
async fn finalize_response(req: Request, next: Next) -> Response {
    // The path decides the Cache-Control policy below; grab it before the
    // request is consumed by the inner service.
    let path = req.uri().path().to_ascii_lowercase();
    let mut res = next.run(req).await;

    // Unknown path: replace the empty-body 404 with the styled page, keeping
    // the 404 status (important — a 200 here would be a soft-404 for SEO).
    // Read from disk rather than include_str! so the page stays editable in
    // the container without a rebuild, same as every other static file.
    if res.status() == StatusCode::NOT_FOUND {
        let body = tokio::fs::read_to_string("static/404.html")
            .await
            .unwrap_or_else(|_| "404 — page not found".to_string());
        res = (StatusCode::NOT_FOUND, Html(body)).into_response();
    }

    let success = res.status().is_success();
    let headers = res.headers_mut();

    // Cache-Control by file type. There's no content hashing on this site, so
    // nothing is safe to mark `immutable`: pages and crawl files revalidate
    // after 5 minutes, heavier assets (styles, images, icons) after a day.
    let cache = if path.ends_with(".css")
        || path.ends_with(".svg")
        || path.ends_with(".jpg")
        || path.ends_with(".png")
        || path.ends_with(".ico")
    {
        "public, max-age=86400"
    } else {
        "public, max-age=300"
    };
    if success {
        headers.insert(header::CACHE_CONTROL, HeaderValue::from_static(cache));
    }

    // Free security headers, sensible for any public page.
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    res
}

#[tokio::main]
async fn main() {
    // `ServeDir` defaults to serving `index.html` when a request resolves to
    // a directory (e.g. `/` -> `static/index.html`), which is exactly the
    // single-page behavior this site needs. Layer order: compression is added
    // last so it wraps everything and also compresses the 404 page the
    // middleware substitutes in.
    let app = Router::new()
        .fallback_service(ServeDir::new("static"))
        .layer(middleware::from_fn(finalize_response))
        .layer(CompressionLayer::new());

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
