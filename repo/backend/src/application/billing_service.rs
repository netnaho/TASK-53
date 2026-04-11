/// Billing service: charge generation from delivery entries + package rules,
/// charge adjustment posting, and invoice generation with line items.

use chrono::NaiveDate;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::domain::billing_types::*;
use crate::domain::error::AppError;
use crate::infrastructure::audit::{AuditEntry, AuditService};

#[derive(Clone)]
pub struct BillingService {
    pool: MySqlPool,
    audit: AuditService,
}

impl BillingService {
    pub fn new(pool: MySqlPool, audit: AuditService) -> Self {
        Self { pool, audit }
    }

    // ------------------------------------------------------------------
    // Charge Generation
    // ------------------------------------------------------------------

    /// Generate a charge record for every verified delivery entry in the
    /// given plan that does not yet have a charge. Skips entries that are
    /// already charged or not in 'verified' status.
    pub async fn generate_charges(
        &self,
        org_id: &str,
        user_id: &str,
        req: &GenerateChargesRequest,
    ) -> Result<GenerateChargesResponse, AppError> {
        // Validate plan belongs to org
        let plan: Option<(String, String)> = sqlx::query_as(
            "SELECT id, org_id FROM client_plans WHERE id = ? AND org_id = ?"
        )
        .bind(&req.plan_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        if plan.is_none() {
            return Err(AppError::NotFound("Plan not found".to_string()));
        }

        // Build date range filter
        let from_date = req.from_date.as_deref().unwrap_or("1970-01-01");
        let to_date = req.to_date.as_deref().unwrap_or("9999-12-31");

        // Validate date formats
        NaiveDate::parse_from_str(from_date, "%Y-%m-%d")
            .map_err(|_| AppError::BadRequest("Invalid from_date format (YYYY-MM-DD)".to_string()))?;
        NaiveDate::parse_from_str(to_date, "%Y-%m-%d")
            .map_err(|_| AppError::BadRequest("Invalid to_date format (YYYY-MM-DD)".to_string()))?;

        // Fetch verified delivery entries for the plan that have no charge yet.
        // CAST DECIMAL columns to DOUBLE for f64 decoding.
        let entries: Vec<(String, String, String, f64, Option<f64>, NaiveDate)> = sqlx::query_as(
            "SELECT d.id, d.plan_package_id, d.service_item_id,
                    CAST(d.units AS DOUBLE) AS units,
                    CAST(d.mileage AS DOUBLE) AS mileage,
                    d.delivery_date
             FROM delivery_entries d
             LEFT JOIN charges c ON c.delivery_entry_id = d.id
             WHERE d.plan_id = ? AND d.org_id = ?
               AND d.status = 'verified'
               AND c.id IS NULL
               AND d.delivery_date BETWEEN ? AND ?
             ORDER BY d.delivery_date ASC"
        )
        .bind(&req.plan_id)
        .bind(org_id)
        .bind(from_date)
        .bind(to_date)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let mut generated = 0u32;
        let mut skipped = 0u32;
        let mut charges = Vec::new();

        for (entry_id, plan_package_id, service_item_id, units, _mileage, delivery_date) in entries {
            // Look up the applicable package rule for this service item + plan assignment.
            // CAST DECIMAL r.rate to DOUBLE for f64 decoding.
            let rule: Option<(String, f64, Option<String>)> = sqlx::query_as(
                "SELECT r.rule_type, CAST(r.rate AS DOUBLE) AS rate, r.tier_config
                 FROM package_rule_definitions r
                 JOIN client_plan_packages cpp ON cpp.package_id = r.package_id
                 WHERE cpp.id = ? AND r.service_item_id = ?
                 LIMIT 1"
            )
            .bind(&plan_package_id)
            .bind(&service_item_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

            let (rule_type, rate, tier_config_json) = match rule {
                Some(r) => r,
                None => {
                    tracing::warn!(
                        entry_id = %entry_id,
                        "No matching package rule found; skipping charge generation"
                    );
                    skipped += 1;
                    continue;
                }
            };

            // Compute gross amount from rule type
            let gross_amount = match rule_type.as_str() {
                "per_visit" => rate,
                "hourly" => units * rate,
                "tiered" => {
                    if let Some(json_str) = &tier_config_json {
                        compute_tiered_amount(units, json_str, rate)
                    } else {
                        units * rate
                    }
                }
                _ => units * rate,
            };

            let charge_id = Uuid::new_v4().to_string();
            let gross_rounded = (gross_amount * 100.0).round() / 100.0;

            sqlx::query(
                "INSERT INTO charges
                 (id, org_id, delivery_entry_id, plan_id, rule_type, computed_units, rate_applied,
                  gross_amount, adjustment_total, net_amount, status)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, 0.00, ?, 'pending')"
            )
            .bind(&charge_id)
            .bind(org_id)
            .bind(&entry_id)
            .bind(&req.plan_id)
            .bind(&rule_type)
            .bind(units)
            .bind(rate)
            .bind(gross_rounded)
            .bind(gross_rounded)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

            self.audit.log(AuditEntry {
                user_id: Some(user_id.to_string()),
                ip_address: None,
            trace_id: None,
                action: "billing.charge.generated".to_string(),
                resource_type: "charge".to_string(),
                resource_id: Some(charge_id.clone()),
                org_id: Some(org_id.to_string()),
                details: Some(serde_json::json!({
                    "delivery_entry_id": entry_id,
                    "rule_type": rule_type,
                    "gross_amount": gross_rounded,
                })),
            }).await;

            if let Some(row) = self.get_charge_row(&charge_id).await? {
                charges.push(row);
            }
            generated += 1;
        }

        Ok(GenerateChargesResponse { generated, skipped, charges })
    }

    async fn get_charge_row(&self, charge_id: &str) -> Result<Option<ChargeRow>, AppError> {
        sqlx::query_as::<_, ChargeRow>(
            "SELECT id, org_id, delivery_entry_id, plan_id, invoice_id, rule_type,
                    computed_units, rate_applied, gross_amount, adjustment_total,
                    net_amount, status, created_at, updated_at
             FROM charges WHERE id = ?"
        )
        .bind(charge_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
    }

    pub async fn list_charges(
        &self,
        org_id: &str,
        plan_id: Option<&str>,
        status_filter: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<ChargeRow>, i64), AppError> {
        let mut where_clause = "WHERE org_id = ?".to_string();
        let mut binds: Vec<String> = vec![org_id.to_string()];

        if let Some(p) = plan_id {
            where_clause.push_str(" AND plan_id = ?");
            binds.push(p.to_string());
        }
        if let Some(s) = status_filter {
            where_clause.push_str(" AND status = ?");
            binds.push(s.to_string());
        }

        let count_q = format!("SELECT COUNT(*) FROM charges {}", where_clause);
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_q);
        for b in &binds { cq = cq.bind(b); }
        let (total,) = cq.fetch_one(&self.pool).await.map_err(|e| AppError::Internal(e.to_string()))?;

        let data_q = format!(
            "SELECT id, org_id, delivery_entry_id, plan_id, invoice_id, rule_type,
                    computed_units, rate_applied, gross_amount, adjustment_total,
                    net_amount, status, created_at, updated_at
             FROM charges {} ORDER BY created_at DESC LIMIT ? OFFSET ?",
            where_clause
        );
        let mut dq = sqlx::query_as::<_, ChargeRow>(&data_q);
        for b in &binds { dq = dq.bind(b); }
        let rows = dq.bind(limit).bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok((rows, total))
    }

    pub async fn get_charge_detail(
        &self,
        charge_id: &str,
        org_id: &str,
    ) -> Result<ChargeDetail, AppError> {
        let charge = sqlx::query_as::<_, ChargeRow>(
            "SELECT id, org_id, delivery_entry_id, plan_id, invoice_id, rule_type,
                    computed_units, rate_applied, gross_amount, adjustment_total,
                    net_amount, status, created_at, updated_at
             FROM charges WHERE id = ? AND org_id = ?"
        )
        .bind(charge_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Charge not found".to_string()))?;

        let adjustments = sqlx::query_as::<_, ChargeAdjustmentRow>(
            "SELECT id, charge_id, adjusted_by, amount, reason, created_at
             FROM charge_adjustments WHERE charge_id = ? ORDER BY created_at ASC"
        )
        .bind(charge_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(ChargeDetail { charge, adjustments })
    }

    // ------------------------------------------------------------------
    // Charge Adjustments
    // ------------------------------------------------------------------

    pub async fn post_adjustment(
        &self,
        charge_id: &str,
        org_id: &str,
        user_id: &str,
        req: &PostAdjustmentRequest,
    ) -> Result<ChargeAdjustmentRow, AppError> {
        if req.reason.trim().is_empty() {
            return Err(AppError::BadRequest("Adjustment reason is required".to_string()));
        }
        if req.amount == 0.0 {
            return Err(AppError::BadRequest("Adjustment amount cannot be zero".to_string()));
        }

        // Verify charge belongs to org and is not invoiced/voided
        let charge: Option<(String, String)> = sqlx::query_as(
            "SELECT id, status FROM charges WHERE id = ? AND org_id = ?"
        )
        .bind(charge_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let (_, status) = charge
            .ok_or_else(|| AppError::NotFound("Charge not found".to_string()))?;

        if matches!(status.as_str(), "invoiced" | "voided") {
            return Err(AppError::BadRequest(format!(
                "Cannot adjust charge with status '{}'", status
            )));
        }

        let adj_id = Uuid::new_v4().to_string();
        let amount_rounded = (req.amount * 100.0).round() / 100.0;

        sqlx::query(
            "INSERT INTO charge_adjustments (id, charge_id, adjusted_by, amount, reason)
             VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&adj_id)
        .bind(charge_id)
        .bind(user_id)
        .bind(amount_rounded)
        .bind(req.reason.trim())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        // Recompute adjustment_total and net_amount on the charge
        sqlx::query(
            "UPDATE charges
             SET adjustment_total = (SELECT COALESCE(SUM(amount), 0) FROM charge_adjustments WHERE charge_id = ?),
                 net_amount = gross_amount + (SELECT COALESCE(SUM(amount), 0) FROM charge_adjustments WHERE charge_id = ?),
                 status = 'adjusted'
             WHERE id = ?"
        )
        .bind(charge_id)
        .bind(charge_id)
        .bind(charge_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        self.audit.log(AuditEntry {
            user_id: Some(user_id.to_string()),
            ip_address: None,
            trace_id: None,
            action: "billing.charge.adjusted".to_string(),
            resource_type: "charge".to_string(),
            resource_id: Some(charge_id.to_string()),
            org_id: Some(org_id.to_string()),
            details: Some(serde_json::json!({
                "adjustment_id": adj_id,
                "amount": amount_rounded,
            })),
        }).await;

        let row = sqlx::query_as::<_, ChargeAdjustmentRow>(
            "SELECT id, charge_id, adjusted_by, amount, reason, created_at
             FROM charge_adjustments WHERE id = ?"
        )
        .bind(&adj_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(row)
    }

    // ------------------------------------------------------------------
    // Invoice Generation
    // ------------------------------------------------------------------

    pub async fn generate_invoice(
        &self,
        org_id: &str,
        user_id: &str,
        req: &GenerateInvoiceRequest,
    ) -> Result<InvoiceDetail, AppError> {
        // Validate plan
        let plan: Option<(String, String)> = sqlx::query_as(
            "SELECT id, org_id FROM client_plans WHERE id = ? AND org_id = ?"
        )
        .bind(&req.plan_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        if plan.is_none() {
            return Err(AppError::NotFound("Plan not found".to_string()));
        }

        // Validate date formats
        let period_start = NaiveDate::parse_from_str(&req.billing_period_start, "%Y-%m-%d")
            .map_err(|_| AppError::BadRequest("Invalid billing_period_start format".to_string()))?;
        let period_end = NaiveDate::parse_from_str(&req.billing_period_end, "%Y-%m-%d")
            .map_err(|_| AppError::BadRequest("Invalid billing_period_end format".to_string()))?;

        if period_end < period_start {
            return Err(AppError::BadRequest(
                "billing_period_end must be >= billing_period_start".to_string(),
            ));
        }

        // Gather pending/adjusted charges in the delivery window
        let pending_charges: Vec<ChargeRow> = sqlx::query_as(
            "SELECT c.id, c.org_id, c.delivery_entry_id, c.plan_id, c.invoice_id, c.rule_type,
                    c.computed_units, c.rate_applied, c.gross_amount, c.adjustment_total,
                    c.net_amount, c.status, c.created_at, c.updated_at
             FROM charges c
             JOIN delivery_entries d ON d.id = c.delivery_entry_id
             WHERE c.plan_id = ? AND c.org_id = ?
               AND c.status IN ('pending', 'adjusted')
               AND c.invoice_id IS NULL
               AND d.delivery_date BETWEEN ? AND ?
             ORDER BY d.delivery_date ASC"
        )
        .bind(&req.plan_id)
        .bind(org_id)
        .bind(period_start)
        .bind(period_end)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        if pending_charges.is_empty() {
            return Err(AppError::BadRequest(
                "No pending charges found for this plan in the specified period".to_string(),
            ));
        }

        // Compute totals
        let subtotal: f64 = pending_charges.iter().map(|c| c.gross_amount).sum();
        let total_adj: f64 = pending_charges.iter().map(|c| c.adjustment_total).sum();
        let total_amount: f64 = pending_charges.iter().map(|c| c.net_amount).sum();
        let subtotal = (subtotal * 100.0).round() / 100.0;
        let total_adj = (total_adj * 100.0).round() / 100.0;
        let total_amount = (total_amount * 100.0).round() / 100.0;

        // Generate invoice number: INV-{YYYYMM}-{plan_id last 6}
        let inv_prefix = format!(
            "INV-{}-{}",
            period_end.format("%Y%m"),
            &req.plan_id[req.plan_id.len().saturating_sub(6)..].to_uppercase()
        );
        // Check for existing invoices with this prefix and increment suffix
        let existing_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM invoices WHERE org_id = ? AND invoice_number LIKE ?"
        )
        .bind(org_id)
        .bind(format!("{}%", inv_prefix))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
        let invoice_number = if existing_count.0 == 0 {
            inv_prefix
        } else {
            format!("{}-{}", inv_prefix, existing_count.0 + 1)
        };

        let invoice_id = Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO invoices
             (id, org_id, plan_id, invoice_number, billing_period_start, billing_period_end,
              subtotal, total_adjustments, total_amount, status, generated_by, notes)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'draft', ?, ?)"
        )
        .bind(&invoice_id)
        .bind(org_id)
        .bind(&req.plan_id)
        .bind(&invoice_number)
        .bind(period_start)
        .bind(period_end)
        .bind(subtotal)
        .bind(total_adj)
        .bind(total_amount)
        .bind(user_id)
        .bind(&req.notes)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        // Build line items and link charges to invoice
        let mut line_items = Vec::new();
        for charge in &pending_charges {
            let line_id = Uuid::new_v4().to_string();

            // Fetch the delivery date for description
            let delivery_info: Option<(NaiveDate, String)> = sqlx::query_as(
                "SELECT d.delivery_date, COALESCE(s.name, 'Service') as svc_name
                 FROM delivery_entries d
                 LEFT JOIN service_catalog_items s ON s.id = d.service_item_id
                 WHERE d.id = ?"
            )
            .bind(&charge.delivery_entry_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

            let (delivery_date, svc_name) = delivery_info.unwrap_or_else(|| {
                (period_start, "Service".to_string())
            });

            let description = format!("{} - {} ({} x ${:.2})",
                svc_name,
                charge.rule_type,
                charge.computed_units,
                charge.rate_applied,
            );

            sqlx::query(
                "INSERT INTO invoice_line_items
                 (id, invoice_id, charge_id, description, delivery_date, units, unit_rate,
                  gross_amount, adjustment_amount, net_amount)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&line_id)
            .bind(&invoice_id)
            .bind(&charge.id)
            .bind(&description)
            .bind(delivery_date)
            .bind(charge.computed_units)
            .bind(charge.rate_applied)
            .bind(charge.gross_amount)
            .bind(charge.adjustment_total)
            .bind(charge.net_amount)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

            // Mark charge as invoiced
            sqlx::query(
                "UPDATE charges SET invoice_id = ?, status = 'invoiced' WHERE id = ?"
            )
            .bind(&invoice_id)
            .bind(&charge.id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

            let line: InvoiceLineItemRow = sqlx::query_as(
                "SELECT id, invoice_id, charge_id, description, delivery_date, units, unit_rate,
                        gross_amount, adjustment_amount, net_amount, created_at
                 FROM invoice_line_items WHERE id = ?"
            )
            .bind(&line_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
            line_items.push(line);
        }

        self.audit.log(AuditEntry {
            user_id: Some(user_id.to_string()),
            ip_address: None,
            trace_id: None,
            action: "billing.invoice.generated".to_string(),
            resource_type: "invoice".to_string(),
            resource_id: Some(invoice_id.clone()),
            org_id: Some(org_id.to_string()),
            details: Some(serde_json::json!({
                "invoice_number": invoice_number,
                "total_amount": total_amount,
                "line_count": line_items.len(),
            })),
        }).await;

        let invoice: InvoiceRow = sqlx::query_as(
            "SELECT id, org_id, plan_id, invoice_number, billing_period_start, billing_period_end,
                    subtotal, total_adjustments, total_amount, status, generated_by, notes, created_at, updated_at
             FROM invoices WHERE id = ?"
        )
        .bind(&invoice_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(InvoiceDetail { invoice, line_items })
    }

    pub async fn list_invoices(
        &self,
        org_id: &str,
        plan_id: Option<&str>,
        status_filter: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<InvoiceRow>, i64), AppError> {
        let mut where_clause = "WHERE org_id = ?".to_string();
        let mut binds: Vec<String> = vec![org_id.to_string()];

        if let Some(p) = plan_id {
            where_clause.push_str(" AND plan_id = ?");
            binds.push(p.to_string());
        }
        if let Some(s) = status_filter {
            where_clause.push_str(" AND status = ?");
            binds.push(s.to_string());
        }

        let count_q = format!("SELECT COUNT(*) FROM invoices {}", where_clause);
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_q);
        for b in &binds { cq = cq.bind(b); }
        let (total,) = cq.fetch_one(&self.pool).await.map_err(|e| AppError::Internal(e.to_string()))?;

        let data_q = format!(
            "SELECT id, org_id, plan_id, invoice_number, billing_period_start, billing_period_end,
                    subtotal, total_adjustments, total_amount, status, generated_by, notes, created_at, updated_at
             FROM invoices {} ORDER BY created_at DESC LIMIT ? OFFSET ?",
            where_clause
        );
        let mut dq = sqlx::query_as::<_, InvoiceRow>(&data_q);
        for b in &binds { dq = dq.bind(b); }
        let rows = dq.bind(limit).bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok((rows, total))
    }

    pub async fn get_invoice_detail(
        &self,
        invoice_id: &str,
        org_id: &str,
    ) -> Result<InvoiceDetail, AppError> {
        let invoice: InvoiceRow = sqlx::query_as(
            "SELECT id, org_id, plan_id, invoice_number, billing_period_start, billing_period_end,
                    subtotal, total_adjustments, total_amount, status, generated_by, notes, created_at, updated_at
             FROM invoices WHERE id = ? AND org_id = ?"
        )
        .bind(invoice_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Invoice not found".to_string()))?;

        let line_items: Vec<InvoiceLineItemRow> = sqlx::query_as(
            "SELECT id, invoice_id, charge_id, description, delivery_date, units, unit_rate,
                    gross_amount, adjustment_amount, net_amount, created_at
             FROM invoice_line_items WHERE invoice_id = ? ORDER BY delivery_date ASC"
        )
        .bind(invoice_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(InvoiceDetail { invoice, line_items })
    }

    pub async fn update_invoice_status(
        &self,
        invoice_id: &str,
        org_id: &str,
        user_id: &str,
        req: &UpdateInvoiceStatusRequest,
    ) -> Result<InvoiceRow, AppError> {
        let invoice: InvoiceRow = sqlx::query_as(
            "SELECT id, org_id, plan_id, invoice_number, billing_period_start, billing_period_end,
                    subtotal, total_adjustments, total_amount, status, generated_by, notes, created_at, updated_at
             FROM invoices WHERE id = ? AND org_id = ?"
        )
        .bind(invoice_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Invoice not found".to_string()))?;

        crate::domain::billing_types::validate_invoice_status_transition(
            &invoice.status,
            &req.status,
        )
        .map_err(|e| AppError::BadRequest(e))?;

        sqlx::query("UPDATE invoices SET status = ? WHERE id = ?")
            .bind(&req.status)
            .bind(invoice_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        self.audit.log(AuditEntry {
            user_id: Some(user_id.to_string()),
            ip_address: None,
            trace_id: None,
            action: "billing.invoice.status_updated".to_string(),
            resource_type: "invoice".to_string(),
            resource_id: Some(invoice_id.to_string()),
            org_id: Some(org_id.to_string()),
            details: Some(serde_json::json!({
                "from": invoice.status,
                "to": req.status,
            })),
        }).await;

        let updated: InvoiceRow = sqlx::query_as(
            "SELECT id, org_id, plan_id, invoice_number, billing_period_start, billing_period_end,
                    subtotal, total_adjustments, total_amount, status, generated_by, notes, created_at, updated_at
             FROM invoices WHERE id = ?"
        )
        .bind(invoice_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(updated)
    }
}

// ------------------------------------------------------------------
// Tiered billing computation helper
// ------------------------------------------------------------------

fn compute_tiered_amount(units: f64, tier_config_json: &str, base_rate: f64) -> f64 {
    let tiers: Vec<serde_json::Value> = match serde_json::from_str(tier_config_json) {
        Ok(v) => v,
        Err(_) => return units * base_rate,
    };

    let mut remaining = units;
    let mut total = 0.0;
    let mut prev_up_to = 0.0;

    for tier in &tiers {
        let rate = tier["rate"].as_f64().unwrap_or(base_rate);
        let up_to = tier["up_to"].as_f64();

        let tier_max = up_to.unwrap_or(f64::INFINITY);
        let tier_units = (remaining).min(tier_max - prev_up_to);

        if tier_units <= 0.0 {
            break;
        }

        total += tier_units * rate;
        remaining -= tier_units;
        prev_up_to = tier_max;

        if remaining <= 0.0 {
            break;
        }
    }

    (total * 100.0).round() / 100.0
}
