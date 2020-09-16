#!/bin/bash -xe

curl -H "X-PMETRICS-API-KEY: dev" localhost:1337/api/v1/event
curl -XPOST -H "X-PMETRICS-API-KEY: dev"  localhost:1337/api/v1/event -d  '{"name":"dylos", "dict" : {}}'
curl -H "X-PMETRICS-API-KEY: dev" localhost:1337/api/v1/event

curl -H "X-PMETRICS-API-KEY: dev" localhost:1337/api/v1/measure
curl -XPOST -H "X-PMETRICS-API-KEY: dev"  localhost:1337/api/v1/measure -d  '{"name":"dylos", "measurement": 4761.0, "dict" : {}}'
curl -H "X-PMETRICS-API-KEY: dev" localhost:1337/api/v1/measure


curl -H "X-PMETRICS-API-KEY: h4x" localhost:1337/api/v1/event
curl -XPOST -H "X-PMETRICS-API-KEY: h4x"  localhost:1337/api/v1/event -d  '{"name":"bacon", "dict" : {"they said this was just a test": true}}'
curl -H "X-PMETRICS-API-KEY: h4x" localhost:1337/api/v1/event

curl -H "X-PMETRICS-API-KEY: h4x" localhost:1337/api/v1/measure
curl -XPOST -H "X-PMETRICS-API-KEY: h4x"  localhost:1337/api/v1/measure -d  '{"name":"baker", "measurement": -2.3, "dict" : {"so much data" : 1}}'
curl -H "X-PMETRICS-API-KEY: h4x" localhost:1337/api/v1/measure
