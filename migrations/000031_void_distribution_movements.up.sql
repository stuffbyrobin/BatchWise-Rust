-- Distribution movements are traceability records: a mistaken movement is
-- voided (kept, with who voided it, when and why) instead of deleted.
ALTER TABLE distribution_movements
    ADD COLUMN IF NOT EXISTS voided_at   TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS voided_by   UUID REFERENCES users(id),
    ADD COLUMN IF NOT EXISTS void_reason TEXT;

ALTER TABLE distribution_movements
    DROP CONSTRAINT IF EXISTS distribution_movements_void_reason,
    ADD CONSTRAINT distribution_movements_void_reason
        CHECK ((voided_at IS NULL) = (void_reason IS NULL));
