DROP TABLE IF EXISTS refresh_tokens;
DROP INDEX IF EXISTS idx_users_tenant;
DROP TABLE IF EXISTS users;
DROP TABLE IF EXISTS tenants;
-- citext extension intentionally not dropped (other DBs in same cluster may use it)
