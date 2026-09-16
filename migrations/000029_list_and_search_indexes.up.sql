-- Indexes for the common list and search paths (remediation plan, phase 8).
-- IF NOT EXISTS keeps the migration safe to re-run.

-- Default lists are newest-first within a tenant.
CREATE INDEX IF NOT EXISTS idx_recipes_tenant_created ON recipes (tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_label_records_tenant_created ON label_records (tenant_id, created_at DESC);

-- Library lists sort by name within a tenant. `yeasts` is already covered by
-- idx_yeasts_unique_name_product (tenant_id, name, ...).
CREATE INDEX IF NOT EXISTS idx_styles_tenant_name ON styles (tenant_id, name);
CREATE INDEX IF NOT EXISTS idx_equipment_profiles_tenant_name ON equipment_profiles (tenant_id, name);
CREATE INDEX IF NOT EXISTS idx_mash_profiles_tenant_name ON mash_profiles (tenant_id, name);
CREATE INDEX IF NOT EXISTS idx_library_fermentables_tenant_name ON library_fermentables (tenant_id, name);

-- Name search is a contains match (`lower(name) LIKE '%term%'`, and
-- `name ILIKE '%term%'` for recipes), which a btree cannot serve. Trigram GIN
-- indexes on the exact searched expressions can.
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE INDEX IF NOT EXISTS idx_ingredients_name_trgm ON ingredients USING gin (lower(name) gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_recipes_name_trgm ON recipes USING gin (name gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_fermenters_name_trgm ON fermenters USING gin (lower(name) gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_styles_name_trgm ON styles USING gin (lower(name) gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_equipment_profiles_name_trgm ON equipment_profiles USING gin (lower(name) gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_mash_profiles_name_trgm ON mash_profiles USING gin (lower(name) gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_yeasts_name_trgm ON yeasts USING gin (lower(name) gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_yeasts_manufacturer_trgm ON yeasts USING gin (lower(manufacturer) gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_library_fermentables_name_trgm ON library_fermentables USING gin (lower(name) gin_trgm_ops);
