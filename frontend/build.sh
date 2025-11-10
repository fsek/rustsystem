#!/bin/bash
set -xe
cd rustsystem-client
wasm-pack build --target web -d ../src/pkg
cd ..
pnpm run build
