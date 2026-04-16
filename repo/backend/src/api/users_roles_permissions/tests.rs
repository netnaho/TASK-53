// Controller-level tests for users, roles, and permissions API layer.
//
// Covers: request type serialization contracts, scope assignment validation,
// role assignment constraints, permission code prefix rules, and error mapping.
// No database, no HTTP server — runs with `cargo test --lib`.

use crate::domain::auth_policy::{action, api};
use crate::domain::auth_types::{
    AssignPermissionRequest, AssignRoleRequest, AssignScopeRequest, CreateRoleRequest,
    CreateUserRequest, UpdateUserRequest,
};
use crate::domain::error::AppError;

// ---------------------------------------------------------------------------
// CreateUserRequest
// ---------------------------------------------------------------------------

#[test]
fn create_user_request_deserializes_all_fields() {
    let json = r#"{
        "username": "nurse01",
        "email": "nurse01@example.com",
        "password": "Secure123!",
        "org_id": "org-1"
    }"#;
    let req: CreateUserRequest =
        serde_json::from_str(json).expect("deserialize CreateUserRequest");
    assert_eq!(req.username, "nurse01");
    assert_eq!(req.email, "nurse01@example.com");
    assert_eq!(req.org_id, "org-1");
    assert!(req.department_id.is_none());
}

#[test]
fn create_user_request_with_optional_department() {
    let json = r#"{
        "username": "nurse02",
        "email": "nurse02@example.com",
        "password": "Secure123!",
        "org_id": "org-1",
        "department_id": "dept-2"
    }"#;
    let req: CreateUserRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.department_id.as_deref(), Some("dept-2"));
}

#[test]
fn create_user_request_missing_username_fails() {
    let json = r#"{"email":"x@x.com","password":"p","org_id":"o"}"#;
    let result: Result<CreateUserRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn create_user_request_missing_email_fails() {
    let json = r#"{"username":"u","password":"p","org_id":"o"}"#;
    let result: Result<CreateUserRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// UpdateUserRequest
// ---------------------------------------------------------------------------

#[test]
fn update_user_request_all_optional() {
    let json = r#"{}"#;
    let req: UpdateUserRequest = serde_json::from_str(json).expect("empty update should work");
    assert!(req.email.is_none());
    assert!(req.status.is_none());
    assert!(req.department_id.is_none());
}

#[test]
fn update_user_request_partial_email_only() {
    let json = r#"{"email":"new@example.com"}"#;
    let req: UpdateUserRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.email.as_deref(), Some("new@example.com"));
    assert!(req.status.is_none());
}

#[test]
fn update_user_request_status_inactive() {
    let json = r#"{"status":"inactive"}"#;
    let req: UpdateUserRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.status.as_deref(), Some("inactive"));
}

// ---------------------------------------------------------------------------
// CreateRoleRequest
// ---------------------------------------------------------------------------

#[test]
fn create_role_request_required_name() {
    let json = r#"{"name":"QA Reviewer"}"#;
    let req: CreateRoleRequest = serde_json::from_str(json).expect("deserialize CreateRoleRequest");
    assert_eq!(req.name, "QA Reviewer");
    assert!(req.description.is_none());
}

#[test]
fn create_role_request_with_description() {
    let json = r#"{"name":"Billing Lead","description":"Manages all billing operations"}"#;
    let req: CreateRoleRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.description.as_deref(), Some("Manages all billing operations"));
}

