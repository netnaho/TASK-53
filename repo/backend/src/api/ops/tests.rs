// Controller-level tests for the ops controls API layer.
//
// Covers: degradation toggle key constants, OpsFlag serialization, known
// toggle validation, permission code contracts, and error mapping.
// No database, no HTTP server — runs with `cargo test --lib`.

use crate::application::degradation_service::{KNOWN_TOGGLES, OpsFlag, TOGGLE_ANALYTICS, TOGGLE_EXPORTS};
use crate::domain::auth_policy::api;
use crate::domain::error::AppError;

// ---------------------------------------------------------------------------
// Degradation toggle key constants
// ---------------------------------------------------------------------------

#[test]
fn toggle_exports_key_name_is_correct() {
    assert_eq!(TOGGLE_EXPORTS, "exports_enabled");
}

#[test]
fn toggle_analytics_key_name_is_correct() {
    assert_eq!(TOGGLE_ANALYTICS, "analytics_enabled");
}

#[test]
fn known_toggles_contains_both_keys() {
    assert!(
        KNOWN_TOGGLES.contains(&"exports_enabled"),
        "KNOWN_TOGGLES must include exports_enabled"
    );
    assert!(
        KNOWN_TOGGLES.contains(&"analytics_enabled"),
        "KNOWN_TOGGLES must include analytics_enabled"
    );
}

#[test]
fn known_toggles_has_exactly_two_entries() {
    assert_eq!(KNOWN_TOGGLES.len(), 2, "exactly 2 known toggles: exports and analytics");
}

// ---------------------------------------------------------------------------
// OpsFlag serialization
// ---------------------------------------------------------------------------

#[test]
fn ops_flag_serializes_correctly() {
    let flag = OpsFlag {
        key_name: "exports_enabled".to_string(),
        value: true,
        updated_by: "admin-user-id".to_string(),
        updated_at: "2024-03-01T12:00:00".to_string(),
    };
    let json = serde_json::to_string(&flag).expect("serialize OpsFlag");
    assert!(json.contains("\"key_name\""));
    assert!(json.contains("\"exports_enabled\""));
    assert!(json.contains("\"value\""));
    assert!(json.contains("true"));
}

#[test]
fn ops_flag_deserializes_correctly() {
    let json = r#"{
        "key_name": "analytics_enabled",
        "value": false,
        "updated_by": "sys-user",
        "updated_at": "2024-03-10T08:30:00"
    }"#;
    let flag: OpsFlag = serde_json::from_str(json).expect("deserialize OpsFlag");
    assert_eq!(flag.key_name, "analytics_enabled");
    assert!(!flag.value);
    assert_eq!(flag.updated_by, "sys-user");
}

#[test]
fn ops_flag_enabled_value_true() {
    let flag = OpsFlag {
        key_name: TOGGLE_EXPORTS.to_string(),
        value: true,
        updated_by: "u1".to_string(),
        updated_at: "2024-01-01".to_string(),
    };
    assert!(flag.value, "enabled flag should have value=true");
}

#[test]
fn ops_flag_disabled_value_false() {
    let flag = OpsFlag {
        key_name: TOGGLE_ANALYTICS.to_string(),
        value: false,
        updated_by: "u1".to_string(),
        updated_at: "2024-01-01".to_string(),
    };
    assert!(!flag.value, "disabled flag should have value=false");
}

// ---------------------------------------------------------------------------
// Authorization codes for ops controller
// ---------------------------------------------------------------------------

#[test]
fn ops_read_permission_code_is_correct() {
    assert_eq!(api::OPS_READ, "api.ops.read");
}

#[test]
fn ops_write_permission_code_is_correct() {
    assert_eq!(api::OPS_WRITE, "api.ops.write");
}

// ---------------------------------------------------------------------------
// Error mapping for ops controller paths
// ---------------------------------------------------------------------------

#[test]
fn unknown_flag_key_maps_to_bad_request() {
    let err = AppError::BadRequest("Unknown flag key: 'nonexistent_flag'".to_string());
    let env = err.envelope();
    assert_eq!(env.error.code, "BAD_REQUEST");
    assert!(env.error.message.contains("nonexistent_flag"));
}

#[test]
fn ops_write_forbidden_for_non_admin_maps_to_forbidden() {
    let err = AppError::Forbidden("Missing required permission: api.ops.write".to_string());
    let env = err.envelope();
    assert_eq!(env.error.code, "FORBIDDEN");
    assert!(env.error.message.contains("api.ops.write"));
}

#[test]
fn ops_read_forbidden_maps_to_forbidden() {
    let err = AppError::Forbidden("Missing required permission: api.ops.read".to_string());
    assert_eq!(err.envelope().error.code, "FORBIDDEN");
}
