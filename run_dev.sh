#!/bin/bash
# Runs trustauth and the server for development, with the settings in .env.
#   ./run_dev.sh          # then, for the frontend with hot reload: cd frontend && pnpm dev
set -euo pipefail
cd "$(dirname "$0")"
source .env
cargo build --bin rustsystem-server --bin rustsystem-trustauth

MTLS_CERT=mtls/trustauth/trustauth.crt MTLS_KEY=mtls/trustauth/trustauth.key \
  ./target/debug/rustsystem-trustauth &
MTLS_CERT=mtls/server/server.crt MTLS_KEY=mtls/server/server.key \
  ./target/debug/rustsystem-server &

trap 'kill $(jobs -p) 2>/dev/null' EXIT
wait
