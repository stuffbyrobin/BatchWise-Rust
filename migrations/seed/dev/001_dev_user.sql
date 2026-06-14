-- DEV-ONLY seed: a stable local login. NOT applied by `make seed`.
-- Applied only via `make seed-dev`. Do not run against any shared/production database.
--
--   email:    dev@batchwise.test
--   password: DevBrewer123!
--
-- The password_hash below is argon2id of the password above, using the app's
-- parameters (m=65536,t=3,p=4). Idempotent via fixed UUIDs + ON CONFLICT.

INSERT INTO tenants (id, tenant_name, tier, country, feature_flags)
VALUES (
    '11111111-1111-1111-1111-111111111111',
    'Dev Brewery',
    'pro',
    'GB',
    '{
        "inventory": true, "recipes": true, "batches": true, "calendar": true,
        "yeastkinetics": true, "library": true, "water": true, "yeast_banking": true,
        "fermentation": true, "tracking": true, "reporting": true, "sales": true,
        "duty": true, "allergens": true, "labels": true, "packaging": true,
        "traceability": true, "equipment_maintenance": true
    }'::jsonb
)
ON CONFLICT (id) DO NOTHING;

INSERT INTO users (id, tenant_id, email, password_hash, display_name, is_owner, is_active)
VALUES (
    '22222222-2222-2222-2222-222222222222',
    '11111111-1111-1111-1111-111111111111',
    'dev@batchwise.test',
    '$argon2id$v=19$m=65536,t=3,p=4$KXSJg0F3Ex872Nt3h5elfA$X/1NtlS33cuqkLcJBu2LukA+c44NIEUFtaqiAjQNfHI',
    'Dev Brewer',
    true,
    true
)
ON CONFLICT (email) DO NOTHING;
