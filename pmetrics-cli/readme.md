brief howto

- setup your pg database

- set your pg environment variables

- run:

`./testout.sh | ./pmetrics-measure `


note that this is a one-off command incurring cost of connection setup/teardown.

this is ~0.015s on a reasonably powerful laptop in 2019.