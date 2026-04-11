/// Export service: permission-aware CSV/JSON data exports with masking defaults.
///
/// By default all exports mask identifying fields (client_name → "****",
/// provider_id → masked UUID prefix). The caller must hold `api.export.unmasked`
/// permission to receive real identifiers.
///
/// Every export event is recorded in the export_audit_logs table.

use sqlx::MySqlPool;
use uuid::Uuid;

use crate::application::chaos_service::ChaosService;
use crate::application::degradation_service::{DegradationService, TOGGLE_EXPORTS};
use crate::domain::error::AppError;
use crate::domain::scoring_types::{ExportRequest, ExportResult};
use crate::infrastructure::audit::{AuditEntry, AuditService};
use crate::infrastructure::permission_cache::PermissionCache;

#[derive(Clone)]
pub struct ExportService {
    pool: MySqlPool,
    audit: AuditService,
    degradation: DegradationService,
}

impl ExportService {
    pub fn new(pool: MySqlPool, audit: AuditService, degradation: DegradationService) -> Self {
        Self { pool, audit, degradation }
    }

    pub async fn export(
        &self,
        org_id: &str,
        exported_by: &str,
        perm_cache: &PermissionCache,
        req: &ExportRequest,
    ) -> Result<ExportResult, AppError> {
        // Check degradation toggle — exports can be centrally disabled
        if !self.degradation.get_flag(TOGGLE_EXPORTS).await {
            tracing::warn!(actor = exported_by, "Export rejected: exports_enabled=false");
            return Err(AppError::ServiceUnavailable(
                "Exports are temporarily disabled. Contact your system administrator.".to_string()
            ));
        }

        // Inject chaos latency if a drill is active
        ChaosService::maybe_inject_latency().await;

        // Validate dates
        if chrono::NaiveDate::parse_from_str(&req.from_date, "%Y-%m-%d").is_err() {
            return Err(AppError::BadRequest("Invalid from_date format — expected YYYY-MM-DD".to_string()));
        }
        if chrono::NaiveDate::parse_from_str(&req.to_date, "%Y-%m-%d").is_err() {
            return Err(AppError::BadRequest("Invalid to_date format — expected YYYY-MM-DD".to_string()));
        }

        // Determine masking: default masked unless user has EXPORT_UNMASKED and explicitly requests it
        let wants_unmasked = req.unmasked.unwrap_or(false);
        let has_unmasked_perm = perm_cache
            .has_permission(exported_by, crate::domain::auth_policy::api::EXPORT_UNMASKED)
            .await
            .unwrap_or(false);
        let unmasked = wants_unmasked && has_unmasked_perm;
        let masked = !unmasked;

        let rows = match req.export_type.as_str() {
            "deliveries" => {
                self.export_deliveries(org_id, &req.from_date, &req.to_date, req.department_id.as_deref(), req.project_id.as_deref(), req.service_route.as_deref(), masked).await?
            }
            "evaluations" => {
                self.export_evaluations(org_id, &req.from_date, &req.to_date, req.department_id.as_deref(), req.project_id.as_deref(), req.service_route.as_deref(), masked).await?
            }
            "revenue" => {
                self.export_revenue(org_id, &req.from_date, &req.to_date, req.department_id.as_deref(), req.project_id.as_deref(), req.service_route.as_deref(), masked).await?
            }
            other => {
                return Err(AppError::BadRequest(format!(
                    "Unknown export_type '{}' — must be 'deliveries', 'evaluations', or 'revenue'",
                    other
                )));
            }
        };

        let row_count = rows.len();

        // Record export audit log
        let log_id = Uuid::new_v4().to_string();
        let filters_json = serde_json::json!({
            "from_date": req.from_date,
            "to_date": req.to_date,
            "department_id": req.department_id,
            "project_id": req.project_id,
            "service_route": req.service_route,
        })
        .to_string();

        let permission_used: Option<&str> = if !masked {
            Some(crate::domain::auth_policy::api::EXPORT_UNMASKED)
        } else {
            None
        };

        sqlx::query(
            "INSERT INTO export_audit_logs
             (id, org_id, exported_by, export_type, filters_json, row_count, masked, permission_used)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&log_id)
        .bind(org_id)
        .bind(exported_by)
        .bind(&req.export_type)
        .bind(&filters_json)
        .bind(row_count as i32)
        .bind(masked)
        .bind(permission_used)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        self.audit.log(AuditEntry {
            user_id: Some(exported_by.to_string()),
            action: "export.performed".to_string(),
            resource_type: "export_audit_log".to_string(),
            resource_id: Some(log_id.clone()),
            org_id: Some(org_id.to_string()),
            details: Some(serde_json::json!({
                "export_type": req.export_type,
                "row_count": row_count,
                "masked": masked,
            })),
            ip_address: None,
            trace_id: None,
        }).await;

        Ok(ExportResult {
            rows,
            row_count,
            masked,
            export_log_id: log_id,
        })
    }

