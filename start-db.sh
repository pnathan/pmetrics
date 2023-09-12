export PGPASSWORD=aargh
export PGHOST=localhost
export PGUSER=postgres
export PGDATABASE=postgres
export PGPORT=5432
psql -1 -f schema/monitoring.log.ddl

psql -1 -c "INSERT INTO monitoring.tenant(tenantname, apikey) values ('test',  'a-wiWimWyilf')"
psql -1 -c "INSERT INTO monitoring.tenant(tenantname, apikey) values ('test',  'a-IbpyucIo')"
