/// Payment service: recorded payment creation with idempotency key enforcement,
/// duplicate detection within 5-minute window, and fund_transaction ledger entries.

use chrono::NaiveDate;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::domain::billing_types::*;
use crate::domain::error::AppError;
use crate::infrastructure::audit::{AuditEntry, AuditService};

#[derive(Clone)]
pub struct PaymentService {
    pool: MySqlPool,
    audit: AuditService,
}

impl PaymentService {
    pub fn new(pool: MySqlPool, audit: AuditService) -> Self {
        Self { pool, audit }
    }

    pub async fn list_refund_reason_codes(&self) -> Result<Vec<RefundReasonCodeRow>, AppError> {
        sqlx::query_as::<_, RefundReasonCodeRow>(
            "SELECT id, code, label, description, is_active, created_at
             FROM refund_reason_codes WHERE is_active = 1 ORDER BY label ASC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
    }

    // ------------------------------------------------------------------
    // Record Payment
    // ------------------------------------------------------------------

    pub async fn record_payment(
        &self,
        org_id: &str,
        user_id: &str,
        req: &RecordPaymentRequest,
    ) -> Result<RecordedPaymentRow, AppError> {
        // Validate amount
        if req.amount <= 0.0 {
            return Err(AppError::BadRequest("Payment amount must be positive".to_string()));
        }

        // Validate payment method
        if !validate_payment_method(&req.payment_method) {
            return Err(AppError::BadRequest(format!(
                "Invalid payment_method '{}'. Must be one of: check, ach, wire, credit_card, cash, other",
                req.payment_method
            )));
        }

        // Validate payment date
        NaiveDate::parse_from_str(&req.payment_date, "%Y-%m-%d")
            .map_err(|_| AppError::BadRequest("Invalid payment_date format (YYYY-MM-DD)".to_string()))?;

        // Verify invoice exists and belongs to org
        let invoice: Option<(String, String, f64)> = sqlx::query_as(
            "SELECT id, status, total_amount FROM invoices WHERE id = ? AND org_id = ?"
        )
        .bind(&req.invoice_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let (_, inv_status, _) = invoice
            .ok_or_else(|| AppError::NotFound("Invoice not found".to_string()))?;

        if matches!(inv_status.as_str(), "draft" | "voided") {
            return Err(AppError::BadRequest(format!(
                "Cannot record payment against invoice with status '{}'", inv_status
            )));
        }

        let payment_id = Uuid::new_v4().to_string();
        let fund_tx_id = Uuid::new_v4().to_string();
        let amount_rounded = (req.amount * 100.0).round() / 100.0;

        // ---------------------------------------------------------------
        // Race-safe 5-minute idempotency window.
        //
        // We use a dedicated `payment_idempotency_keys` table with a PRIMARY
        // KEY on (org_id, idempotency_key).  Strategy:
        //
        // 1. DELETE any expired key (>5 minutes old) for this (org, key).
        //    Idempotent: deletes 0 or 1 rows.
        // 2. Try to INSERT a fresh row.  If the unique key is still occupied
        //    (i.e. an ACTIVE key is present), the INSERT fails with a
        //    duplicate-key error — we map that to 409 Conflict.
        //
        // This is race-safe because step 2 is atomic at the DB level: either
        // the row gets inserted (no active key) or the unique constraint
        // rejects it (active key present).  The DELETE-then-INSERT pattern
        // is safe because two concurrent requests for the SAME (org, key)
        // can both DELETE (no-op for the second) but only ONE can INSERT.
        // ---------------------------------------------------------------

        // Step 1: clean up expired keys
        sqlx::query(
            "DELETE FROM payment_idempotency_keys
             WHERE org_id = ? AND idempotency_key = ?
               AND created_at <= NOW() - INTERVAL 5 MINUTE"
        )
        .bind(org_id)
        .bind(&req.idempotency_key)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        // Step 2: atomically claim the key
        let claim = sqlx::query(
            "INSERT INTO payment_idempotency_keys (org_id, idempotency_key, created_at)
             VALUES (?, ?, NOW())"
        )
        .bind(org_id)
        .bind(&req.idempotency_key)
        .execute(&self.pool)
        .await;

        if let Err(e) = claim {
            let err_str = e.to_string();
            // MySQL duplicate key error → active key present within window
            if err_str.contains("Duplicate entry") || err_str.contains("1062") {
                return Err(AppError::Conflict(format!(
                    "Duplicate request: a payment with idempotency_key '{}' was already recorded \
                     for this organization within the last 5 minutes",
                    req.idempotency_key
                )));
            }
            return Err(AppError::Internal(err_str));
        }

        // Key is either new (rows_affected=1) or expired and refreshed
        // (rows_affected=2) — proceed with payment recording.
        //
        // IMPORTANT: Insert the fund_transaction FIRST because recorded_payments
        // has a FK referencing fund_transactions(id). The reference_id in
        // fund_transactions is forward-referencing to the payment, which is
        // not FK-constrained, so we can insert it with the future payment_id.
        sqlx::query(
            "INSERT INTO fund_transactions
             (id, org_id, invoice_id, transaction_type, amount, direction, reference_id, actor_id)
             VALUES (?, ?, ?, 'payment', ?, 'credit', ?, ?)"
        )
        .bind(&fund_tx_id)
        .bind(org_id)
        .bind(&req.invoice_id)
        .bind(amount_rounded)
        .bind(&payment_id)  // reference_id points to the payment record (forward ref)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let _payment_insert = sqlx::query(
            "INSERT INTO recorded_payments
             (id, org_id, invoice_id, fund_transaction_id, idempotency_key, payment_method,
              amount, reference_number, payment_date, recorded_by, notes)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&payment_id)
        .bind(org_id)
        .bind(&req.invoice_id)
        .bind(&fund_tx_id)
        .bind(&req.idempotency_key)
        .bind(&req.payment_method)
        .bind(amount_rounded)
        .bind(&req.reference_number)
        .bind(&req.payment_date)
        .bind(user_id)
        .bind(&req.notes)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        // Update invoice status based on payments vs total
        self.reconcile_invoice_payment_status(&req.invoice_id, org_id).await?;

        self.audit.log(AuditEntry {
            user_id: Some(user_id.to_string()),
            ip_address: None,
            trace_id: None,
            action: "billing.payment.recorded".to_string(),
            resource_type: "payment".to_string(),
            resource_id: Some(payment_id.clone()),
            org_id: Some(org_id.to_string()),
            details: Some(serde_json::json!({
                "invoice_id": req.invoice_id,
                "amount": amount_rounded,
                "method": req.payment_method,
                "idempotency_key": req.idempotency_key,
            })),
        }).await;

        let row: RecordedPaymentRow = sqlx::query_as(
            "SELECT id, org_id, invoice_id, fund_transaction_id, idempotency_key, payment_method,
                    amount, reference_number, payment_date, recorded_by, notes, created_at
             FROM recorded_payments WHERE id = ?"
        )
        .bind(&payment_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(row)
    }

    pub async fn list_payments(
        &self,
        org_id: &str,
        invoice_id: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<RecordedPaymentRow>, i64), AppError> {
        let mut where_clause = "WHERE org_id = ?".to_string();
        let mut binds: Vec<String> = vec![org_id.to_string()];

        if let Some(inv) = invoice_id {
            where_clause.push_str(" AND invoice_id = ?");
            binds.push(inv.to_string());
        }

        let count_q = format!("SELECT COUNT(*) FROM recorded_payments {}", where_clause);
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_q);
        for b in &binds { cq = cq.bind(b); }
        let (total,) = cq.fetch_one(&self.pool).await.map_err(|e| AppError::Internal(e.to_string()))?;

        let data_q = format!(
            "SELECT id, org_id, invoice_id, fund_transaction_id, idempotency_key, payment_method,
                    amount, reference_number, payment_date, recorded_by, notes, created_at
             FROM recorded_payments {} ORDER BY created_at DESC LIMIT ? OFFSET ?",
            where_clause
        );
        let mut dq = sqlx::query_as::<_, RecordedPaymentRow>(&data_q);
        for b in &binds { dq = dq.bind(b); }
        let rows = dq.bind(limit).bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok((rows, total))
    }

    pub async fn get_payment(
        &self,
        payment_id: &str,
        org_id: &str,
    ) -> Result<RecordedPaymentRow, AppError> {
        sqlx::query_as::<_, RecordedPaymentRow>(
            "SELECT id, org_id, invoice_id, fund_transaction_id, idempotency_key, payment_method,
                    amount, reference_number, payment_date, recorded_by, notes, created_at
             FROM recorded_payments WHERE id = ? AND org_id = ?"
        )
        .bind(payment_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Payment not found".to_string()))
    }

    // ------------------------------------------------------------------
    // Record Refund
    // ------------------------------------------------------------------

    pub async fn record_refund(
        &self,
        org_id: &str,
        user_id: &str,
        req: &RecordRefundRequest,
    ) -> Result<RecordedRefundRow, AppError> {
        // Validate amount
        if req.amount <= 0.0 {
            return Err(AppError::BadRequest("Refund amount must be positive".to_string()));
        }

        // Validate refund method
        if !validate_payment_method(&req.refund_method) {
            return Err(AppError::BadRequest(format!(
                "Invalid refund_method '{}'. Must be one of: check, ach, wire, credit_card, cash, other",
                req.refund_method
            )));
        }

        // Validate refund date
        NaiveDate::parse_from_str(&req.refund_date, "%Y-%m-%d")
            .map_err(|_| AppError::BadRequest("Invalid refund_date format (YYYY-MM-DD)".to_string()))?;

        // Verify invoice
        let invoice: Option<(String, String)> = sqlx::query_as(
            "SELECT id, status FROM invoices WHERE id = ? AND org_id = ?"
        )
        .bind(&req.invoice_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let (_, inv_status) = invoice
            .ok_or_else(|| AppError::NotFound("Invoice not found".to_string()))?;

        if matches!(inv_status.as_str(), "draft" | "voided") {
            return Err(AppError::BadRequest(format!(
                "Cannot record refund against invoice with status '{}'", inv_status
            )));
        }

        // Validate reason code
        let reason_code: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM refund_reason_codes WHERE code = ? AND is_active = 1"
        )
        .bind(&req.reason_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let (reason_code_id,) = reason_code.ok_or_else(|| AppError::BadRequest(format!(
            "Unknown or inactive refund reason code: '{}'", req.reason_code
        )))?;

        // Enforce net-paid cap: sum(payments) - sum(prior refunds) must be >= refund amount
        let (total_paid,): (Option<f64>,) = sqlx::query_as(
            "SELECT SUM(amount) FROM recorded_payments WHERE invoice_id = ? AND org_id = ?"
        )
        .bind(&req.invoice_id)
        .bind(org_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let (total_refunded,): (Option<f64>,) = sqlx::query_as(
            "SELECT SUM(amount) FROM recorded_refunds WHERE invoice_id = ? AND org_id = ?"
        )
        .bind(&req.invoice_id)
        .bind(org_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let total_paid = total_paid.unwrap_or(0.0);
        let total_refunded = total_refunded.unwrap_or(0.0);
        let net_paid = round_to_2(total_paid - total_refunded);
        let amount_rounded = (req.amount * 100.0).round() / 100.0;

        if amount_rounded > net_paid + 0.001 {
            return Err(AppError::BadRequest(format!(
                "Refund amount ({:.2}) exceeds net paid amount ({:.2}). \
                 Total paid: {:.2}, total already refunded: {:.2}",
                amount_rounded, net_paid, total_paid, total_refunded
            )));
        }

        let refund_id = Uuid::new_v4().to_string();
        let fund_tx_id = Uuid::new_v4().to_string();

        // Insert fund_transaction (immutable ledger — debit direction for refund)
        sqlx::query(
            "INSERT INTO fund_transactions
             (id, org_id, invoice_id, transaction_type, amount, direction, reference_id, actor_id)
             VALUES (?, ?, ?, 'refund', ?, 'debit', ?, ?)"
        )
        .bind(&fund_tx_id)
        .bind(org_id)
        .bind(&req.invoice_id)
        .bind(amount_rounded)
        .bind(&refund_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        // Insert recorded_refund
        sqlx::query(
            "INSERT INTO recorded_refunds
             (id, org_id, invoice_id, fund_transaction_id, reason_code_id, amount, reason_notes,
              refund_method, reference_number, refund_date, recorded_by)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&refund_id)
        .bind(org_id)
        .bind(&req.invoice_id)
        .bind(&fund_tx_id)
        .bind(&reason_code_id)
        .bind(amount_rounded)
        .bind(&req.reason_notes)
        .bind(&req.refund_method)
        .bind(&req.reference_number)
        .bind(&req.refund_date)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        // Update invoice payment status after refund
        self.reconcile_invoice_payment_status(&req.invoice_id, org_id).await?;

        self.audit.log(AuditEntry {
            user_id: Some(user_id.to_string()),
            ip_address: None,
            trace_id: None,
            action: "billing.refund.recorded".to_string(),
            resource_type: "refund".to_string(),
            resource_id: Some(refund_id.clone()),
            org_id: Some(org_id.to_string()),
            details: Some(serde_json::json!({
                "invoice_id": req.invoice_id,
                "amount": amount_rounded,
                "reason_code": req.reason_code,
            })),
        }).await;

        let row: RecordedRefundRow = sqlx::query_as(
            "SELECT id, org_id, invoice_id, fund_transaction_id, reason_code_id, amount,
                    reason_notes, refund_method, reference_number, refund_date, recorded_by, created_at
             FROM recorded_refunds WHERE id = ?"
        )
        .bind(&refund_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(row)
    }

    pub async fn list_refunds(
        &self,
        org_id: &str,
        invoice_id: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<RecordedRefundRow>, i64), AppError> {
        let mut where_clause = "WHERE org_id = ?".to_string();
        let mut binds: Vec<String> = vec![org_id.to_string()];

        if let Some(inv) = invoice_id {
            where_clause.push_str(" AND invoice_id = ?");
            binds.push(inv.to_string());
        }

        let count_q = format!("SELECT COUNT(*) FROM recorded_refunds {}", where_clause);
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_q);
        for b in &binds { cq = cq.bind(b); }
        let (total,) = cq.fetch_one(&self.pool).await.map_err(|e| AppError::Internal(e.to_string()))?;

        let data_q = format!(
            "SELECT id, org_id, invoice_id, fund_transaction_id, reason_code_id, amount,
                    reason_notes, refund_method, reference_number, refund_date, recorded_by, created_at
             FROM recorded_refunds {} ORDER BY created_at DESC LIMIT ? OFFSET ?",
            where_clause
        );
        let mut dq = sqlx::query_as::<_, RecordedRefundRow>(&data_q);
        for b in &binds { dq = dq.bind(b); }
        let rows = dq.bind(limit).bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok((rows, total))
    }

    pub async fn get_refund(
        &self,
        refund_id: &str,
        org_id: &str,
    ) -> Result<RecordedRefundRow, AppError> {
        sqlx::query_as::<_, RecordedRefundRow>(
            "SELECT id, org_id, invoice_id, fund_transaction_id, reason_code_id, amount,
                    reason_notes, refund_method, reference_number, refund_date, recorded_by, created_at
             FROM recorded_refunds WHERE id = ? AND org_id = ?"
        )
        .bind(refund_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Refund not found".to_string()))
    }

    // ------------------------------------------------------------------
    // Fund Transactions (read-only; immutable ledger)
    // ------------------------------------------------------------------

    pub async fn list_fund_transactions(
        &self,
        org_id: &str,
        invoice_id: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<FundTransactionRow>, i64), AppError> {
        let mut where_clause = "WHERE org_id = ?".to_string();
        let mut binds: Vec<String> = vec![org_id.to_string()];

        if let Some(inv) = invoice_id {
            where_clause.push_str(" AND invoice_id = ?");
            binds.push(inv.to_string());
        }

        let count_q = format!("SELECT COUNT(*) FROM fund_transactions {}", where_clause);
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_q);
        for b in &binds { cq = cq.bind(b); }
        let (total,) = cq.fetch_one(&self.pool).await.map_err(|e| AppError::Internal(e.to_string()))?;

        let data_q = format!(
            "SELECT id, org_id, invoice_id, transaction_type, amount, direction, reference_id, actor_id, created_at
             FROM fund_transactions {} ORDER BY created_at ASC LIMIT ? OFFSET ?",
            where_clause
        );
        let mut dq = sqlx::query_as::<_, FundTransactionRow>(&data_q);
        for b in &binds { dq = dq.bind(b); }
        let rows = dq.bind(limit).bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok((rows, total))
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    /// Recompute invoice payment status after a payment or refund is recorded.
    async fn reconcile_invoice_payment_status(
        &self,
        invoice_id: &str,
        org_id: &str,
    ) -> Result<(), AppError> {
        let (total_amount,): (f64,) = sqlx::query_as(
            "SELECT total_amount FROM invoices WHERE id = ? AND org_id = ?"
        )
        .bind(invoice_id)
        .bind(org_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let (total_paid,): (Option<f64>,) = sqlx::query_as(
            "SELECT SUM(amount) FROM recorded_payments WHERE invoice_id = ?"
        )
        .bind(invoice_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let (total_refunded,): (Option<f64>,) = sqlx::query_as(
            "SELECT SUM(amount) FROM recorded_refunds WHERE invoice_id = ?"
        )
        .bind(invoice_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let net = round_to_2(total_paid.unwrap_or(0.0) - total_refunded.unwrap_or(0.0));

        // Only update if invoice is in a payment-trackable state (issued or partially_paid)
        let current_status: Option<(String,)> = sqlx::query_as(
            "SELECT status FROM invoices WHERE id = ?"
        )
        .bind(invoice_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        if let Some((status,)) = current_status {
            if !matches!(status.as_str(), "issued" | "partially_paid") {
                return Ok(());
            }
            if let Some(new_status) = determine_payment_status(net, total_amount) {
                sqlx::query("UPDATE invoices SET status = ? WHERE id = ?")
                    .bind(new_status)
                    .bind(invoice_id)
                    .execute(&self.pool)
                    .await
                    .map_err(|e| AppError::Internal(e.to_string()))?;
            }
        }

        Ok(())
    }
}

/// Round an f64 to two decimal places.
fn round_to_2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// Pure function: given a rounded net-paid amount and the invoice total,
/// return the new status (`"paid"` or `"partially_paid"`) or `None` to
/// leave the status unchanged (net <= 0).
fn determine_payment_status(net: f64, total_amount: f64) -> Option<&'static str> {
    let threshold = total_amount - 0.01; // $0.01 rounding tolerance
    if net >= threshold {
        Some("paid")
    } else if net > 0.0 {
        Some("partially_paid")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- round_to_2 ---------------------------------------------------------

    #[test]
    fn round_to_2_basic() {
        assert_eq!(round_to_2(1.005), 1.0);   // IEEE 754: 1.005 stores as ≈1.00499... → rounds down
        assert_eq!(round_to_2(1.015), 1.01);  // IEEE 754: 1.015 stores as ≈1.01499... → rounds down
        assert_eq!(round_to_2(99.999), 100.0);
        assert_eq!(round_to_2(0.0), 0.0);
        assert_eq!(round_to_2(-0.005), -0.01);
    }

    // -- determine_payment_status -------------------------------------------

    #[test]
    fn exact_paid_threshold() {
        // net == total_amount  ->  "paid"
        assert_eq!(determine_payment_status(500.00, 500.00), Some("paid"));
    }

    #[test]
    fn paid_within_one_cent_tolerance() {
        // net is 1 cent below total — still within $0.01 tolerance -> "paid"
        assert_eq!(determine_payment_status(499.99, 500.00), Some("paid"));
    }

    #[test]
    fn partial_payment() {
        // net is well below total -> "partially_paid"
        assert_eq!(determine_payment_status(250.00, 500.00), Some("partially_paid"));
    }

    #[test]
    fn payment_with_partial_refund() {
        // paid 500, refunded 200 -> net 300, total 500 -> "partially_paid"
        let net = round_to_2(500.00 - 200.00);
        assert_eq!(determine_payment_status(net, 500.00), Some("partially_paid"));
    }

    #[test]
    fn full_refund_to_zero() {
        // net == 0.0 -> None (no status change)
        let net = round_to_2(500.00 - 500.00);
        assert_eq!(determine_payment_status(net, 500.00), None);
    }

    #[test]
    fn rounding_edge_one_cent_below_tolerance() {
        // net is 2 cents below total — outside $0.01 tolerance -> "partially_paid"
        assert_eq!(determine_payment_status(499.98, 500.00), Some("partially_paid"));
    }

    #[test]
    fn rounding_edge_tiny_net() {
        // net is $0.01 -> "partially_paid" (positive but far below total)
        assert_eq!(determine_payment_status(0.01, 500.00), Some("partially_paid"));
    }

    #[test]
    fn rounding_edge_net_slightly_above_zero() {
        // Floating-point sum that rounds to 0.01
        let net = round_to_2(0.004 + 0.006);
        assert_eq!(net, 0.01);
        assert_eq!(determine_payment_status(net, 500.00), Some("partially_paid"));
    }

    #[test]
    fn rounding_edge_net_rounds_to_zero() {
        // Floating-point residue that rounds to 0.00 -> None
        let net = round_to_2(100.00 - 99.999);
        assert_eq!(net, 0.0);
        assert_eq!(determine_payment_status(net, 100.00), None);
    }

    #[test]
    fn net_computation_matches_requirement() {
        // Verify the old buggy formula would give a wrong answer,
        // while round_to_2(paid - refunded) gives the correct one.
        let total_paid = 500.0_f64;
        let total_refunded = 200.0_f64;

        // Buggy: (paid - refunded * 100).round() / 100 = (500 - 20000).round() / 100 = -195.0
        let buggy_net = (total_paid - total_refunded * 100.0).round() / 100.0;
        assert!(buggy_net < 0.0, "buggy formula produces a negative net");

        // Fixed
        let correct_net = round_to_2(total_paid - total_refunded);
        assert_eq!(correct_net, 300.0);
    }
}
