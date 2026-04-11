-- Migration: Atomic payment idempotency enforcement
--
-- Adds a unique constraint on (org_id, idempotency_key) to the recorded_payments
-- table so the database itself prevents duplicate payments under concurrent
-- requests.  The previous approach relied on an application-layer SELECT-then-INSERT
-- with a 5-minute time window, which is susceptible to race conditions.
--
-- The unique constraint replaces the time-window check: a given idempotency key
-- can now only ever be used once per org, regardless of timing.  The existing
-- index on (org_id, idempotency_key, created_at) is dropped in favor of the
-- stricter unique index.
--
-- NOTE: Migration 20240108 immediately reverses this to restore 5-minute
-- window semantics.  Both migrations use add-before-drop ordering to avoid
-- MySQL error 1553 (cannot drop index needed by a foreign key constraint).

-- Step 1: Add the unique constraint FIRST.  This gives MySQL an alternative
-- index covering org_id, which is needed by the FK to organizations.
ALTER TABLE recorded_payments
    ADD CONSTRAINT uq_payments_org_idempotency_key UNIQUE (org_id, idempotency_key);

-- Step 2: NOW safe to drop the old non-unique index, because the new unique
-- constraint above already covers org_id for the FK.
DROP INDEX idx_payment_idempotency ON recorded_payments;
