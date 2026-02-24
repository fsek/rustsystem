#!/bin/bash

(cd mtls || exit 1;
  ./mkcerts.sh prod
)

docker build -t rustsystem-server -f Dockerfile.server .
docker save rustsystem-server | pv | ssh -C felix@server.fsek.studentorg.lu.se docker load

docker build -t rustsystem-trustauth -f Dockerfile.trustauth .
docker save rustsystem-trustauth | pv | ssh -C felix@server.fsek.studentorg.lu.se docker load
