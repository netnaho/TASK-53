-- CareOps Phase 3: Service Catalog, Packages, Client Plans, Delivery
-- Migration: 20240103000000_catalog_packages_plans_delivery
--
-- Replaces/extends Phase 1 stub tables with production schema for:
--   - Typed service catalog items
--   - Package definitions with per-visit/hourly/tiered billing rules
--   - Client plan lifecycle with package assignments
--   - Delivery entry capture with validation constraints
--   - Eligibility notes with encryption support

-- ============================================================
-- Drop Phase 1 stub tables that are being replaced
-- (delivery_entries has FK to client_plans, so drop in dependency order)
-- ============================================================
DROP TABLE IF EXISTS quality_scores;
DROP TABLE IF EXISTS payments;
DROP TABLE IF EXISTS invoices;
DROP TABLE IF EXISTS delivery_entries;
DROP TABLE IF EXISTS package_services;
DROP TABLE IF EXISTS client_plans;
DROP TABLE IF EXISTS packages;
DROP TABLE IF EXISTS service_catalog;

-- ============================================================
-- Service Catalog Items
-- ============================================================
CREATE TABLE service_catalog_items (
    id CHAR(36) PRIMARY KEY,
    org_id CHAR(36) NOT NULL,
    code VARCHAR(50) NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    category ENUM('nursing', 'rehab', 'meals', 'companionship', 'transportation', 'other') NOT NULL,
    unit_type ENUM('visit', 'hour', 'mile', 'meal', 'session') NOT NULL,
    default_rate DECIMAL(10,2) NOT NULL,
    is_active TINYINT(1) NOT NULL DEFAULT 1,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (org_id) REFERENCES organizations(id),
    UNIQUE KEY uq_catalog_org_code (org_id, code)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- Package Definitions
-- ============================================================
CREATE TABLE package_definitions (
    id CHAR(36) PRIMARY KEY,
    org_id CHAR(36) NOT NULL,
    code VARCHAR(50) NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    is_active TINYINT(1) NOT NULL DEFAULT 1,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (org_id) REFERENCES organizations(id),
    UNIQUE KEY uq_pkg_org_code (org_id, code)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- Package Rule Definitions
-- Each rule links a service to a package with a billing model
-- ============================================================
CREATE TABLE package_rule_definitions (
    id CHAR(36) PRIMARY KEY,
    package_id CHAR(36) NOT NULL,
    service_item_id CHAR(36) NOT NULL,
    rule_type ENUM('per_visit', 'hourly', 'tiered') NOT NULL,
    rate DECIMAL(10,2) NOT NULL,
    -- For hourly: minimum increment (0.25 = quarter-hour)
    min_increment DECIMAL(5,2) DEFAULT NULL,
    -- For tiered: thresholds stored as structured JSON
    -- e.g. [{"up_to": 4, "rate": 50.00}, {"up_to": 8, "rate": 45.00}, {"up_to": null, "rate": 40.00}]
    tier_config JSON DEFAULT NULL,
    -- Max units allowed per delivery (e.g. mileage cap)
    max_units_per_delivery DECIMAL(10,2) DEFAULT NULL,
    -- Max units allowed per billing period
    max_units_per_period INT DEFAULT NULL,
    is_active TINYINT(1) NOT NULL DEFAULT 1,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (package_id) REFERENCES package_definitions(id) ON DELETE CASCADE,
    FOREIGN KEY (service_item_id) REFERENCES service_catalog_items(id),
    UNIQUE KEY uq_rule_pkg_svc (package_id, service_item_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- Client Plans (enhanced)
-- ============================================================
CREATE TABLE client_plans (
    id CHAR(36) PRIMARY KEY,
    org_id CHAR(36) NOT NULL,
    department_id CHAR(36),
    project_id CHAR(36),
    client_name VARCHAR(255) NOT NULL,
    client_identifier_enc TEXT COMMENT 'AES-256-GCM encrypted client identifier',
    status ENUM('draft', 'active', 'paused', 'completed', 'cancelled') NOT NULL DEFAULT 'draft',
    start_date DATE NOT NULL,
    end_date DATE,
    notes_enc TEXT COMMENT 'AES-256-GCM encrypted plan notes',
    created_by CHAR(36),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (org_id) REFERENCES organizations(id),
    FOREIGN KEY (department_id) REFERENCES departments(id),
    FOREIGN KEY (project_id) REFERENCES projects(id),
    FOREIGN KEY (created_by) REFERENCES users(id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- Client Plan Package Assignments
-- Links packages to plans (a plan can have multiple packages)
-- ============================================================
CREATE TABLE client_plan_packages (
    id CHAR(36) PRIMARY KEY,
    plan_id CHAR(36) NOT NULL,
    package_id CHAR(36) NOT NULL,
    effective_date DATE NOT NULL,
    end_date DATE,
    status ENUM('active', 'paused', 'ended') NOT NULL DEFAULT 'active',
    assigned_by CHAR(36),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (plan_id) REFERENCES client_plans(id) ON DELETE CASCADE,
    FOREIGN KEY (package_id) REFERENCES package_definitions(id),
    FOREIGN KEY (assigned_by) REFERENCES users(id),
    UNIQUE KEY uq_plan_pkg_active (plan_id, package_id, effective_date)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- Delivery Entries
-- ============================================================
CREATE TABLE delivery_entries (
    id CHAR(36) PRIMARY KEY,
    org_id CHAR(36) NOT NULL,
    plan_id CHAR(36) NOT NULL,
    plan_package_id CHAR(36) NOT NULL,
    service_item_id CHAR(36) NOT NULL,
    provider_id CHAR(36) NOT NULL,
    delivery_date DATE NOT NULL,
    start_time TIME,
    end_time TIME,
    units DECIMAL(10,2) NOT NULL,
    mileage DECIMAL(10,2) DEFAULT NULL,
    notes_enc TEXT COMMENT 'AES-256-GCM encrypted delivery notes',
    status ENUM('draft', 'submitted', 'verified', 'rejected', 'billed') NOT NULL DEFAULT 'draft',
    verified_by CHAR(36),
    verified_at TIMESTAMP NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (org_id) REFERENCES organizations(id),
    FOREIGN KEY (plan_id) REFERENCES client_plans(id),
    FOREIGN KEY (plan_package_id) REFERENCES client_plan_packages(id),
    FOREIGN KEY (service_item_id) REFERENCES service_catalog_items(id),
    FOREIGN KEY (provider_id) REFERENCES users(id),
    INDEX idx_delivery_plan (plan_id),
    INDEX idx_delivery_provider (provider_id),
    INDEX idx_delivery_date (delivery_date),
    INDEX idx_delivery_status (status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- Eligibility Notes
-- Linked to plans or delivery entries for clinical context
-- ============================================================
CREATE TABLE eligibility_notes (
    id CHAR(36) PRIMARY KEY,
    org_id CHAR(36) NOT NULL,
    plan_id CHAR(36),
    delivery_entry_id CHAR(36),
    author_id CHAR(36) NOT NULL,
    note_enc TEXT NOT NULL COMMENT 'AES-256-GCM encrypted note content',
    note_type ENUM('eligibility', 'clinical', 'administrative') NOT NULL DEFAULT 'eligibility',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (org_id) REFERENCES organizations(id),
    FOREIGN KEY (plan_id) REFERENCES client_plans(id),
    FOREIGN KEY (delivery_entry_id) REFERENCES delivery_entries(id),
    FOREIGN KEY (author_id) REFERENCES users(id),
    INDEX idx_note_plan (plan_id),
    INDEX idx_note_delivery (delivery_entry_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- Re-create tables that depend on the new schema
-- ============================================================
CREATE TABLE invoices (
    id CHAR(36) PRIMARY KEY,
    plan_id CHAR(36) NOT NULL,
    period_start DATE NOT NULL,
    period_end DATE NOT NULL,
    total DECIMAL(12,2) NOT NULL,
    status ENUM('draft', 'issued', 'paid', 'overdue', 'cancelled') NOT NULL DEFAULT 'draft',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (plan_id) REFERENCES client_plans(id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE payments (
    id CHAR(36) PRIMARY KEY,
    invoice_id CHAR(36) NOT NULL,
    amount DECIMAL(12,2) NOT NULL,
    method ENUM('cash', 'check', 'eft', 'credit_card') NOT NULL,
    status ENUM('pending', 'completed', 'refunded', 'failed') NOT NULL DEFAULT 'pending',
    reference_number VARCHAR(100),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (invoice_id) REFERENCES invoices(id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE quality_scores (
    id CHAR(36) PRIMARY KEY,
    delivery_entry_id CHAR(36) NOT NULL,
    reviewer_id CHAR(36) NOT NULL,
    score DECIMAL(5,2) NOT NULL,
    notes TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (delivery_entry_id) REFERENCES delivery_entries(id),
    FOREIGN KEY (reviewer_id) REFERENCES users(id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
