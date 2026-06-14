-- 0004: recipes with nested fermentables, hops, yeasts, mash steps

CREATE TABLE recipes (
    id                     UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id              UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name                   TEXT NOT NULL,
    type                   TEXT NOT NULL CHECK (type IN ('all_grain', 'extract', 'partial_mash', 'cider', 'mead', 'other')),
    style_id               UUID NULL REFERENCES styles(id) ON DELETE SET NULL,
    equipment_profile_id   UUID NULL REFERENCES equipment_profiles(id) ON DELETE SET NULL,
    mash_profile_id        UUID NULL REFERENCES mash_profiles(id) ON DELETE SET NULL,
    batch_size_liters      NUMERIC NOT NULL,
    boil_size_liters       NUMERIC,
    boil_time_minutes      INTEGER,
    efficiency_pct         NUMERIC,
    calc_og                NUMERIC,
    calc_fg                NUMERIC,
    calc_abv_pct           NUMERIC,
    calc_ibu               NUMERIC,
    calc_color_ebc         NUMERIC,
    tasting_aroma          TEXT,
    tasting_flavour        TEXT,
    tasting_mouthfeel      TEXT,
    tasting_finish         TEXT,
    notes                  TEXT,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, name)
);
CREATE INDEX idx_recipes_tenant ON recipes (tenant_id);

CREATE TABLE recipe_fermentables (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recipe_id     UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    step_order    INTEGER NOT NULL,
    name          TEXT NOT NULL,
    amount        NUMERIC NOT NULL,
    unit          TEXT NOT NULL CHECK (unit IN ('kg', 'g')),
    color_ebc     NUMERIC,
    potential_ppg NUMERIC,
    type          TEXT,
    addition      TEXT,
    UNIQUE (recipe_id, step_order)
);

CREATE TABLE recipe_hops (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recipe_id         UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    step_order        INTEGER NOT NULL,
    name              TEXT NOT NULL,
    amount            NUMERIC NOT NULL,
    unit              TEXT NOT NULL CHECK (unit IN ('g', 'kg')),
    alpha_acid_pct    NUMERIC NOT NULL,
    boil_time_minutes NUMERIC NOT NULL,
    form              TEXT CHECK (form IN ('pellet', 'leaf', 'extract')),
    use               TEXT CHECK (use IN ('boil', 'whirlpool', 'dry-hop', 'first-wort', 'mash')),
    UNIQUE (recipe_id, step_order)
);

CREATE TABLE recipe_yeasts (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recipe_id       UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    yeast_id        UUID NULL REFERENCES yeasts(id) ON DELETE SET NULL,
    name            TEXT NOT NULL,
    amount          NUMERIC NOT NULL,
    unit            TEXT NOT NULL CHECK (unit IN ('g', 'mL', 'count')),
    attenuation_pct NUMERIC
);

CREATE TABLE recipe_mash_steps (
    id                     UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recipe_id              UUID NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    step_order             INTEGER NOT NULL,
    step_type              TEXT NOT NULL CHECK (step_type IN ('infusion', 'temperature', 'decoction')),
    target_temp_c          NUMERIC NOT NULL,
    hold_minutes           INTEGER NOT NULL,
    infusion_volume_liters NUMERIC,
    UNIQUE (recipe_id, step_order)
);
