// Controller-level tests for the admin_org API layer.
//
// Covers: request type deserialization, org/department/project field contracts,
// and error-code mapping for common failure paths.
// No database, no HTTP server — runs with `cargo test --lib`.

use crate::domain::auth_types::{
    CreateDepartmentRequest, CreateOrgRequest, CreateProjectRequest, UpdateOrgRequest,
};
use crate::domain::error::AppError;

// ---------------------------------------------------------------------------
// CreateOrgRequest
// ---------------------------------------------------------------------------

#[test]
fn create_org_request_deserializes_name_field() {
    let json = r#"{"name":"Sunrise Care Group"}"#;
    let req: CreateOrgRequest = serde_json::from_str(json).expect("deserialize CreateOrgRequest");
    assert_eq!(req.name, "Sunrise Care Group");
}

#[test]
fn create_org_request_missing_name_fails() {
    let json = r#"{}"#;
    let result: Result<CreateOrgRequest, _> = serde_json::from_str(json);
    assert!(result.is_err(), "missing 'name' should fail deserialization");
}

// ---------------------------------------------------------------------------
// UpdateOrgRequest
// ---------------------------------------------------------------------------

#[test]
fn update_org_request_all_optional_fields() {
    let json = r#"{"name":"Updated Name","status":"active"}"#;
    let req: UpdateOrgRequest =
        serde_json::from_str(json).expect("deserialize UpdateOrgRequest");
    assert_eq!(req.name.as_deref(), Some("Updated Name"));
    assert_eq!(req.status.as_deref(), Some("active"));
}

#[test]
fn update_org_request_empty_object_is_valid() {
    // All fields are Option, so an empty JSON object should be valid (no-op update)
    let json = r#"{}"#;
    let req: UpdateOrgRequest =
        serde_json::from_str(json).expect("empty UpdateOrgRequest should deserialize");
    assert!(req.name.is_none());
    assert!(req.status.is_none());
}

#[test]
fn update_org_request_partial_update_name_only() {
    let json = r#"{"name":"New Name"}"#;
    let req: UpdateOrgRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.name.as_deref(), Some("New Name"));
    assert!(req.status.is_none());
}

#[test]
fn update_org_request_partial_update_status_only() {
    let json = r#"{"status":"inactive"}"#;
    let req: UpdateOrgRequest = serde_json::from_str(json).unwrap();
    assert!(req.name.is_none());
    assert_eq!(req.status.as_deref(), Some("inactive"));
}

// ---------------------------------------------------------------------------
// CreateDepartmentRequest
// ---------------------------------------------------------------------------

#[test]
fn create_department_request_deserializes() {
    let json = r#"{"name":"Nursing Division","org_id":"org-abc"}"#;
    let req: CreateDepartmentRequest =
        serde_json::from_str(json).expect("deserialize CreateDepartmentRequest");
    assert_eq!(req.name, "Nursing Division");
    assert_eq!(req.org_id, "org-abc");
}

#[test]
fn create_department_request_missing_name_fails() {
    let json = r#"{"org_id":"org-abc"}"#;
    let result: Result<CreateDepartmentRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn create_department_request_missing_org_id_fails() {
    let json = r#"{"name":"Rehab Division"}"#;
    let result: Result<CreateDepartmentRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// CreateProjectRequest
// ---------------------------------------------------------------------------

#[test]
fn create_project_request_deserializes_without_department() {
    let json = r#"{"name":"Q1 Expansion","org_id":"org-1"}"#;
    let req: CreateProjectRequest =
        serde_json::from_str(json).expect("deserialize CreateProjectRequest");
    assert_eq!(req.name, "Q1 Expansion");
    assert_eq!(req.org_id, "org-1");
    assert!(req.department_id.is_none());
}

#[test]
fn create_project_request_deserializes_with_department() {
    let json = r#"{"name":"Q2 Project","org_id":"org-2","department_id":"dept-5"}"#;
    let req: CreateProjectRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.department_id.as_deref(), Some("dept-5"));
}

#[test]
fn create_project_request_missing_required_fields_fails() {
    // missing org_id
    let json = r#"{"name":"Orphan Project"}"#;
    let result: Result<CreateProjectRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// AppError mapping for org/dept/project failures
// ---------------------------------------------------------------------------

#[test]
fn org_not_found_maps_to_not_found_code() {
    let err = AppError::NotFound("Org not found".to_string());
    let env = err.envelope();
    assert_eq!(env.error.code, "NOT_FOUND");
    assert!(env.error.message.contains("Org not found"));
}

#[test]
fn duplicate_org_name_maps_to_conflict_code() {
    let err = AppError::Conflict("Org name already exists".to_string());
    let env = err.envelope();
    assert_eq!(env.error.code, "CONFLICT");
}

#[test]
fn forbidden_access_maps_to_forbidden_code() {
    let err = AppError::Forbidden("Insufficient scope".to_string());
    let env = err.envelope();
    assert_eq!(env.error.code, "FORBIDDEN");
}

#[test]
fn invalid_org_data_maps_to_bad_request_code() {
    let err = AppError::BadRequest("Name cannot be empty".to_string());
    let env = err.envelope();
    assert_eq!(env.error.code, "BAD_REQUEST");
}
