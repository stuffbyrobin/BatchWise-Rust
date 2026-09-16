-- The pg_trgm extension is left installed: other objects may come to rely on it.
DROP INDEX IF EXISTS idx_library_fermentables_name_trgm;
DROP INDEX IF EXISTS idx_yeasts_manufacturer_trgm;
DROP INDEX IF EXISTS idx_yeasts_name_trgm;
DROP INDEX IF EXISTS idx_mash_profiles_name_trgm;
DROP INDEX IF EXISTS idx_equipment_profiles_name_trgm;
DROP INDEX IF EXISTS idx_styles_name_trgm;
DROP INDEX IF EXISTS idx_fermenters_name_trgm;
DROP INDEX IF EXISTS idx_recipes_name_trgm;
DROP INDEX IF EXISTS idx_ingredients_name_trgm;
DROP INDEX IF EXISTS idx_library_fermentables_tenant_name;
DROP INDEX IF EXISTS idx_mash_profiles_tenant_name;
DROP INDEX IF EXISTS idx_equipment_profiles_tenant_name;
DROP INDEX IF EXISTS idx_styles_tenant_name;
DROP INDEX IF EXISTS idx_label_records_tenant_created;
DROP INDEX IF EXISTS idx_recipes_tenant_created;
