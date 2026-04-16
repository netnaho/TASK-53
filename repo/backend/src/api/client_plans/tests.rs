// Controller-level tests for the client_plans API layer.
//
// Covers: plan/package-assignment request deserialization, status transitions,
// date field handling, permission codes, and error mapping.
// No database, no HTTP server — runs with `cargo test --lib`.

use crate::domain::auth_policy::{action, api};
use crate::domain::catalog_types::{
    AssignPackageRequest, CreateClientPlanRequest, UpdateClientPlanRequest,
};
use crate::domain::error::AppError;

// ---------------------------------------------------------------------------
// CreateClientPlanRequest
// ---------------------------------------------------------------------------

#[test]
fn create_plan_request_minimal_required_fields() {
    let json = r#"{
        "client_name": "Jane Doe",
        "start_date": "2024-01-15"
    }"#;
    let req: CreateClientPlanRequest =
        serde_json::from_str(json).expect("deserialize CreateClientPlanRequest");
    assert_eq!(req.client_name, "Jane Doe");
    assert_eq!(req.start_date, "2024-01-15");
    assert!(req.end_date.is_none());
    assert!(req.department_id.is_none());
    assert!(req.project_id.is_none());
    assert!(req.notes.is_none());
}

#[test]
fn create_plan_request_with_all_optional_fields() {
    let json = r#"{
        "client_name": "John Smith",
        "client_identifier": "CLIENT-001",
        "department_id": "dept-5",
        "project_id": "proj-3",
        "start_date": "2024-02-01",
        "end_date": "2024-12-31",
        "notes": "Long-term care plan"
    }"#;
    let req: CreateClientPlanRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.client_identifier.as_deref(), Some("CLIENT-001"));
    assert_eq!(req.department_id.as_deref(), Some("dept-5"));
    assert_eq!(req.project_id.as_deref(), Some("proj-3"));
    assert_eq!(req.end_date.as_deref(), Some("2024-12-31"));
    assert_eq!(req.notes.as_deref(), Some("Long-term care plan"));
}

#[test]
fn create_plan_request_missing_client_name_fails() {
    let json = r#"{"start_date":"2024-01-01"}"#;
    let result: Result<CreateClientPlanRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn create_plan_request_missing_start_date_fails() {
    let json = r#"{"client_name":"Test Client"}"#;
    let result: Result<CreateClientPlanRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// UpdateClientPlanRequest
// ---------------------------------------------------------------------------

#[test]
fn update_plan_request_all_optional() {
    let json = r#"{}"#;
    let req: UpdateClientPlanRequest = serde_json::from_str(json).unwrap();
    assert!(req.status.is_none());
    assert!(req.end_date.is_none());
    assert!(req.notes.is_none());
}

#[test]
fn update_plan_activate_status() {
    let json = r#"{"status":"active"}"#;
    let req: UpdateClientPlanRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.status.as_deref(), Some("active"));
}

#[test]
fn update_plan_complete_status() {
    let json = r#"{"status":"completed","end_date":"2024-06-30"}"#;
    let req: UpdateClientPlanRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.status.as_deref(), Some("completed"));
    assert_eq!(req.end_date.as_deref(), Some("2024-06-30"));
}

#[test]
fn update_plan_update_notes() {
    let json = r#"{"notes":"Updated care notes"}"#;
    let req: UpdateClientPlanRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.notes.as_deref(), Some("Updated care notes"));
}

// ---------------------------------------------------------------------------
// AssignPackageRequest
// ---------------------------------------------------------------------------

#[test]
fn assign_package_request_required_fields() {
    let json = r#"{
        "package_id": "pkg-abc",
        "effective_date": "2024-03-01"
    }"#;
    let req: AssignPackageRequest =
        serde_json::from_str(json).expect("deserialize AssignPackageRequest");
    assert_eq!(req.package_id, "pkg-abc");
    assert_eq!(req.effective_date, "2024-03-01");
    assert!(req.end_date.is_none());
}

#[test]
fn assign_package_request_with_end_date() {
    let json = r#"{
        "package_id": "pkg-xyz",
        "effective_date": "2024-04-01",
        "end_date": "2024-09-30"
    }"#;
    let req: AssignPackageRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.end_date.as_deref(), Some("2024-09-30"));
}

#[test]
fn assign_package_request_missing_package_id_fails() {
    let json = r#"{"effective_date":"2024-03-01"}"#;
    let result: Result<AssignPackageRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn assign_package_request_missing_effective_date_fails() {
    let json = r#"{"package_id":"pkg-1"}"#;
    let result: Result<AssignPackageRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Plan status values — valid set
// ---------------------------------------------------------------------------

#[test]
fn plan_status_values_are_recognized() {
    // Check that common plan status values can be parsed from JSON
    for status in &["draft", "active", "completed", "suspended"] {
        let json = format!(r#"{{"status":"{}"}}"#, status);
        let req: UpdateClientPlanRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req.status.as_deref(), Some(*status));
    }
}

// ---------------------------------------------------------------------------
// Authorization codes for client_plans controller
// ---------------------------------------------------------------------------

#[test]
fn plans_read_permission_code_is_correct() {
    assert_eq!(api::PLANS_READ, "api.plans.read");
}

#[test]
fn plans_write_permission_code_is_correct() {
    assert_eq!(api::PLANS_WRITE, "api.plans.write");
}

#[test]
fn create_plan_action_code_is_correct() {
    assert_eq!(action::CREATE_PLAN, "action.plans.create");
}

#[test]
fn edit_plan_action_code_is_correct() {
    assert_eq!(action::EDIT_PLAN, "action.plans.edit");
}

// ---------------------------------------------------------------------------
// Error mapping for client_plans controller paths
// ---------------------------------------------------------------------------

#[test]
fn plan_not_found_maps_to_not_found() {
    let err = AppError::NotFound("Plan not found".to_string());
    let env = err.envelope();
    assert_eq!(env.error.code, "NOT_FOUND");
    assert!(env.error.message.contains("Plan not found"));
}

#[test]
fn invalid_plan_dates_map_to_bad_request() {
    let err = AppError::BadRequest("end_date must be after start_date".to_string());
    assert_eq!(err.envelope().error.code, "BAD_REQUEST");
}

#[test]
fn plan_access_forbidden_maps_to_forbidden() {
    let err = AppError::Forbidden("Data scope check failed".to_string());
    assert_eq!(err.envelope().error.code, "FORBIDDEN");
}

#[test]
fn inactive_package_assignment_maps_to_bad_request() {
    let err = AppError::BadRequest("Package is inactive and cannot be assigned".to_string());
    assert_eq!(err.envelope().error.code, "BAD_REQUEST");
}
