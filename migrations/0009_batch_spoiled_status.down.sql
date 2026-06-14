-- Revert spoiled status: remove from CHECK constraint and restore original FSM trigger.

ALTER TABLE batches DROP CONSTRAINT IF EXISTS batches_status_check;
ALTER TABLE batches ADD CONSTRAINT batches_status_check
    CHECK (status IN ('planned', 'brewing', 'fermenting', 'conditioning', 'packaging', 'completed', 'cancelled'));

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
