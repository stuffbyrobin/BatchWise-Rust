CREATE TABLE batches (
    id                     UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id              UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    recipe_id              UUID NULL REFERENCES recipes(id) ON DELETE SET NULL,
    batch_number           TEXT NOT NULL,
    name                   TEXT NOT NULL,
    status                 TEXT NOT NULL DEFAULT 'planned'
                            CHECK (status IN ('planned', 'brewing', 'fermenting', 'conditioning', 'packaging', 'completed', 'cancelled')),
    brew_date              DATE NULL,
    package_date           DATE NULL,
    target_og              NUMERIC NULL,
    actual_og              NUMERIC NULL,
    target_fg              NUMERIC NULL,
    actual_fg              NUMERIC NULL,
    actual_volume_liters   NUMERIC NULL,
    notes                  TEXT,
    duty_status            TEXT NOT NULL DEFAULT 'suspended'
                            CHECK (duty_status IN ('suspended', 'released')),
    batch_recipe_snapshot  JSONB NOT NULL,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, batch_number)
);
CREATE INDEX idx_batches_tenant_status ON batches (tenant_id, status);
CREATE INDEX idx_batches_tenant_brewdate ON batches (tenant_id, brew_date);

CREATE OR REPLACE FUNCTION enforce_batch_status_fsm()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.status = OLD.status THEN RETURN NEW; END IF;
    IF OLD.status = 'planned'      AND NEW.status IN ('brewing','cancelled')      THEN RETURN NEW; END IF;
    IF OLD.status = 'brewing'      AND NEW.status IN ('fermenting','cancelled')   THEN RETURN NEW; END IF;
    IF OLD.status = 'fermenting'   AND NEW.status IN ('conditioning','cancelled') THEN RETURN NEW; END IF;
    IF OLD.status = 'conditioning' AND NEW.status IN ('packaging','cancelled')    THEN RETURN NEW; END IF;
    IF OLD.status = 'packaging'    AND NEW.status IN ('completed','cancelled')    THEN RETURN NEW; END IF;
    RAISE EXCEPTION 'invalid batch status transition: % -> %', OLD.status, NEW.status
        USING ERRCODE = 'check_violation';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_batches_fsm
    BEFORE UPDATE OF status ON batches
    FOR EACH ROW EXECUTE FUNCTION enforce_batch_status_fsm();

CREATE TABLE batch_ingredients (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    batch_id            UUID NOT NULL REFERENCES batches(id) ON DELETE CASCADE,
    ingredient_id       UUID NOT NULL REFERENCES ingredients(id) ON DELETE RESTRICT,
    amount_deducted     NUMERIC NOT NULL,
    unit                TEXT NOT NULL,
    cost_pence          BIGINT NOT NULL DEFAULT 0,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_batch_ingredients_batch ON batch_ingredients (batch_id);
CREATE INDEX idx_batch_ingredients_ingredient ON batch_ingredients (ingredient_id);
