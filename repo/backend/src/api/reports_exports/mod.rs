/// Reports & Exports API
///
/// Endpoints:
///   GET /order-volume   - delivery counts by week with optional department/project filter
///   GET /revenue        - invoiced/paid/refunded aggregates by week
///   GET /utilization    - provider visits/units/mileage by week
///   GET /kpi            - attendance rate, repurchase rate, staff utilization, avg score
///   POST /export        - permission-aware data export (masked by default)

use rocket::serde::json::Json;
use rocket::{Route, State};

use crate::api::guards::{self, AuthenticatedUser};
use crate::application::export_service::ExportService;
use crate::application::report_service::ReportService;
use crate::domain::auth_policy::{action, api};
use crate::domain::error::AppError;
use crate::domain::scoring_types::*;
use crate::infrastructure::permission_cache::PermissionCache;

// ============================================================
// Order volume
// ============================================================

#[rocket::get("/order-volume?<from_date>&<to_date>&<department_id>&<project_id>&<service_route>&<limit>&<offset>")]
pub async fn order_volume(
    from_date: String,
    to_date: String,
    department_id: Option<String>,
    project_id: Option<String>,
    service_route: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    report: &State<ReportService>,
) -> Result<Json<Vec<OrderVolumeRow>>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::REPORTS_READ).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, department_id.as_deref(), project_id.as_deref(), "read").await?;
    let filters = ReportFilters { from_date, to_date, department_id, project_id, service_route, limit, offset };
    let rows = report.order_volume(&user.org_id, &filters).await?;
    Ok(Json(rows))
}

// ============================================================
// Revenue report
// ============================================================

#[rocket::get("/revenue?<from_date>&<to_date>&<department_id>&<project_id>&<service_route>&<limit>&<offset>")]
pub async fn revenue_report(
    from_date: String,
    to_date: String,
    department_id: Option<String>,
    project_id: Option<String>,
    service_route: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    report: &State<ReportService>,
) -> Result<Json<Vec<RevenueReportRow>>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::REPORTS_READ).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, department_id.as_deref(), project_id.as_deref(), "read").await?;
    let filters = ReportFilters { from_date, to_date, department_id, project_id, service_route, limit, offset };
    let rows = report.revenue_report(&user.org_id, &filters).await?;
    Ok(Json(rows))
}

// ============================================================
// Utilization report
// ============================================================

#[rocket::get("/utilization?<from_date>&<to_date>&<department_id>&<project_id>&<service_route>&<limit>&<offset>")]
pub async fn utilization_report(
    from_date: String,
    to_date: String,
    department_id: Option<String>,
    project_id: Option<String>,
    service_route: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    report: &State<ReportService>,
) -> Result<Json<Vec<UtilizationRow>>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::REPORTS_READ).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, department_id.as_deref(), project_id.as_deref(), "read").await?;
    let filters = ReportFilters { from_date, to_date, department_id, project_id, service_route, limit, offset };
    let rows = report.utilization_report(&user.org_id, &filters).await?;
    Ok(Json(rows))
}

// ============================================================
// KPI summary
// ============================================================

#[rocket::get("/kpi?<from_date>&<to_date>&<department_id>&<project_id>&<service_route>")]
pub async fn kpi_summary(
    from_date: String,
    to_date: String,
    department_id: Option<String>,
    project_id: Option<String>,
    service_route: Option<String>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    report: &State<ReportService>,
) -> Result<Json<KpiSummary>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::REPORTS_READ).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, department_id.as_deref(), project_id.as_deref(), "read").await?;
    let filters = ReportFilters {
        from_date,
        to_date,
        department_id,
        project_id,
        service_route,
        limit: None,
        offset: None,
    };
    let summary = report.kpi_summary(&user.org_id, &filters).await?;
    Ok(Json(summary))
}

// ============================================================
// Export
// ============================================================

#[rocket::post("/export", data = "<body>")]
pub async fn export_data(
    body: Json<ExportRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    export: &State<ExportService>,
) -> Result<Json<ExportResult>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, action::EXPORT_DATA).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, body.department_id.as_deref(), body.project_id.as_deref(), "read").await?;
    let result = export.export(&user.org_id, &user.user_id, &**perm_cache, &body).await?;
    Ok(Json(result))
}

pub fn routes() -> Vec<Route> {
    rocket::routes![
        order_volume,
        revenue_report,
        utilization_report,
        kpi_summary,
        export_data,
    ]
}

#[cfg(test)]
mod tests;
