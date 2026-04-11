/// Reporting service: order volume, revenue, utilization, KPI analytics.
/// All results are derived from real stored data. No mocks.
///
/// Reports accept ISO date strings (YYYY-MM-DD) for from_date and to_date,
/// and optional department_id / project_id filters.

use chrono::NaiveDate;
use sqlx::MySqlPool;

use crate::application::chaos_service::ChaosService;
use crate::application::degradation_service::{DegradationService, TOGGLE_ANALYTICS};
use crate::domain::error::AppError;
use crate::domain::scoring_types::{
    KpiSummary, OrderVolumeRow, ReportFilters, RevenueReportRow, UtilizationRow,
};

#[derive(Clone)]
pub struct ReportService {
    pool: MySqlPool,
    degradation: DegradationService,
}

impl ReportService {
    pub fn new(pool: MySqlPool, degradation: DegradationService) -> Self {
        Self { pool, degradation }
    }

    async fn check_analytics_enabled(&self) -> Result<(), AppError> {
        if !self.degradation.get_flag(TOGGLE_ANALYTICS).await {
            tracing::warn!("Report rejected: analytics_enabled=false");
            return Err(AppError::ServiceUnavailable(
                "Heavy analytics are temporarily disabled. Contact your system administrator.".to_string()
            ));
        }
        ChaosService::maybe_inject_latency().await;
        Ok(())
    }

    fn validate_filters(filters: &ReportFilters) -> Result<(), AppError> {
        NaiveDate::parse_from_str(&filters.from_date, "%Y-%m-%d")
            .map_err(|_| AppError::BadRequest("Invalid from_date format — expected YYYY-MM-DD".to_string()))?;
        NaiveDate::parse_from_str(&filters.to_date, "%Y-%m-%d")
            .map_err(|_| AppError::BadRequest("Invalid to_date format — expected YYYY-MM-DD".to_string()))?;
        // Validate service_route if provided: must be non-empty, max 100 chars
        if let Some(ref route) = filters.service_route {
            let trimmed = route.trim();
            if trimmed.is_empty() {
                return Err(AppError::BadRequest(
                    "service_route must be a non-empty string when provided".to_string(),
                ));
            }
            if trimmed.len() > 100 {
                return Err(AppError::BadRequest(
                    "service_route must not exceed 100 characters".to_string(),
                ));
            }
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // Order volume: deliveries grouped by week (YYYY-WNN)
    // ------------------------------------------------------------------

    pub async fn order_volume(
        &self,
        org_id: &str,
        filters: &ReportFilters,
    ) -> Result<Vec<OrderVolumeRow>, AppError> {
        Self::validate_filters(filters)?;
        self.check_analytics_enabled().await?;
        // Build optional JOIN/WHERE fragments for department, project, route
        let dept_join = if filters.department_id.is_some() {
            "JOIN client_plans cp2 ON cp2.id = de.plan_id AND cp2.department_id = ?"
        } else {
            ""
        };
        let project_join = if filters.project_id.is_some() {
            "JOIN client_plans cp3 ON cp3.id = de.plan_id AND cp3.project_id = ?"
        } else {
            ""
        };
        let route_clause = if filters.service_route.is_some() {
            "AND cp.service_route = ?"
        } else {
            ""
        };

        let sql = format!(
            "SELECT
                DATE_FORMAT(de.delivery_date, '%Y-W%v') AS period,
                COUNT(de.id)                              AS delivery_count,
                COUNT(DISTINCT de.plan_id)                AS unique_plans,
                COUNT(DISTINCT de.provider_id)            AS unique_providers
             FROM delivery_entries de
             JOIN client_plans cp ON cp.id = de.plan_id AND cp.org_id = ?
             {} {}
             WHERE de.delivery_date BETWEEN ? AND ?
             {}
             GROUP BY period
             ORDER BY period ASC
             LIMIT ? OFFSET ?",
            dept_join, project_join, route_clause
        );

        let mut q = sqlx::query_as::<_, (String, i64, i64, i64)>(&sql).bind(org_id);
        if let Some(d) = &filters.department_id { q = q.bind(d); }
        if let Some(p) = &filters.project_id   { q = q.bind(p); }
        let mut q = q
            .bind(&filters.from_date)
            .bind(&filters.to_date);
        if let Some(r) = &filters.service_route { q = q.bind(r); }
        let q = q
            .bind(filters.limit.unwrap_or(200))
            .bind(filters.offset.unwrap_or(0));

        let rows = q
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|(period, delivery_count, unique_plans, unique_providers)| OrderVolumeRow {
                period,
                delivery_count,
                unique_plans,
                unique_providers,
            })
            .collect())
    }

