-- 000027 down: remove batch assignment then the fermenters table

DROP INDEX IF EXISTS idx_batches_fermenter;
ALTER TABLE batches DROP COLUMN IF EXISTS fermenter_id;

DROP INDEX IF EXISTS idx_fermenters_tenant;
DROP TABLE IF EXISTS fermenters;
