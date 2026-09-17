-- Roles replace the single is_owner flag (docs/remediation-plan.md, Phase 15).
-- Existing owners become owners and everyone else a manager, so no one loses
-- access. is_owner stays until nothing reads it.
ALTER TABLE users ADD COLUMN IF NOT EXISTS role TEXT NOT NULL DEFAULT 'manager';
UPDATE users SET role = 'owner' WHERE is_owner AND role <> 'owner';

ALTER TABLE users
    DROP CONSTRAINT IF EXISTS users_role_check,
    ADD CONSTRAINT users_role_check
        CHECK (role IN ('owner', 'manager', 'brewer', 'sales', 'viewer'));