#[test]
fn create_role_request_missing_name_fails() {
    let json = r#"{"description":"No name"}"#;
    let result: Result<CreateRoleRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// AssignRoleRequest
// ---------------------------------------------------------------------------

#[test]
fn assign_role_request_deserializes() {
    let json = r#"{"role_id":"role-abc-123"}"#;
    let req: AssignRoleRequest = serde_json::from_str(json).expect("deserialize AssignRoleRequest");
    assert_eq!(req.role_id, "role-abc-123");
}

#[test]
fn assign_role_request_missing_role_id_fails() {
    let json = r#"{}"#;
    let result: Result<AssignRoleRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// AssignPermissionRequest
// ---------------------------------------------------------------------------

#[test]
fn assign_permission_request_deserializes() {
    let json = r#"{"permission_id":"perm-xyz-456"}"#;
    let req: AssignPermissionRequest =
        serde_json::from_str(json).expect("deserialize AssignPermissionRequest");
    assert_eq!(req.permission_id, "perm-xyz-456");
}

#[test]
fn assign_permission_request_missing_id_fails() {
    let json = r#"{}"#;
    let result: Result<AssignPermissionRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// AssignScopeRequest
// ---------------------------------------------------------------------------

#[test]
fn assign_scope_request_org_level_read() {
    let json = r#"{"org_id":"org-1","access_level":"read"}"#;
    let req: AssignScopeRequest =
        serde_json::from_str(json).expect("deserialize AssignScopeRequest");
    assert_eq!(req.org_id, "org-1");
    assert_eq!(req.access_level, "read");
    assert!(req.department_id.is_none());
    assert!(req.project_id.is_none());
}

#[test]
fn assign_scope_request_dept_level_write() {
    let json = r#"{"org_id":"org-2","department_id":"dept-3","access_level":"write"}"#;
    let req: AssignScopeRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.department_id.as_deref(), Some("dept-3"));
    assert_eq!(req.access_level, "write");
}

#[test]
fn assign_scope_request_project_level_admin() {
    let json = r#"{"org_id":"org-3","department_id":"dept-4","project_id":"proj-5","access_level":"admin"}"#;
    let req: AssignScopeRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.project_id.as_deref(), Some("proj-5"));
    assert_eq!(req.access_level, "admin");
}

#[test]
fn assign_scope_request_missing_org_id_fails() {
    let json = r#"{"access_level":"read"}"#;
    let result: Result<AssignScopeRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Authorization boundary: permission codes used by this controller
// ---------------------------------------------------------------------------

#[test]
fn users_read_permission_is_correct_code() {
    assert_eq!(api::USERS_READ, "api.users.read");
}

#[test]
fn users_write_permission_is_correct_code() {
    assert_eq!(api::USERS_WRITE, "api.users.write");
}

#[test]
fn roles_read_permission_is_correct_code() {
    assert_eq!(api::ROLES_READ, "api.roles.read");
}

#[test]
fn roles_write_permission_is_correct_code() {
    assert_eq!(api::ROLES_WRITE, "api.roles.write");
}

#[test]
fn assign_role_action_is_correct_code() {
    assert_eq!(action::ASSIGN_ROLE, "action.roles.assign");
}

#[test]
fn manage_permissions_action_is_correct_code() {
    assert_eq!(action::MANAGE_PERMISSIONS, "action.permissions.manage");
}

#[test]
fn manage_scopes_action_is_correct_code() {
    assert_eq!(action::MANAGE_SCOPES, "action.scopes.manage");
}

// ---------------------------------------------------------------------------
// Error mapping for user/role/permission controller paths
// ---------------------------------------------------------------------------

#[test]
fn user_not_found_maps_to_not_found() {
    let err = AppError::NotFound("User not found".to_string());
    assert_eq!(err.envelope().error.code, "NOT_FOUND");
}

#[test]
fn duplicate_username_maps_to_conflict() {
    let err = AppError::Conflict("Username already taken".to_string());
    assert_eq!(err.envelope().error.code, "CONFLICT");
}

#[test]
fn missing_role_maps_to_not_found() {
    let err = AppError::NotFound("Role not found".to_string());
    let env = err.envelope();
    assert_eq!(env.error.code, "NOT_FOUND");
    assert!(env.error.message.contains("Role not found"));
}

#[test]
fn cross_org_scope_violation_maps_to_forbidden() {
    let err = AppError::Forbidden("Data scope check failed".to_string());
    assert_eq!(err.envelope().error.code, "FORBIDDEN");
}
