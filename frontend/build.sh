#!/bin/bash
set -xe

# Set default API_ENDPOINT for local development if not already set
if [ -z "$API_ENDPOINT" ]; then
    export API_ENDPOINT="http://localhost:3000"
fi

echo "Building with API_ENDPOINT=$API_ENDPOINT"

cd rustsystem-client
# wasm-pack build --target web -d ../src/pkg
cd ..
pnpm run build
