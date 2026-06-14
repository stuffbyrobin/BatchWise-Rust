DROP TABLE IF EXISTS purchase_order_lines;
DROP TABLE IF EXISTS purchase_orders;
DROP TABLE IF EXISTS suppliers;
ALTER TABLE tenants DROP COLUMN IF EXISTS next_po_number;
