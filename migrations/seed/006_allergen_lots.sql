-- Seed: system-tenant ingredient lots with allergen declarations.
-- These rows are owned by the system tenant (all-zeros UUID) and are
-- visible to every brewery's allergen lookup queries.
-- Idempotent via ON CONFLICT (id) DO NOTHING.

INSERT INTO ingredients (
    id, tenant_id, type, name, amount, unit, lot_number,
    cost_pence, cost_currency, allergens, created_at, updated_at
) VALUES
-- Barley-based malts → gluten + barley
('66666666-6666-6666-6666-000000000001','00000000-0000-0000-0000-000000000000','fermentable','Maris Otter',          0,'kg','SYSTEM',0,'GBP','{"gluten","barley"}',now(),now()),
('66666666-6666-6666-6666-000000000002','00000000-0000-0000-0000-000000000000','fermentable','Pale Malt',            0,'kg','SYSTEM',0,'GBP','{"gluten","barley"}',now(),now()),
('66666666-6666-6666-6666-000000000003','00000000-0000-0000-0000-000000000000','fermentable','Pilsner Malt',         0,'kg','SYSTEM',0,'GBP','{"gluten","barley"}',now(),now()),
('66666666-6666-6666-6666-000000000004','00000000-0000-0000-0000-000000000000','fermentable','Lager Malt',           0,'kg','SYSTEM',0,'GBP','{"gluten","barley"}',now(),now()),
('66666666-6666-6666-6666-000000000005','00000000-0000-0000-0000-000000000000','fermentable','Munich Malt',          0,'kg','SYSTEM',0,'GBP','{"gluten","barley"}',now(),now()),
('66666666-6666-6666-6666-000000000006','00000000-0000-0000-0000-000000000000','fermentable','Vienna Malt',          0,'kg','SYSTEM',0,'GBP','{"gluten","barley"}',now(),now()),
('66666666-6666-6666-6666-000000000007','00000000-0000-0000-0000-000000000000','fermentable','Crystal Malt',         0,'kg','SYSTEM',0,'GBP','{"gluten","barley"}',now(),now()),
('66666666-6666-6666-6666-000000000008','00000000-0000-0000-0000-000000000000','fermentable','Chocolate Malt',       0,'kg','SYSTEM',0,'GBP','{"gluten","barley"}',now(),now()),
('66666666-6666-6666-6666-000000000009','00000000-0000-0000-0000-000000000000','fermentable','Black Malt',           0,'kg','SYSTEM',0,'GBP','{"gluten","barley"}',now(),now()),
('66666666-6666-6666-6666-000000000010','00000000-0000-0000-0000-000000000000','fermentable','Roasted Barley',       0,'kg','SYSTEM',0,'GBP','{"gluten","barley"}',now(),now()),
('66666666-6666-6666-6666-000000000011','00000000-0000-0000-0000-000000000000','fermentable','Flaked Barley',        0,'kg','SYSTEM',0,'GBP','{"gluten","barley"}',now(),now()),
-- Wheat-based → gluten + wheat
('66666666-6666-6666-6666-000000000012','00000000-0000-0000-0000-000000000000','fermentable','Wheat Malt',           0,'kg','SYSTEM',0,'GBP','{"gluten","wheat"}',now(),now()),
('66666666-6666-6666-6666-000000000013','00000000-0000-0000-0000-000000000000','fermentable','Flaked Wheat',         0,'kg','SYSTEM',0,'GBP','{"gluten","wheat"}',now(),now()),
('66666666-6666-6666-6666-000000000014','00000000-0000-0000-0000-000000000000','fermentable','Raw Wheat',            0,'kg','SYSTEM',0,'GBP','{"gluten","wheat"}',now(),now()),
-- Oat-based → gluten + oats
('66666666-6666-6666-6666-000000000015','00000000-0000-0000-0000-000000000000','fermentable','Oat Malt',             0,'kg','SYSTEM',0,'GBP','{"gluten","oats"}',now(),now()),
('66666666-6666-6666-6666-000000000016','00000000-0000-0000-0000-000000000000','fermentable','Flaked Oats',          0,'kg','SYSTEM',0,'GBP','{"gluten","oats"}',now(),now()),
('66666666-6666-6666-6666-000000000017','00000000-0000-0000-0000-000000000000','fermentable','Rolled Oats',          0,'kg','SYSTEM',0,'GBP','{"gluten","oats"}',now(),now()),
-- Rye-based → gluten + rye
('66666666-6666-6666-6666-000000000018','00000000-0000-0000-0000-000000000000','fermentable','Rye Malt',             0,'kg','SYSTEM',0,'GBP','{"gluten","rye"}',now(),now()),
('66666666-6666-6666-6666-000000000019','00000000-0000-0000-0000-000000000000','fermentable','Flaked Rye',           0,'kg','SYSTEM',0,'GBP','{"gluten","rye"}',now(),now()),
-- Dairy adjuncts → milk
('66666666-6666-6666-6666-000000000020','00000000-0000-0000-0000-000000000000','fermentable','Lactose',              0,'kg','SYSTEM',0,'GBP','{"milk"}',now(),now()),
('66666666-6666-6666-6666-000000000021','00000000-0000-0000-0000-000000000000','fermentable','Milk Sugar',           0,'kg','SYSTEM',0,'GBP','{"milk"}',now(),now()),
-- Nut adjuncts → nuts
('66666666-6666-6666-6666-000000000022','00000000-0000-0000-0000-000000000000','fermentable','Hazelnuts',            0,'kg','SYSTEM',0,'GBP','{"nuts"}',now(),now()),
('66666666-6666-6666-6666-000000000023','00000000-0000-0000-0000-000000000000','fermentable','Almonds',              0,'kg','SYSTEM',0,'GBP','{"nuts"}',now(),now()),
('66666666-6666-6666-6666-000000000024','00000000-0000-0000-0000-000000000000','fermentable','Coconut',              0,'kg','SYSTEM',0,'GBP','{"nuts"}',now(),now()),
-- Legume adjuncts
('66666666-6666-6666-6666-000000000025','00000000-0000-0000-0000-000000000000','fermentable','Peanuts',              0,'kg','SYSTEM',0,'GBP','{"peanuts"}',now(),now()),
('66666666-6666-6666-6666-000000000026','00000000-0000-0000-0000-000000000000','fermentable','Soya',                 0,'kg','SYSTEM',0,'GBP','{"soya"}',now(),now()),
('66666666-6666-6666-6666-000000000027','00000000-0000-0000-0000-000000000000','fermentable','Soya Flour',           0,'kg','SYSTEM',0,'GBP','{"soya"}',now(),now()),
-- Sulphites (when used as a preservative)
('66666666-6666-6666-6666-000000000028','00000000-0000-0000-0000-000000000000','adjunct',    'Potassium Metabisulphite',0,'g','SYSTEM',0,'GBP','{"sulphites"}',now(),now()),
('66666666-6666-6666-6666-000000000029','00000000-0000-0000-0000-000000000000','adjunct',    'Sodium Metabisulphite', 0,'g','SYSTEM',0,'GBP','{"sulphites"}',now(),now()),
-- Sesame
('66666666-6666-6666-6666-000000000030','00000000-0000-0000-0000-000000000000','adjunct',    'Sesame Seeds',         0,'g','SYSTEM',0,'GBP','{"sesame"}',now(),now()),
-- Lupin
('66666666-6666-6666-6666-000000000031','00000000-0000-0000-0000-000000000000','adjunct',    'Lupin Flour',          0,'g','SYSTEM',0,'GBP','{"lupin"}',now(),now())
ON CONFLICT (id) DO NOTHING;
