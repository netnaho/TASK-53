-- CareOps Phase 6: Operational Controls — degradation toggles and ops audit
-- Migration: 20240106000000_ops_config
--
-- Implements:
--   - ops_config: key/value store for degradation toggles and operational settings
--   - ops_events: immutable log of every ops control change (separate from general audit_logs)

-- ============================================================
-- Operational configuration table
-- Key/value store for degradation toggles and ops settings.
-- All changes must be audited before writing to this table.
-- ============================================================
CREATE TABLE ops_config (
    key_name   VARCHAR(64) PRIMARY KEY,
    value      VARCHAR(255) NOT NULL,
    updated_by VARCHAR(36) NOT NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (updated_by) REFERENCES users(id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- Ops events log (immutable — no UPDATE/DELETE)
-- Separate from audit_logs to allow targeted compliance queries
-- on operational control changes specifically.
-- ============================================================
CREATE TABLE ops_events (
    id         CHAR(36) PRIMARY KEY,
    event_type VARCHAR(64) NOT NULL,  -- 'toggle.changed', 'chaos.started', 'chaos.stopped', 'alert.fired', 'alert.cleared'
    key_name   VARCHAR(64),           -- for toggle changes: the key that changed
    old_value  VARCHAR(255),
    new_value  VARCHAR(255),
    actor_id   VARCHAR(36) NOT NULL,
    note       TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- Seed: default operational configuration
-- Note: foreign key to users will be satisfied after seed_service
-- runs. Migration inserts a placeholder that seed_service overwrites.
-- The seed_service uses INSERT ... ON DUPLICATE KEY UPDATE.
-- ============================================================
-- (Actual seeding done in seed_service.rs using the system admin user ID)
