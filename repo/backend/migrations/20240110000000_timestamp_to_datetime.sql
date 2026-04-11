-- Migration: Convert all TIMESTAMP columns to DATETIME
--
-- sqlx 0.7 maps `chrono::NaiveDateTime` to SQL type `DATETIME` and rejects
-- TIMESTAMP columns at decode time with "mismatched types" errors.  Every
-- domain FromRow struct in the backend uses `NaiveDateTime`, so we convert
-- the underlying columns once and for all.
--
-- DATETIME vs TIMESTAMP differences:
--   - DATETIME: no timezone conversion, range 1000-01-01..9999-12-31
--   - TIMESTAMP: UTC storage, range 1970-01-01..2038-01-19
-- For this application's use (audit timestamps, created_at/updated_at), both
-- are functionally equivalent.  DATETIME is also the default in modern MySQL.
--
-- Data preservation: MODIFY COLUMN ... DATETIME on a TIMESTAMP column is a
-- zero-loss conversion — existing values are read in UTC and written verbatim.
-- Default values and ON UPDATE clauses are restored where originally defined.

-- ============================================================
-- Phase 1 tables
-- ============================================================
ALTER TABLE organizations
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    MODIFY COLUMN updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP;

ALTER TABLE users
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    MODIFY COLUMN updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP;

-- ============================================================
-- Phase 2 tables: RBAC, auth, audit
-- ============================================================
ALTER TABLE departments
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    MODIFY COLUMN updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP;

ALTER TABLE projects
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    MODIFY COLUMN updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP;

ALTER TABLE user_credentials
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    MODIFY COLUMN updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP;

ALTER TABLE roles
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    MODIFY COLUMN updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP;

ALTER TABLE permissions
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP;

ALTER TABLE role_permissions
    MODIFY COLUMN granted_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP;

ALTER TABLE user_data_scopes
    MODIFY COLUMN granted_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP;

ALTER TABLE permission_version
    MODIFY COLUMN updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP;

-- audit_logs uses `timestamp` as its column name
ALTER TABLE audit_logs
    MODIFY COLUMN timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP;

-- ============================================================
-- Phase 3 tables: catalog, packages, plans, delivery
-- ============================================================
ALTER TABLE service_catalog_items
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    MODIFY COLUMN updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP;

ALTER TABLE package_definitions
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    MODIFY COLUMN updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP;

ALTER TABLE package_rule_definitions
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    MODIFY COLUMN updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP;

ALTER TABLE client_plans
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    MODIFY COLUMN updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP;

ALTER TABLE client_plan_packages
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    MODIFY COLUMN updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP;

ALTER TABLE delivery_entries
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    MODIFY COLUMN updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    MODIFY COLUMN verified_at DATETIME NULL;

ALTER TABLE eligibility_notes
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP;

-- ============================================================
-- Phase 4 tables: billing
-- ============================================================
ALTER TABLE charges
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    MODIFY COLUMN updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP;

ALTER TABLE charge_adjustments
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP;

ALTER TABLE invoices
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    MODIFY COLUMN updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP;

ALTER TABLE invoice_line_items
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP;

ALTER TABLE fund_transactions
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP;

ALTER TABLE recorded_payments
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP;

ALTER TABLE recorded_refunds
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP;

ALTER TABLE refund_reason_codes
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP;

ALTER TABLE reconciliation_runs
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP;

ALTER TABLE payment_idempotency_keys
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP;

-- ============================================================
-- Phase 5 tables: scoring, reporting
-- ============================================================
ALTER TABLE scoring_templates
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    MODIFY COLUMN updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP;

ALTER TABLE evaluation_questions
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP;

ALTER TABLE evaluations
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    MODIFY COLUMN updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP;

ALTER TABLE evaluation_answers
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    MODIFY COLUMN updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    MODIFY COLUMN graded_at DATETIME NULL;

ALTER TABLE score_reviews
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    MODIFY COLUMN reviewed_at DATETIME NULL;

ALTER TABLE export_audit_logs
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP;

ALTER TABLE quality_scores
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP;

-- ============================================================
-- Phase 6 tables: ops
-- ============================================================
ALTER TABLE ops_config
    MODIFY COLUMN updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP;

ALTER TABLE ops_events
    MODIFY COLUMN created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP;

