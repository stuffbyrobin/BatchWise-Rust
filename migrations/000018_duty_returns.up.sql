ALTER TABLE tenants
    ADD COLUMN sbr_annual_production_hl_pa NUMERIC NOT NULL DEFAULT 0;

CREATE TABLE duty_returns (
    id            UUID    PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id     UUID    NOT NULL REFERENCES tenants(id),
    period_start  DATE    NOT NULL,
    period_end    DATE    NOT NULL,
    status        TEXT    NOT NULL DEFAULT 'draft'
                          CHECK (status IN ('draft','submitted')),

    -- Aggregated from duty_events (only abv_pct < 8.5 events)
    event_count          INT     NOT NULL DEFAULT 0,
    total_volume_liters  NUMERIC NOT NULL DEFAULT 0,
    gross_duty_pence     BIGINT  NOT NULL DEFAULT 0,

    -- SPR snapshot at compile time
    sbr_annual_production_hl_pa  NUMERIC NOT NULL DEFAULT 0,
    sbr_relief_rate_pct          NUMERIC NOT NULL DEFAULT 0,
    sbr_relief_pence             BIGINT  NOT NULL DEFAULT 0,

    net_duty_pence  BIGINT NOT NULL DEFAULT 0,

    submitted_at  TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE (tenant_id, period_start)
);
