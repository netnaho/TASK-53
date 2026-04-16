// Controller-level tests for the reports & exports API layer.
//
// Covers: report filter request deserialization, date range constraints,
// export permission differentiation (masked vs unmasked), and error mapping.
// No database, no HTTP server — runs with `cargo test --lib`.

use crate::domain::auth_policy::{action, api};
use crate::domain::error::AppError;
use crate::domain::scoring_types::ReportFilters;

// ---------------------------------------------------------------------------
// ReportFilters — used by all four report endpoints
// ---------------------------------------------------------------------------

#[test]
fn report_filters_required_date_fields() {
    let json = r#"{"from_date":"2024-01-01","to_date":"2024-03-31"}"#;
    let f: ReportFilters = serde_json::from_str(json).expect("deserialize ReportFilters");
    assert_eq!(f.from_date, "2024-01-01");
    assert_eq!(f.to_date, "2024-03-31");
    assert!(f.department_id.is_none());
    assert!(f.project_id.is_none());
    assert!(f.service_route.is_none());
}

#[test]
fn report_filters_with_optional_scope_fields() {
    let json = r#"{
        "from_date": "2024-04-01",
        "to_date": "2024-06-30",
        "department_id": "dept-2",
        "project_id": "proj-5"
    }"#;
    let f: ReportFilters = serde_json::from_str(json).unwrap();
    assert_eq!(f.department_id.as_deref(), Some("dept-2"));
    assert_eq!(f.project_id.as_deref(), Some("proj-5"));
}

#[test]
fn report_filters_with_service_route() {
    let json = r#"{"from_date":"2024-01-01","to_date":"2024-12-31","service_route":"north-metro"}"#;
    let f: ReportFilters = serde_json::from_str(json).unwrap();
    assert_eq!(f.service_route.as_deref(), Some("north-metro"));
}

#[test]
fn report_filters_with_pagination() {
    let json = r#"{"from_date":"2024-01-01","to_date":"2024-12-31","limit":50,"offset":100}"#;
    let f: ReportFilters = serde_json::from_str(json).unwrap();
    assert_eq!(f.limit, Some(50));
    assert_eq!(f.offset, Some(100));
}

#[test]
fn report_filters_missing_from_date_fails() {
    let json = r#"{"to_date":"2024-12-31"}"#;
    let result: Result<ReportFilters, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn report_filters_missing_to_date_fails() {
    let json = r#"{"from_date":"2024-01-01"}"#;
    let result: Result<ReportFilters, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Authorization codes for reports/exports controller
// ---------------------------------------------------------------------------

#[test]
fn reports_read_permission_code_is_correct() {
    assert_eq!(api::REPORTS_READ, "api.reports.read");
}

#[test]
fn export_data_action_code_is_correct() {
    assert_eq!(action::EXPORT_DATA, "action.reports.export");
}

#[test]
fn export_unmasked_permission_code_is_correct() {
    assert_eq!(api::EXPORT_UNMASKED, "api.export.unmasked");
}

#[test]
fn generate_report_action_code_is_correct() {
    assert_eq!(action::GENERATE_REPORT, "action.reports.generate");
}

// ---------------------------------------------------------------------------
// Error mapping for reports/exports controller paths
// ---------------------------------------------------------------------------

#[test]
fn analytics_disabled_maps_to_service_unavailable() {
    let err = AppError::ServiceUnavailable("Analytics reporting is currently disabled".to_string());
    let env = err.envelope();
    assert_eq!(env.error.code, "SERVICE_UNAVAILABLE");
    assert!(env.error.message.contains("disabled"));
}

#[test]
fn exports_disabled_maps_to_service_unavailable() {
    let err = AppError::ServiceUnavailable("Data exports are currently disabled".to_string());
    assert_eq!(err.envelope().error.code, "SERVICE_UNAVAILABLE");
}

#[test]
fn invalid_date_range_maps_to_bad_request() {
    let err = AppError::BadRequest("to_date must be after from_date".to_string());
    assert_eq!(err.envelope().error.code, "BAD_REQUEST");
}

#[test]
fn forbidden_unmasked_export_maps_to_forbidden() {
    let err = AppError::Forbidden("Missing permission: api.export.unmasked".to_string());
    assert_eq!(err.envelope().error.code, "FORBIDDEN");
}

#[test]
fn malformed_date_maps_to_bad_request() {
    let err = AppError::BadRequest("Invalid date format: expected YYYY-MM-DD".to_string());
    let env = err.envelope();
    assert_eq!(env.error.code, "BAD_REQUEST");
    assert!(env.error.message.contains("YYYY-MM-DD"));
}
