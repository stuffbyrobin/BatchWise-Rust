ALTER TABLE tenants ADD COLUMN next_order_number INT NULL;

CREATE TABLE customers (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    contact_name    TEXT,
    email           TEXT,
    phone           TEXT,
    address_line1   TEXT,
    address_line2   TEXT,
    city            TEXT,
    postcode        TEXT,
    country         CHAR(2) NOT NULL DEFAULT 'GB',
    notes           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, name)
);
CREATE INDEX idx_customers_tenant ON customers (tenant_id);

CREATE TABLE orders (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id        UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    customer_id      UUID NOT NULL REFERENCES customers(id),
    order_number     TEXT NOT NULL,
    status           TEXT NOT NULL DEFAULT 'draft'
                     CHECK (status IN ('draft','confirmed','fulfilled','invoiced','cancelled')),
    order_date       DATE NOT NULL DEFAULT CURRENT_DATE,
    fulfillment_date DATE,
    notes            TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, order_number)
);
CREATE INDEX idx_orders_tenant_status   ON orders (tenant_id, status);
CREATE INDEX idx_orders_tenant_customer ON orders (tenant_id, customer_id);
CREATE INDEX idx_orders_tenant_date     ON orders (tenant_id, order_date DESC);

CREATE TABLE order_items (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id         UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    order_id          UUID NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    batch_id          UUID REFERENCES batches(id) ON DELETE SET NULL,
    product_name      TEXT NOT NULL,
    volume_liters     NUMERIC NOT NULL CHECK (volume_liters > 0),
    unit_price_pence  BIGINT NOT NULL CHECK (unit_price_pence >= 0),
    quantity          INTEGER NOT NULL DEFAULT 1 CHECK (quantity > 0),
    total_price_pence BIGINT GENERATED ALWAYS AS (unit_price_pence * quantity) STORED,
    notes             TEXT,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_order_items_order ON order_items (order_id);
CREATE INDEX idx_order_items_batch ON order_items (batch_id) WHERE batch_id IS NOT NULL;

CREATE TABLE duty_events (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    order_id        UUID NOT NULL REFERENCES orders(id),
    batch_id        UUID REFERENCES batches(id) ON DELETE SET NULL,
    event_type      TEXT NOT NULL CHECK (event_type IN ('sale','sample','waste','export')),
    volume_liters   NUMERIC NOT NULL,
    abv_pct         NUMERIC NOT NULL,
    duty_pence      BIGINT NOT NULL,
    jurisdiction    CHAR(2) NOT NULL DEFAULT 'GB',
    crystallised_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_duty_events_tenant_crystallised ON duty_events (tenant_id, crystallised_at DESC);
CREATE INDEX idx_duty_events_order               ON duty_events (order_id);
CREATE INDEX idx_duty_events_batch               ON duty_events (batch_id) WHERE batch_id IS NOT NULL;
