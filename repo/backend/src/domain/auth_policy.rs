/// Authorization policy enforcement module.
///
/// Defines the canonical permission codes used throughout the system.
/// These codes are seeded into the permissions table and referenced
/// by route guards, frontend navigation, and audit logging.

/// Menu visibility permissions
pub mod menu {
    pub const DASHBOARD: &str = "menu.dashboard";
    pub const ADMIN: &str = "menu.admin";
    pub const USERS: &str = "menu.users";
    pub const CATALOG: &str = "menu.catalog";
    pub const PLANS: &str = "menu.plans";
    pub const DELIVERY: &str = "menu.delivery";
    pub const BILLING: &str = "menu.billing";
    pub const SCORING: &str = "menu.scoring";
    pub const REPORTS: &str = "menu.reports";
    pub const AUDIT: &str = "menu.audit";
}

/// Action/button permissions
pub mod action {
    pub const CREATE_USER: &str = "action.users.create";
    pub const EDIT_USER: &str = "action.users.edit";
    pub const DEACTIVATE_USER: &str = "action.users.deactivate";
    pub const ASSIGN_ROLE: &str = "action.roles.assign";
    pub const MANAGE_ROLES: &str = "action.roles.manage";
    pub const MANAGE_PERMISSIONS: &str = "action.permissions.manage";
    pub const MANAGE_SCOPES: &str = "action.scopes.manage";
    pub const MANAGE_ORG: &str = "action.org.manage";
    pub const MANAGE_DEPT: &str = "action.dept.manage";
    pub const MANAGE_PROJECT: &str = "action.project.manage";
    pub const CREATE_SERVICE: &str = "action.catalog.create";
    pub const EDIT_SERVICE: &str = "action.catalog.edit";
    pub const CREATE_PACKAGE: &str = "action.packages.create";
    pub const CREATE_PLAN: &str = "action.plans.create";
    pub const EDIT_PLAN: &str = "action.plans.edit";
    pub const LOG_DELIVERY: &str = "action.delivery.log";
    pub const VERIFY_DELIVERY: &str = "action.delivery.verify";
    pub const GENERATE_INVOICE: &str = "action.billing.generate";
    pub const APPROVE_INVOICE: &str = "action.billing.approve";
    pub const RECORD_PAYMENT: &str = "action.payments.record";
    pub const PROCESS_REFUND: &str = "action.payments.refund";
    pub const SUBMIT_SCORE: &str = "action.scoring.submit";
    pub const GENERATE_REPORT: &str = "action.reports.generate";
    pub const EXPORT_DATA: &str = "action.reports.export";
}

/// API-level authorization permissions
pub mod api {
    pub const USERS_READ: &str = "api.users.read";
    pub const USERS_WRITE: &str = "api.users.write";
    pub const ROLES_READ: &str = "api.roles.read";
    pub const ROLES_WRITE: &str = "api.roles.write";
    pub const PERMISSIONS_READ: &str = "api.permissions.read";
    pub const ORG_READ: &str = "api.org.read";
    pub const ORG_WRITE: &str = "api.org.write";
    pub const DEPT_READ: &str = "api.dept.read";
    pub const DEPT_WRITE: &str = "api.dept.write";
    pub const PROJECT_READ: &str = "api.project.read";
    pub const PROJECT_WRITE: &str = "api.project.write";
    pub const CATALOG_READ: &str = "api.catalog.read";
    pub const CATALOG_WRITE: &str = "api.catalog.write";
    pub const PLANS_READ: &str = "api.plans.read";
    pub const PLANS_WRITE: &str = "api.plans.write";
    pub const DELIVERY_READ: &str = "api.delivery.read";
    pub const DELIVERY_WRITE: &str = "api.delivery.write";
    pub const BILLING_READ: &str = "api.billing.read";
    pub const BILLING_WRITE: &str = "api.billing.write";
    pub const PAYMENTS_READ: &str = "api.payments.read";
    pub const PAYMENTS_WRITE: &str = "api.payments.write";
    pub const SCORING_READ: &str = "api.scoring.read";
    pub const SCORING_WRITE: &str = "api.scoring.write";
    pub const REPORTS_READ: &str = "api.reports.read";
    pub const EXPORT_UNMASKED: &str = "api.export.unmasked";
    pub const OPS_READ: &str = "api.ops.read";
    pub const OPS_WRITE: &str = "api.ops.write";
    pub const AUDIT_READ: &str = "api.audit.read";
}

