CREATE TABLE monitoring.api_key (
    id          serial primary key,
    tenant_id   int not null references monitoring.tenant(uid) on delete cascade,
    key         text not null unique,
    label       text not null default '',
    permissions text[] not null default '{tenant_read,tenant_write}',
    created_at  timestamptz not null default now(),
    revoked_at  timestamptz,
    constraint revoked_after_created check (revoked_at is null or revoked_at >= created_at)
);

CREATE INDEX idx_api_key_active ON monitoring.api_key(key) WHERE revoked_at IS NULL;

-- Migrate existing single keys, granting full permissions
INSERT INTO monitoring.api_key (tenant_id, key, label, permissions)
SELECT uid, apikey, tenantname, '{tenant_read,tenant_write,make_api_key,disable_api_key}'
FROM monitoring.tenant;

ALTER TABLE monitoring.tenant DROP COLUMN apikey;
DROP INDEX IF EXISTS only_one_no_dupes;
