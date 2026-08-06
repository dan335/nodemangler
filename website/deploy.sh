#!/usr/bin/env bash
# Build the site's Docker image, push it to GitHub Container Registry, and
# restart the container on the host. deploy.bat is a thin wrapper around this.
# Requires a one-time `docker login ghcr.io` (see website/README or setup notes).
set -euo pipefail
cd "$(dirname "$0")"

docker build -t ghcr.io/dan335/nodemangler-site --platform linux/x86_64 .
docker push ghcr.io/dan335/nodemangler-site
# Single SSH session: the server rate-limits SSH connections, so ALL remote
# steps must run inside this one invocation. `set -e` makes the session abort
# if the pull fails instead of restarting with a stale image.
# Pull only this service (a bare `compose pull` drags every project on the
# host), and --force-recreate because ghcr `latest` pulls can lag and compose's
# up-to-date check otherwise keeps the old container running.
ssh dan@104.236.39.83 "set -e; cd ~/server; docker compose pull nodemangler; docker compose up -d --force-recreate nodemangler"
