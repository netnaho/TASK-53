// Controller-level tests for the service catalog API layer.
//
// Covers: catalog item request types, validation helper functions
// (category, rule_type, quarter_hour, mileage), and error code mapping.
// No database, no HTTP server — runs with `cargo test --lib`.

use crate::domain::auth_policy::{action, api};
use crate::domain::catalog_types::{
    CreateServiceItemRequest, UpdateServiceItemRequest,
    validate_category, validate_mileage, validate_quarter_hour, validate_rule_type,
};
use crate::domain::error::AppError;

// ---------------------------------------------------------------------------
// CreateServiceItemRequest
// ---------------------------------------------------------------------------

#[test]
fn create_service_item_request_required_fields() {
    let json = r#"{
        "code": "NURSING-01",
        "name": "Skilled Nursing Visit",
        "category": "nursing",
        "unit_type": "visit",
        "default_rate": 95.00
    }"#;
    let req: CreateServiceItemRequest =
        serde_json::from_str(json).expect("deserialize CreateServiceItemRequest");
    assert_eq!(req.code, "NURSING-01");
    assert_eq!(req.category, "nursing");
    assert_eq!(req.unit_type, "visit");
    assert_eq!(req.default_rate, 95.0);
    assert!(req.description.is_none());
}

#[test]
fn create_service_item_request_with_description() {
    let json = r#"{
        "code": "REHAB-01",
        "name": "PT Session",
        "description": "Physical therapy one-hour session",
        "category": "rehab",
        "unit_type": "hour",
        "default_rate": 110.00
    }"#;
    let req: CreateServiceItemRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.description.as_deref(), Some("Physical therapy one-hour session"));
}

#[test]
fn create_service_item_request_missing_code_fails() {
    let json = r#"{"name":"N","category":"nursing","unit_type":"visit","default_rate":50.0}"#;
    let result: Result<CreateServiceItemRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// UpdateServiceItemRequest
// ---------------------------------------------------------------------------

#[test]
fn update_service_item_request_all_optional() {
    let json = r#"{}"#;
    let req: UpdateServiceItemRequest = serde_json::from_str(json).unwrap();
    assert!(req.name.is_none());
    assert!(req.default_rate.is_none());
    assert!(req.is_active.is_none());
}

#[test]
fn update_service_item_request_deactivate() {
    let json = r#"{"is_active":false}"#;
    let req: UpdateServiceItemRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.is_active, Some(false));
}

#[test]
fn update_service_item_request_update_rate() {
    let json = r#"{"default_rate":85.50,"name":"Updated Nursing Visit"}"#;
    let req: UpdateServiceItemRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.default_rate, Some(85.50));
    assert_eq!(req.name.as_deref(), Some("Updated Nursing Visit"));
}

// ---------------------------------------------------------------------------
// validate_category
// ---------------------------------------------------------------------------

#[test]
fn all_valid_categories_pass() {
    for cat in &["nursing", "rehab", "meals", "companionship", "transportation", "other"] {
        assert!(
            validate_category(cat).is_ok(),
            "'{}' should be a valid category",
            cat
        );
    }
}

#[test]
fn unknown_category_fails() {
    assert!(validate_category("physical_therapy").is_err());
    assert!(validate_category("").is_err());
    assert!(validate_category("NURSING").is_err()); // case-sensitive
}

// ---------------------------------------------------------------------------
// validate_rule_type
// ---------------------------------------------------------------------------

#[test]
fn all_valid_rule_types_pass() {
    assert!(validate_rule_type("per_visit").is_ok());
    assert!(validate_rule_type("hourly").is_ok());
    assert!(validate_rule_type("tiered").is_ok());
}

#[test]
fn unknown_rule_type_fails() {
    assert!(validate_rule_type("flat_fee").is_err());
    assert!(validate_rule_type("").is_err());
    assert!(validate_rule_type("PER_VISIT").is_err());
}

// ---------------------------------------------------------------------------
// validate_quarter_hour
// ---------------------------------------------------------------------------

#[test]
fn valid_quarter_hour_increments_pass() {
    for hours in &[0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 4.0, 8.0] {
        assert!(
            validate_quarter_hour(*hours).is_ok(),
            "{} hours should be valid",
            hours
        );
    }
}

#[test]
fn non_quarter_hour_increments_fail() {
    // Non-quarter-hour fractional values
    assert!(validate_quarter_hour(0.1).is_err());
    assert!(validate_quarter_hour(1.3).is_err());
    assert!(validate_quarter_hour(0.33).is_err());
    assert!(validate_quarter_hour(2.7).is_err());
}

#[test]
fn zero_hours_fails() {
    assert!(validate_quarter_hour(0.0).is_err());
}

#[test]
fn negative_hours_fails() {
    assert!(validate_quarter_hour(-1.0).is_err());
    assert!(validate_quarter_hour(-0.25).is_err());
}

// ---------------------------------------------------------------------------
// validate_mileage
// ---------------------------------------------------------------------------

#[test]
fn zero_mileage_is_valid() {
    assert!(validate_mileage(0.0).is_ok());
}

#[test]
fn positive_mileage_up_to_200_is_valid() {
    assert!(validate_mileage(1.0).is_ok());
    assert!(validate_mileage(100.0).is_ok());
    assert!(validate_mileage(199.9).is_ok());
    assert!(validate_mileage(200.0).is_ok());
}

#[test]
fn mileage_over_200_fails() {
    assert!(validate_mileage(200.1).is_err());
    assert!(validate_mileage(250.0).is_err());
    assert!(validate_mileage(1000.0).is_err());
}

#[test]
fn negative_mileage_fails() {
    assert!(validate_mileage(-1.0).is_err());
    assert!(validate_mileage(-0.01).is_err());
}

// ---------------------------------------------------------------------------
// Authorization codes for catalog controller
// ---------------------------------------------------------------------------

#[test]
fn catalog_read_permission_code_is_correct() {
    assert_eq!(api::CATALOG_READ, "api.catalog.read");
}

#[test]
fn catalog_write_permission_code_is_correct() {
    assert_eq!(api::CATALOG_WRITE, "api.catalog.write");
}

#[test]
fn create_service_action_code_is_correct() {
    assert_eq!(action::CREATE_SERVICE, "action.catalog.create");
}

// ---------------------------------------------------------------------------
// Error mapping for catalog controller paths
// ---------------------------------------------------------------------------

#[test]
fn service_item_not_found_maps_to_not_found() {
    let err = AppError::NotFound("Service item not found".to_string());
    assert_eq!(err.envelope().error.code, "NOT_FOUND");
}

#[test]
fn duplicate_service_code_maps_to_bad_request() {
    let err = AppError::BadRequest("Service code already exists".to_string());
    assert_eq!(err.envelope().error.code, "BAD_REQUEST");
}

#[test]
fn invalid_service_category_maps_to_bad_request() {
    let err = AppError::BadRequest("Invalid category".to_string());
    assert_eq!(err.envelope().error.code, "BAD_REQUEST");
}
