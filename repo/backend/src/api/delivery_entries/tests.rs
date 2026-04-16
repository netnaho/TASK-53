// Controller-level tests for the delivery_entries API layer.
//
// Covers: delivery entry/note request deserialization, quarter-hour and
// mileage validation, delivery status values, permission codes, and error mapping.
// No database, no HTTP server — runs with `cargo test --lib`.

use crate::domain::auth_policy::{action, api};
use crate::domain::catalog_types::{
    CreateDeliveryEntryRequest, CreateEligibilityNoteRequest, UpdateDeliveryEntryRequest,
    validate_mileage, validate_quarter_hour,
};
use crate::domain::error::AppError;

// ---------------------------------------------------------------------------
// CreateDeliveryEntryRequest
// ---------------------------------------------------------------------------

#[test]
fn create_delivery_entry_required_fields() {
    let json = r#"{
        "plan_id": "pl-1",
        "plan_package_id": "pp-1",
        "service_item_id": "svc-1",
        "delivery_date": "2024-03-15",
        "units": 1.0
    }"#;
    let req: CreateDeliveryEntryRequest =
        serde_json::from_str(json).expect("deserialize CreateDeliveryEntryRequest");
    assert_eq!(req.plan_id, "pl-1");
    assert_eq!(req.plan_package_id, "pp-1");
    assert_eq!(req.service_item_id, "svc-1");
    assert_eq!(req.delivery_date, "2024-03-15");
    assert_eq!(req.units, 1.0);
    assert!(req.start_time.is_none());
    assert!(req.end_time.is_none());
    assert!(req.mileage.is_none());
    assert!(req.notes.is_none());
}

#[test]
fn create_delivery_entry_with_all_optional_fields() {
    let json = r#"{
        "plan_id": "pl-2",
        "plan_package_id": "pp-2",
        "service_item_id": "svc-2",
        "delivery_date": "2024-03-20",
        "start_time": "09:00",
        "end_time": "11:30",
        "units": 2.5,
        "mileage": 15.5,
        "notes": "Provider notes here"
    }"#;
    let req: CreateDeliveryEntryRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.start_time.as_deref(), Some("09:00"));
    assert_eq!(req.end_time.as_deref(), Some("11:30"));
    assert_eq!(req.units, 2.5);
    assert_eq!(req.mileage, Some(15.5));
    assert_eq!(req.notes.as_deref(), Some("Provider notes here"));
}

#[test]
fn create_delivery_entry_missing_plan_id_fails() {
    let json = r#"{
        "plan_package_id":"pp-1","service_item_id":"svc-1",
        "delivery_date":"2024-03-15","units":1.0
    }"#;
    let result: Result<CreateDeliveryEntryRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn create_delivery_entry_missing_units_fails() {
    let json = r#"{
        "plan_id":"pl-1","plan_package_id":"pp-1",
        "service_item_id":"svc-1","delivery_date":"2024-03-15"
    }"#;
    let result: Result<CreateDeliveryEntryRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// UpdateDeliveryEntryRequest
// ---------------------------------------------------------------------------

#[test]
fn update_delivery_entry_all_optional() {
    let json = r#"{}"#;
    let req: UpdateDeliveryEntryRequest = serde_json::from_str(json).unwrap();
    assert!(req.status.is_none());
    assert!(req.units.is_none());
    assert!(req.mileage.is_none());
    assert!(req.notes.is_none());
}

#[test]
fn update_delivery_entry_verify_status() {
    let json = r#"{"status":"verified"}"#;
    let req: UpdateDeliveryEntryRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.status.as_deref(), Some("verified"));
}

#[test]
fn update_delivery_entry_correct_units() {
    let json = r#"{"units":3.0}"#;
    let req: UpdateDeliveryEntryRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.units, Some(3.0));
}

// ---------------------------------------------------------------------------
// CreateEligibilityNoteRequest
// ---------------------------------------------------------------------------

#[test]
fn create_note_with_required_note_text() {
    let json = r#"{"note":"Client requested schedule change"}"#;
    let req: CreateEligibilityNoteRequest =
        serde_json::from_str(json).expect("deserialize CreateEligibilityNoteRequest");
    assert_eq!(req.note, "Client requested schedule change");
    assert!(req.plan_id.is_none());
    assert!(req.delivery_entry_id.is_none());
    assert!(req.note_type.is_none());
}

