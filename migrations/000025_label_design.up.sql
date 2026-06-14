-- Uploaded brand assets (logos). Stored in-DB per ADR 0011.
CREATE TABLE brand_assets (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id    UUID        NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    filename     TEXT        NOT NULL,
    content_type TEXT        NOT NULL CHECK (content_type IN ('image/png','image/jpeg')),
    byte_size    INTEGER     NOT NULL CHECK (byte_size > 0 AND byte_size <= 2097152), -- ≤ 2 MiB
    data         BYTEA       NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_brand_assets_tenant ON brand_assets (tenant_id);

-- Per-tenant branding profiles.
CREATE TABLE brand_profiles (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       UUID        NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name            TEXT        NOT NULL,
    primary_color   CHAR(7)     NOT NULL DEFAULT '#000000' CHECK (primary_color ~ '^#[0-9a-fA-F]{6}$'),
    secondary_color CHAR(7)     NOT NULL DEFAULT '#ffffff' CHECK (secondary_color ~ '^#[0-9a-fA-F]{6}$'),
    font_family     TEXT        NOT NULL DEFAULT 'helvetica'
                        CHECK (font_family IN ('helvetica','times','courier')),
    logo_asset_id   UUID        NULL REFERENCES brand_assets(id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, name)
);
CREATE INDEX idx_brand_profiles_tenant ON brand_profiles (tenant_id);

-- A label/clip/lens design instance.
CREATE TABLE label_designs (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id        UUID        NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    kind             TEXT        NOT NULL CHECK (kind IN ('bottle','can','pump_clip','cask_lens')),
    name             TEXT        NOT NULL,
    batch_id         UUID        NULL REFERENCES batches(id)  ON DELETE CASCADE,
    recipe_id        UUID        NULL REFERENCES recipes(id)  ON DELETE CASCADE,
    brand_profile_id UUID        NULL REFERENCES brand_profiles(id) ON DELETE SET NULL,
    size_key         TEXT        NOT NULL,
    template_key     TEXT        NOT NULL,
    options          JSONB       NOT NULL DEFAULT '{}'::jsonb,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- bottle/can are batch-bound; pump_clip/cask_lens are recipe-bound.
    CHECK (
        (kind IN ('bottle','can')          AND batch_id  IS NOT NULL AND recipe_id IS NULL) OR
        (kind IN ('pump_clip','cask_lens') AND recipe_id IS NOT NULL AND batch_id  IS NULL)
    )
);
CREATE INDEX idx_label_designs_tenant ON label_designs (tenant_id, created_at DESC);
CREATE INDEX idx_label_designs_batch  ON label_designs (batch_id);
CREATE INDEX idx_label_designs_recipe ON label_designs (recipe_id);
