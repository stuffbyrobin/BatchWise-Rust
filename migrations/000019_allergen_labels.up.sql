-- 000019: allergen declaration & UK label compliance

-- Add address field to tenants (needed for label responsible_party auto-population).
ALTER TABLE tenants ADD COLUMN IF NOT EXISTS address TEXT NOT NULL DEFAULT '';

CREATE TABLE label_records (
    id                        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id                 UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    batch_id                  UUID NOT NULL REFERENCES batches(id) ON DELETE CASCADE,
    status                    TEXT NOT NULL DEFAULT 'draft'
                              CHECK (status IN ('draft','approved')),

    -- Mandatory fields (auto-populated on creation; can be overridden before approval)
    product_name              TEXT NOT NULL,
    abv_percent               NUMERIC(5,2) NOT NULL CHECK (abv_percent >= 0),
    allergens                 TEXT[] NOT NULL DEFAULT '{}',
    net_volume_ml             INTEGER NOT NULL CHECK (net_volume_ml > 0),
    responsible_party         TEXT NOT NULL,
    country_of_origin         CHAR(2) NOT NULL DEFAULT 'GB',
    best_before_date          DATE,
    lot_identifier            TEXT NOT NULL,

    -- Voluntary fields
    ingredient_list           TEXT,
    energy_kj_per_100ml       NUMERIC(6,2),
    energy_kcal_per_100ml     NUMERIC(6,2),
    alcohol_units_per_serving NUMERIC(5,2),
    serving_volume_ml         INTEGER,

    created_at                TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE (tenant_id, batch_id)
);

CREATE INDEX label_records_tenant_idx ON label_records (tenant_id);
