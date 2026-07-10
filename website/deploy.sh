#!/usr/bin/env bash
# Build the site's Docker image, push it to GitHub Container Registry, and
# restart the container on the host. Same steps as deploy.bat.
# Requires a one-time `docker login ghcr.io` (see website/README or setup notes).
set -euo pipefail
cd "$(dirname "$0")"

docker build -t ghcr.io/dan335/nodemangler-site --platform linux/x86_64 .
docker push ghcr.io/dan335/nodemangler-site
ssh dan@104.236.39.83 "cd ~/server; docker compose pull; docker compose up -d;"
