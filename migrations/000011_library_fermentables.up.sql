CREATE TABLE library_fermentables (
    id                    UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id             UUID        NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name                  TEXT        NOT NULL,
    supplier              TEXT,
    type                  TEXT,
    colour_ebc_min        NUMERIC(10,2),
    colour_ebc_max        NUMERIC(10,2),
    extract_litres_per_kg NUMERIC(7,2),
    moisture_pct_max      NUMERIC(5,2),
    tn_min                NUMERIC(6,3),
    tn_max                NUMERIC(6,3),
    snr_min               NUMERIC(7,2),
    snr_max               NUMERIC(7,2),
    attributes            TEXT,
    notes                 TEXT,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_library_fermentables_tenant ON library_fermentables(tenant_id);
CREATE INDEX idx_library_fermentables_name   ON library_fermentables(lower(name));
