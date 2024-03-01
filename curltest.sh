#!/bin/bash -xe


KEY=tyfsOcGoQuetber8
HOST=${HOST:-localhost:1337}
curl -H "X-PMETRICS-API-KEY: $KEY" $HOST/api/v1/event
curl -XPOST -H "X-PMETRICS-API-KEY: $KEY"  $HOST/api/v1/event -d '{"name":"bacon", "dict" : {"they said this was just a test": true}}'
curl -H "X-PMETRICS-API-KEY: $KEY" $HOST/api/v1/event

curl -H "X-PMETRICS-API-KEY: $KEY" $HOST/api/v1/measure
curl -XPOST -H "X-PMETRICS-API-KEY: $KEY"  $HOST/api/v1/measure -d  '{"name":"dylos", "measurement": 4761.0, "dict" :  {"so much data" : 1}}'
curl -H "X-PMETRICS-API-KEY: $KEY" $HOST/api/v1/measure
