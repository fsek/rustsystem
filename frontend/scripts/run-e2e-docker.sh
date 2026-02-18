#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# run-e2e-docker.sh
#
# Builds the e2e test image and runs Playwright inside an Ubuntu container.
#
# Usage:
#   pnpm test:e2e:docker                  # run all tests
#   pnpm test:e2e:docker --project=webkit # single browser
#   pnpm test:e2e:docker --ui             # Playwright UI mode (requires X11)
#
# Prerequisites:
#   - Docker running
#   - Rust backend running on localhost:3000:
#       API_ENDPOINT=http://localhost:3000 cargo run --bin rustsystem-server
#
# Network:
#   --network=host lets the container reach the host's Rust backend on :3000.
#   Vite's dev server is started inside the container by Playwright's
#   webServer option and binds to :5173 on the shared host network.
# ---------------------------------------------------------------------------

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
FRONTEND_DIR="$(dirname "$SCRIPT_DIR")"
IMAGE="rustsystem-e2e"

echo "Building e2e test image…"
docker build -f "$FRONTEND_DIR/Dockerfile.e2e" -t "$IMAGE" "$FRONTEND_DIR"

echo "Running e2e tests…"
exec docker run --rm \
  --network=host \
  "$IMAGE" \
  pnpm test:e2e "$@"
