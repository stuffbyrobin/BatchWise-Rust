-- Access-token revocation. Every access token carries its user's token_version
-- (`ver`) and a unique id (`jti`). Bumping token_version revokes all of a user's
-- access tokens at once (password change, deactivation, a replayed refresh
-- token); revoked_access_tokens holds single tokens revoked at logout until
-- they would have expired anyway.
ALTER TABLE users ADD COLUMN IF NOT EXISTS token_version INTEGER NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS revoked_access_tokens (
    jti        UUID PRIMARY KEY,
    user_id    UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_revoked_access_tokens_user
    ON revoked_access_tokens (user_id, expires_at);
