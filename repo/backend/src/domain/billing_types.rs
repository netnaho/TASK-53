/// Domain types for the billing engine: charges, invoices, payments, refunds,
/// charge adjustments, fund transactions, and reconciliation snapshots.

use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};

// ============================================================
// Database row types (sqlx FromRow)
// ============================================================

#[derive(Debug, Serialize, Clone, sqlx::FromRow)]
pub struct ChargeRow {
    pub id: String,
    pub org_id: String,
    pub delivery_entry_id: String,
    pub plan_id: String,
    pub invoice_id: Option<String>,
    pub rule_type: String,
    pub computed_units: f64,
    pub rate_applied: f64,
    pub gross_amount: f64,
    pub adjustment_total: f64,
    pub net_amount: f64,
    pub status: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Clone, sqlx::FromRow)]
pub struct ChargeAdjustmentRow {
    pub id: String,
    pub charge_id: String,
    pub adjusted_by: String,
    pub amount: f64,
    pub reason: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Clone, sqlx::FromRow)]
pub struct InvoiceRow {
    pub id: String,
    pub org_id: String,
    pub plan_id: String,
    pub invoice_number: String,
    pub billing_period_start: NaiveDate,
    pub billing_period_end: NaiveDate,
    pub subtotal: f64,
    pub total_adjustments: f64,
    pub total_amount: f64,
    pub status: String,
    pub generated_by: String,
    pub notes: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Clone, sqlx::FromRow)]
pub struct InvoiceLineItemRow {
    pub id: String,
    pub invoice_id: String,
    pub charge_id: String,
    pub description: String,
    pub delivery_date: NaiveDate,
    pub units: f64,
    pub unit_rate: f64,
    pub gross_amount: f64,
    pub adjustment_amount: f64,
    pub net_amount: f64,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Clone, sqlx::FromRow)]
pub struct FundTransactionRow {
    pub id: String,
    pub org_id: String,
    pub invoice_id: String,
    pub transaction_type: String,
    pub amount: f64,
    pub direction: String,
    pub reference_id: String,
    pub actor_id: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Clone, sqlx::FromRow)]
pub struct RecordedPaymentRow {
    pub id: String,
    pub org_id: String,
    pub invoice_id: String,
    pub fund_transaction_id: String,
    pub idempotency_key: String,
    pub payment_method: String,
    pub amount: f64,
    pub reference_number: Option<String>,
    pub payment_date: NaiveDate,
    pub recorded_by: String,
    pub notes: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Clone, sqlx::FromRow)]
pub struct RecordedRefundRow {
    pub id: String,
    pub org_id: String,
    pub invoice_id: String,
    pub fund_transaction_id: String,
    pub reason_code_id: String,
    pub amount: f64,
    pub reason_notes: Option<String>,
    pub refund_method: String,
    pub reference_number: Option<String>,
    pub refund_date: NaiveDate,
    pub recorded_by: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Clone, sqlx::FromRow)]
pub struct RefundReasonCodeRow {
    pub id: String,
    pub code: String,
    pub label: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Clone, sqlx::FromRow)]
pub struct ReconciliationRunRow {
    pub id: String,
    pub org_id: String,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub run_by: String,
    pub total_charges: f64,
    pub total_adjustments: f64,
    pub total_invoiced: f64,
    pub total_paid: f64,
    pub total_refunded: f64,
    pub net_collected: f64,
    pub pending_charge_count: i64,
    pub invoiced_charge_count: i64,
    pub paid_invoice_count: i64,
    pub outstanding_balance: f64,
    pub created_at: NaiveDateTime,
}

// ============================================================
// Request types (API input)
// ============================================================

