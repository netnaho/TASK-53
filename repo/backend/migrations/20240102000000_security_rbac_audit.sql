-- CareOps Security, RBAC, and Audit Foundation
-- Migration: 20240102000000_security_rbac_audit
-- Description: Auth sessions, fine-grained RBAC, departments, projects,
--   data-scope mappings, permission versioning, encryption markers, audit logs

-- ============================================================
-- Drop the old role column from users (moving to separate role tables)
-- ============================================================
ALTER TABLE users DROP COLUMN role;
ALTER TABLE users DROP COLUMN password_hash;

-- ============================================================
-- Departments (sub-divisions of an organization)
-- ============================================================
CREATE TABLE IF NOT EXISTS departments (
    id CHAR(36) PRIMARY KEY,
    org_id CHAR(36) NOT NULL,
    name VARCHAR(255) NOT NULL,
    status ENUM('active', 'inactive') NOT NULL DEFAULT 'active',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (org_id) REFERENCES organizations(id),
    UNIQUE KEY uq_dept_org_name (org_id, name)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- Projects (scoped work units within a department)
-- ============================================================
CREATE TABLE IF NOT EXISTS projects (
    id CHAR(36) PRIMARY KEY,
    org_id CHAR(36) NOT NULL,
    department_id CHAR(36),
    name VARCHAR(255) NOT NULL,
    status ENUM('active', 'inactive', 'completed') NOT NULL DEFAULT 'active',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (org_id) REFERENCES organizations(id),
    FOREIGN KEY (department_id) REFERENCES departments(id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- Password credentials (separate from users for clean separation)
-- ============================================================
CREATE TABLE IF NOT EXISTS user_credentials (
    user_id CHAR(36) PRIMARY KEY,
    password_hash VARCHAR(512) NOT NULL,
    must_change_password TINYINT(1) NOT NULL DEFAULT 0,
    last_password_change TIMESTAMP NULL,
    failed_attempts INT NOT NULL DEFAULT 0,
    locked_until TIMESTAMP NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- Sessions (JWT tracking for invalidation support)
-- ============================================================
CREATE TABLE IF NOT EXISTS sessions (
    id CHAR(36) PRIMARY KEY,
    user_id CHAR(36) NOT NULL,
    token_hash VARCHAR(512) NOT NULL,
    issued_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP NOT NULL,
    revoked_at TIMESTAMP NULL,
    ip_address VARCHAR(45),
    user_agent VARCHAR(512),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    INDEX idx_sessions_user (user_id),
    INDEX idx_sessions_token (token_hash)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- Roles (named role definitions)
-- ============================================================
CREATE TABLE IF NOT EXISTS roles (
    id CHAR(36) PRIMARY KEY,
    name VARCHAR(100) NOT NULL UNIQUE,
    description TEXT,
    is_system TINYINT(1) NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- Permissions (granular permission definitions)
-- ============================================================
CREATE TABLE IF NOT EXISTS permissions (
    id CHAR(36) PRIMARY KEY,
    code VARCHAR(100) NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    category ENUM('menu', 'action', 'api', 'data') NOT NULL,
    description TEXT,
    resource VARCHAR(100),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- Role-Permission mapping
-- ============================================================
CREATE TABLE IF NOT EXISTS role_permissions (
    role_id CHAR(36) NOT NULL,
    permission_id CHAR(36) NOT NULL,
    granted_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (role_id, permission_id),
    FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE,
    FOREIGN KEY (permission_id) REFERENCES permissions(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- User-Role mapping (users can have multiple roles)
-- ============================================================
CREATE TABLE IF NOT EXISTS user_roles (
    user_id CHAR(36) NOT NULL,
    role_id CHAR(36) NOT NULL,
    assigned_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    assigned_by CHAR(36),
    PRIMARY KEY (user_id, role_id),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- User data-scope mappings
-- Defines what org/department/project data a user can access
-- ============================================================
CREATE TABLE IF NOT EXISTS user_data_scopes (
    id CHAR(36) PRIMARY KEY,
    user_id CHAR(36) NOT NULL,
    org_id CHAR(36) NOT NULL,
    department_id CHAR(36),
    project_id CHAR(36),
    access_level ENUM('read', 'write', 'admin') NOT NULL DEFAULT 'read',
    granted_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    granted_by CHAR(36),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (org_id) REFERENCES organizations(id),
    FOREIGN KEY (department_id) REFERENCES departments(id),
    FOREIGN KEY (project_id) REFERENCES projects(id),
    INDEX idx_scope_user (user_id),
    INDEX idx_scope_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- Permission version tracking (for cache invalidation)
-- Global counter incremented on any permission/role/scope change
-- ============================================================
CREATE TABLE IF NOT EXISTS permission_version (
    id INT PRIMARY KEY DEFAULT 1,
    version BIGINT UNSIGNED NOT NULL DEFAULT 1,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    CHECK (id = 1)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

INSERT INTO permission_version (id, version) VALUES (1, 1);

-- ============================================================
-- Audit logs (immutable append-only)
-- ============================================================
CREATE TABLE IF NOT EXISTS audit_logs (
    id CHAR(36) PRIMARY KEY,
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    user_id CHAR(36),
    action VARCHAR(100) NOT NULL,
    resource_type VARCHAR(100) NOT NULL,
    resource_id VARCHAR(100),
    org_id CHAR(36),
    details JSON,
    ip_address VARCHAR(45),
    trace_id VARCHAR(100),
    INDEX idx_audit_time (timestamp),
    INDEX idx_audit_user (user_id),
    INDEX idx_audit_resource (resource_type, resource_id),
    INDEX idx_audit_action (action),
    INDEX idx_audit_org (org_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- Add department_id and encrypted fields support to existing tables
-- ============================================================
ALTER TABLE users ADD COLUMN department_id CHAR(36) AFTER org_id;
ALTER TABLE users ADD CONSTRAINT fk_users_dept FOREIGN KEY (department_id) REFERENCES departments(id);

-- Encrypted client identifiers on client_plans
ALTER TABLE client_plans ADD COLUMN client_identifier_enc TEXT AFTER client_name;
ALTER TABLE client_plans ADD COLUMN notes_enc TEXT AFTER end_date;

-- Encrypted notes on delivery entries
ALTER TABLE delivery_entries MODIFY COLUMN notes TEXT COMMENT 'Stored encrypted when containing sensitive content';

-- Encrypted notes on quality scores
ALTER TABLE quality_scores MODIFY COLUMN notes TEXT COMMENT 'Stored encrypted when containing sensitive content';
