# what is all this, then

pmetrics is paul's metrics system.

pmetrics tracks 2 essential quantities:

- named numerical measurements with kv store
- named events with kv store.

and provides the ability to query against these.

# remarks


pmetrics is not fancy. It's not glorious. It's a way to manage that
operational data for companies that aren't Internet Scale in a way
that's straightforward to manage for competent hackers.

It is not "Web Scale", and it does not "crush it".

If you're interested, you should deploy it on a testbed system and
measure it yourself against your current load and expected growth
tolerances for the next 1-3 years. Due diligence is advised. Source
reading is expected.


# design.

pmetrics is, at the core, a relatively carefully written set of
software running around a Postgres server. Postgres can reach a
significant transaction volume on a well-tuned cluster; writes can go
over 1000 TPS with care and feeding according to random reports on the
internet, which are exactly as trustworthy as they sound. The general
presumption is that your fine institution _probably_ doesn't need
extreme scalability with a sophisticated distributed system. The
author's experience suggests that this is the fundamental reality for
a typical smaller software shop today. Further, it is presumed that
the consumers of pmetrics are software engineers with a strong
preference for data driven choices and software driven systems, not
graphical interfaces.

The source should be read before you put pmetrics
into production.

The current design is synchronous HTTP calls against the database,
without connection pooling. The next phase of optimization would be to
introduce a custom connection pool with a threadsafe queue for each
replica of pmetrics.

It is presumed that all data is _valuable_ - pmetrics does not
downsample its data to spare space.

From an API perspective, the API focuses primarily on the _measure_
and _event_ calls. Note that each has an unbounded kv store. The
preferential design here is for POSTS to include _very wide_ insertion
of information - as much information as possible. The goal is to allow
enormous queryability in ad-hoc/post-hoc incident response. As has
been well documented, single-point metrics systems with limited
tagging such as Prometheus struggle having rich enough
information. `pmetrics` rejects this limit, using the power of a
Postgres JSONB format to deliver a more effective observability (o11y)
capability.

# TODO

- Connection pool.

- A frequent ask of this sort of system is *alerting*; that is,
intelligently responding to events. In order to do this in a
reasonable way, a relatively ordered event stream (i.e., ordered
correctly relative to the event reader) must be produced with a soft
real-time constraint(usually within 5 minutes or less).
    - Efficient and algorithmically effective time-based alerting
      needs to be done.

- Tenancy/sharding.
    - What if you want to deploy a single system, but present
    different views per "tier" or what-have-you?


- RBAC
  - how should user access control work? service users?


# license

pmetrics is AGPL 3, copyright Paul Nathan

# hacking

This software is in stable Rust.

## ci

pmetrics has a ci system running on gitlab.

## db notes

Connection pooling software is not mature yet in Rust.

As near as I can tell, r2d2 is an immature project, and is both wrong
in documentation and infrequently released.

the libpq binding pq-sql is probably the correct one, but no one has
adequately wrapped it except maybe diesel, and diesel doesn't comment
on the threadedness of its wrapping in the files I examined.

use pgbouncer if we need connection pools. Or write our own backend
pooler that shares out a multi-producer-single-consumer queue to each
handler.
