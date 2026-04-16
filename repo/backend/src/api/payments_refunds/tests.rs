// Controller-level tests for the payments & refunds API layer.
//
// Covers: payment/refund/reconciliation request types, payment method validation,
// idempotency key structure, refund reason code handling, and error mapping.
// No database, no HTTP server — runs with `cargo test --lib`.

use crate::domain::auth_policy::{action, api};
use crate::domain::billing_types::{
    ReconciliationRequest, RecordPaymentRequest, RecordRefundRequest, validate_payment_method,
};
use crate::domain::error::AppError;

// ---------------------------------------------------------------------------
// RecordPaymentRequest
// ---------------------------------------------------------------------------

#[test]
fn record_payment_request_required_fields() {
    let json = r#"{
        "invoice_id": "inv-001",
        "idempotency_key": "pay-key-abc",
        "payment_method": "check",
        "amount": 150.00,
        "payment_date": "2024-03-15"
    }"#;
    let req: RecordPaymentRequest =
        serde_json::from_str(json).expect("deserialize RecordPaymentRequest");
    assert_eq!(req.invoice_id, "inv-001");
    assert_eq!(req.idempotency_key, "pay-key-abc");
    assert_eq!(req.payment_method, "check");
    assert_eq!(req.amount, 150.00);
    assert_eq!(req.payment_date, "2024-03-15");
    assert!(req.reference_number.is_none());
    assert!(req.notes.is_none());
}

#[test]
fn record_payment_request_with_optional_fields() {
    let json = r#"{
        "invoice_id": "inv-002",
        "idempotency_key": "pay-key-xyz",
        "payment_method": "ach",
        "amount": 75.25,
        "payment_date": "2024-03-20",
        "reference_number": "ACH-REF-001",
        "notes": "Monthly payment"
    }"#;
    let req: RecordPaymentRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.reference_number.as_deref(), Some("ACH-REF-001"));
    assert_eq!(req.notes.as_deref(), Some("Monthly payment"));
}

#[test]
fn record_payment_request_missing_invoice_id_fails() {
    let json = r#"{"idempotency_key":"k","payment_method":"check","amount":1.0,"payment_date":"2024-01-01"}"#;
    let result: Result<RecordPaymentRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn record_payment_request_missing_amount_fails() {
    let json = r#"{"invoice_id":"i","idempotency_key":"k","payment_method":"check","payment_date":"2024-01-01"}"#;
    let result: Result<RecordPaymentRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// RecordRefundRequest
// ---------------------------------------------------------------------------

#[test]
fn record_refund_request_required_fields() {
    let json = r#"{
        "invoice_id": "inv-003",
        "reason_code": "BILLING_ERROR",
        "amount": 25.00,
        "refund_method": "check",
        "refund_date": "2024-03-25"
    }"#;
    let req: RecordRefundRequest =
        serde_json::from_str(json).expect("deserialize RecordRefundRequest");
    assert_eq!(req.invoice_id, "inv-003");
    assert_eq!(req.reason_code, "BILLING_ERROR");
    assert_eq!(req.amount, 25.00);
    assert_eq!(req.refund_method, "check");
    assert!(req.reason_notes.is_none());
}

#[test]
fn record_refund_request_with_notes() {
    let json = r#"{
        "invoice_id": "inv-004",
        "reason_code": "DUPLICATE_CHARGE",
        "amount": 50.00,
        "refund_method": "ach",
        "refund_date": "2024-04-01",
        "reason_notes": "Client was charged twice for same visit"
    }"#;
    let req: RecordRefundRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.reason_notes.as_deref(), Some("Client was charged twice for same visit"));
}

#[test]
fn record_refund_request_missing_reason_code_fails() {
    let json = r#"{"invoice_id":"i","amount":10.0,"refund_method":"check","refund_date":"2024-01-01"}"#;
    let result: Result<RecordRefundRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// ReconciliationRequest
// ---------------------------------------------------------------------------

#[test]
fn reconciliation_request_required_period_fields() {
    let json = r#"{"period_start":"2024-01-01","period_end":"2024-03-31"}"#;
    let req: ReconciliationRequest =
        serde_json::from_str(json).expect("deserialize ReconciliationRequest");
    assert_eq!(req.period_start, "2024-01-01");
    assert_eq!(req.period_end, "2024-03-31");
}

#[test]
fn reconciliation_request_missing_period_start_fails() {
    let json = r#"{"period_end":"2024-03-31"}"#;
    let result: Result<ReconciliationRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn reconciliation_request_missing_period_end_fails() {
    let json = r#"{"period_start":"2024-01-01"}"#;
    let result: Result<ReconciliationRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Payment method validation (shared with billing tests)
// ---------------------------------------------------------------------------

#[test]
fn all_valid_payment_methods_accepted() {
    for method in &["check", "ach", "wire", "credit_card", "cash", "other"] {
        assert!(validate_payment_method(method), "'{}' must be valid", method);
    }
}

#[test]
fn invalid_payment_method_bitcoin_rejected() {
    assert!(!validate_payment_method("bitcoin"));
}

#[test]
fn empty_payment_method_rejected() {
    assert!(!validate_payment_method(""));
}

#[test]
fn uppercase_payment_method_rejected() {
    // Validation is case-sensitive
    assert!(!validate_payment_method("ACH"));
    assert!(!validate_payment_method("CHECK"));
}

// ---------------------------------------------------------------------------
// Authorization boundaries for payments controller
// ---------------------------------------------------------------------------

#[test]
fn payments_read_permission_code_is_correct() {
    assert_eq!(api::PAYMENTS_READ, "api.payments.read");
}

#[test]
fn record_payment_action_code_is_correct() {
    assert_eq!(action::RECORD_PAYMENT, "action.payments.record");
}

#[test]
fn process_refund_action_code_is_correct() {
    assert_eq!(action::PROCESS_REFUND, "action.payments.refund");
}

// ---------------------------------------------------------------------------
// Error mapping for payment/refund controller paths
// ---------------------------------------------------------------------------

#[test]
fn payment_not_found_maps_to_not_found() {
    let err = AppError::NotFound("Payment not found".to_string());
    assert_eq!(err.envelope().error.code, "NOT_FOUND");
}

#[test]
fn duplicate_idempotency_key_maps_to_conflict() {
    let err = AppError::Conflict("Idempotency key already used within 5 minutes".to_string());
    let env = err.envelope();
    assert_eq!(env.error.code, "CONFLICT");
    assert!(env.error.message.contains("Idempotency"));
}

#[test]
fn refund_exceeds_net_paid_maps_to_bad_request() {
    let err = AppError::BadRequest("Refund amount exceeds net-paid balance".to_string());
    assert_eq!(err.envelope().error.code, "BAD_REQUEST");
}

#[test]
fn invalid_refund_reason_code_maps_to_bad_request() {
    let err = AppError::BadRequest("Unknown refund reason code".to_string());
    assert_eq!(err.envelope().error.code, "BAD_REQUEST");
}

#[test]
fn payment_against_draft_invoice_maps_to_bad_request() {
    let err =
        AppError::BadRequest("Cannot record payment against invoice with status 'draft'".to_string());
    let env = err.envelope();
    assert_eq!(env.error.code, "BAD_REQUEST");
    assert!(env.error.message.contains("draft"));
}
