use crate::models::UserProfile;

/// Global authentication state signal.
/// Stored in context at the App root.
#[derive(Debug, Clone, Default)]
pub struct AuthState {
    pub token: Option<String>,
    pub user: Option<UserProfile>,
}

impl AuthState {
    pub fn is_authenticated(&self) -> bool {
        self.token.is_some() && self.user.is_some()
    }

    pub fn has_permission(&self, code: &str) -> bool {
        self.user
            .as_ref()
            .map(|u| u.permissions.iter().any(|p| p == code))
            .unwrap_or(false)
    }

    pub fn has_any_permission(&self, codes: &[&str]) -> bool {
        codes.iter().any(|c| self.has_permission(c))
    }

    pub fn has_role(&self, role_name: &str) -> bool {
        self.user
            .as_ref()
            .map(|u| u.roles.iter().any(|r| r == role_name))
            .unwrap_or(false)
    }

    /// Display username for topbar.  Returns empty string when unauthenticated.
    pub fn display_username(&self) -> String {
        self.user
            .as_ref()
            .map(|u| u.username.clone())
            .unwrap_or_default()
    }

    /// Display roles as comma-separated string for topbar.
    pub fn display_roles(&self) -> String {
        self.user
            .as_ref()
            .map(|u| u.roles.join(", "))
            .unwrap_or_default()
    }

    /// Returns list of sidebar nav items (label, permission) the user is allowed to see.
    /// Mirrors the permission gates in `components/sidebar.rs`.
    pub fn visible_nav_items(&self) -> Vec<&'static str> {
        use crate::state::perms;
        let mut items = Vec::new();
        let checks: &[(&str, &str)] = &[
            ("Dashboard",        perms::MENU_DASHBOARD),
            ("Service Catalog",  perms::MENU_CATALOG),
            ("Client Plans",     perms::MENU_PLANS),
            ("Service Delivery", perms::MENU_DELIVERY),
            ("Billing",          perms::MENU_BILLING),
            ("Quality Scoring",  perms::MENU_SCORING),
            ("Reports",          perms::MENU_REPORTS),
            ("Audit Log",        perms::MENU_AUDIT),
            ("Administration",   perms::MENU_ADMIN),
            ("User Management",  perms::MENU_USERS),
            ("Ops Controls",     perms::API_OPS_READ),
        ];
        for (label, perm) in checks {
            if self.has_permission(perm) {
                items.push(*label);
            }
        }
        items
    }
}

/// Permission code constants matching backend auth_policy.
pub mod perms {
    pub const MENU_DASHBOARD: &str = "menu.dashboard";
    pub const MENU_ADMIN: &str = "menu.admin";
    pub const MENU_USERS: &str = "menu.users";
    pub const MENU_CATALOG: &str = "menu.catalog";
    pub const MENU_PLANS: &str = "menu.plans";
    pub const MENU_DELIVERY: &str = "menu.delivery";
    pub const MENU_BILLING: &str = "menu.billing";
    pub const MENU_SCORING: &str = "menu.scoring";
    pub const MENU_REPORTS: &str = "menu.reports";
    pub const MENU_AUDIT: &str = "menu.audit";

    pub const ACTION_CREATE_USER: &str = "action.users.create";
    pub const ACTION_MANAGE_ROLES: &str = "action.roles.manage";
    pub const ACTION_MANAGE_SCOPES: &str = "action.scopes.manage";
    pub const ACTION_MANAGE_ORG: &str = "action.org.manage";
    pub const ACTION_MANAGE_DEPT: &str = "action.dept.manage";
    pub const ACTION_MANAGE_PROJECT: &str = "action.project.manage";

    pub const API_OPS_READ: &str = "api.ops.read";
    pub const API_OPS_WRITE: &str = "api.ops.write";
}

#[cfg(test)]
mod state_test;
