# conveyance

The conveyance system is an ad-hoc system for getting data into
pmetrics.

It is the observation of the pmetrics author that data are profoundly
more debuggable if the data is *offered* and the data system *accepts*
it via a pull mechanism, i.e., the Prometheus architecture.

Unlike Prometheus, which samples the metrics presented at a specific
point in time, to implement the above adequately, the services must
present a list of events for a pull, which are then read once and
_only_ once, presenting an engineering distributed systems problem.

We specify the following system interfaces as an initial offering:

- A `/pmetrix` route per HTTP server per instance/pod.
A GET on `/pmetrics` will return a JSON list of pmetrics datum:

        [ { "E": { "name": <name>, "dict": <dict>}},
        {"M" : {"name": <name>, "measurement":<number>, "dict":dict}}]

A GET will be presumed to flush the event queue on the API, and the
`conveyance` tool will bear responsibility at this point,

- Periodically, at `n` second intervals, the `conveyance` tool will
  sweep a list of registered URLs/routes and upload them to the pmetrics
  server. Conveyance in particular will be a durable system.

Note that this can be rewritten in several different ways. Notably,
the different services could all write to a single event bus, which
the pmetrics server could read from. Rather, here, we mostly presume a
service oriented architecture. Further, the aspect that we do not ACK
that data has been successfully written is a substantial design
risk. An ad-hoc design here might be a system wherein pmetrix returns
both the above JSON blob and a `generation id`. Internally pmetrix
"freezes" the result as a generation. A POST to pemtrics with that
generation ID would confirm that it is safe to remove. Generations
would only be created on GET.
