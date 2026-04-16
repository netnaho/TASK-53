// Frontend unit tests: app shell, router variants, and layout integration.
//
// These tests target pure-Rust logic in app.rs, router.rs, and layouts —
// no browser APIs or DOM rendering needed.

use crate::models::UserProfile;
use crate::router::Route;
use crate::state::AuthState;

// ---------------------------------------------------------------------------
// Router: Route enum variant construction
// ---------------------------------------------------------------------------

#[test]
fn route_login_variant_constructible() {
    let route = Route::Login {};
    // If this compiles and runs, the Login route variant exists.
    // We verify by matching.
    assert!(matches!(route, Route::Login {}));
}

#[test]
fn route_not_found_carries_segments() {
    let route = Route::NotFound {
        segments: vec!["unknown".to_string(), "path".to_string()],
    };
    match route {
        Route::NotFound { segments } => {
            assert_eq!(segments.len(), 2);
            assert_eq!(segments[0], "unknown");
        }
        _ => panic!("Expected NotFound variant"),
    }
}

#[test]
fn route_dashboard_variant_constructible() {
    let route = Route::Dashboard {};
    assert!(matches!(route, Route::Dashboard {}));
}

#[test]
fn route_admin_variant_constructible() {
    let route = Route::Admin {};
    assert!(matches!(route, Route::Admin {}));
}

#[test]
fn route_users_variant_constructible() {
    let route = Route::Users {};
    assert!(matches!(route, Route::Users {}));
}

// ---------------------------------------------------------------------------
// Auth state: unauthenticated state blocks all permissions
// ---------------------------------------------------------------------------

#[test]
fn auth_state_default_blocks_all_permissions() {
    let state = AuthState::default();
    assert!(!state.is_authenticated());
    assert!(!state.has_permission("menu.dashboard"));
    assert!(!state.has_permission("api.billing.read"));
    assert!(!state.has_any_permission(&["api.scoring.write", "api.reports.read"]));
    assert!(!state.has_role("System Administrator"));
}

#[test]
fn auth_state_with_admin_perms_passes_all_checks() {
    let state = AuthState {
        token: Some("test-token".to_string()),
        user: Some(UserProfile {
            id: "u-admin".to_string(),
            org_id: "o-1".to_string(),
            department_id: None,
            username: "admin".to_string(),
            email: "admin@example.com".to_string(),
            status: "active".to_string(),
            roles: vec!["System Administrator".to_string()],
            permissions: vec![
                "menu.dashboard".to_string(),
                "menu.admin".to_string(),
                "api.billing.read".to_string(),
                "api.billing.write".to_string(),
            ],
        }),
    };
    assert!(state.is_authenticated());
    assert!(state.has_permission("menu.dashboard"));
    assert!(state.has_permission("api.billing.read"));
    assert!(state.has_role("System Administrator"));
    assert!(!state.has_role("Auditor"));
}

// ---------------------------------------------------------------------------
// App module: verify module imports compile
// ---------------------------------------------------------------------------

#[test]
fn router_module_is_importable() {
    // If this test function compiles, the router module is correctly declared.
    // The Route type used above proves the import works.
    let _: Option<Route> = None;
}

#[test]
fn state_module_is_importable() {
    let _: Option<AuthState> = None;
}
