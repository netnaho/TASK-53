/// Reconciliation service: generate point-in-time summary snapshots for a billing period.
/// Snapshots are immutable once created.

use chrono::NaiveDate;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::domain::billing_types::*;
use crate::domain::error::AppError;
use crate::infrastructure::audit::{AuditEntry, AuditService};

#[derive(Clone)]
pub struct ReconciliationService {
    pool: MySqlPool,
    audit: AuditService,
}

impl ReconciliationService {
    pub fn new(pool: MySqlPool, audit: AuditService) -> Self {
        Self { pool, audit }
    }

    pub async fn generate_reconciliation(
        &self,
        org_id: &str,
        user_id: &str,
        req: &ReconciliationRequest,
    ) -> Result<ReconciliationRunRow, AppError> {
        let period_start = NaiveDate::parse_from_str(&req.period_start, "%Y-%m-%d")
            .map_err(|_| AppError::BadRequest("Invalid period_start format (YYYY-MM-DD)".to_string()))?;
        let period_end = NaiveDate::parse_from_str(&req.period_end, "%Y-%m-%d")
            .map_err(|_| AppError::BadRequest("Invalid period_end format (YYYY-MM-DD)".to_string()))?;

        if period_end < period_start {
            return Err(AppError::BadRequest(
                "period_end must be >= period_start".to_string(),
            ));
        }

        // Total charges (by delivery date in period)
        let (total_charges,): (Option<f64>,) = sqlx::query_as(
            "SELECT SUM(c.gross_amount)
             FROM charges c
             JOIN delivery_entries d ON d.id = c.delivery_entry_id
             WHERE c.org_id = ? AND d.delivery_date BETWEEN ? AND ?
               AND c.status != 'voided'"
        )
        .bind(org_id)
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        // Total adjustments in period
        let (total_adjustments,): (Option<f64>,) = sqlx::query_as(
            "SELECT SUM(c.adjustment_total)
             FROM charges c
             JOIN delivery_entries d ON d.id = c.delivery_entry_id
             WHERE c.org_id = ? AND d.delivery_date BETWEEN ? AND ?
               AND c.status != 'voided'"
        )
        .bind(org_id)
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        // Total invoiced (invoices created in period)
        let (total_invoiced,): (Option<f64>,) = sqlx::query_as(
            "SELECT SUM(total_amount) FROM invoices
             WHERE org_id = ? AND DATE(created_at) BETWEEN ? AND ?
               AND status != 'voided'"
        )
        .bind(org_id)
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        // Total paid in period
        let (total_paid,): (Option<f64>,) = sqlx::query_as(
            "SELECT SUM(amount) FROM recorded_payments
             WHERE org_id = ? AND payment_date BETWEEN ? AND ?"
        )
        .bind(org_id)
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        // Total refunded in period
        let (total_refunded,): (Option<f64>,) = sqlx::query_as(
            "SELECT SUM(amount) FROM recorded_refunds
             WHERE org_id = ? AND refund_date BETWEEN ? AND ?"
        )
        .bind(org_id)
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        // Pending charge count
        let (pending_charge_count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM charges c
             JOIN delivery_entries d ON d.id = c.delivery_entry_id
             WHERE c.org_id = ? AND d.delivery_date BETWEEN ? AND ?
               AND c.status IN ('pending', 'adjusted')"
        )
        .bind(org_id)
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        // Invoiced charge count
        let (invoiced_charge_count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM charges c
             JOIN delivery_entries d ON d.id = c.delivery_entry_id
             WHERE c.org_id = ? AND d.delivery_date BETWEEN ? AND ?
               AND c.status = 'invoiced'"
        )
        .bind(org_id)
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        // Paid invoice count
        let (paid_invoice_count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM invoices
             WHERE org_id = ? AND DATE(created_at) BETWEEN ? AND ?
               AND status = 'paid'"
        )
        .bind(org_id)
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        // Outstanding balance: total invoiced - total paid + total refunded
        let total_charges = round2(total_charges.unwrap_or(0.0));
        let total_adj = round2(total_adjustments.unwrap_or(0.0));
        let total_invoiced = round2(total_invoiced.unwrap_or(0.0));
        let total_paid = round2(total_paid.unwrap_or(0.0));
        let total_refunded = round2(total_refunded.unwrap_or(0.0));
        let net_collected = round2(total_paid - total_refunded);
        let outstanding_balance = round2(total_invoiced - net_collected);

        let run_id = Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO reconciliation_runs
             (id, org_id, period_start, period_end, run_by,
              total_charges, total_adjustments, total_invoiced, total_paid, total_refunded,
              net_collected, pending_charge_count, invoiced_charge_count, paid_invoice_count,
              outstanding_balance)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&run_id)
        .bind(org_id)
        .bind(period_start)
        .bind(period_end)
        .bind(user_id)
        .bind(total_charges)
        .bind(total_adj)
        .bind(total_invoiced)
        .bind(total_paid)
        .bind(total_refunded)
        .bind(net_collected)
        .bind(pending_charge_count)
        .bind(invoiced_charge_count)
        .bind(paid_invoice_count)
        .bind(outstanding_balance)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        self.audit.log(AuditEntry {
            user_id: Some(user_id.to_string()),
            ip_address: None,
            trace_id: None,
            action: "billing.reconciliation.generated".to_string(),
            resource_type: "reconciliation_run".to_string(),
            resource_id: Some(run_id.clone()),
            org_id: Some(org_id.to_string()),
            details: Some(serde_json::json!({
                "period_start": req.period_start,
                "period_end": req.period_end,
                "outstanding_balance": outstanding_balance,
            })),
        }).await;

        let row: ReconciliationRunRow = sqlx::query_as(
            "SELECT id, org_id, period_start, period_end, run_by,
                    total_charges, total_adjustments, total_invoiced, total_paid, total_refunded,
                    net_collected, pending_charge_count, invoiced_charge_count, paid_invoice_count,
                    outstanding_balance, created_at
             FROM reconciliation_runs WHERE id = ?"
        )
        .bind(&run_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(row)
    }

    pub async fn list_reconciliation_runs(
        &self,
        org_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<ReconciliationRunRow>, i64), AppError> {
        let (total,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM reconciliation_runs WHERE org_id = ?"
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let rows: Vec<ReconciliationRunRow> = sqlx::query_as(
            "SELECT id, org_id, period_start, period_end, run_by,
                    total_charges, total_adjustments, total_invoiced, total_paid, total_refunded,
                    net_collected, pending_charge_count, invoiced_charge_count, paid_invoice_count,
                    outstanding_balance, created_at
             FROM reconciliation_runs WHERE org_id = ?
             ORDER BY created_at DESC LIMIT ? OFFSET ?"
        )
        .bind(org_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok((rows, total))
    }

    pub async fn get_reconciliation_run(
        &self,
        run_id: &str,
        org_id: &str,
    ) -> Result<ReconciliationRunRow, AppError> {
        sqlx::query_as::<_, ReconciliationRunRow>(
            "SELECT id, org_id, period_start, period_end, run_by,
                    total_charges, total_adjustments, total_invoiced, total_paid, total_refunded,
                    net_collected, pending_charge_count, invoiced_charge_count, paid_invoice_count,
                    outstanding_balance, created_at
             FROM reconciliation_runs WHERE id = ? AND org_id = ?"
        )
        .bind(run_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Reconciliation run not found".to_string()))
    }
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}
