CREATE TABLE yeast_kinetics (
    id                            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id                     UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    yeast_id                      UUID NOT NULL REFERENCES yeasts(id) ON DELETE CASCADE,
    fermentation_temp_c           NUMERIC NOT NULL,
    primary_fermentation_days     INTEGER NOT NULL,
    conditioning_days             INTEGER NOT NULL,
    lag_phase_hours               INTEGER,
    attenuation_pct               NUMERIC,
    notes                         TEXT,
    created_at                    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                    TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, yeast_id, fermentation_temp_c)
);
CREATE INDEX idx_yeast_kinetics_tenant_yeast ON yeast_kinetics (tenant_id, yeast_id);

CREATE TABLE calendar_events (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id               UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    batch_id                UUID NULL REFERENCES batches(id) ON DELETE CASCADE,
    event_type              TEXT NOT NULL
                              CHECK (event_type IN ('brew_day','dry_hop','fermentation_complete','transfer','package','condition_complete','custom')),
    title                   TEXT NOT NULL,
    start_time              TIMESTAMPTZ NOT NULL,
    end_time                TIMESTAMPTZ NULL,
    status                  TEXT NOT NULL DEFAULT 'pending'
                              CHECK (status IN ('pending','completed','skipped')),
    notify_minutes_before   INTEGER NULL,
    notes                   TEXT,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_calendar_tenant_time ON calendar_events (tenant_id, start_time);
CREATE INDEX idx_calendar_batch ON calendar_events (batch_id);
