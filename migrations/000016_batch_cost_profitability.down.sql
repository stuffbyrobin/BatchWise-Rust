ALTER TABLE cost_reports
    DROP CONSTRAINT cost_reports_report_type_check;
ALTER TABLE cost_reports
    ADD CONSTRAINT cost_reports_report_type_check
    CHECK (report_type IN ('batch','recipe','period','inventory'));

ALTER TABLE batch_costs DROP COLUMN revenue_pence;
