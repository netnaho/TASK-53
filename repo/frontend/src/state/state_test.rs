use crate::models::UserProfile;
use crate::state::{perms, AuthState};

// (Extended with display_username, display_roles, visible_nav_items tests below)

fn make_user(permissions: Vec<&str>, roles: Vec<&str>) -> UserProfile {
    UserProfile {
        id: "user-1".to_string(),
        org_id: "org-1".to_string(),
        department_id: None,
        username: "testuser".to_string(),
        email: "test@example.com".to_string(),
        status: "active".to_string(),
        roles: roles.into_iter().map(String::from).collect(),
        permissions: permissions.into_iter().map(String::from).collect(),
    }
}

#[test]
fn unauthenticated_by_default() {
    let state = AuthState::default();
    assert!(!state.is_authenticated());
}

#[test]
fn authenticated_when_token_and_user_set() {
    let state = AuthState {
        token: Some("tok".to_string()),
        user: Some(make_user(vec![], vec![])),
    };
    assert!(state.is_authenticated());
}

#[test]
fn not_authenticated_when_only_token_set() {
    let state = AuthState {
        token: Some("tok".to_string()),
        user: None,
    };
    assert!(!state.is_authenticated());
}

#[test]
fn not_authenticated_when_only_user_set() {
    let state = AuthState {
        token: None,
        user: Some(make_user(vec![], vec![])),
    };
    assert!(!state.is_authenticated());
}

#[test]
fn has_permission_false_when_unauthenticated() {
    let state = AuthState::default();
    assert!(!state.has_permission("api.billing.read"));
}

#[test]
fn has_permission_true_when_permission_present() {
    let state = AuthState {
        token: Some("tok".to_string()),
        user: Some(make_user(vec!["api.billing.read", "menu.dashboard"], vec![])),
    };
    assert!(state.has_permission("api.billing.read"));
    assert!(state.has_permission("menu.dashboard"));
}

#[test]
fn has_permission_false_when_permission_absent() {
    let state = AuthState {
        token: Some("tok".to_string()),
        user: Some(make_user(vec!["api.billing.read"], vec![])),
    };
    assert!(!state.has_permission("api.scoring.write"));
}

#[test]
fn has_any_permission_returns_true_on_first_match() {
    let state = AuthState {
        token: Some("tok".to_string()),
        user: Some(make_user(vec!["api.reports.read"], vec![])),
    };
    assert!(state.has_any_permission(&["api.scoring.write", "api.reports.read"]));
}

#[test]
fn has_any_permission_returns_false_when_none_match() {
    let state = AuthState {
        token: Some("tok".to_string()),
        user: Some(make_user(vec!["api.billing.read"], vec![])),
    };
    assert!(!state.has_any_permission(&["api.ops.write", "api.scoring.write"]));
}

#[test]
fn has_role_returns_true_when_role_present() {
    let state = AuthState {
        token: Some("tok".to_string()),
        user: Some(make_user(vec![], vec!["System Administrator"])),
    };
    assert!(state.has_role("System Administrator"));
}

#[test]
fn has_role_returns_false_when_role_absent() {
    let state = AuthState {
        token: Some("tok".to_string()),
        user: Some(make_user(vec![], vec!["Auditor"])),
    };
    assert!(!state.has_role("System Administrator"));
}

#[test]
fn perm_constants_are_nonempty_strings() {
    assert!(!perms::MENU_DASHBOARD.is_empty());
    assert!(!perms::MENU_ADMIN.is_empty());
    assert!(!perms::MENU_BILLING.is_empty());
    assert!(!perms::ACTION_CREATE_USER.is_empty());
    assert!(!perms::API_OPS_READ.is_empty());
    assert!(!perms::API_OPS_WRITE.is_empty());
}

#[test]
fn perm_constants_have_expected_prefixes() {
    assert!(perms::MENU_DASHBOARD.starts_with("menu."));
    assert!(perms::MENU_ADMIN.starts_with("menu."));
    assert!(perms::ACTION_CREATE_USER.starts_with("action."));
    assert!(perms::API_OPS_READ.starts_with("api."));
}

// ---------------------------------------------------------------------------
// display_username — topbar logic
// ---------------------------------------------------------------------------

#[test]
fn display_username_returns_empty_when_unauthenticated() {
    let state = AuthState::default();
    assert_eq!(state.display_username(), "");
}

