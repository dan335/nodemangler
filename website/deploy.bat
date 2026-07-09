REM Image is hosted on GitHub Container Registry (ghcr.io) under the repo owner.
REM Requires a one-time `docker login ghcr.io` (see website/README or setup notes).
docker build -t ghcr.io/dan335/nodemangler-site --platform linux/x86_64 .
docker push ghcr.io/dan335/nodemangler-site
ssh dan@104.236.39.83 "cd ~/server; docker compose pull; docker compose up -d;"