    // ------------------------------------------------------------------
    // Revenue report: invoice/payment/refund aggregates by week
    // ------------------------------------------------------------------

    pub async fn revenue_report(
        &self,
        org_id: &str,
        filters: &ReportFilters,
    ) -> Result<Vec<RevenueReportRow>, AppError> {
        Self::validate_filters(filters)?;
        self.check_analytics_enabled().await?;
        let dept_join = if filters.department_id.is_some() {
            "JOIN client_plans cp2 ON cp2.id = i.plan_id AND cp2.department_id = ?"
        } else {
            ""
        };
        let project_join = if filters.project_id.is_some() {
            "JOIN client_plans cp3 ON cp3.id = i.plan_id AND cp3.project_id = ?"
        } else {
            ""
        };
        let route_clause = if filters.service_route.is_some() {
            "AND cp.service_route = ?"
        } else {
            ""
        };

        // CAST SUM results from DECIMAL to DOUBLE to match the f64 tuple type.
        let sql = format!(
            "SELECT
                DATE_FORMAT(i.created_at, '%Y-W%v')  AS period,
                CAST(COALESCE(SUM(i.subtotal), 0) AS DOUBLE)          AS gross_charges,
                CAST(COALESCE(SUM(i.total_amount), 0) AS DOUBLE)      AS net_charges,
                CAST(COALESCE(SUM(i.total_amount), 0) AS DOUBLE)      AS total_invoiced,
                CAST(COALESCE(SUM(
                    (SELECT COALESCE(SUM(rp.amount), 0)
                     FROM recorded_payments rp WHERE rp.invoice_id = i.id)
                ), 0) AS DOUBLE)                                      AS total_paid,
                CAST(COALESCE(SUM(
                    (SELECT COALESCE(SUM(rr.amount), 0)
                     FROM recorded_refunds rr WHERE rr.invoice_id = i.id)
                ), 0) AS DOUBLE)                                      AS total_refunded
             FROM invoices i
             JOIN client_plans cp ON cp.id = i.plan_id AND cp.org_id = ?
             {} {}
             WHERE DATE(i.created_at) BETWEEN ? AND ?
             {}
             GROUP BY period
             ORDER BY period ASC
             LIMIT ? OFFSET ?",
            dept_join, project_join, route_clause
        );

        let mut q = sqlx::query_as::<_, (String, f64, f64, f64, f64, f64)>(&sql).bind(org_id);
        if let Some(d) = &filters.department_id { q = q.bind(d); }
        if let Some(p) = &filters.project_id   { q = q.bind(p); }
        let mut q = q
            .bind(&filters.from_date)
            .bind(&filters.to_date);
        if let Some(r) = &filters.service_route { q = q.bind(r); }
        let q = q
            .bind(filters.limit.unwrap_or(200))
            .bind(filters.offset.unwrap_or(0));

        let rows = q
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|(period, gross_charges, net_charges, total_invoiced, total_paid, total_refunded)| {
                let refund_rate_pct = if total_paid > 0.0 {
                    (total_refunded / total_paid) * 100.0
                } else {
                    0.0
                };
                RevenueReportRow {
                    period,
                    gross_charges,
                    net_charges,
                    total_invoiced,
                    total_paid,
                    total_refunded,
                    refund_rate_pct: round_2dp(refund_rate_pct),
                }
            })
            .collect())
    }

    // ------------------------------------------------------------------
    // Provider utilization: visits/units/mileage per provider per week
    // ------------------------------------------------------------------

    pub async fn utilization_report(
        &self,
        org_id: &str,
        filters: &ReportFilters,
    ) -> Result<Vec<UtilizationRow>, AppError> {
        Self::validate_filters(filters)?;
        self.check_analytics_enabled().await?;
        let dept_join = if filters.department_id.is_some() {
            "JOIN client_plans cp2 ON cp2.id = de.plan_id AND cp2.department_id = ?"
        } else {
            ""
        };
        let project_join = if filters.project_id.is_some() {
            "JOIN client_plans cp3 ON cp3.id = de.plan_id AND cp3.project_id = ?"
        } else {
            ""
        };
        let route_clause = if filters.service_route.is_some() {
            "AND cp.service_route = ?"
        } else {
            ""
        };

        // CAST DECIMAL SUMs to DOUBLE to match the f64 tuple type.
        let sql = format!(
            "SELECT
                de.provider_id,
                DATE_FORMAT(de.delivery_date, '%Y-W%v') AS period,
                COUNT(de.id)                                            AS total_visits,
                CAST(COALESCE(SUM(de.units), 0) AS DOUBLE)              AS total_units,
                CAST(COALESCE(SUM(de.mileage), 0) AS DOUBLE)            AS total_mileage
             FROM delivery_entries de
             JOIN client_plans cp ON cp.id = de.plan_id AND cp.org_id = ?
             {} {}
             WHERE de.delivery_date BETWEEN ? AND ?
               AND de.status = 'verified'
             {}
             GROUP BY de.provider_id, period
             ORDER BY period ASC, de.provider_id ASC
             LIMIT ? OFFSET ?",
            dept_join, project_join, route_clause
        );

        let mut q = sqlx::query_as::<_, (String, String, i64, f64, f64)>(&sql).bind(org_id);
        if let Some(d) = &filters.department_id { q = q.bind(d); }
        if let Some(p) = &filters.project_id   { q = q.bind(p); }
        let mut q = q
            .bind(&filters.from_date)
            .bind(&filters.to_date);
        if let Some(r) = &filters.service_route { q = q.bind(r); }
        let q = q
            .bind(filters.limit.unwrap_or(500))
            .bind(filters.offset.unwrap_or(0));

        let rows = q
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|(provider_id, period, total_visits, total_units, total_mileage)| UtilizationRow {
                provider_id,
                period,
                total_visits,
                total_units,
                total_mileage,
            })
            .collect())
    }

    // ------------------------------------------------------------------
    // KPI summary
    // Attendance rate: verified / (submitted + verified)
    // Repurchase rate: plans with 2+ distinct billing periods / total plans
    // Staff utilization: avg deliveries per active provider vs capacity proxy (20/week)
    // ------------------------------------------------------------------

    pub async fn kpi_summary(
        &self,
        org_id: &str,
        filters: &ReportFilters,
    ) -> Result<KpiSummary, AppError> {
        Self::validate_filters(filters)?;
        self.check_analytics_enabled().await?;
        // 1. Attendance rate
        let (submitted_count, verified_count): (i64, i64) = {
            let row: (i64, i64) = sqlx::query_as(
                "SELECT
                    CAST(COALESCE(SUM(CASE WHEN de.status = 'submitted' THEN 1 ELSE 0 END), 0) AS SIGNED),
                    CAST(COALESCE(SUM(CASE WHEN de.status = 'verified'  THEN 1 ELSE 0 END), 0) AS SIGNED)
                 FROM delivery_entries de
                 JOIN client_plans cp ON cp.id = de.plan_id AND cp.org_id = ?
                 WHERE de.delivery_date BETWEEN ? AND ?"
            )
            .bind(org_id)
            .bind(&filters.from_date)
            .bind(&filters.to_date)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
            row
        };

        let attendance_rate_pct = {
            let total = submitted_count + verified_count;
            if total > 0 {
                (verified_count as f64 / total as f64) * 100.0
            } else {
                0.0
            }
        };

        // 2. Repurchase rate: plans with 2+ invoice billing periods / total plans
        let (plans_with_repeat, total_plans): (i64, i64) = {
            let row: (i64, i64) = sqlx::query_as(
                "SELECT
                    CAST(COALESCE(SUM(CASE WHEN invoice_periods >= 2 THEN 1 ELSE 0 END), 0) AS SIGNED),
                    COUNT(*)
                 FROM (
                    SELECT cp.id, COUNT(DISTINCT DATE_FORMAT(i.created_at, '%Y-%m')) AS invoice_periods
                    FROM client_plans cp
                    LEFT JOIN invoices i ON i.plan_id = cp.id
                        AND DATE(i.created_at) BETWEEN ? AND ?
                    WHERE cp.org_id = ?
                    GROUP BY cp.id
                 ) plan_periods"
            )
            .bind(&filters.from_date)
            .bind(&filters.to_date)
            .bind(org_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
            row
        };

        let repurchase_rate_pct = if total_plans > 0 {
            (plans_with_repeat as f64 / total_plans as f64) * 100.0
        } else {
            0.0
        };

        // 3. Staff utilization: avg deliveries per active provider / 20 (capacity proxy per week)
        let (total_deliveries, provider_count): (i64, i64) = {
            let row: (i64, i64) = sqlx::query_as(
                "SELECT COUNT(de.id), COUNT(DISTINCT de.provider_id)
                 FROM delivery_entries de
                 JOIN client_plans cp ON cp.id = de.plan_id AND cp.org_id = ?
                 WHERE de.delivery_date BETWEEN ? AND ?"
            )
            .bind(org_id)
            .bind(&filters.from_date)
            .bind(&filters.to_date)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
            row
        };

        // Approximate weeks in range
        let weeks: f64 = {
            let from = chrono::NaiveDate::parse_from_str(&filters.from_date, "%Y-%m-%d")
                .unwrap_or_default();
            let to = chrono::NaiveDate::parse_from_str(&filters.to_date, "%Y-%m-%d")
                .unwrap_or_default();
            let days = (to - from).num_days().max(1) as f64;
            (days / 7.0).ceil()
        };

        let staff_utilization_pct = if provider_count > 0 && weeks > 0.0 {
            let avg_per_provider_per_week = total_deliveries as f64 / provider_count as f64 / weeks;
            // Capacity proxy: 20 deliveries per week = 100%
            (avg_per_provider_per_week / 20.0 * 100.0).min(100.0)
        } else {
            0.0
        };

        // 4. Average quality score
        let avg_score: Option<f64> = {
            let row: (Option<f64>,) = sqlx::query_as(
                "SELECT AVG(e.final_score)
                 FROM evaluations e
                 WHERE e.org_id = ?
                   AND e.status = 'finalized'
                   AND DATE(e.updated_at) BETWEEN ? AND ?"
            )
            .bind(org_id)
            .bind(&filters.from_date)
            .bind(&filters.to_date)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
            row.0
        };

        // 5. Second review rate
        let (total_evals, second_review_evals): (i64, i64) = {
            let row: (i64, i64) = sqlx::query_as(
                "SELECT COUNT(*), CAST(COALESCE(SUM(CASE WHEN requires_second_review = 1 THEN 1 ELSE 0 END), 0) AS SIGNED)
                 FROM evaluations
                 WHERE org_id = ?
                   AND DATE(created_at) BETWEEN ? AND ?"
            )
            .bind(org_id)
            .bind(&filters.from_date)
            .bind(&filters.to_date)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
            row
        };

        let second_review_rate_pct = if total_evals > 0 {
            (second_review_evals as f64 / total_evals as f64) * 100.0
        } else {
            0.0
        };

        Ok(KpiSummary {
            period_start: filters.from_date.clone(),
            period_end: filters.to_date.clone(),
            // NOTE: all _pct values are already on the 0–100 percentage scale
            // (ratio × 100 was applied above).  round_2dp rounds to 2 decimal
            // places — it does NOT convert to percentage again.
            attendance_rate_pct: round_2dp(attendance_rate_pct),
            repurchase_rate_pct: round_2dp(repurchase_rate_pct),
            staff_utilization_pct: round_2dp(staff_utilization_pct),
            avg_score: avg_score.map(round_2dp),
            second_review_rate_pct: round_2dp(second_review_rate_pct),
        })
    }
}

/// Round an f64 to two decimal places.  Used for KPI output formatting.
/// This is purely a rounding operation — it does NOT convert fractions to
/// percentages.  Values are expected to already be on their final scale.
fn round_2dp(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}
