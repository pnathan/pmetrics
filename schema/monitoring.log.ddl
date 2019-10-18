CREATE SCHEMA monitoring;

-- a measure is, e.g., a sample similar to an existing metrics system,
-- e.g., prometheus.
CREATE TABLE  monitoring.measure(
id serial,
insertion_time timestamp with time zone not null default now(),
name text not null,
measurement float not null,
dict jsonb not null);


CREATE TABLE  monitoring.event(
id serial,
insertion_time timestamp with time zone not null default now(),
name text not null,
dict jsonb not null);
