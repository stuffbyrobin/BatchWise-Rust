CREATE TABLE packaging_runs (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id        UUID        NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    batch_id         UUID        NOT NULL REFERENCES batches(id) ON DELETE RESTRICT,
    format           TEXT        NOT NULL
                                 CHECK (format IN ('can','bottle','keg','cask','polypin','bag_in_box','other')),
    unit_volume_ml   INTEGER     NOT NULL CHECK (unit_volume_ml > 0),
    quantity         INTEGER     NOT NULL CHECK (quantity > 0),
    lot_number       TEXT        NOT NULL,
    packaged_at      DATE        NOT NULL,
    best_before_date DATE,
    notes            TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, lot_number)
);

CREATE INDEX idx_packaging_runs_tenant       ON packaging_runs (tenant_id);
CREATE INDEX idx_packaging_runs_tenant_batch ON packaging_runs (tenant_id, batch_id);

CREATE TABLE distribution_movements (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id        UUID        NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    packaging_run_id UUID        NOT NULL REFERENCES packaging_runs(id) ON DELETE RESTRICT,
    movement_type    TEXT        NOT NULL
                                 CHECK (movement_type IN
                                     ('sale','taproom_transfer','internal_transfer','sample','return','disposal')),
    quantity         INTEGER     NOT NULL CHECK (quantity > 0),
    from_location    TEXT        NOT NULL DEFAULT 'brewery',
    to_location      TEXT        NOT NULL,
    order_id         UUID        REFERENCES orders(id) ON DELETE SET NULL,
    reference        TEXT,
    notes            TEXT,
    moved_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_distribution_movements_tenant        ON distribution_movements (tenant_id);
CREATE INDEX idx_distribution_movements_packaging_run ON distribution_movements (packaging_run_id);
CREATE INDEX idx_distribution_movements_order         ON distribution_movements (order_id)
    WHERE order_id IS NOT NULL;
CREATE INDEX idx_distribution_movements_tenant_moved  ON distribution_movements (tenant_id, moved_at DESC);
