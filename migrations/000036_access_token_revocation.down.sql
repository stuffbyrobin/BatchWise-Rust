DROP TABLE IF EXISTS revoked_access_tokens;
ALTER TABLE users DROP COLUMN IF EXISTS token_version;
