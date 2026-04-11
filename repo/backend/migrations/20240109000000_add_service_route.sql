-- Migration: Add service_route dimension to client_plans for route-based reporting
--
-- Business context: service route describes the geographic / logistics dimension
-- of a care plan (e.g. "client-to-clinic", "provider region", "north-metro").
-- This enables first-class route-based filtering in all reports and exports.
--
-- Non-breaking: column is nullable, default NULL.  Existing rows remain valid.
-- No existing column or index is modified.

ALTER TABLE client_plans
    ADD COLUMN service_route VARCHAR(100) DEFAULT NULL
    COMMENT 'Service route label for route-based report filtering (e.g. north-metro, client-to-clinic)'
    AFTER project_id;

-- Index for efficient report filtering when route is specified
CREATE INDEX idx_client_plans_service_route ON client_plans (service_route);
