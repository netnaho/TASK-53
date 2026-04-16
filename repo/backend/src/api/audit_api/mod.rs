use rocket::serde::json::Json;
use rocket::{Route, State};

use crate::api::guards::{self, AuthenticatedUser};
use crate::domain::auth_policy;
use crate::domain::error::AppError;
use crate::infrastructure::audit::{AuditLogRow, AuditService};
use crate::infrastructure::permission_cache::PermissionCache;

#[rocket::get("/?<user_id>&<action>&<resource_type>&<limit>&<offset>")]
pub async fn list_audit_logs(
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    audit_service: &State<AuditService>,
    user_id: Option<String>,
    action: Option<String>,
    resource_type: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Json<Vec<AuditLogRow>>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::AUDIT_READ).await?;

    let logs = audit_service
        .query(
            Some(&user.org_id),
            user_id.as_deref(),
            action.as_deref(),
            resource_type.as_deref(),
            limit.unwrap_or(50).min(200),
            offset.unwrap_or(0),
        )
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(logs))
}

pub fn routes() -> Vec<Route> {
    rocket::routes![list_audit_logs]
}

#[cfg(test)]
mod tests;