#[test]
fn display_username_returns_username_when_authenticated() {
    let state = AuthState {
        token: Some("tok".to_string()),
        user: Some(make_user(vec![], vec![])),
    };
    assert_eq!(state.display_username(), "testuser");
}

// ---------------------------------------------------------------------------
// display_roles — topbar logic
// ---------------------------------------------------------------------------

#[test]
fn display_roles_returns_empty_when_unauthenticated() {
    let state = AuthState::default();
    assert_eq!(state.display_roles(), "");
}

#[test]
fn display_roles_single_role() {
    let state = AuthState {
        token: Some("tok".to_string()),
        user: Some(make_user(vec![], vec!["System Administrator"])),
    };
    assert_eq!(state.display_roles(), "System Administrator");
}

#[test]
fn display_roles_multiple_roles_comma_separated() {
    let state = AuthState {
        token: Some("tok".to_string()),
        user: Some(make_user(vec![], vec!["Billing Specialist", "Auditor"])),
    };
    let roles = state.display_roles();
    assert!(roles.contains("Billing Specialist"));
    assert!(roles.contains("Auditor"));
    assert!(roles.contains(", "));
}

// ---------------------------------------------------------------------------
// visible_nav_items — sidebar logic
// ---------------------------------------------------------------------------

#[test]
fn visible_nav_items_empty_for_unauthenticated() {
    let state = AuthState::default();
    assert!(state.visible_nav_items().is_empty());
}

#[test]
fn visible_nav_items_empty_for_user_with_no_permissions() {
    let state = AuthState {
        token: Some("tok".to_string()),
        user: Some(make_user(vec![], vec![])),
    };
    assert!(state.visible_nav_items().is_empty());
}

#[test]
fn visible_nav_items_dashboard_requires_menu_dashboard() {
    let state = AuthState {
        token: Some("tok".to_string()),
        user: Some(make_user(vec!["menu.dashboard"], vec![])),
    };
    let items = state.visible_nav_items();
    assert!(items.contains(&"Dashboard"));
    assert!(!items.contains(&"Administration"));
}

#[test]
fn visible_nav_items_admin_user_sees_all_items() {
    let all_perms = vec![
        "menu.dashboard", "menu.catalog", "menu.plans", "menu.delivery",
        "menu.billing", "menu.scoring", "menu.reports", "menu.audit",
        "menu.admin", "menu.users", "api.ops.read",
    ];
    let state = AuthState {
        token: Some("tok".to_string()),
        user: Some(make_user(all_perms, vec!["System Administrator"])),
    };
    let items = state.visible_nav_items();
    assert!(items.contains(&"Dashboard"));
    assert!(items.contains(&"Service Catalog"));
    assert!(items.contains(&"Administration"));
    assert!(items.contains(&"User Management"));
    assert!(items.contains(&"Ops Controls"));
    assert_eq!(items.len(), 11, "admin should see all 11 nav items");
}

#[test]
fn visible_nav_items_auditor_sees_limited_items() {
    // Auditor typically has: menu.audit, menu.reports
    let state = AuthState {
        token: Some("tok".to_string()),
        user: Some(make_user(vec!["menu.audit", "menu.reports"], vec!["Auditor"])),
    };
    let items = state.visible_nav_items();
    assert!(items.contains(&"Audit Log"));
    assert!(items.contains(&"Reports"));
    assert!(!items.contains(&"Administration"));
    assert!(!items.contains(&"User Management"));
    assert!(!items.contains(&"Billing"));
    assert_eq!(items.len(), 2);
}

#[test]
fn visible_nav_items_billing_staff_sees_billing_and_reports() {
    let state = AuthState {
        token: Some("tok".to_string()),
        user: Some(make_user(
            vec!["menu.billing", "menu.reports", "menu.dashboard"],
            vec!["Billing Specialist"],
        )),
    };
    let items = state.visible_nav_items();
    assert!(items.contains(&"Billing"));
    assert!(items.contains(&"Reports"));
    assert!(items.contains(&"Dashboard"));
    assert!(!items.contains(&"Ops Controls"));
}

#[test]
fn visible_nav_items_coach_sees_delivery_and_plans() {
    let state = AuthState {
        token: Some("tok".to_string()),
        user: Some(make_user(
            vec!["menu.delivery", "menu.plans"],
            vec!["Coach/Clinician"],
        )),
    };
    let items = state.visible_nav_items();
    assert!(items.contains(&"Service Delivery"));
    assert!(items.contains(&"Client Plans"));
    assert!(!items.contains(&"Billing"));
    assert!(!items.contains(&"Administration"));
}
