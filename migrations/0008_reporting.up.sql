CREATE TABLE cost_rates (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    rate_type       TEXT NOT NULL CHECK (rate_type IN ('energy','labor','water','duty','overhead')),
    rate_name       TEXT NOT NULL,
    unit            TEXT NOT NULL,
    rate_value      NUMERIC NOT NULL,
    currency        CHAR(3) NOT NULL DEFAULT 'GBP',
    effective_from  DATE NOT NULL,
    effective_to    DATE NULL,
    notes           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, rate_type, rate_name, effective_from)
);
CREATE INDEX idx_cost_rates_tenant_type ON cost_rates (tenant_id, rate_type);

CREATE TABLE batch_costs (
    id                       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id                UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    batch_id                 UUID NOT NULL REFERENCES batches(id) ON DELETE CASCADE,
    ingredient_cost_pence    BIGINT NOT NULL DEFAULT 0,
    energy_cost_pence        BIGINT NOT NULL DEFAULT 0,
    labor_cost_pence         BIGINT NOT NULL DEFAULT 0,
    water_cost_pence         BIGINT NOT NULL DEFAULT 0,
    overhead_cost_pence      BIGINT NOT NULL DEFAULT 0,
    estimated_duty_pence     BIGINT NOT NULL DEFAULT 0,
    total_cost_pence         BIGINT GENERATED ALWAYS AS
                                (ingredient_cost_pence + energy_cost_pence + labor_cost_pence + water_cost_pence + overhead_cost_pence + estimated_duty_pence) STORED,
    cost_per_liter_pence     BIGINT,
    cost_per_unit_pence      BIGINT,
    computed_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, batch_id)
);
CREATE INDEX idx_batch_costs_tenant ON batch_costs (tenant_id);

CREATE TABLE cost_reports (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id      UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    report_type    TEXT NOT NULL CHECK (report_type IN ('batch','recipe','period','inventory')),
    period_start   DATE,
    period_end     DATE,
    report_data    JSONB NOT NULL,
    generated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_cost_reports_tenant_generated ON cost_reports (tenant_id, generated_at DESC);
