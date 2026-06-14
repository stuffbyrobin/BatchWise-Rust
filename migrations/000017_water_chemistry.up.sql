CREATE TABLE water_profiles (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       UUID        NOT NULL REFERENCES tenants(id),
    name            TEXT        NOT NULL,
    description     TEXT,
    calcium_ppm     NUMERIC     NOT NULL DEFAULT 0,
    magnesium_ppm   NUMERIC     NOT NULL DEFAULT 0,
    sodium_ppm      NUMERIC     NOT NULL DEFAULT 0,
    sulfate_ppm     NUMERIC     NOT NULL DEFAULT 0,
    chloride_ppm    NUMERIC     NOT NULL DEFAULT 0,
    bicarbonate_ppm NUMERIC     NOT NULL DEFAULT 0,
    notes           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT water_profiles_tenant_name_unique UNIQUE (tenant_id, name)
);

CREATE INDEX idx_water_profiles_tenant ON water_profiles(tenant_id);

CREATE TABLE water_adjustments (
    id                         UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id                  UUID        NOT NULL REFERENCES tenants(id),
    name                       TEXT        NOT NULL,
    source_profile_id          UUID        REFERENCES water_profiles(id),
    target_profile_id          UUID        REFERENCES water_profiles(id),
    batch_id                   UUID        REFERENCES batches(id),
    recipe_id                  UUID        REFERENCES recipes(id),
    volume_liters              NUMERIC     NOT NULL,
    mineral_additions          JSONB       NOT NULL DEFAULT '[]',
    acid_additions             JSONB       NOT NULL DEFAULT '[]',
    grain_additions            JSONB       NOT NULL DEFAULT '[]',
    result_calcium_ppm         NUMERIC,
    result_magnesium_ppm       NUMERIC,
    result_sodium_ppm          NUMERIC,
    result_sulfate_ppm         NUMERIC,
    result_chloride_ppm        NUMERIC,
    result_bicarbonate_ppm     NUMERIC,
    result_alkalinity          NUMERIC,
    result_residual_alk        NUMERIC,
    result_sulfate_to_chloride NUMERIC,
    result_mash_ph             NUMERIC,
    notes                      TEXT,
    created_at                 TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                 TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_water_adjustments_tenant ON water_adjustments(tenant_id);
CREATE INDEX idx_water_adjustments_batch  ON water_adjustments(batch_id)  WHERE batch_id  IS NOT NULL;
CREATE INDEX idx_water_adjustments_recipe ON water_adjustments(recipe_id) WHERE recipe_id IS NOT NULL;