/// Data-scope access levels
pub mod scope {
    pub const READ: &str = "read";
    pub const WRITE: &str = "write";
    pub const ADMIN: &str = "admin";
}

/// Returns all permission definitions for seeding.
/// Each tuple: (code, name, category, description, resource)
pub fn all_permissions() -> Vec<(&'static str, &'static str, &'static str, &'static str, &'static str)> {
    vec![
        // Menu permissions
        (menu::DASHBOARD, "View Dashboard", "menu", "Access to dashboard overview", "dashboard"),
        (menu::ADMIN, "View Administration", "menu", "Access to admin section", "admin"),
        (menu::USERS, "View Users", "menu", "Access to user management", "users"),
        (menu::CATALOG, "View Catalog", "menu", "Access to service catalog", "catalog"),
        (menu::PLANS, "View Plans", "menu", "Access to client plans", "plans"),
        (menu::DELIVERY, "View Delivery", "menu", "Access to delivery entries", "delivery"),
        (menu::BILLING, "View Billing", "menu", "Access to billing", "billing"),
        (menu::SCORING, "View Scoring", "menu", "Access to quality scoring", "scoring"),
        (menu::REPORTS, "View Reports", "menu", "Access to reports", "reports"),
        (menu::AUDIT, "View Audit Log", "menu", "Access to audit trail", "audit"),

        // Action permissions
        (action::CREATE_USER, "Create User", "action", "Create new users", "users"),
        (action::EDIT_USER, "Edit User", "action", "Edit user details", "users"),
        (action::DEACTIVATE_USER, "Deactivate User", "action", "Deactivate user accounts", "users"),
        (action::ASSIGN_ROLE, "Assign Role", "action", "Assign roles to users", "roles"),
        (action::MANAGE_ROLES, "Manage Roles", "action", "Create and edit roles", "roles"),
        (action::MANAGE_PERMISSIONS, "Manage Permissions", "action", "Assign permissions to roles", "permissions"),
        (action::MANAGE_SCOPES, "Manage Scopes", "action", "Manage data scope assignments", "scopes"),
        (action::MANAGE_ORG, "Manage Organization", "action", "Create and edit organizations", "org"),
        (action::MANAGE_DEPT, "Manage Departments", "action", "Create and edit departments", "departments"),
        (action::MANAGE_PROJECT, "Manage Projects", "action", "Create and edit projects", "projects"),
        (action::CREATE_SERVICE, "Create Service", "action", "Add services to catalog", "catalog"),
        (action::EDIT_SERVICE, "Edit Service", "action", "Edit catalog services", "catalog"),
        (action::CREATE_PACKAGE, "Create Package", "action", "Create service packages", "packages"),
        (action::CREATE_PLAN, "Create Plan", "action", "Create client plans", "plans"),
        (action::EDIT_PLAN, "Edit Plan", "action", "Edit client plans", "plans"),
        (action::LOG_DELIVERY, "Log Delivery", "action", "Log delivery entries", "delivery"),
        (action::VERIFY_DELIVERY, "Verify Delivery", "action", "Verify delivery entries", "delivery"),
        (action::GENERATE_INVOICE, "Generate Invoice", "action", "Generate billing invoices", "billing"),
        (action::APPROVE_INVOICE, "Approve Invoice", "action", "Approve invoices", "billing"),
        (action::RECORD_PAYMENT, "Record Payment", "action", "Record payments", "payments"),
        (action::PROCESS_REFUND, "Process Refund", "action", "Process refunds", "payments"),
        (action::SUBMIT_SCORE, "Submit Score", "action", "Submit quality scores", "scoring"),
        (action::GENERATE_REPORT, "Generate Report", "action", "Generate reports", "reports"),
        (action::EXPORT_DATA, "Export Data", "action", "Export data", "reports"),

        // API permissions
        (api::USERS_READ, "API: Read Users", "api", "Read user data via API", "users"),
        (api::USERS_WRITE, "API: Write Users", "api", "Create/update users via API", "users"),
        (api::ROLES_READ, "API: Read Roles", "api", "Read role data via API", "roles"),
        (api::ROLES_WRITE, "API: Write Roles", "api", "Create/update roles via API", "roles"),
        (api::PERMISSIONS_READ, "API: Read Permissions", "api", "Read permissions via API", "permissions"),
        (api::ORG_READ, "API: Read Orgs", "api", "Read organization data via API", "org"),
        (api::ORG_WRITE, "API: Write Orgs", "api", "Create/update organizations via API", "org"),
        (api::DEPT_READ, "API: Read Departments", "api", "Read department data via API", "departments"),
        (api::DEPT_WRITE, "API: Write Departments", "api", "Create/update departments via API", "departments"),
        (api::PROJECT_READ, "API: Read Projects", "api", "Read project data via API", "projects"),
        (api::PROJECT_WRITE, "API: Write Projects", "api", "Create/update projects via API", "projects"),
        (api::CATALOG_READ, "API: Read Catalog", "api", "Read catalog via API", "catalog"),
        (api::CATALOG_WRITE, "API: Write Catalog", "api", "Create/update catalog via API", "catalog"),
        (api::PLANS_READ, "API: Read Plans", "api", "Read plans via API", "plans"),
        (api::PLANS_WRITE, "API: Write Plans", "api", "Create/update plans via API", "plans"),
        (api::DELIVERY_READ, "API: Read Delivery", "api", "Read delivery entries via API", "delivery"),
        (api::DELIVERY_WRITE, "API: Write Delivery", "api", "Create/update delivery via API", "delivery"),
        (api::BILLING_READ, "API: Read Billing", "api", "Read billing data via API", "billing"),
        (api::BILLING_WRITE, "API: Write Billing", "api", "Create/update billing via API", "billing"),
        (api::PAYMENTS_READ, "API: Read Payments", "api", "Read payment data via API", "payments"),
        (api::PAYMENTS_WRITE, "API: Write Payments", "api", "Create/update payments via API", "payments"),
        (api::SCORING_READ, "API: Read Scoring", "api", "Read scoring data via API", "scoring"),
        (api::SCORING_WRITE, "API: Write Scoring", "api", "Create/update scoring via API", "scoring"),
        (api::REPORTS_READ, "API: Read Reports", "api", "Read reports via API", "reports"),
        (api::EXPORT_UNMASKED, "API: Export Unmasked", "api", "Export identifiers without masking", "reports"),
        (api::OPS_READ, "API: Read Ops Controls", "api", "Read operational flags and metrics", "ops"),
        (api::OPS_WRITE, "API: Write Ops Controls", "api", "Modify operational flags and degradation toggles", "ops"),
        (api::AUDIT_READ, "API: Read Audit", "api", "Read audit logs via API", "audit"),
    ]
}

