-- Recipe child rows carry their recipe's tenant. A composite foreign key to
-- recipes (id, tenant_id) makes a child row with any other tenant impossible,
-- so every query on a child table can filter by tenant on its own instead of
-- relying on the caller having checked the recipe first.
ALTER TABLE recipes ADD CONSTRAINT recipes_id_tenant_id_key UNIQUE (id, tenant_id);

ALTER TABLE recipe_fermentables ADD COLUMN IF NOT EXISTS tenant_id UUID;
UPDATE recipe_fermentables c SET tenant_id = r.tenant_id FROM recipes r WHERE r.id = c.recipe_id AND c.tenant_id IS NULL;
ALTER TABLE recipe_fermentables ALTER COLUMN tenant_id SET NOT NULL;
ALTER TABLE recipe_fermentables DROP CONSTRAINT IF EXISTS recipe_fermentables_recipe_id_fkey;
ALTER TABLE recipe_fermentables ADD CONSTRAINT recipe_fermentables_recipe_tenant_fkey
    FOREIGN KEY (recipe_id, tenant_id) REFERENCES recipes (id, tenant_id) ON DELETE CASCADE;

ALTER TABLE recipe_hops ADD COLUMN IF NOT EXISTS tenant_id UUID;
UPDATE recipe_hops c SET tenant_id = r.tenant_id FROM recipes r WHERE r.id = c.recipe_id AND c.tenant_id IS NULL;
ALTER TABLE recipe_hops ALTER COLUMN tenant_id SET NOT NULL;
ALTER TABLE recipe_hops DROP CONSTRAINT IF EXISTS recipe_hops_recipe_id_fkey;
ALTER TABLE recipe_hops ADD CONSTRAINT recipe_hops_recipe_tenant_fkey
    FOREIGN KEY (recipe_id, tenant_id) REFERENCES recipes (id, tenant_id) ON DELETE CASCADE;

ALTER TABLE recipe_yeasts ADD COLUMN IF NOT EXISTS tenant_id UUID;
UPDATE recipe_yeasts c SET tenant_id = r.tenant_id FROM recipes r WHERE r.id = c.recipe_id AND c.tenant_id IS NULL;
ALTER TABLE recipe_yeasts ALTER COLUMN tenant_id SET NOT NULL;
ALTER TABLE recipe_yeasts DROP CONSTRAINT IF EXISTS recipe_yeasts_recipe_id_fkey;
ALTER TABLE recipe_yeasts ADD CONSTRAINT recipe_yeasts_recipe_tenant_fkey
    FOREIGN KEY (recipe_id, tenant_id) REFERENCES recipes (id, tenant_id) ON DELETE CASCADE;

ALTER TABLE recipe_mash_steps ADD COLUMN IF NOT EXISTS tenant_id UUID;
UPDATE recipe_mash_steps c SET tenant_id = r.tenant_id FROM recipes r WHERE r.id = c.recipe_id AND c.tenant_id IS NULL;
ALTER TABLE recipe_mash_steps ALTER COLUMN tenant_id SET NOT NULL;
ALTER TABLE recipe_mash_steps DROP CONSTRAINT IF EXISTS recipe_mash_steps_recipe_id_fkey;
ALTER TABLE recipe_mash_steps ADD CONSTRAINT recipe_mash_steps_recipe_tenant_fkey
    FOREIGN KEY (recipe_id, tenant_id) REFERENCES recipes (id, tenant_id) ON DELETE CASCADE;

-- The other child tables are indexed on recipe_id through UNIQUE (recipe_id, step_order).
CREATE INDEX IF NOT EXISTS idx_recipe_yeasts_recipe ON recipe_yeasts (recipe_id);
