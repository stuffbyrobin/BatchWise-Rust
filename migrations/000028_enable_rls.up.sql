-- 000028: enable row-level security on all application tables.
--
-- Supabase exposes an auto-generated Data API (PostgREST) at the project URL,
-- usable with the public `anon` key. Any table in the `public` schema without
-- RLS is fully readable/writable by the `anon`/`authenticated` roles, which the
-- security advisor flags ("RLS not enabled").
--
-- The app connects as the `postgres` role, which has BYPASSRLS, so enabling RLS
-- with NO policies denies all access to the API roles while leaving the app's
-- own (BYPASSRLS) connections completely unaffected. This is pure hardening.
--
-- Underscore-prefixed bookkeeping tables (e.g. `_sqlx_migrations`) are skipped.
DO $$
DECLARE
    r record;
BEGIN
    FOR r IN
        SELECT tablename
        FROM pg_tables
        WHERE schemaname = 'public'
          AND left(tablename, 1) <> '_'
    LOOP
        EXECUTE format('ALTER TABLE public.%I ENABLE ROW LEVEL SECURITY;', r.tablename);
    END LOOP;
END $$;
