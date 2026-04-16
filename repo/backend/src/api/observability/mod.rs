/// Observability endpoints: health, readiness, metrics snapshot, and alert state.
///
/// Public (no auth): /live, /ready — used by orchestrators and load balancers.
/// Protected (api.ops.read): /metrics, /alerts, /chaos — internal operational data.

use rocket::serde::json::Json;
use rocket::Route;
use rocket::State;
use serde::Serialize;
use sqlx::MySqlPool;

use crate::api::guards::{self, AuthenticatedUser};
use crate::application::alert_engine::{AlertEngine, AlarmState};
use crate::application::chaos_service::ChaosService;
use crate::application::metrics_service::MetricsService;
use crate::domain::auth_policy::api;
use crate::domain::error::AppError;
use crate::infrastructure::permission_cache::PermissionCache;

// ============================================================
// Health / Readiness (public — no auth)
// ============================================================

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub version: String,
}

#[derive(Serialize)]
pub struct ReadinessResponse {
    pub status: String,
    pub db_ok: bool,
    pub chaos_active: bool,
}

#[rocket::get("/live")]
pub fn liveness() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        service: "careops-backend".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[rocket::get("/ready")]
pub async fn readiness(pool: &State<MySqlPool>) -> Json<ReadinessResponse> {
    let db_ok = sqlx::query("SELECT 1").execute(pool.inner()).await.is_ok();

    if !db_ok {
        tracing::error!("Readiness check failed — database unreachable");
    }

    Json(ReadinessResponse {
        status: if db_ok { "ok" } else { "degraded" }.to_string(),
        db_ok,
        chaos_active: ChaosService::drill_active(),
    })
}

// ============================================================
// Metrics snapshot (protected — api.ops.read)
// ============================================================

#[derive(Serialize)]
pub struct MetricsSnapshot {
    pub total_requests: u64,
    pub total_errors: u64,
    pub window_requests: usize,
    pub window_errors: usize,
    pub window_error_rate_pct: f64,
    pub alert_rule: &'static str,
    pub threshold_pct: f64,
}

#[rocket::get("/metrics")]
pub async fn metrics(
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    metrics_svc: &State<MetricsService>,
) -> Result<Json<MetricsSnapshot>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::OPS_READ).await?;
    let rate = metrics_svc.window_error_rate();
    Ok(Json(MetricsSnapshot {
        total_requests: metrics_svc.total_requests(),
        total_errors: metrics_svc.total_errors(),
        window_requests: metrics_svc.window_request_count(),
        window_errors: metrics_svc.window_error_count(),
        window_error_rate_pct: (rate * 10000.0).round() / 100.0,
        alert_rule: crate::application::alert_engine::ALERT_RULE_DESCRIPTION,
        threshold_pct: crate::application::alert_engine::ALERT_THRESHOLD * 100.0,
    }))
}

// ============================================================
// Current alarm state (protected — api.ops.read)
// ============================================================

#[rocket::get("/alerts")]
pub async fn alerts(
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    engine: &State<AlertEngine>,
) -> Result<Json<AlarmState>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::OPS_READ).await?;
    Ok(Json(engine.current_alarm()))
}

// ============================================================
// Chaos drill status (protected — api.ops.read)
// ============================================================

#[rocket::get("/chaos")]
pub async fn chaos_status(
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
) -> Result<Json<crate::application::chaos_service::ChaosStatus>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::OPS_READ).await?;
    Ok(Json(ChaosService::status()))
}

pub fn routes() -> Vec<Route> {
    rocket::routes![liveness, readiness, metrics, alerts, chaos_status]
}

#[cfg(test)]
mod tests;
