#!/bin/bash -xe
cargo build --release
SHA=`git rev-parse HEAD`
docker build --no-cache -t "pmetrics:latest" .
docker tag pmetrics:latest "pmetrics:${SHA}"
docker tag pmetrics:latest "gcr.io/sapient-fabric-207305/pmetrics:${SHA}"
docker tag pmetrics:latest "gcr.io/sapient-fabric-207305/pmetrics:latest"
docker push "gcr.io/sapient-fabric-207305/pmetrics:${SHA}"
docker push "gcr.io/sapient-fabric-207305/pmetrics:latest"
