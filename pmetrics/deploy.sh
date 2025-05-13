#!/bin/bash -xe

helm upgrade pmetrics .  --install  -f values.yaml  --namespace pmetrics
