-- CareOps Phase 4: Billing Engine, Invoices, Recorded Payments, Refunds, Reconciliation
-- Migration: 20240104000000_billing_payments_refunds
--
-- Implements the full financial transaction layer:
--   - Charges computed from delivery entries + package rules
--   - Charge adjustments (additive/subtractive corrections, immutable audit trail)
--   - Invoice header + line items
--   - Immutable fund_transactions ledger (no UPDATE/DELETE endpoints)
--   - Recorded payments with idempotency key + 5-minute duplicate window
--   - Recorded refunds with mandatory reason code + net-paid-amount cap
--   - Refund reason codes (seeded lookup)
--   - Reconciliation run snapshots

-- ============================================================
-- Drop legacy tables from initial schema that are replaced here
-- ============================================================
SET FOREIGN_KEY_CHECKS=0;
DROP TABLE IF EXISTS payments;
DROP TABLE IF EXISTS invoices;
SET FOREIGN_KEY_CHECKS=1;

-- ============================================================
-- Refund Reason Codes (lookup, seeded below)
-- ============================================================
CREATE TABLE refund_reason_codes (
    id CHAR(36) PRIMARY KEY,
    code VARCHAR(20) NOT NULL,
    label VARCHAR(100) NOT NULL,
    description TEXT,
    is_active TINYINT(1) NOT NULL DEFAULT 1,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE KEY uq_refund_reason_code (code)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- Charges
-- One charge record per delivery entry (1-to-1); computed on demand
-- and persisted for invoice generation and reconciliation.
-- ============================================================
CREATE TABLE charges (
    id CHAR(36) PRIMARY KEY,
    org_id CHAR(36) NOT NULL,
    delivery_entry_id CHAR(36) NOT NULL,
    plan_id CHAR(36) NOT NULL,
    invoice_id CHAR(36) NULL,                          -- NULL until invoiced
    rule_type ENUM('per_visit', 'hourly', 'tiered') NOT NULL,
    computed_units DECIMAL(10,4) NOT NULL,
    rate_applied DECIMAL(10,2) NOT NULL,
    gross_amount DECIMAL(10,2) NOT NULL,
    adjustment_total DECIMAL(10,2) NOT NULL DEFAULT 0.00,
    net_amount DECIMAL(10,2) NOT NULL,                  -- gross_amount + adjustment_total
    status ENUM('pending', 'adjusted', 'invoiced', 'voided') NOT NULL DEFAULT 'pending',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (org_id) REFERENCES organizations(id),
    FOREIGN KEY (delivery_entry_id) REFERENCES delivery_entries(id),
    FOREIGN KEY (plan_id) REFERENCES client_plans(id),
    UNIQUE KEY uq_charge_per_delivery (delivery_entry_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- Charge Adjustments
-- Additive corrections against a charge; immutable once created.
-- Positive amount increases charge; negative decreases it.
-- ============================================================
CREATE TABLE charge_adjustments (
    id CHAR(36) PRIMARY KEY,
    charge_id CHAR(36) NOT NULL,
    adjusted_by CHAR(36) NOT NULL,
    amount DECIMAL(10,2) NOT NULL,
    reason TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- No updated_at: immutable record
    FOREIGN KEY (charge_id) REFERENCES charges(id),
    FOREIGN KEY (adjusted_by) REFERENCES users(id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- Invoices
-- Billing document header. Contains period, totals, and status.
-- ============================================================
CREATE TABLE invoices (
    id CHAR(36) PRIMARY KEY,
    org_id CHAR(36) NOT NULL,
    plan_id CHAR(36) NOT NULL,
    invoice_number VARCHAR(50) NOT NULL,
    billing_period_start DATE NOT NULL,
    billing_period_end DATE NOT NULL,
    subtotal DECIMAL(10,2) NOT NULL DEFAULT 0.00,
    total_adjustments DECIMAL(10,2) NOT NULL DEFAULT 0.00,
    total_amount DECIMAL(10,2) NOT NULL DEFAULT 0.00,
    status ENUM('draft', 'issued', 'paid', 'partially_paid', 'voided') NOT NULL DEFAULT 'draft',
    generated_by CHAR(36) NOT NULL,
    notes TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (org_id) REFERENCES organizations(id),
    FOREIGN KEY (plan_id) REFERENCES client_plans(id),
    FOREIGN KEY (generated_by) REFERENCES users(id),
    UNIQUE KEY uq_invoice_number (org_id, invoice_number)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- Add FK from charges to invoices now that invoices exists
ALTER TABLE charges
    ADD CONSTRAINT fk_charge_invoice
    FOREIGN KEY (invoice_id) REFERENCES invoices(id);

-- ============================================================
-- Invoice Line Items
-- One row per charge included in an invoice; immutable snapshot.
-- ============================================================
CREATE TABLE invoice_line_items (
    id CHAR(36) PRIMARY KEY,
    invoice_id CHAR(36) NOT NULL,
    charge_id CHAR(36) NOT NULL,
    description VARCHAR(255) NOT NULL,
    delivery_date DATE NOT NULL,
    units DECIMAL(10,4) NOT NULL,
    unit_rate DECIMAL(10,2) NOT NULL,
    gross_amount DECIMAL(10,2) NOT NULL,
    adjustment_amount DECIMAL(10,2) NOT NULL DEFAULT 0.00,
    net_amount DECIMAL(10,2) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- No updated_at: immutable snapshot
    FOREIGN KEY (invoice_id) REFERENCES invoices(id),
    FOREIGN KEY (charge_id) REFERENCES charges(id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- Fund Transactions (immutable financial ledger)
-- Every real money movement is recorded here.
-- NO UPDATE or DELETE endpoints — corrections are new records.
-- ============================================================
CREATE TABLE fund_transactions (
    id CHAR(36) PRIMARY KEY,
    org_id CHAR(36) NOT NULL,
    invoice_id CHAR(36) NOT NULL,
    transaction_type ENUM('payment', 'refund') NOT NULL,
    amount DECIMAL(10,2) NOT NULL,                     -- always positive
    direction ENUM('credit', 'debit') NOT NULL,        -- credit=payment in, debit=refund out
    reference_id CHAR(36) NOT NULL,                    -- FK to recorded_payments or recorded_refunds
    actor_id CHAR(36) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- No updated_at: immutable ledger record
    FOREIGN KEY (org_id) REFERENCES organizations(id),
    FOREIGN KEY (invoice_id) REFERENCES invoices(id),
    FOREIGN KEY (actor_id) REFERENCES users(id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- Recorded Payments
-- Internal records of payments received against an invoice.
-- Idempotency key prevents duplicate submissions within 5 minutes.
-- ============================================================
CREATE TABLE recorded_payments (
    id CHAR(36) PRIMARY KEY,
    org_id CHAR(36) NOT NULL,
    invoice_id CHAR(36) NOT NULL,
    fund_transaction_id CHAR(36) NOT NULL,
    idempotency_key VARCHAR(255) NOT NULL,
    payment_method ENUM('check', 'ach', 'wire', 'credit_card', 'cash', 'other') NOT NULL,
    amount DECIMAL(10,2) NOT NULL,
    reference_number VARCHAR(100),
    payment_date DATE NOT NULL,
    recorded_by CHAR(36) NOT NULL,
    notes TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (org_id) REFERENCES organizations(id),
    FOREIGN KEY (invoice_id) REFERENCES invoices(id),
    FOREIGN KEY (fund_transaction_id) REFERENCES fund_transactions(id),
    FOREIGN KEY (recorded_by) REFERENCES users(id),
    UNIQUE KEY uq_payment_fund_tx (fund_transaction_id),
    -- Idempotency enforced at application layer (within 5-min window)
    -- A composite unique on (org_id, idempotency_key) would prevent ALL duplicates;
    -- we intentionally allow the same key after >5 minutes (retry scenario), so
    -- the application layer checks created_at > NOW() - INTERVAL 5 MINUTE.
    KEY idx_payment_idempotency (org_id, idempotency_key, created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- Recorded Refunds
-- Partial or full refunds against a recorded invoice.
-- Amount capped at: sum(recorded_payments) - sum(prior refunds).
-- Mandatory reason code required.
-- ============================================================
CREATE TABLE recorded_refunds (
    id CHAR(36) PRIMARY KEY,
    org_id CHAR(36) NOT NULL,
    invoice_id CHAR(36) NOT NULL,
    fund_transaction_id CHAR(36) NOT NULL,
    reason_code_id CHAR(36) NOT NULL,
    amount DECIMAL(10,2) NOT NULL,
    reason_notes TEXT,
    refund_method ENUM('check', 'ach', 'wire', 'credit_card', 'cash', 'other') NOT NULL,
    reference_number VARCHAR(100),
    refund_date DATE NOT NULL,
    recorded_by CHAR(36) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (org_id) REFERENCES organizations(id),
    FOREIGN KEY (invoice_id) REFERENCES invoices(id),
    FOREIGN KEY (fund_transaction_id) REFERENCES fund_transactions(id),
    FOREIGN KEY (reason_code_id) REFERENCES refund_reason_codes(id),
    FOREIGN KEY (recorded_by) REFERENCES users(id),
    UNIQUE KEY uq_refund_fund_tx (fund_transaction_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- Reconciliation Runs
-- Point-in-time summary snapshots for a billing period.
-- Immutable once generated.
-- ============================================================
CREATE TABLE reconciliation_runs (
    id CHAR(36) PRIMARY KEY,
    org_id CHAR(36) NOT NULL,
    period_start DATE NOT NULL,
    period_end DATE NOT NULL,
    run_by CHAR(36) NOT NULL,
    total_charges DECIMAL(10,2) NOT NULL DEFAULT 0.00,
    total_adjustments DECIMAL(10,2) NOT NULL DEFAULT 0.00,
    total_invoiced DECIMAL(10,2) NOT NULL DEFAULT 0.00,
    total_paid DECIMAL(10,2) NOT NULL DEFAULT 0.00,
    total_refunded DECIMAL(10,2) NOT NULL DEFAULT 0.00,
    net_collected DECIMAL(10,2) NOT NULL DEFAULT 0.00,
    pending_charge_count INT NOT NULL DEFAULT 0,
    invoiced_charge_count INT NOT NULL DEFAULT 0,
    paid_invoice_count INT NOT NULL DEFAULT 0,
    outstanding_balance DECIMAL(10,2) NOT NULL DEFAULT 0.00,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- No updated_at: immutable snapshot
    FOREIGN KEY (org_id) REFERENCES organizations(id),
    FOREIGN KEY (run_by) REFERENCES users(id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- Seed: Refund Reason Codes
-- ============================================================
INSERT INTO refund_reason_codes (id, code, label, description) VALUES
    (UUID(), 'BILLING_ERROR',     'Billing Error',              'Charge was computed or invoiced incorrectly'),
    (UUID(), 'SERVICE_NOT_REND',  'Service Not Rendered',       'Billed service was not actually delivered'),
    (UUID(), 'DUPLICATE_CHARGE',  'Duplicate Charge',           'Same service was charged more than once'),
    (UUID(), 'CONTRACT_CHANGE',   'Contract/Rate Change',       'Rate or contract terms changed retroactively'),
    (UUID(), 'CLIENT_REQUEST',    'Client-Requested Adjustment', 'Client disputed and adjustment was approved'),
    (UUID(), 'PARTIAL_SERVICE',   'Partial Service Delivered',  'Less than billed quantity was delivered'),
    (UUID(), 'QUALITY_ISSUE',     'Quality Issue',              'Service did not meet required quality standards'),
    (UUID(), 'INSURANCE_ADJ',     'Insurance/Payer Adjustment', 'Third-party payer required adjustment'),
    (UUID(), 'OTHER',             'Other',                      'Reason not covered by standard codes; see notes');
