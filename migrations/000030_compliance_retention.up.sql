-- Compliance records must outlive tenant closure for the retention period:
-- deleting a tenant that has duty events or audit entries now fails instead of
-- cascading. (duty_returns already blocks it: its tenant FK has no ON DELETE.)
ALTER TABLE duty_events
    DROP CONSTRAINT IF EXISTS duty_events_tenant_id_fkey,
    ADD CONSTRAINT duty_events_tenant_id_fkey
        FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE RESTRICT;

ALTER TABLE compliance_audit_log
    DROP CONSTRAINT IF EXISTS compliance_audit_log_tenant_id_fkey,
    ADD CONSTRAINT compliance_audit_log_tenant_id_fkey
        FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE RESTRICT;

-- The audit log is append-only, even against a code bug or a manual query.
-- Users are deactivated rather than deleted, so the actor_user_id
-- ON DELETE SET NULL action (an UPDATE) is never needed.
CREATE OR REPLACE FUNCTION compliance_audit_log_append_only() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'compliance_audit_log is append-only';
END;
$$;

DROP TRIGGER IF EXISTS compliance_audit_log_append_only ON compliance_audit_log;
CREATE TRIGGER compliance_audit_log_append_only
    BEFORE UPDATE OR DELETE ON compliance_audit_log
    FOR EACH ROW EXECUTE FUNCTION compliance_audit_log_append_only();