#[test]
fn create_note_with_all_optional_fields() {
    let json = r#"{
        "plan_id": "pl-1",
        "delivery_entry_id": "entry-5",
        "note": "Insurance verification complete",
        "note_type": "eligibility"
    }"#;
    let req: CreateEligibilityNoteRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.plan_id.as_deref(), Some("pl-1"));
    assert_eq!(req.delivery_entry_id.as_deref(), Some("entry-5"));
    assert_eq!(req.note_type.as_deref(), Some("eligibility"));
}

#[test]
fn create_note_missing_note_text_fails() {
    let json = r#"{"plan_id":"pl-1"}"#;
    let result: Result<CreateEligibilityNoteRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// validate_quarter_hour — delivery entry validation
// ---------------------------------------------------------------------------

#[test]
fn quarter_hour_values_are_valid() {
    for v in &[0.25_f64, 0.5, 1.0, 1.25, 2.0, 4.0, 8.0] {
        assert!(validate_quarter_hour(*v).is_ok(), "{} should be valid quarter-hour", v);
    }
}

#[test]
fn non_quarter_hour_values_are_invalid() {
    assert!(validate_quarter_hour(0.1).is_err());
    assert!(validate_quarter_hour(1.3).is_err());
    assert!(validate_quarter_hour(0.0).is_err());
    assert!(validate_quarter_hour(-1.0).is_err());
}

#[test]
fn quarter_hour_error_message_contains_valid_examples() {
    let err = validate_quarter_hour(1.3).unwrap_err();
    assert!(err.contains("0.25") || err.contains("increments"), "error: {}", err);
}

// ---------------------------------------------------------------------------
// validate_mileage — delivery entry mileage cap
// ---------------------------------------------------------------------------

#[test]
fn mileage_within_cap_is_valid() {
    assert!(validate_mileage(0.0).is_ok());
    assert!(validate_mileage(50.0).is_ok());
    assert!(validate_mileage(200.0).is_ok());
}

#[test]
fn mileage_above_cap_is_invalid() {
    assert!(validate_mileage(200.1).is_err());
    assert!(validate_mileage(500.0).is_err());
}

#[test]
fn negative_mileage_is_invalid() {
    assert!(validate_mileage(-0.1).is_err());
}

// ---------------------------------------------------------------------------
// Authorization codes for delivery_entries controller
// ---------------------------------------------------------------------------

#[test]
fn delivery_read_permission_code_is_correct() {
    assert_eq!(api::DELIVERY_READ, "api.delivery.read");
}

#[test]
fn delivery_write_permission_code_is_correct() {
    assert_eq!(api::DELIVERY_WRITE, "api.delivery.write");
}

#[test]
fn log_delivery_action_code_is_correct() {
    assert_eq!(action::LOG_DELIVERY, "action.delivery.log");
}

#[test]
fn verify_delivery_action_code_is_correct() {
    assert_eq!(action::VERIFY_DELIVERY, "action.delivery.verify");
}

// ---------------------------------------------------------------------------
// Error mapping for delivery_entries controller paths
// ---------------------------------------------------------------------------

#[test]
fn delivery_entry_not_found_maps_to_not_found() {
    let err = AppError::NotFound("Delivery entry not found".to_string());
    let env = err.envelope();
    assert_eq!(env.error.code, "NOT_FOUND");
    assert!(env.error.message.contains("Delivery entry not found"));
}

#[test]
fn invalid_units_maps_to_bad_request() {
    let err = AppError::BadRequest("Hours must be in 0.25-hour increments".to_string());
    let env = err.envelope();
    assert_eq!(env.error.code, "BAD_REQUEST");
    assert!(env.error.message.contains("0.25"));
}

#[test]
fn mileage_exceeded_maps_to_bad_request() {
    let err = AppError::BadRequest("Mileage cannot exceed 200 miles per visit".to_string());
    assert_eq!(err.envelope().error.code, "BAD_REQUEST");
}

#[test]
fn delivery_forbidden_maps_to_forbidden() {
    let err = AppError::Forbidden("Missing required permission: api.delivery.write".to_string());
    assert_eq!(err.envelope().error.code, "FORBIDDEN");
}
