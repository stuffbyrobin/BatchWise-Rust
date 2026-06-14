-- 0001: extensions, tenants, users, refresh_tokens, system tenant seed row

CREATE EXTENSION IF NOT EXISTS citext;

CREATE TABLE tenants (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_name    TEXT NOT NULL UNIQUE,
    tier           TEXT NOT NULL DEFAULT 'home'
                    CHECK (tier IN ('home', 'pro', 'enterprise')),
    country        CHAR(2) NOT NULL DEFAULT 'GB',
    region         TEXT NULL,
    feature_flags  JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO tenants (id, tenant_name, tier, country, feature_flags)
VALUES ('00000000-0000-0000-0000-000000000000', 'system', 'enterprise', 'GB', '{}'::jsonb)
ON CONFLICT (id) DO NOTHING;

CREATE TABLE users (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id      UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    email          CITEXT NOT NULL UNIQUE,
    password_hash  TEXT NOT NULL,
    display_name   TEXT NOT NULL,
    is_owner       BOOLEAN NOT NULL DEFAULT false,
    is_active      BOOLEAN NOT NULL DEFAULT true,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_users_tenant ON users (tenant_id);

CREATE TABLE refresh_tokens (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash  TEXT NOT NULL UNIQUE,
    expires_at  TIMESTAMPTZ NOT NULL,
    used_at     TIMESTAMPTZ NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_refresh_tokens_user_expires
    ON refresh_tokens (user_id, expires_at);
