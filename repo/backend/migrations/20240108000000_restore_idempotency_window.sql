-- Migration: Restore 5-minute idempotency window for payments
--
-- The previous migration (20240107) replaced the time-windowed idempotency
-- check with a permanent UNIQUE constraint on (org_id, idempotency_key).
-- This broke the documented 5-minute window behavior: callers should be able
-- to reuse the same key after 5 minutes (network retry scenario).
--
-- This migration introduces a dedicated `payment_idempotency_keys` table
-- with a PRIMARY KEY on (org_id, idempotency_key).  The application uses
-- INSERT ... ON DUPLICATE KEY UPDATE with a 5-minute time check, which is
-- both race-safe (no SELECT-then-INSERT gap) and time-windowed.
--
-- The UNIQUE constraint on recorded_payments is dropped because the same
-- key can now appear in multiple payment records (after window expiry).
--
-- Index operations use add-before-drop ordering to avoid MySQL error 1553
-- (cannot drop index needed by a foreign key on org_id).

-- 1. Create the idempotency tracking table
CREATE TABLE IF NOT EXISTS payment_idempotency_keys (
    org_id CHAR(36) NOT NULL,
    idempotency_key VARCHAR(255) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (org_id, idempotency_key)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- 2. Seed from existing payments so in-flight keys are still protected
INSERT IGNORE INTO payment_idempotency_keys (org_id, idempotency_key, created_at)
SELECT org_id, idempotency_key, created_at FROM recorded_payments;

-- 3. Restore the non-unique index FIRST (covers org_id for the FK)
CREATE INDEX idx_payment_idempotency ON recorded_payments (org_id, idempotency_key, created_at);

-- 4. NOW safe to drop the unique constraint (org_id FK is backed by the new index)
ALTER TABLE recorded_payments DROP INDEX uq_payments_org_idempotency_key;