    // ------------------------------------------------------------------
    // Delivery export
    // ------------------------------------------------------------------

    async fn export_deliveries(
        &self,
        org_id: &str,
        from_date: &str,
        to_date: &str,
        department_id: Option<&str>,
        project_id: Option<&str>,
        service_route: Option<&str>,
        masked: bool,
    ) -> Result<Vec<serde_json::Value>, AppError> {
        let mut extra_clauses = String::new();
        if department_id.is_some() {
            extra_clauses.push_str(" AND cp.department_id = ?");
        }
        if project_id.is_some() {
            extra_clauses.push_str(" AND cp.project_id = ?");
        }
        if service_route.is_some() {
            extra_clauses.push_str(" AND cp.service_route = ?");
        }

        // CAST DATE and TIME columns to CHAR so sqlx decodes them as String
        // (matching the tuple type below).  de.units is DECIMAL so cast to DOUBLE.
        let sql = format!(
            "SELECT
                de.id,
                CAST(de.delivery_date AS CHAR) AS delivery_date,
                de.plan_id,
                cp.client_name,
                de.provider_id,
                de.service_item_id,
                si.name       AS service_name,
                CAST(de.units AS DOUBLE) AS units,
                CAST(de.mileage AS DOUBLE) AS mileage,
                de.status,
                CAST(de.start_time AS CHAR) AS start_time,
                CAST(de.end_time AS CHAR) AS end_time
             FROM delivery_entries de
             JOIN client_plans cp ON cp.id = de.plan_id AND cp.org_id = ?
             JOIN service_catalog_items si ON si.id = de.service_item_id
             WHERE de.delivery_date BETWEEN ? AND ?
             {}
             ORDER BY de.delivery_date ASC, de.id ASC
             LIMIT 10000",
            extra_clauses
        );

        let mut q = sqlx::query_as::<
            _,
            (
                String, String, String, String, String, String, String,
                f64, Option<f64>, String, Option<String>, Option<String>,
            ),
        >(&sql)
        .bind(org_id)
        .bind(from_date)
        .bind(to_date);

        if let Some(d) = department_id {
            q = q.bind(d);
        }
        if let Some(p) = project_id {
            q = q.bind(p);
        }
        if let Some(r) = service_route {
            q = q.bind(r);
        }

        let rows = q
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(
                |(id, delivery_date, plan_id, client_name, provider_id, service_item_id,
                  service_name, units, mileage, status, start_time, end_time)| {
                    serde_json::json!({
                        "id": id,
                        "delivery_date": delivery_date,
                        "plan_id": if masked { mask_id(&plan_id) } else { plan_id },
                        "client_name": if masked { "****".to_string() } else { client_name },
                        "provider_id": if masked { mask_id(&provider_id) } else { provider_id },
                        "service_item_id": service_item_id,
                        "service_name": service_name,
                        "units": units,
                        "mileage": mileage,
                        "status": status,
                        "start_time": start_time,
                        "end_time": end_time,
                    })
                },
            )
            .collect())
    }

    // ------------------------------------------------------------------
    // Evaluations export
    // ------------------------------------------------------------------

    async fn export_evaluations(
        &self,
        org_id: &str,
        from_date: &str,
        to_date: &str,
        department_id: Option<&str>,
        project_id: Option<&str>,
        service_route: Option<&str>,
        masked: bool,
    ) -> Result<Vec<serde_json::Value>, AppError> {
        // When filtering by department, project, or route we must join through
        // delivery_entries → client_plans to reach the scope columns.
        let needs_plan_join = department_id.is_some() || project_id.is_some() || service_route.is_some();
        let plan_join = if needs_plan_join {
            "JOIN delivery_entries de ON de.id = e.delivery_entry_id \
             JOIN client_plans cp ON cp.id = de.plan_id"
        } else {
            ""
        };
        let mut extra_clauses = String::new();
        if department_id.is_some() {
            extra_clauses.push_str(" AND cp.department_id = ?");
        }
        if project_id.is_some() {
            extra_clauses.push_str(" AND cp.project_id = ?");
        }
        if service_route.is_some() {
            extra_clauses.push_str(" AND cp.service_route = ?");
        }

        // CAST DECIMAL scores to DOUBLE and DATETIME timestamps to CHAR
        // so sqlx decodes them as Option<f64> and String (matching the tuple below).
        let sql = format!(
            "SELECT
                e.id,
                e.delivery_entry_id,
                e.template_id,
                st.name   AS template_name,
                e.evaluator_id,
                CAST(e.raw_score AS DOUBLE) AS raw_score,
                CAST(e.weighted_score AS DOUBLE) AS weighted_score,
                CAST(e.final_score AS DOUBLE) AS final_score,
                CAST(e.score_delta AS DOUBLE) AS score_delta,
                e.status,
                CAST(e.created_at AS CHAR) AS created_at,
                CAST(e.updated_at AS CHAR) AS updated_at
             FROM evaluations e
             JOIN scoring_templates st ON st.id = e.template_id
             {}
             WHERE e.org_id = ?
               AND DATE(e.created_at) BETWEEN ? AND ?
             {}
             ORDER BY e.created_at ASC
             LIMIT 10000",
            plan_join, extra_clauses
        );

        let mut q = sqlx::query_as::<
            _,
            (
                String, String, String, String, String, Option<f64>, Option<f64>, Option<f64>,
                Option<f64>, String, String, String,
            ),
        >(&sql)
        .bind(org_id)
        .bind(from_date)
        .bind(to_date);

        if let Some(d) = department_id {
            q = q.bind(d);
        }
        if let Some(p) = project_id {
            q = q.bind(p);
        }
        if let Some(r) = service_route {
            q = q.bind(r);
        }

        let rows = q
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(
                |(id, delivery_entry_id, template_id, template_name, evaluator_id,
                  raw_score, weighted_score, final_score, score_delta, status, created_at, updated_at)| {
                    serde_json::json!({
                        "id": id,
                        "delivery_entry_id": delivery_entry_id,
                        "template_id": template_id,
                        "template_name": template_name,
                        "evaluator_id": if masked { mask_id(&evaluator_id) } else { evaluator_id },
                        "raw_score": raw_score,
                        "weighted_score": weighted_score,
                        "final_score": final_score,
                        "score_delta": score_delta,
                        "status": status,
                        "created_at": created_at,
                        "updated_at": updated_at,
                    })
                },
            )
            .collect())
    }

    // ------------------------------------------------------------------
    // Revenue export
    // ------------------------------------------------------------------

    async fn export_revenue(
        &self,
        org_id: &str,
        from_date: &str,
        to_date: &str,
        department_id: Option<&str>,
        project_id: Option<&str>,
        service_route: Option<&str>,
        masked: bool,
    ) -> Result<Vec<serde_json::Value>, AppError> {
        let mut extra_clauses = String::new();
        if department_id.is_some() {
            extra_clauses.push_str(" AND cp.department_id = ?");
        }
        if project_id.is_some() {
            extra_clauses.push_str(" AND cp.project_id = ?");
        }
        if service_route.is_some() {
            extra_clauses.push_str(" AND cp.service_route = ?");
        }

        // CAST DECIMAL amounts to DOUBLE and DATE columns to CHAR
        // so sqlx decodes them matching the tuple type below.
        let sql = format!(
            "SELECT
                i.id,
                i.invoice_number,
                i.plan_id,
                cp.client_name,
                CAST(i.subtotal AS DOUBLE) AS subtotal,
                CAST(i.total_adjustments AS DOUBLE) AS total_adjustments,
                CAST(i.total_amount AS DOUBLE) AS total_amount,
                i.status,
                CAST(i.billing_period_start AS CHAR) AS billing_period_start,
                CAST(i.billing_period_end AS CHAR) AS billing_period_end
             FROM invoices i
             JOIN client_plans cp ON cp.id = i.plan_id AND cp.org_id = ?
             WHERE DATE(i.created_at) BETWEEN ? AND ?
             {}
             ORDER BY i.created_at ASC
             LIMIT 10000",
            extra_clauses
        );

        let mut q = sqlx::query_as::<
            _,
            (String, String, String, String, f64, f64, f64, String, String, String),
        >(&sql)
        .bind(org_id)
        .bind(from_date)
        .bind(to_date);

        if let Some(d) = department_id {
            q = q.bind(d);
        }
        if let Some(p) = project_id {
            q = q.bind(p);
        }
        if let Some(r) = service_route {
            q = q.bind(r);
        }

        let rows = q
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(
                |(id, invoice_number, plan_id, client_name, subtotal,
                  total_adjustments, total_amount, status, billing_period_start, billing_period_end)| {
                    serde_json::json!({
                        "id": id,
                        "invoice_number": invoice_number,
                        "plan_id": if masked { mask_id(&plan_id) } else { plan_id },
                        "client_name": if masked { "****".to_string() } else { client_name },
                        "subtotal": subtotal,
                        "total_adjustments": total_adjustments,
                        "total_amount": total_amount,
                        "status": status,
                        "billing_period_start": billing_period_start,
                        "billing_period_end": billing_period_end,
                    })
                },
            )
            .collect())
    }
}

/// Returns the first 8 chars of a UUID-style id, rest replaced with "****".
/// e.g. "550e8400-e29b-..." → "550e8400-****"
fn mask_id(id: &str) -> String {
    if id.len() <= 8 {
        return "****".to_string();
    }
    format!("{}-****", &id[..8])
}
