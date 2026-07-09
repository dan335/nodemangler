# NodeMangler website — multi-stage build so the shipped image only carries
# the compiled binary + static assets, not the whole Rust toolchain.

# ---- Stage 1: build ---------------------------------------------------
FROM rust:1-slim AS builder
WORKDIR /build

# Copy the whole package (Cargo.toml/.lock + src/ + static/) and build the
# release binary. This is a small, standalone crate (not part of the app/
# workspace) so there's no benefit to a separate dependency-caching layer.
COPY . .
RUN cargo build --release

# ---- Stage 2: runtime ---------------------------------------------------
# debian:bookworm-slim (not distroless) because it keeps a shell/package
# manager available for quick `docker exec` debugging on the host; the image
# is still small and this server has no other reason to need a full distro.
FROM debian:bookworm-slim
WORKDIR /app

# Link this image to the GitHub repo so it shows up as a package under
# github.com/dan335/nodemangler (and inherits the repo's package settings).
LABEL org.opencontainers.image.source="https://github.com/dan335/nodemangler"

# Run as a dedicated non-root user rather than the container default root.
RUN useradd --system --no-create-home --shell /usr/sbin/nologin site
COPY --from=builder /build/target/release/mangler_site ./mangler_site
COPY --from=builder /build/static ./static
USER site

# Traefik runs on the host and routes to this container via labels declared
# in the server's own docker-compose file (outside this repo) — there is
# intentionally nothing Traefik-specific in here; don't go looking for it.
EXPOSE 8080
CMD ["./mangler_site"]
