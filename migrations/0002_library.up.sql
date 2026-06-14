-- 0002: styles, equipment_profiles, mash_profiles, mash_steps, yeasts

CREATE TABLE styles (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id      UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name           TEXT NOT NULL,
    category       TEXT,
    og_min         NUMERIC, og_max          NUMERIC,
    fg_min         NUMERIC, fg_max          NUMERIC,
    abv_min        NUMERIC, abv_max         NUMERIC,
    ibu_min        NUMERIC, ibu_max         NUMERIC,
    color_ebc_min  NUMERIC, color_ebc_max   NUMERIC,
    description    TEXT,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, name)
);
CREATE INDEX idx_styles_tenant ON styles (tenant_id);

CREATE TABLE equipment_profiles (
    id                              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id                       UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name                            TEXT NOT NULL,
    batch_size_liters               NUMERIC,
    boil_size_liters                NUMERIC,
    boil_time_minutes               INTEGER,
    boil_off_rate_liters_per_hour   NUMERIC,
    mash_efficiency_pct             NUMERIC,
    brewhouse_efficiency_pct        NUMERIC,
    trub_loss_liters                NUMERIC,
    grain_absorption_l_per_kg       NUMERIC,
    notes                           TEXT,
    created_at                      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, name)
);
CREATE INDEX idx_equipment_tenant ON equipment_profiles (tenant_id);

CREATE TABLE mash_profiles (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id   UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    notes       TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, name)
);
CREATE INDEX idx_mash_profiles_tenant ON mash_profiles (tenant_id);

CREATE TABLE mash_steps (
    id                       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    mash_profile_id          UUID NOT NULL REFERENCES mash_profiles(id) ON DELETE CASCADE,
    step_order               INTEGER NOT NULL,
    step_type                TEXT NOT NULL CHECK (step_type IN ('infusion', 'temperature', 'decoction')),
    target_temp_c            NUMERIC NOT NULL,
    hold_minutes             INTEGER NOT NULL,
    infusion_volume_liters   NUMERIC,
    UNIQUE (mash_profile_id, step_order)
);

CREATE TABLE yeasts (
    id                     UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id              UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name                   TEXT NOT NULL,
    manufacturer           TEXT,
    product_code           TEXT,
    type                   TEXT CHECK (type IN ('ale', 'lager', 'wild', 'bacteria', 'other')),
    form                   TEXT CHECK (form IN ('dry', 'liquid', 'slant')),
    attenuation_min_pct    NUMERIC,
    attenuation_max_pct    NUMERIC,
    temp_min_c             NUMERIC,
    temp_max_c             NUMERIC,
    flocculation           TEXT CHECK (flocculation IN ('low', 'medium', 'high')),
    notes                  TEXT,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT now()
);
-- COALESCE expression in UNIQUE requires a separate index (not supported in inline UNIQUE constraint)
CREATE UNIQUE INDEX idx_yeasts_unique_name_product ON yeasts (tenant_id, name, COALESCE(product_code, ''));
CREATE INDEX idx_yeasts_tenant ON yeasts (tenant_id);
