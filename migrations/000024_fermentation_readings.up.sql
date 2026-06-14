CREATE TABLE fermentation_readings (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id   UUID        NOT NULL REFERENCES tenants(id)  ON DELETE CASCADE,
    batch_id    UUID        NOT NULL REFERENCES batches(id)  ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    stage       TEXT        NOT NULL DEFAULT 'primary'
                    CHECK (stage IN ('primary','secondary','conditioning','lagering','other')),
    gravity     NUMERIC     NULL CHECK (gravity > 0),
    temp_c      NUMERIC     NULL,
    ph          NUMERIC     NULL CHECK (ph BETWEEN 0 AND 14),
    notes       TEXT        NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_fermentation_readings_batch   ON fermentation_readings (batch_id);
CREATE INDEX idx_fermentation_readings_tenant  ON fermentation_readings (tenant_id, recorded_at DESC);
