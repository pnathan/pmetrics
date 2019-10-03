CREATE SCHEMA monitoring;

-- a log entry is analagous to a typical logger system.
CREATE TABLE monitoring.log (
id serial,
insertion_time timestamp with time zone not null default now(),
logtext text not null);

-- a measure is, e.g., a sample similar to an existing metrics system,
-- e.g., prometheus.
CREATE TABLE  monitoring.measure(
id serial,
insertion_time timestamp with time zone not null default now(),
name text not null,
measurement float not null,
dict jsonb);


CREATE TABLE  monitoring.event(
id serial,
insertion_time timestamp with time zone not null default now(),
name text not null,
dict jsonb not null);
