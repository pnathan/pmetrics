CREATE SCHEMA monitoring;
CREATE TABLE monitoring.log (
id serial,
insertion_time timestamp with time zone not null,
logtext text not null);


CREATE TABLE  monitoring.measure(
id serial,
insertion_time timestamp with time zone not null,
name varchar(255) not null,
measurement float not null,
dict jsonb);


CREATE TABLE  monitoring.event(
id serial,
insertion_time timestamp with time zone not null,
name varchar(255) not null,
dict jsonb not null);
