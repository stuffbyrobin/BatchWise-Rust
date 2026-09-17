-- Invitations let an Owner or Manager add a member to their tenant. Only a hash
-- of the one-time token is stored; the link is shown once to the inviter.
CREATE TABLE IF NOT EXISTS invitations (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id        UUID        NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    email            CITEXT      NOT NULL,
    role             TEXT        NOT NULL
                                 CHECK (role IN ('owner', 'manager', 'brewer', 'sales', 'viewer')),
    token_hash       TEXT        NOT NULL UNIQUE,
    invited_by       UUID        REFERENCES users(id),
    expires_at       TIMESTAMPTZ NOT NULL,
    accepted_at      TIMESTAMPTZ,
    accepted_user_id UUID        REFERENCES users(id),
    revoked_at       TIMESTAMPTZ,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- One open invitation per email per tenant (an expired one is revoked before
-- a replacement is created).
CREATE UNIQUE INDEX IF NOT EXISTS invitations_one_open_per_email
    ON invitations (tenant_id, email)
    WHERE accepted_at IS NULL AND revoked_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_invitations_tenant_created
    ON invitations (tenant_id, created_at DESC);
