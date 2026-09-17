ALTER TABLE distribution_movements
    DROP CONSTRAINT IF EXISTS distribution_movements_void_reason,
    DROP COLUMN IF EXISTS void_reason,
    DROP COLUMN IF EXISTS voided_by,
    DROP COLUMN IF EXISTS voided_at;
