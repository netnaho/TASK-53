// Controller-level unit tests for the auth API layer.
//
// These tests verify:
//   - request/response type serialization contracts
//   - error domain type → HTTP status mapping
//   - permission-code constants used by auth guards
//
// No database, no HTTP server, no Rocket client needed — these run with
// `cargo test --lib` and complete in milliseconds.

use crate::domain::auth_types::{LoginRequest, LoginResponse, UserProfile};
use crate::domain::error::AppError;

// ---------------------------------------------------------------------------
// LoginRequest — input deserialization
// ---------------------------------------------------------------------------

#[test]
fn login_request_deserializes_from_json() {
    let json = r#"{"username":"admin","password":"Admin123!"}"#;
    let req: LoginRequest =
        serde_json::from_str(json).expect("LoginRequest should deserialize from valid JSON");
    assert_eq!(req.username, "admin");
    assert_eq!(req.password, "Admin123!");
}

#[test]
fn login_request_missing_password_fails_deserialization() {
    let json = r#"{"username":"admin"}"#;
    let result: Result<LoginRequest, _> = serde_json::from_str(json);
    assert!(result.is_err(), "missing password field should fail deserialization");
}

#[test]
fn login_request_missing_username_fails_deserialization() {
    let json = r#"{"password":"Admin123!"}"#;
    let result: Result<LoginRequest, _> = serde_json::from_str(json);
    assert!(result.is_err(), "missing username field should fail deserialization");
}

// ---------------------------------------------------------------------------
// LoginResponse — output serialization
// ---------------------------------------------------------------------------

#[test]
fn login_response_serializes_token_field() {
    let resp = LoginResponse {
        token: "eyJ.test.jwt".to_string(),
        user: UserProfile {
            id: "u-1".to_string(),
            org_id: "o-1".to_string(),
            department_id: None,
            username: "admin".to_string(),
            email: "admin@example.com".to_string(),
            status: "active".to_string(),
            roles: vec!["System Administrator".to_string()],
            permissions: vec!["menu.dashboard".to_string()],
        },
    };
    let json = serde_json::to_string(&resp).expect("LoginResponse should serialize");
    assert!(json.contains("\"token\""), "serialized response must have 'token' key");
    assert!(json.contains("eyJ.test.jwt"), "serialized response must contain token value");
    assert!(json.contains("\"user\""), "serialized response must have 'user' key");
}

// ---------------------------------------------------------------------------
// UserProfile — structure and field access
// ---------------------------------------------------------------------------

#[test]
fn user_profile_fields_accessible() {
    let profile = UserProfile {
        id: "u-2".to_string(),
        org_id: "o-2".to_string(),
        department_id: Some("dept-1".to_string()),
        username: "coach".to_string(),
        email: "coach@example.com".to_string(),
        status: "active".to_string(),
        roles: vec!["Coach/Clinician".to_string()],
        permissions: vec!["api.delivery.read".to_string(), "api.delivery.write".to_string()],
    };
    assert_eq!(profile.username, "coach");
    assert_eq!(profile.permissions.len(), 2);
    assert!(profile.department_id.is_some());
    assert!(profile.roles.contains(&"Coach/Clinician".to_string()));
}

#[test]
fn user_profile_optional_department_can_be_none() {
    let profile = UserProfile {
        id: "u-3".to_string(),
        org_id: "o-1".to_string(),
        department_id: None,
        username: "auditor".to_string(),
        email: "auditor@example.com".to_string(),
        status: "active".to_string(),
        roles: vec![],
        permissions: vec![],
    };
    assert!(profile.department_id.is_none());
    assert!(profile.permissions.is_empty());
}

// ---------------------------------------------------------------------------
// AppError — HTTP status and error code mapping
// ---------------------------------------------------------------------------

#[test]
fn app_error_not_found_envelope_has_not_found_code() {
    let err = AppError::NotFound("user not found".to_string());
    let env = err.envelope();
    assert_eq!(env.error.code, "NOT_FOUND");
    assert!(!env.error.trace_id.is_empty());
}

#[test]
fn app_error_unauthorized_envelope_has_unauthorized_code() {
    let err = AppError::Unauthorized("invalid token".to_string());
    let env = err.envelope();
    assert_eq!(env.error.code, "UNAUTHORIZED");
}

#[test]
fn app_error_forbidden_envelope_has_forbidden_code() {
    let err = AppError::Forbidden("insufficient permissions".to_string());
    let env = err.envelope();
    assert_eq!(env.error.code, "FORBIDDEN");
}

#[test]
fn app_error_bad_request_envelope_has_bad_request_code() {
    let err = AppError::BadRequest("invalid field".to_string());
    let env = err.envelope();
    assert_eq!(env.error.code, "BAD_REQUEST");
}

#[test]
fn app_error_conflict_envelope_has_conflict_code() {
    let err = AppError::Conflict("duplicate key".to_string());
    let env = err.envelope();
    assert_eq!(env.error.code, "CONFLICT");
}

#[test]
fn app_error_service_unavailable_envelope_code() {
    let err = AppError::ServiceUnavailable("exports disabled".to_string());
    let env = err.envelope();
    assert_eq!(env.error.code, "SERVICE_UNAVAILABLE");
}

#[test]
fn app_error_envelope_message_contains_error_detail() {
    let detail = "org not found: org-99";
    let err = AppError::NotFound(detail.to_string());
    let env = err.envelope();
    assert!(
        env.error.message.contains(detail),
        "envelope message '{}' should contain '{}'",
        env.error.message,
        detail
    );
}

#[test]
fn app_error_envelope_trace_id_is_valid_uuid_format() {
    let err = AppError::Internal("db error".to_string());
    let env = err.envelope();
    // UUID v4 format: 8-4-4-4-12 hex digits with hyphens, 36 chars total
    assert_eq!(env.error.trace_id.len(), 36);
    let parts: Vec<&str> = env.error.trace_id.split('-').collect();
    assert_eq!(parts.len(), 5, "trace_id should be a hyphen-separated UUID");
}

// ---------------------------------------------------------------------------
// Auth policy constants — verify permission code strings are well-formed
// ---------------------------------------------------------------------------

#[test]
fn auth_policy_permission_codes_have_expected_prefixes() {
    use crate::domain::auth_policy::{action, api};

    // API-level read/write permissions use "api." prefix
    assert!(api::USERS_READ.starts_with("api."), "USERS_READ should start with 'api.'");
    assert!(api::BILLING_READ.starts_with("api."), "BILLING_READ should start with 'api.'");
    assert!(api::ROLES_READ.starts_with("api."), "ROLES_READ should start with 'api.'");

    // Action-level permissions use "action." prefix
    assert!(action::ASSIGN_ROLE.starts_with("action."), "ASSIGN_ROLE should start with 'action.'");
    assert!(
        action::MANAGE_PERMISSIONS.starts_with("action."),
        "MANAGE_PERMISSIONS should start with 'action.'"
    );
}

#[test]
fn auth_policy_permission_codes_are_nonempty() {
    use crate::domain::auth_policy::{action, api};
    assert!(!api::USERS_READ.is_empty());
    assert!(!api::USERS_WRITE.is_empty());
    assert!(!api::BILLING_READ.is_empty());
    assert!(!api::CATALOG_READ.is_empty());
    assert!(!api::PLANS_READ.is_empty());
    assert!(!action::ASSIGN_ROLE.is_empty());
    assert!(!action::MANAGE_PERMISSIONS.is_empty());
}
