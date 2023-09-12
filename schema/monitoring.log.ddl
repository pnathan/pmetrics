CREATE SCHEMA monitoring;

CREATE TABLE monitoring.tenant(
uid serial primary key,
tenantname text not null,
apikey text not null);

create unique index only_one_no_dupes on monitoring.tenant (tenantname, apikey);

-- a measure is, e.g., a sample similar to an existing metrics system,
-- e.g., prometheus.
CREATE TABLE  monitoring.measure(
id serial,
tenant_id int not null references monitoring.tenant(uid),
insertion_time timestamp with time zone not null default now(),
name text not null,
measurement float not null,
dict jsonb not null);


CREATE TABLE  monitoring.event(
id serial,
tenant_id int not null references monitoring.tenant(uid),
insertion_time timestamp with time zone not null default now(),
name text not null,
dict jsonb not null);
