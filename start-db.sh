export PGPASSWORD=aargh
export PGHOST=localhost
export PGUSER=postgres
export PGDATABASE=postgres
export PGPORT=5432
psql -1 -f schema/monitoring.log.ddl
psql -1 -f schema/002_multi_api_keys.sql

psql -1 -c "INSERT INTO monitoring.tenant(tenantname) values ('test')"
psql -1 -c "INSERT INTO monitoring.api_key(tenant_id, key, label, permissions) SELECT uid, 'a-wiWimWyilf', 'test-key-1', '{tenant_read,tenant_write,make_api_key,disable_api_key}' FROM monitoring.tenant WHERE tenantname='test' LIMIT 1"
psql -1 -c "INSERT INTO monitoring.api_key(tenant_id, key, label, permissions) SELECT uid, 'a-IbpyucIo', 'test-key-2', '{tenant_read,tenant_write}' FROM monitoring.tenant WHERE tenantname='test' LIMIT 1"
