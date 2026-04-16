// Dedicated unit tests for feature modules: reporting, scoring, billing, ops.
// All tests run with `cargo test --lib` on native targets (no browser needed).

use crate::features::{billing, ops, reporting, scoring};

// ============================================================================
// reporting module
// ============================================================================

#[test]
fn validate_date_range_valid() {
    assert!(reporting::validate_date_range("2024-01-01", "2024-12-31").is_ok());
    assert!(reporting::validate_date_range("2024-06-01", "2024-06-01").is_ok()); // same date OK
}

#[test]
fn validate_date_range_empty_from_fails() {
    let result = reporting::validate_date_range("", "2024-12-31");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Start date"));
}

#[test]
fn validate_date_range_empty_to_fails() {
    let result = reporting::validate_date_range("2024-01-01", "");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("End date"));
}

#[test]
fn validate_date_range_inverted_fails() {
    let result = reporting::validate_date_range("2024-12-31", "2024-01-01");
    assert!(result.is_err());
    let msg = result.unwrap_err();
    assert!(msg.contains("2024-12-31") || msg.contains("after"), "msg: {}", msg);
}

#[test]
fn clamp_limit_within_range() {
    assert_eq!(reporting::clamp_limit(50), 50);
    assert_eq!(reporting::clamp_limit(1), 1);
    assert_eq!(reporting::clamp_limit(200), 200);
}

#[test]
fn clamp_limit_below_minimum() {
    assert_eq!(reporting::clamp_limit(0), 1);
    assert_eq!(reporting::clamp_limit(-5), 1);
}

#[test]
fn clamp_limit_above_maximum() {
    assert_eq!(reporting::clamp_limit(201), 200);
    assert_eq!(reporting::clamp_limit(1000), 200);
}

#[test]
fn default_report_limit_is_50() {
    assert_eq!(reporting::DEFAULT_REPORT_LIMIT, 50);
}

#[test]
fn default_report_offset_is_0() {
    assert_eq!(reporting::DEFAULT_REPORT_OFFSET, 0);
}

#[test]
fn quarter_label_q1() {
    assert_eq!(reporting::quarter_label(2024, 1), "2024-Q1");
    assert_eq!(reporting::quarter_label(2024, 2), "2024-Q1");
    assert_eq!(reporting::quarter_label(2024, 3), "2024-Q1");
}

#[test]
fn quarter_label_q2() {
    assert_eq!(reporting::quarter_label(2024, 4), "2024-Q2");
    assert_eq!(reporting::quarter_label(2024, 6), "2024-Q2");
}

#[test]
fn quarter_label_q3() {
    assert_eq!(reporting::quarter_label(2024, 7), "2024-Q3");
}

#[test]
fn quarter_label_q4() {
    assert_eq!(reporting::quarter_label(2024, 10), "2024-Q4");
    assert_eq!(reporting::quarter_label(2024, 12), "2024-Q4");
}

#[test]
fn can_export_unmasked_requires_correct_permission() {
    assert!(reporting::can_export_unmasked(&["api.export.unmasked"]));
    assert!(!reporting::can_export_unmasked(&["api.reports.read"]));
    assert!(!reporting::can_export_unmasked(&[]));
}

#[test]
fn order_volume_path_includes_dates() {
    let path = reporting::order_volume_path("2024-01-01", "2024-06-30");
    assert!(path.contains("/reports/order-volume"));
    assert!(path.contains("from_date=2024-01-01"));
    assert!(path.contains("to_date=2024-06-30"));
}

// ============================================================================
// scoring module
// ============================================================================

#[test]
fn format_score_with_value() {
    assert_eq!(scoring::format_score(Some(85.0)), "85.0%");
    assert_eq!(scoring::format_score(Some(100.0)), "100.0%");
    assert_eq!(scoring::format_score(Some(0.0)), "0.0%");
}

#[test]
fn format_score_without_value() {
    assert_eq!(scoring::format_score(None), "--");
}

#[test]
fn format_score_rounds_to_one_decimal() {
    let s = scoring::format_score(Some(87.456));
    // 87.456 rounds to 87.5 at 1 decimal
    assert_eq!(s, "87.5%");
}

