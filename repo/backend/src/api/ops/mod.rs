/// Operational controls API: degradation toggles.
///
/// Only System Administrators can modify toggles (api.ops.write permission).
/// Any authenticated user with api.ops.read can view current toggle state.
///
/// Endpoints:
///   GET  /ops/flags              - list all degradation flags
///   POST /ops/flags/:key/enable  - enable a flag
///   POST /ops/flags/:key/disable - disable a flag

use rocket::serde::json::Json;
use rocket::{Route, State};

use crate::api::guards::{self, AuthenticatedUser};
use crate::application::degradation_service::DegradationService;
use crate::domain::auth_policy::api;
use crate::domain::error::AppError;
use crate::infrastructure::permission_cache::PermissionCache;

// ============================================================
// List flags (read)
// ============================================================

#[rocket::get("/flags")]
pub async fn list_flags(
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    degradation: &State<DegradationService>,
) -> Result<Json<Vec<crate::application::degradation_service::OpsFlag>>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::OPS_READ).await?;
    let flags = degradation.list_flags().await?;
    Ok(Json(flags))
}

// ============================================================
// Enable a flag
// ============================================================

#[rocket::post("/flags/<key>/enable")]
pub async fn enable_flag(
    key: String,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    degradation: &State<DegradationService>,
) -> Result<Json<serde_json::Value>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::OPS_WRITE).await?;
    degradation.set_flag(&key, true, &user.user_id).await?;
    Ok(Json(serde_json::json!({
        "key": key,
        "value": true,
        "message": format!("Flag '{}' enabled", key)
    })))
}

// ============================================================
// Disable a flag
// ============================================================

#[rocket::post("/flags/<key>/disable")]
pub async fn disable_flag(
    key: String,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    degradation: &State<DegradationService>,
) -> Result<Json<serde_json::Value>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::OPS_WRITE).await?;
    degradation.set_flag(&key, false, &user.user_id).await?;
    Ok(Json(serde_json::json!({
        "key": key,
        "value": false,
        "message": format!("Flag '{}' disabled", key)
    })))
}

pub fn routes() -> Vec<Route> {
    rocket::routes![list_flags, enable_flag, disable_flag]
}

#[cfg(test)]
mod tests;
