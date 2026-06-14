-- 000026: equipment register, maintenance schedules, and maintenance events

CREATE TABLE equipment (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id           UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name                TEXT NOT NULL,
    equipment_type      TEXT NOT NULL,
    serial_number       TEXT NULL,
    location            TEXT NULL,
    status              TEXT NOT NULL DEFAULT 'active'
                        CHECK (status IN ('active', 'retired')),
    purchased_at        DATE NULL,
    notes               TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_equipment_tenant_id ON equipment (tenant_id);

CREATE TABLE maintenance_schedules (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id           UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    equipment_id        UUID NOT NULL REFERENCES equipment(id) ON DELETE CASCADE,
    task_name           TEXT NOT NULL,
    interval_days       INT NOT NULL CHECK (interval_days > 0),
    last_performed_at   TIMESTAMPTZ NULL,
    active              BOOLEAN NOT NULL DEFAULT true,
    notes               TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_maintenance_schedules_equipment ON maintenance_schedules (equipment_id);
CREATE INDEX idx_maintenance_schedules_tenant ON maintenance_schedules (tenant_id);

CREATE TABLE maintenance_events (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id           UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    equipment_id        UUID NOT NULL REFERENCES equipment(id) ON DELETE CASCADE,
    schedule_id         UUID NULL REFERENCES maintenance_schedules(id) ON DELETE SET NULL,
    event_type          TEXT NOT NULL
                        CHECK (event_type IN ('service', 'calibration', 'repair', 'inspection', 'cleaning', 'other')),
    performed_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    performed_by        TEXT NULL,
    cost_pence          BIGINT NULL CHECK (cost_pence >= 0),
    cost_currency       CHAR(3) NOT NULL DEFAULT 'GBP',
    notes               TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_maintenance_events_equipment ON maintenance_events (equipment_id);
CREATE INDEX idx_maintenance_events_schedule ON maintenance_events (schedule_id);
CREATE INDEX idx_maintenance_events_tenant ON maintenance_events (tenant_id);
