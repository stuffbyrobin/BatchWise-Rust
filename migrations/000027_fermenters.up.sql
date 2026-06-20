-- 000027: fermentation vessels (fermenters) + batch assignment

CREATE TABLE fermenters (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    -- DOUBLE PRECISION (not NUMERIC) so it decodes straight into f64.
    capacity_liters DOUBLE PRECISION,
    notes           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, name)
);

CREATE INDEX idx_fermenters_tenant ON fermenters (tenant_id);

-- A batch is assigned to (occupies) one fermenter while it ferments. Nullable so
-- planned/unassigned batches are allowed; SET NULL if the fermenter is deleted.
ALTER TABLE batches
    ADD COLUMN fermenter_id UUID NULL REFERENCES fermenters(id) ON DELETE SET NULL;

CREATE INDEX idx_batches_fermenter ON batches (fermenter_id);