#[derive(Debug, Deserialize)]
pub struct GenerateChargesRequest {
    /// Generate charges for all verified delivery entries in this plan.
    pub plan_id: String,
    /// Optional: restrict to entries on or after this date.
    pub from_date: Option<String>,
    /// Optional: restrict to entries on or before this date.
    pub to_date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PostAdjustmentRequest {
    pub amount: f64,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct GenerateInvoiceRequest {
    pub plan_id: String,
    pub billing_period_start: String,
    pub billing_period_end: String,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateInvoiceStatusRequest {
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct RecordPaymentRequest {
    pub invoice_id: String,
    /// Caller-supplied idempotency key. Same key within 5 minutes -> 409.
    pub idempotency_key: String,
    pub payment_method: String,
    pub amount: f64,
    pub reference_number: Option<String>,
    pub payment_date: String,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RecordRefundRequest {
    pub invoice_id: String,
    pub reason_code: String,
    pub amount: f64,
    pub reason_notes: Option<String>,
    pub refund_method: String,
    pub reference_number: Option<String>,
    pub refund_date: String,
}

#[derive(Debug, Deserialize)]
pub struct ReconciliationRequest {
    pub period_start: String,
    pub period_end: String,
}

// ============================================================
// Response types (API output)
// ============================================================

#[derive(Debug, Serialize)]
pub struct ChargeDetail {
    pub charge: ChargeRow,
    pub adjustments: Vec<ChargeAdjustmentRow>,
}

#[derive(Debug, Serialize)]
pub struct InvoiceDetail {
    pub invoice: InvoiceRow,
    pub line_items: Vec<InvoiceLineItemRow>,
}

#[derive(Debug, Serialize)]
pub struct GenerateChargesResponse {
    pub generated: u32,
    pub skipped: u32,
    pub charges: Vec<ChargeRow>,
}

// ============================================================
// Validation helpers
// ============================================================

pub fn validate_payment_method(method: &str) -> bool {
    matches!(method, "check" | "ach" | "wire" | "credit_card" | "cash" | "other")
}

pub fn validate_invoice_status_transition(current: &str, next: &str) -> Result<(), String> {
    let allowed: &[&str] = match current {
        "draft" => &["issued", "voided"],
        "issued" => &["paid", "partially_paid", "voided"],
        "partially_paid" => &["paid", "voided"],
        "paid" => &[],
        "voided" => &[],
        _ => return Err(format!("Unknown current status: {}", current)),
    };
    if allowed.contains(&next) {
        Ok(())
    } else {
        Err(format!(
            "Cannot transition invoice from '{}' to '{}'. Allowed: {:?}",
            current, next, allowed
        ))
    }
}

// ============================================================
// Unit tests
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payment_method_validation() {
        assert!(validate_payment_method("check"));
        assert!(validate_payment_method("ach"));
        assert!(validate_payment_method("wire"));
        assert!(validate_payment_method("credit_card"));
        assert!(validate_payment_method("cash"));
        assert!(validate_payment_method("other"));
        assert!(!validate_payment_method("bitcoin"));
        assert!(!validate_payment_method(""));
        assert!(!validate_payment_method("CASH")); // case-sensitive
    }

    #[test]
    fn test_invoice_status_transitions() {
        // Valid transitions
        assert!(validate_invoice_status_transition("draft", "issued").is_ok());
        assert!(validate_invoice_status_transition("draft", "voided").is_ok());
        assert!(validate_invoice_status_transition("issued", "paid").is_ok());
        assert!(validate_invoice_status_transition("issued", "partially_paid").is_ok());
        assert!(validate_invoice_status_transition("issued", "voided").is_ok());
        assert!(validate_invoice_status_transition("partially_paid", "paid").is_ok());
        assert!(validate_invoice_status_transition("partially_paid", "voided").is_ok());

        // Invalid transitions
        assert!(validate_invoice_status_transition("paid", "issued").is_err());
        assert!(validate_invoice_status_transition("paid", "draft").is_err());
        assert!(validate_invoice_status_transition("voided", "issued").is_err());
        assert!(validate_invoice_status_transition("draft", "paid").is_err());
    }
}
