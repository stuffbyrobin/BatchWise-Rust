DROP TRIGGER IF EXISTS compliance_audit_log_append_only ON compliance_audit_log;
DROP FUNCTION IF EXISTS compliance_audit_log_append_only();

ALTER TABLE compliance_audit_log
    DROP CONSTRAINT IF EXISTS compliance_audit_log_tenant_id_fkey,
    ADD CONSTRAINT compliance_audit_log_tenant_id_fkey
        FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE;

ALTER TABLE duty_events
    DROP CONSTRAINT IF EXISTS duty_events_tenant_id_fkey,
    ADD CONSTRAINT duty_events_tenant_id_fkey
        FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE;
