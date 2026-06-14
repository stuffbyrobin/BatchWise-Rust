-- 000023: yeast bank entries and propagation events

CREATE TABLE yeast_bank (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id           UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name                TEXT NOT NULL,
    library_yeast_id    UUID NULL REFERENCES yeasts(id) ON DELETE SET NULL,
    generation          INT NOT NULL DEFAULT 1,
    harvested_at        TIMESTAMPTZ NULL,
    viability_percent   NUMERIC NULL CHECK (viability_percent BETWEEN 0 AND 100),
    quantity_ml         NUMERIC NULL CHECK (quantity_ml >= 0),
    storage_temp_c      NUMERIC NULL,
    location            TEXT NULL,
    status              TEXT NOT NULL DEFAULT 'active'
                        CHECK (status IN ('active', 'depleted', 'discarded')),
    notes               TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_yeast_bank_tenant ON yeast_bank (tenant_id);

CREATE TABLE yeast_propagations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    yeast_bank_id   UUID NOT NULL REFERENCES yeast_bank(id) ON DELETE CASCADE,
    batch_id        UUID NULL,
    started_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at    TIMESTAMPTZ NULL,
    volume_ml       NUMERIC NULL CHECK (volume_ml > 0),
    notes           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_yeast_propagations_bank ON yeast_propagations (yeast_bank_id);