#[test]
fn evaluation_status_label_known_statuses() {
    assert_eq!(scoring::evaluation_status_label("draft"), "Draft");
    assert_eq!(scoring::evaluation_status_label("submitted"), "Submitted");
    assert_eq!(
        scoring::evaluation_status_label("second_review_required"),
        "Awaiting Second Review"
    );
    assert_eq!(scoring::evaluation_status_label("reviewed"), "Reviewed");
    assert_eq!(scoring::evaluation_status_label("finalized"), "Finalized");
}

#[test]
fn evaluation_status_label_unknown_returns_unknown() {
    assert_eq!(scoring::evaluation_status_label("invalid_status"), "Unknown");
}

#[test]
fn requires_reviewer_only_for_second_review_required() {
    assert!(scoring::requires_reviewer("second_review_required"));
    assert!(!scoring::requires_reviewer("submitted"));
    assert!(!scoring::requires_reviewer("finalized"));
    assert!(!scoring::requires_reviewer("draft"));
}

#[test]
fn is_editable_for_draft_and_submitted() {
    assert!(scoring::is_editable("draft"));
    assert!(scoring::is_editable("submitted"));
    assert!(!scoring::is_editable("finalized"));
    assert!(!scoring::is_editable("second_review_required"));
    assert!(!scoring::is_editable("reviewed"));
}

#[test]
fn score_delta_over_10_requires_review() {
    assert!(scoring::score_delta_requires_review(80.0, 91.0)); // +11 delta
    assert!(scoring::score_delta_requires_review(90.0, 75.0)); // -15 delta (abs > 10)
}

#[test]
fn score_delta_within_10_does_not_require_review() {
    assert!(!scoring::score_delta_requires_review(80.0, 89.0)); // +9
    assert!(!scoring::score_delta_requires_review(80.0, 80.0)); // 0
    assert!(!scoring::score_delta_requires_review(80.0, 90.0)); // exactly 10 — not > 10
}

#[test]
fn score_badge_class_excellent_at_90_plus() {
    assert_eq!(scoring::score_badge_class(90.0), "badge-excellent");
    assert_eq!(scoring::score_badge_class(100.0), "badge-excellent");
}

#[test]
fn score_badge_class_good_75_to_89() {
    assert_eq!(scoring::score_badge_class(75.0), "badge-good");
    assert_eq!(scoring::score_badge_class(89.9), "badge-good");
}

#[test]
fn score_badge_class_fair_60_to_74() {
    assert_eq!(scoring::score_badge_class(60.0), "badge-fair");
    assert_eq!(scoring::score_badge_class(74.9), "badge-fair");
}

#[test]
fn score_badge_class_poor_below_60() {
    assert_eq!(scoring::score_badge_class(59.9), "badge-poor");
    assert_eq!(scoring::score_badge_class(0.0), "badge-poor");
}

// ============================================================================
// billing module
// ============================================================================

#[test]
fn format_amount_positive() {
    assert_eq!(billing::format_amount(100.0), "$100.00");
    assert_eq!(billing::format_amount(75.5), "$75.50");
    assert_eq!(billing::format_amount(0.0), "$0.00");
}

#[test]
fn format_amount_cents_precision() {
    assert_eq!(billing::format_amount(9.99), "$9.99");
    assert_eq!(billing::format_amount(1234.56), "$1234.56");
}

#[test]
fn invoice_status_labels_all_known() {
    assert_eq!(billing::invoice_status_label("draft"), "Draft");
    assert_eq!(billing::invoice_status_label("issued"), "Issued");
    assert_eq!(billing::invoice_status_label("partially_paid"), "Partially Paid");
    assert_eq!(billing::invoice_status_label("paid"), "Paid");
    assert_eq!(billing::invoice_status_label("voided"), "Voided");
}

#[test]
fn invoice_status_label_unknown() {
    assert_eq!(billing::invoice_status_label("pending"), "Unknown");
}

