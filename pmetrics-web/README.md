# hacking

# db notes

Connection pooling software is not done yet in Rust.

As near as I can tell, r2d2 is an immature project, and is both wrong
in documentation and infrequently released.

the libpq binding pq-sql is probably the correct one, but no one has
adequately wrapped it except maybe diesel, and diesel doesn't comment
on the threadedness of its wrapping in the files I examined.

use pgbouncer if we need connection pools. Or write our own backend
pooler that shares out a multi-producer-single-consumer queue to each
handler.
