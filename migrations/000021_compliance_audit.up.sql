CREATE TABLE compliance_audit_log (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id     UUID        NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    event_type    TEXT        NOT NULL,
    entity_type   TEXT        NOT NULL,
    entity_id     UUID,
    actor_user_id UUID        REFERENCES users(id) ON DELETE SET NULL,
    event_data    JSONB       NOT NULL DEFAULT '{}',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
    -- No updated_at — this table is append-only.
);

CREATE INDEX idx_audit_tenant_created ON compliance_audit_log (tenant_id, created_at DESC);
CREATE INDEX idx_audit_entity         ON compliance_audit_log (entity_type, entity_id)
    WHERE entity_id IS NOT NULL;
CREATE INDEX idx_audit_event_type     ON compliance_audit_log (tenant_id, event_type);
