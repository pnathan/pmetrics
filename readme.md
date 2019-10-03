pmetrics
---


pmetrics is paul's metrics system.

It's not fancy. It's not glorious. It's a way to manage that
operational data for companies that aren't Internet Scale in a way
that's straightforward to manage for nerdy engineers.

It is not Web Scale, and it does not crush it.

If you're interested, you should deploy it on a testbed system and
measure it yourself against your current load and expected growth
tolerances for the next 1-3 years. Due diligence is advised.


pmetrics is, at the core, a relatively carefully designed set of
software running around a postgres core. Postgres can reach a
significant transaction volume on a well-tuned cluster; writes can go
over 1000 TPS with care and feeding according to random reports on the
internet, which are exactly as trustworthy as they sound.

This software (pmetrics) source should be read before you put pmetrics
into production.

pmetrics has three essential functions:

- Log collection. pmetrics-logger.cpp is designed to take log
statements and record them into the monitoring.log table.

- Measure collection. pmetrics-measure.cpp is designed to take a name
(text), measurement (a float), and a list of key/value pairs, and
record them into the monitoring.measure table.

- Playback. pgAdminIII is a useful gratis UI for querying, which is
the supported mechanism today.


Looking forward, pmetrics may grow three more features: HTTP POST for
the tables of interest; HTTP GET for same, and a Web UI for charting
etc.

A frequent ask of this sort of system is *alerting*; that is,
intelligently responding to events. In order to do this in a
reasonable way, a relatively ordered event stream (i.e., ordered
correctly relative to the event reader) must be produced with a soft
real-time constraint(usually within 5 minutes or less).

The most appropriate way to manage this today would be to add
post-insert triggers to the Postgres tales that would inject the
information into a RabbitMQ topic; your existing alerting system and
logic can be integrated from there. That said, pull requests would be
accepted, providing they are (1) ruthlessly readable and (2) well
engineered.



Pmetrics is AGPL 3.
