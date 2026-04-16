// Controller-level tests for the audit_api layer.
//
// Covers: audit action constants, AuditEntry field contracts, query parameter
// defaults, permission codes, and error mapping.
// No database, no HTTP server — runs with `cargo test --lib`.

use crate::domain::auth_policy::api;
use crate::domain::error::AppError;
use crate::infrastructure::audit::{AuditEntry, actions};

// ---------------------------------------------------------------------------
// Audit action constants
// ---------------------------------------------------------------------------

#[test]
fn login_success_action_constant_is_correct() {
    assert_eq!(actions::LOGIN_SUCCESS, "auth.login.success");
}

#[test]
fn login_failed_action_constant_is_correct() {
    assert_eq!(actions::LOGIN_FAILED, "auth.login.failed");
}

#[test]
fn logout_action_constant_is_correct() {
    assert_eq!(actions::LOGOUT, "auth.logout");
}

#[test]
fn user_created_action_constant_is_correct() {
    assert_eq!(actions::USER_CREATED, "user.created");
}

#[test]
fn user_updated_action_constant_is_correct() {
    assert_eq!(actions::USER_UPDATED, "user.updated");
}

#[test]
fn role_assigned_action_constant_is_correct() {
    assert_eq!(actions::ROLE_ASSIGNED, "role.assigned");
}

#[test]
fn role_revoked_action_constant_is_correct() {
    assert_eq!(actions::ROLE_REVOKED, "role.revoked");
}

#[test]
fn permission_granted_action_constant_is_correct() {
    assert_eq!(actions::PERMISSION_GRANTED, "permission.granted");
}

#[test]
fn permission_revoked_action_constant_is_correct() {
    assert_eq!(actions::PERMISSION_REVOKED, "permission.revoked");
}

#[test]
fn scope_granted_action_constant_is_correct() {
    assert_eq!(actions::SCOPE_GRANTED, "scope.granted");
}

#[test]
fn scope_revoked_action_constant_is_correct() {
    assert_eq!(actions::SCOPE_REVOKED, "scope.revoked");
}

#[test]
fn org_created_action_constant_is_correct() {
    assert_eq!(actions::ORG_CREATED, "org.created");
}

#[test]
fn config_changed_action_constant_is_correct() {
    assert_eq!(actions::CONFIG_CHANGED, "config.changed");
}

// ---------------------------------------------------------------------------
// Auth-prefixed actions have consistent prefix
// ---------------------------------------------------------------------------

#[test]
fn auth_actions_share_auth_prefix() {
    assert!(actions::LOGIN_SUCCESS.starts_with("auth."));
    assert!(actions::LOGIN_FAILED.starts_with("auth."));
    assert!(actions::LOGOUT.starts_with("auth."));
    assert!(actions::SESSION_EXPIRED.starts_with("auth."));
}

#[test]
fn user_actions_share_user_prefix() {
    assert!(actions::USER_CREATED.starts_with("user."));
    assert!(actions::USER_UPDATED.starts_with("user."));
    assert!(actions::USER_DEACTIVATED.starts_with("user."));
}

#[test]
fn role_actions_share_role_prefix() {
    assert!(actions::ROLE_CREATED.starts_with("role."));
    assert!(actions::ROLE_ASSIGNED.starts_with("role."));
    assert!(actions::ROLE_REVOKED.starts_with("role."));
}

// ---------------------------------------------------------------------------
// AuditEntry struct — field contract
// ---------------------------------------------------------------------------

#[test]
fn audit_entry_minimal_required_fields() {
    let entry = AuditEntry {
        user_id: None,
        action: actions::LOGIN_SUCCESS.to_string(),
        resource_type: "session".to_string(),
        resource_id: None,
        org_id: None,
        details: None,
        ip_address: None,
        trace_id: None,
    };
    assert_eq!(entry.action, "auth.login.success");
    assert_eq!(entry.resource_type, "session");
    assert!(entry.user_id.is_none());
}

#[test]
fn audit_entry_with_all_fields() {
    let entry = AuditEntry {
        user_id: Some("u-1".to_string()),
        action: actions::USER_CREATED.to_string(),
        resource_type: "user".to_string(),
        resource_id: Some("u-new".to_string()),
        org_id: Some("org-1".to_string()),
        details: Some(serde_json::json!({"username": "new_user"})),
        ip_address: Some("192.168.1.1".to_string()),
        trace_id: Some("trace-abc-123".to_string()),
    };
    assert_eq!(entry.user_id.as_deref(), Some("u-1"));
    assert_eq!(entry.org_id.as_deref(), Some("org-1"));
    assert!(entry.details.is_some());
    assert_eq!(entry.ip_address.as_deref(), Some("192.168.1.1"));
}

#[test]
fn audit_entry_details_accepts_json_object() {
    let entry = AuditEntry {
        user_id: Some("u-1".to_string()),
        action: actions::ROLE_ASSIGNED.to_string(),
        resource_type: "user".to_string(),
        resource_id: Some("u-2".to_string()),
        org_id: Some("org-1".to_string()),
        details: Some(serde_json::json!({
            "role_id": "role-admin",
            "role_name": "System Administrator"
        })),
        ip_address: None,
        trace_id: None,
    };
    let details = entry.details.as_ref().unwrap();
    assert_eq!(details["role_id"], "role-admin");
    assert_eq!(details["role_name"], "System Administrator");
}

// ---------------------------------------------------------------------------
// Query parameter defaults (tested as pure logic)
// ---------------------------------------------------------------------------

#[test]
fn audit_query_default_limit_is_50() {
    // The handler uses limit.unwrap_or(50).min(200)
    let limit: Option<i64> = None;
    let effective = limit.unwrap_or(50).min(200);
    assert_eq!(effective, 50);
}

#[test]
fn audit_query_limit_clamps_at_200() {
    let limit: Option<i64> = Some(500);
    let effective = limit.unwrap_or(50).min(200);
    assert_eq!(effective, 200);
}

#[test]
fn audit_query_default_offset_is_0() {
    let offset: Option<i64> = None;
    let effective = offset.unwrap_or(0);
    assert_eq!(effective, 0);
}

// ---------------------------------------------------------------------------
// Authorization codes for audit_api controller
// ---------------------------------------------------------------------------

#[test]
fn audit_read_permission_code_is_correct() {
    assert_eq!(api::AUDIT_READ, "api.audit.read");
}

#[test]
fn audit_read_has_api_prefix() {
    assert!(api::AUDIT_READ.starts_with("api."));
}

// ---------------------------------------------------------------------------
// Error mapping for audit_api controller paths
// ---------------------------------------------------------------------------

#[test]
fn audit_forbidden_maps_to_forbidden() {
    let err = AppError::Forbidden("Missing required permission: api.audit.read".to_string());
    let env = err.envelope();
    assert_eq!(env.error.code, "FORBIDDEN");
    assert!(env.error.message.contains("api.audit.read"));
}

#[test]
fn audit_internal_error_maps_correctly() {
    let err = AppError::Internal("Database query failed".to_string());
    assert_eq!(err.envelope().error.code, "INTERNAL_ERROR");
}
