DROP TABLE IF EXISTS batch_ingredients;
DROP TRIGGER IF EXISTS trg_batches_fsm ON batches;
DROP FUNCTION IF EXISTS enforce_batch_status_fsm();
DROP TABLE IF EXISTS batches;