-- ============================================================
-- DECIMAL → DOUBLE conversions
--
-- sqlx 0.7 maps Rust `f64` to SQL type `DOUBLE` and rejects DECIMAL columns
-- at decode time.  Several FromRow structs in the backend domain use `f64`
-- for monetary/score fields while their DB columns are DECIMAL.  We convert
-- those specific columns here so runtime decoding succeeds.
--
-- Columns that map to `sqlx::types::Decimal` in Rust (ServiceItemRow,
-- PackageRuleRow, DeliveryEntryRow) are intentionally left as DECIMAL.
--
-- Data loss: DOUBLE has ~15-17 decimal digits of precision.  All monetary
-- amounts in this application fit well within that range — no practical
-- precision loss for the amounts and scores involved.
-- ============================================================

-- scoring_templates: rounding_interval, max_score → f64
ALTER TABLE scoring_templates
    MODIFY COLUMN rounding_interval DOUBLE NOT NULL,
    MODIFY COLUMN max_score DOUBLE NOT NULL;

-- evaluation_questions: weight, max_points → f64
ALTER TABLE evaluation_questions
    MODIFY COLUMN weight DOUBLE NOT NULL,
    MODIFY COLUMN max_points DOUBLE NOT NULL;

-- evaluations: all score columns → Option<f64>
ALTER TABLE evaluations
    MODIFY COLUMN prior_final_score DOUBLE NULL,
    MODIFY COLUMN raw_score DOUBLE NULL,
    MODIFY COLUMN weighted_score DOUBLE NULL,
    MODIFY COLUMN final_score DOUBLE NULL,
    MODIFY COLUMN score_delta DOUBLE NULL;

-- evaluation_answers: all score columns → f64
ALTER TABLE evaluation_answers
    MODIFY COLUMN auto_score DOUBLE NOT NULL,
    MODIFY COLUMN manual_score DOUBLE NOT NULL,
    MODIFY COLUMN partial_credit_fraction DOUBLE NOT NULL,
    MODIFY COLUMN final_score DOUBLE NOT NULL;

-- score_reviews: score columns → f64/Option<f64>
ALTER TABLE score_reviews
    MODIFY COLUMN score_before_review DOUBLE NOT NULL,
    MODIFY COLUMN score_delta DOUBLE NOT NULL,
    MODIFY COLUMN revised_score DOUBLE NULL;

-- charges: all amount columns → f64
ALTER TABLE charges
    MODIFY COLUMN computed_units DOUBLE NOT NULL,
    MODIFY COLUMN rate_applied DOUBLE NOT NULL,
    MODIFY COLUMN gross_amount DOUBLE NOT NULL,
    MODIFY COLUMN adjustment_total DOUBLE NOT NULL DEFAULT 0.0,
    MODIFY COLUMN net_amount DOUBLE NOT NULL;

-- charge_adjustments.amount → f64
ALTER TABLE charge_adjustments
    MODIFY COLUMN amount DOUBLE NOT NULL;

-- invoices: all amount columns → f64
ALTER TABLE invoices
    MODIFY COLUMN subtotal DOUBLE NOT NULL DEFAULT 0.0,
    MODIFY COLUMN total_adjustments DOUBLE NOT NULL DEFAULT 0.0,
    MODIFY COLUMN total_amount DOUBLE NOT NULL DEFAULT 0.0;

-- invoice_line_items: all amount columns → f64
ALTER TABLE invoice_line_items
    MODIFY COLUMN units DOUBLE NOT NULL,
    MODIFY COLUMN unit_rate DOUBLE NOT NULL,
    MODIFY COLUMN gross_amount DOUBLE NOT NULL,
    MODIFY COLUMN adjustment_amount DOUBLE NOT NULL DEFAULT 0.0,
    MODIFY COLUMN net_amount DOUBLE NOT NULL;

-- fund_transactions.amount → f64
ALTER TABLE fund_transactions
    MODIFY COLUMN amount DOUBLE NOT NULL;

-- recorded_payments.amount → f64
ALTER TABLE recorded_payments
    MODIFY COLUMN amount DOUBLE NOT NULL;

-- recorded_refunds.amount → f64
ALTER TABLE recorded_refunds
    MODIFY COLUMN amount DOUBLE NOT NULL;

-- reconciliation_runs: all aggregates → f64
ALTER TABLE reconciliation_runs
    MODIFY COLUMN total_charges DOUBLE NOT NULL DEFAULT 0.0,
    MODIFY COLUMN total_adjustments DOUBLE NOT NULL DEFAULT 0.0,
    MODIFY COLUMN total_invoiced DOUBLE NOT NULL DEFAULT 0.0,
    MODIFY COLUMN total_paid DOUBLE NOT NULL DEFAULT 0.0,
    MODIFY COLUMN total_refunded DOUBLE NOT NULL DEFAULT 0.0,
    MODIFY COLUMN net_collected DOUBLE NOT NULL DEFAULT 0.0,
    MODIFY COLUMN outstanding_balance DOUBLE NOT NULL DEFAULT 0.0;
