-- Add 'spoiled' as a terminal batch status reachable from fermenting, conditioning, packaging, completed.

-- 1. Widen the CHECK constraint to allow 'spoiled'
ALTER TABLE batches DROP CONSTRAINT IF EXISTS batches_status_check;
ALTER TABLE batches ADD CONSTRAINT batches_status_check
    CHECK (status IN ('planned', 'brewing', 'fermenting', 'conditioning', 'packaging', 'completed', 'cancelled', 'spoiled'));

-- 2. Replace the FSM trigger to permit spoiled transitions
CREATE OR REPLACE FUNCTION enforce_batch_status_fsm()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.status = OLD.status THEN RETURN NEW; END IF;
    IF OLD.status = 'planned'      AND NEW.status IN ('brewing','cancelled')                THEN RETURN NEW; END IF;
    IF OLD.status = 'brewing'      AND NEW.status IN ('fermenting','cancelled')             THEN RETURN NEW; END IF;
    IF OLD.status = 'fermenting'   AND NEW.status IN ('conditioning','cancelled','spoiled') THEN RETURN NEW; END IF;
    IF OLD.status = 'conditioning' AND NEW.status IN ('packaging','cancelled','spoiled')    THEN RETURN NEW; END IF;
    IF OLD.status = 'packaging'    AND NEW.status IN ('completed','cancelled','spoiled')    THEN RETURN NEW; END IF;
    IF OLD.status = 'completed'    AND NEW.status IN ('spoiled')                            THEN RETURN NEW; END IF;
    RAISE EXCEPTION 'invalid batch status transition: % -> %', OLD.status, NEW.status
        USING ERRCODE = 'check_violation';
END;
$$ LANGUAGE plpgsql;
