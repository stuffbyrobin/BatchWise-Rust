-- users.role (000032) replaces is_owner, and nothing reads is_owner any more.
ALTER TABLE users DROP COLUMN IF EXISTS is_owner;
