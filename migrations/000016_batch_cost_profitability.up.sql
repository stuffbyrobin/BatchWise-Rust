-- Add revenue column to batch_costs.
ALTER TABLE batch_costs
    ADD COLUMN revenue_pence BIGINT NOT NULL DEFAULT 0;

-- Update the cost_reports report_type constraint to include 'profitability'.
ALTER TABLE cost_reports
    DROP CONSTRAINT cost_reports_report_type_check;
ALTER TABLE cost_reports
    ADD CONSTRAINT cost_reports_report_type_check
    CHECK (report_type IN ('batch','recipe','period','inventory','profitability'));
