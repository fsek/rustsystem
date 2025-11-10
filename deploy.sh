#!/bin/bash

docker build -t rustsystem .
docker save rustsystem | pv | ssh -C ake@server.fsek.studentorg.lu.se docker load
