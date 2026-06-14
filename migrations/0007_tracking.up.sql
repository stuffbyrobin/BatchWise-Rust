CREATE TABLE container_assets (
    id                       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id                UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    asset_number             TEXT NOT NULL,
    container_type           TEXT NOT NULL CHECK (container_type IN ('keg','cask','firkin','bottle_case','ibc','tank','other')),
    capacity_liters          NUMERIC NOT NULL,
    deposit_pence            BIGINT NOT NULL DEFAULT 0,
    status                   TEXT NOT NULL DEFAULT 'empty'
                              CHECK (status IN ('empty','filled','delivered','returned','lost','retired')),
    current_batch_id         UUID NULL REFERENCES batches(id) ON DELETE SET NULL,
    current_customer_name    TEXT,
    last_fill_date           DATE,
    last_return_date         DATE,
    notes                    TEXT,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, asset_number)
);
CREATE INDEX idx_container_assets_tenant_status ON container_assets (tenant_id, status);

CREATE TABLE container_logs (
    id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id            UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    container_id         UUID NOT NULL REFERENCES container_assets(id) ON DELETE CASCADE,
    event_type           TEXT NOT NULL CHECK (event_type IN ('fill','deliver','return','clean','inspect','status_change','custom')),
    from_status          TEXT,
    to_status            TEXT,
    batch_id             UUID NULL REFERENCES batches(id) ON DELETE SET NULL,
    customer_name        TEXT,
    notes                TEXT,
    logged_by_user_id    UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_container_logs_container_created ON container_logs (container_id, created_at DESC);
CREATE INDEX idx_container_logs_tenant_created ON container_logs (tenant_id, created_at DESC);
