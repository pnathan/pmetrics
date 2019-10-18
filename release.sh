#!/bin/bash -xe
cargo build --release
SHA=`git rev-parse HEAD`
docker build --no-cache -t "pmetrics:latest" .
docker build -t "pmetrics:${SHA}" .
