#!/bin/bash
set -euo pipefail

cleanup() {
    rm -rf mtls-prod
}
trap cleanup EXIT

mkdir -p mtls-prod
(cd mtls-prod && bash ../mtls/mkcerts.sh prod)

docker build -t rustsystem-server -f Dockerfile.server .
docker save rustsystem-server | pv | ssh -C felix@server.fsek.studentorg.lu.se docker load

docker build -t rustsystem-trustauth -f Dockerfile.trustauth .
docker save rustsystem-trustauth | pv | ssh -C felix@server.fsek.studentorg.lu.se docker load
