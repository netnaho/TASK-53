use dioxus::prelude::*;
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