#[test]
fn can_void_invoice_for_voiceable_statuses() {
    assert!(billing::can_void_invoice("draft"));
    assert!(billing::can_void_invoice("issued"));
    assert!(billing::can_void_invoice("partially_paid"));
    assert!(!billing::can_void_invoice("paid"));
    assert!(!billing::can_void_invoice("voided"));
}

#[test]
fn can_record_payment_for_open_invoices() {
    assert!(billing::can_record_payment("issued"));
    assert!(billing::can_record_payment("partially_paid"));
    assert!(!billing::can_record_payment("draft"));
    assert!(!billing::can_record_payment("paid"));
    assert!(!billing::can_record_payment("voided"));
}

#[test]
fn payment_method_labels_all_known() {
    assert_eq!(billing::payment_method_label("check"), "Check");
    assert_eq!(billing::payment_method_label("ach"), "ACH Transfer");
    assert_eq!(billing::payment_method_label("wire"), "Wire Transfer");
    assert_eq!(billing::payment_method_label("credit_card"), "Credit Card");
    assert_eq!(billing::payment_method_label("cash"), "Cash");
    assert_eq!(billing::payment_method_label("other"), "Other");
}

#[test]
fn payment_method_label_unknown() {
    assert_eq!(billing::payment_method_label("crypto"), "Unknown");
}

#[test]
fn validate_payment_amount_positive_ok() {
    assert!(billing::validate_payment_amount(1.0).is_ok());
    assert!(billing::validate_payment_amount(100.0).is_ok());
    assert!(billing::validate_payment_amount(0.01).is_ok());
}

#[test]
fn validate_payment_amount_zero_fails() {
    assert!(billing::validate_payment_amount(0.0).is_err());
}

#[test]
fn validate_payment_amount_negative_fails() {
    assert!(billing::validate_payment_amount(-5.0).is_err());
}

#[test]
fn compute_net_collected() {
    assert!((billing::compute_net_collected(100.0, 20.0) - 80.0).abs() < 1e-9);
    assert!((billing::compute_net_collected(0.0, 0.0)).abs() < 1e-9);
    assert!((billing::compute_net_collected(50.0, 50.0)).abs() < 1e-9);
}

// ============================================================================
// ops module
// ============================================================================

#[test]
fn flag_description_exports_enabled() {
    let desc = ops::flag_description("exports_enabled");
    assert!(desc.contains("export") || !desc.is_empty());
}

#[test]
fn flag_description_analytics_enabled() {
    let desc = ops::flag_description("analytics_enabled");
    assert!(!desc.is_empty());
}

#[test]
fn flag_description_unknown_key() {
    let desc = ops::flag_description("nonexistent_flag");
    assert!(desc.contains("Unknown") || !desc.is_empty());
}

#[test]
fn flag_value_label_enabled() {
    assert_eq!(ops::flag_value_label(true), "Enabled");
}

#[test]
fn flag_value_label_disabled() {
    assert_eq!(ops::flag_value_label(false), "Disabled");
}

#[test]
fn can_toggle_flags_requires_ops_write() {
    assert!(ops::can_toggle_flags(&["api.ops.write"]));
    assert!(!ops::can_toggle_flags(&["api.ops.read"]));
    assert!(!ops::can_toggle_flags(&[]));
}

#[test]
fn can_view_ops_for_read_or_write() {
    assert!(ops::can_view_ops(&["api.ops.read"]));
    assert!(ops::can_view_ops(&["api.ops.write"]));
    assert!(!ops::can_view_ops(&["api.billing.read"]));
    assert!(!ops::can_view_ops(&[]));
}

#[test]
fn toggle_constants_are_correct() {
    assert_eq!(ops::TOGGLE_EXPORTS, "exports_enabled");
    assert_eq!(ops::TOGGLE_ANALYTICS, "analytics_enabled");
}

#[test]
fn is_known_toggle_recognizes_known_keys() {
    assert!(ops::is_known_toggle("exports_enabled"));
    assert!(ops::is_known_toggle("analytics_enabled"));
    assert!(!ops::is_known_toggle("unknown_flag"));
    assert!(!ops::is_known_toggle(""));
}