/// Role definitions with their assigned permission codes.
/// Used for seeding the default RBAC matrix.
pub fn default_role_permissions() -> Vec<(&'static str, &'static str, Vec<&'static str>)> {
    vec![
        ("System Administrator", "Full system access with all permissions", vec![
            // All permissions (explicit, not a wildcard)
            menu::DASHBOARD, menu::ADMIN, menu::USERS, menu::CATALOG, menu::PLANS,
            menu::DELIVERY, menu::BILLING, menu::SCORING, menu::REPORTS, menu::AUDIT,
            action::CREATE_USER, action::EDIT_USER, action::DEACTIVATE_USER,
            action::ASSIGN_ROLE, action::MANAGE_ROLES, action::MANAGE_PERMISSIONS,
            action::MANAGE_SCOPES, action::MANAGE_ORG, action::MANAGE_DEPT, action::MANAGE_PROJECT,
            action::CREATE_SERVICE, action::EDIT_SERVICE, action::CREATE_PACKAGE,
            action::CREATE_PLAN, action::EDIT_PLAN, action::LOG_DELIVERY, action::VERIFY_DELIVERY,
            action::GENERATE_INVOICE, action::APPROVE_INVOICE, action::RECORD_PAYMENT,
            action::PROCESS_REFUND, action::SUBMIT_SCORE, action::GENERATE_REPORT, action::EXPORT_DATA,
            api::USERS_READ, api::USERS_WRITE, api::ROLES_READ, api::ROLES_WRITE,
            api::PERMISSIONS_READ, api::ORG_READ, api::ORG_WRITE,
            api::DEPT_READ, api::DEPT_WRITE, api::PROJECT_READ, api::PROJECT_WRITE,
            api::CATALOG_READ, api::CATALOG_WRITE, api::PLANS_READ, api::PLANS_WRITE,
            api::DELIVERY_READ, api::DELIVERY_WRITE, api::BILLING_READ, api::BILLING_WRITE,
            api::PAYMENTS_READ, api::PAYMENTS_WRITE, api::SCORING_READ, api::SCORING_WRITE,
            api::REPORTS_READ, api::EXPORT_UNMASKED, api::OPS_READ, api::OPS_WRITE, api::AUDIT_READ,
        ]),
        ("Operations Manager", "Operational oversight across delivery, billing, and quality", vec![
            menu::DASHBOARD, menu::CATALOG, menu::PLANS, menu::DELIVERY, menu::BILLING,
            menu::SCORING, menu::REPORTS, menu::USERS,
            action::CREATE_SERVICE, action::EDIT_SERVICE, action::CREATE_PACKAGE,
            action::CREATE_PLAN, action::EDIT_PLAN, action::VERIFY_DELIVERY,
            action::GENERATE_INVOICE, action::APPROVE_INVOICE, action::GENERATE_REPORT,
            api::USERS_READ, api::CATALOG_READ, api::CATALOG_WRITE, api::PLANS_READ, api::PLANS_WRITE,
            api::DELIVERY_READ, api::DELIVERY_WRITE, api::BILLING_READ, api::BILLING_WRITE,
            api::SCORING_READ, api::REPORTS_READ, api::OPS_READ,
        ]),
        ("Billing Specialist", "Billing and payment processing", vec![
            menu::DASHBOARD, menu::BILLING, menu::PLANS, menu::DELIVERY, menu::REPORTS,
            action::GENERATE_INVOICE, action::APPROVE_INVOICE, action::RECORD_PAYMENT,
            action::PROCESS_REFUND, action::GENERATE_REPORT,
            api::PLANS_READ, api::DELIVERY_READ, api::BILLING_READ, api::BILLING_WRITE,
            api::PAYMENTS_READ, api::PAYMENTS_WRITE, api::REPORTS_READ,
        ]),
        ("Coach/Clinician", "Service delivery logging and plan viewing", vec![
            menu::DASHBOARD, menu::PLANS, menu::DELIVERY, menu::SCORING,
            action::LOG_DELIVERY,
            api::PLANS_READ, api::DELIVERY_READ, api::DELIVERY_WRITE, api::SCORING_READ,
        ]),
        ("QA Reviewer", "Quality scoring and review", vec![
            menu::DASHBOARD, menu::DELIVERY, menu::SCORING, menu::REPORTS,
            action::SUBMIT_SCORE, action::GENERATE_REPORT,
            api::DELIVERY_READ, api::SCORING_READ, api::SCORING_WRITE, api::REPORTS_READ,
        ]),
        ("Auditor", "Read-only audit and reporting access", vec![
            menu::DASHBOARD, menu::AUDIT, menu::REPORTS,
            action::GENERATE_REPORT, action::EXPORT_DATA,
            api::AUDIT_READ, api::REPORTS_READ, api::OPS_READ,
        ]),
    ]
}
