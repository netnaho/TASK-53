// Controller-level tests for the billing API layer.
//
// Covers: charge/invoice/adjustment request deserialization, date format validation,
// payment method validation, invoice status transition rules, error mapping.
// No database, no HTTP server — runs with `cargo test --lib`.

use crate::domain::auth_policy::{action, api};
use crate::domain::billing_types::{
    GenerateChargesRequest, GenerateInvoiceRequest, PostAdjustmentRequest,
    UpdateInvoiceStatusRequest, validate_invoice_status_transition, validate_payment_method,
};
use crate::domain::error::AppError;

// ---------------------------------------------------------------------------
// GenerateChargesRequest
// ---------------------------------------------------------------------------

#[test]
fn generate_charges_request_required_plan_id() {
    let json = r#"{"plan_id":"plan-001"}"#;
    let req: GenerateChargesRequest =
        serde_json::from_str(json).expect("deserialize GenerateChargesRequest");
    assert_eq!(req.plan_id, "plan-001");
    assert!(req.from_date.is_none());
    assert!(req.to_date.is_none());
}

#[test]
fn generate_charges_request_with_date_range() {
    let json = r#"{"plan_id":"plan-002","from_date":"2024-01-01","to_date":"2024-06-30"}"#;
    let req: GenerateChargesRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.from_date.as_deref(), Some("2024-01-01"));
    assert_eq!(req.to_date.as_deref(), Some("2024-06-30"));
}

#[test]
fn generate_charges_request_missing_plan_id_fails() {
    let json = r#"{"from_date":"2024-01-01"}"#;
    let result: Result<GenerateChargesRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// PostAdjustmentRequest
// ---------------------------------------------------------------------------

#[test]
fn post_adjustment_request_positive_amount() {
    let json = r#"{"amount":25.50,"reason":"Rate correction"}"#;
    let req: PostAdjustmentRequest =
        serde_json::from_str(json).expect("deserialize PostAdjustmentRequest");
    assert_eq!(req.amount, 25.50);
    assert_eq!(req.reason, "Rate correction");
}

#[test]
fn post_adjustment_request_negative_amount_allowed() {
    // Negative adjustments (credits) are valid JSON; business logic rejects zero
    let json = r#"{"amount":-10.00,"reason":"Overpayment credit"}"#;
    let req: PostAdjustmentRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.amount, -10.0);
}

#[test]
fn post_adjustment_request_missing_amount_fails() {
    let json = r#"{"reason":"No amount"}"#;
    let result: Result<PostAdjustmentRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn post_adjustment_request_missing_reason_fails() {
    let json = r#"{"amount":5.00}"#;
    let result: Result<PostAdjustmentRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// GenerateInvoiceRequest
// ---------------------------------------------------------------------------

#[test]
fn generate_invoice_request_required_fields() {
    let json = r#"{"plan_id":"pl-1","billing_period_start":"2024-01-01","billing_period_end":"2024-01-31"}"#;
    let req: GenerateInvoiceRequest =
        serde_json::from_str(json).expect("deserialize GenerateInvoiceRequest");
    assert_eq!(req.plan_id, "pl-1");
    assert_eq!(req.billing_period_start, "2024-01-01");
    assert_eq!(req.billing_period_end, "2024-01-31");
    assert!(req.notes.is_none());
}

#[test]
fn generate_invoice_request_with_notes() {
    let json = r#"{"plan_id":"pl-2","billing_period_start":"2024-02-01","billing_period_end":"2024-02-29","notes":"February cycle"}"#;
    let req: GenerateInvoiceRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.notes.as_deref(), Some("February cycle"));
}

#[test]
fn generate_invoice_request_missing_period_fails() {
    let json = r#"{"plan_id":"pl-3"}"#;
    let result: Result<GenerateInvoiceRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// UpdateInvoiceStatusRequest
// ---------------------------------------------------------------------------

#[test]
fn update_invoice_status_request_issued() {
    let json = r#"{"status":"issued"}"#;
    let req: UpdateInvoiceStatusRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.status, "issued");
}

#[test]
fn update_invoice_status_request_paid() {
    let json = r#"{"status":"paid"}"#;
    let req: UpdateInvoiceStatusRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.status, "paid");
}

#[test]
fn update_invoice_status_request_missing_status_fails() {
    let json = r#"{}"#;
    let result: Result<UpdateInvoiceStatusRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Invoice status transition validation
// ---------------------------------------------------------------------------

#[test]
fn draft_can_transition_to_issued() {
    assert!(validate_invoice_status_transition("draft", "issued").is_ok());
}

#[test]
fn draft_can_transition_to_voided() {
    assert!(validate_invoice_status_transition("draft", "voided").is_ok());
}

#[test]
fn draft_cannot_transition_to_paid() {
    let result = validate_invoice_status_transition("draft", "paid");
    assert!(result.is_err(), "draft → paid should be rejected");
}

#[test]
fn issued_can_transition_to_paid() {
    assert!(validate_invoice_status_transition("issued", "paid").is_ok());
}

#[test]
fn issued_can_transition_to_partially_paid() {
    assert!(validate_invoice_status_transition("issued", "partially_paid").is_ok());
}

#[test]
fn paid_cannot_transition_to_draft() {
    let result = validate_invoice_status_transition("paid", "draft");
    assert!(result.is_err(), "paid → draft should be rejected");
}

#[test]
fn voided_cannot_transition_to_any() {
    assert!(validate_invoice_status_transition("voided", "draft").is_err());
    assert!(validate_invoice_status_transition("voided", "issued").is_err());
    assert!(validate_invoice_status_transition("voided", "paid").is_err());
}

// ---------------------------------------------------------------------------
// Payment method validation
// ---------------------------------------------------------------------------

#[test]
fn valid_payment_methods_all_pass() {
    for method in &["check", "ach", "wire", "credit_card", "cash", "other"] {
        assert!(
            validate_payment_method(method),
            "'{}' should be a valid payment method",
            method
        );
    }
}

#[test]
fn invalid_payment_method_fails() {
    assert!(!validate_payment_method("paypal"));
    assert!(!validate_payment_method("crypto"));
    assert!(!validate_payment_method(""));
    assert!(!validate_payment_method("CHECK")); // case-sensitive
}

// ---------------------------------------------------------------------------
// Authorization boundaries for billing controller
// ---------------------------------------------------------------------------

#[test]
fn billing_read_permission_code_is_correct() {
    assert_eq!(api::BILLING_READ, "api.billing.read");
}

#[test]
fn generate_invoice_action_code_is_correct() {
    assert_eq!(action::GENERATE_INVOICE, "action.billing.generate");
}

#[test]
fn approve_invoice_action_code_is_correct() {
    assert_eq!(action::APPROVE_INVOICE, "action.billing.approve");
}

// ---------------------------------------------------------------------------
// Error mapping for billing controller paths
// ---------------------------------------------------------------------------

#[test]
fn invoice_not_found_maps_to_not_found() {
    let err = AppError::NotFound("Invoice not found".to_string());
    assert_eq!(err.envelope().error.code, "NOT_FOUND");
}

#[test]
fn zero_adjustment_maps_to_bad_request() {
    let err = AppError::BadRequest("Adjustment amount cannot be zero".to_string());
    let env = err.envelope();
    assert_eq!(env.error.code, "BAD_REQUEST");
    assert!(env.error.message.contains("zero"));
}

#[test]
fn no_pending_charges_maps_to_bad_request() {
    let err = AppError::BadRequest("No pending charges found".to_string());
    assert_eq!(err.envelope().error.code, "BAD_REQUEST");
}
