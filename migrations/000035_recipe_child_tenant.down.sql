DROP INDEX IF EXISTS idx_recipe_yeasts_recipe;

ALTER TABLE recipe_fermentables DROP CONSTRAINT IF EXISTS recipe_fermentables_recipe_tenant_fkey;
ALTER TABLE recipe_fermentables ADD CONSTRAINT recipe_fermentables_recipe_id_fkey
    FOREIGN KEY (recipe_id) REFERENCES recipes (id) ON DELETE CASCADE;
ALTER TABLE recipe_fermentables DROP COLUMN IF EXISTS tenant_id;

ALTER TABLE recipe_hops DROP CONSTRAINT IF EXISTS recipe_hops_recipe_tenant_fkey;
ALTER TABLE recipe_hops ADD CONSTRAINT recipe_hops_recipe_id_fkey
    FOREIGN KEY (recipe_id) REFERENCES recipes (id) ON DELETE CASCADE;
ALTER TABLE recipe_hops DROP COLUMN IF EXISTS tenant_id;

ALTER TABLE recipe_yeasts DROP CONSTRAINT IF EXISTS recipe_yeasts_recipe_tenant_fkey;
ALTER TABLE recipe_yeasts ADD CONSTRAINT recipe_yeasts_recipe_id_fkey
    FOREIGN KEY (recipe_id) REFERENCES recipes (id) ON DELETE CASCADE;
ALTER TABLE recipe_yeasts DROP COLUMN IF EXISTS tenant_id;

ALTER TABLE recipe_mash_steps DROP CONSTRAINT IF EXISTS recipe_mash_steps_recipe_tenant_fkey;
ALTER TABLE recipe_mash_steps ADD CONSTRAINT recipe_mash_steps_recipe_id_fkey
    FOREIGN KEY (recipe_id) REFERENCES recipes (id) ON DELETE CASCADE;
ALTER TABLE recipe_mash_steps DROP COLUMN IF EXISTS tenant_id;


ALTER TABLE recipes DROP CONSTRAINT IF EXISTS recipes_id_tenant_id_key;
