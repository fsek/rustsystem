#!/bin/bash
set -xe
# wasm-pack build --target web -d frontend/src/pkg
cd frontend
pnpm run build
