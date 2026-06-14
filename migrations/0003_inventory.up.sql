-- 0003: ingredients (lot-based), stock_movements

CREATE TABLE ingredients (
    id                 UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id          UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    type               TEXT NOT NULL CHECK (type IN ('fermentable', 'hop', 'yeast', 'adjunct', 'chemical', 'other')),
    name               TEXT NOT NULL,
    amount             NUMERIC NOT NULL DEFAULT 0,
    unit               TEXT NOT NULL CHECK (unit IN ('kg', 'g', 'L', 'mL', 'count')),
    lot_number         TEXT NOT NULL,
    best_before_date   DATE NULL,
    cost_pence         BIGINT NOT NULL DEFAULT 0,
    cost_currency      CHAR(3) NOT NULL DEFAULT 'GBP',
    supplier           TEXT,
    origin             TEXT,
    color_ebc          NUMERIC,
    allergens          TEXT[] NOT NULL DEFAULT '{}',
    notes              TEXT,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, lot_number)
);

-- Partial index covering only in-stock lots; directly supports the FIFO selection query.
CREATE INDEX idx_ingredients_fifo
    ON ingredients (tenant_id, type, lower(name), unit,
                    best_before_date ASC NULLS LAST, created_at ASC, lot_number ASC)
    WHERE amount > 0;

CREATE INDEX idx_ingredients_tenant ON ingredients (tenant_id);

CREATE TABLE stock_movements (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id           UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    ingredient_id       UUID NOT NULL REFERENCES ingredients(id) ON DELETE CASCADE,
    amount_delta        NUMERIC NOT NULL,
    balance_after       NUMERIC NOT NULL,
    reference_type      TEXT NOT NULL CHECK (reference_type IN ('batch', 'manual', 'waste', 'transfer', 'stock_in')),
    reference_id        UUID NULL,
    notes               TEXT,
    created_by_user_id  UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_stock_movements_tenant_created ON stock_movements (tenant_id, created_at DESC);
CREATE INDEX idx_stock_movements_ingredient ON stock_movements (ingredient_id, created_at DESC);
CREATE INDEX idx_stock_movements_reference ON stock_movements (tenant_id, reference_type, reference_id);
