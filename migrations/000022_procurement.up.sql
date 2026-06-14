ALTER TABLE tenants ADD COLUMN next_po_number INT NULL;

CREATE TABLE suppliers (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id    UUID        NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name         TEXT        NOT NULL,
    contact_name TEXT,
    email        TEXT,
    phone        TEXT,
    website      TEXT,
    notes        TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_suppliers_tenant ON suppliers (tenant_id);
CREATE UNIQUE INDEX idx_suppliers_tenant_name ON suppliers (tenant_id, lower(name));

CREATE TABLE purchase_orders (
    id                UUID  PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id         UUID  NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    supplier_id       UUID  NOT NULL REFERENCES suppliers(id) ON DELETE RESTRICT,
    po_number         TEXT  NOT NULL,
    status            TEXT  NOT NULL DEFAULT 'draft'
                            CHECK (status IN ('draft','sent','partially_received','received','cancelled')),
    order_date        DATE  NOT NULL DEFAULT CURRENT_DATE,
    expected_delivery DATE,
    notes             TEXT,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, po_number)
);
CREATE INDEX idx_purchase_orders_tenant          ON purchase_orders (tenant_id);
CREATE INDEX idx_purchase_orders_tenant_supplier ON purchase_orders (tenant_id, supplier_id);
CREATE INDEX idx_purchase_orders_tenant_status   ON purchase_orders (tenant_id, status);

CREATE TABLE purchase_order_lines (
    id                 UUID    PRIMARY KEY DEFAULT gen_random_uuid(),
    purchase_order_id  UUID    NOT NULL REFERENCES purchase_orders(id) ON DELETE CASCADE,
    ingredient_type    TEXT    NOT NULL
                               CHECK (ingredient_type IN ('fermentable','hop','yeast','adjunct','other')),
    ingredient_name    TEXT    NOT NULL,
    quantity           NUMERIC NOT NULL CHECK (quantity > 0),
    unit               TEXT    NOT NULL,
    unit_cost_pence    BIGINT  NOT NULL CHECK (unit_cost_pence >= 0),
    unit_cost_currency CHAR(3) NOT NULL DEFAULT 'GBP',
    received_quantity  NUMERIC,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_po_lines_order ON purchase_order_lines (purchase_order_id);
