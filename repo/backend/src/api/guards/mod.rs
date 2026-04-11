use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::serde::json::Json;

use crate::application::auth_service::AuthService;
use crate::domain::auth_types::JwtClaims;
use crate::domain::error::{AppError, ErrorEnvelope};
use crate::infrastructure::permission_cache::PermissionCache;

/// Request guard that extracts and validates the JWT from the Authorization header.
/// Populates request-local state with the authenticated user's claims.
pub struct AuthenticatedUser {
    pub user_id: String,
    pub session_id: String,
    pub org_id: String,
    pub claims: JwtClaims,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthenticatedUser {
    type Error = Json<ErrorEnvelope>;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let auth_header = match request.headers().get_one("Authorization") {
            Some(h) => h,
            None => {
                return Outcome::Error((
                    Status::Unauthorized,
                    Json(AppError::Unauthorized("Missing Authorization header".to_string()).envelope()),
                ));
            }
        };

        let token = if auth_header.starts_with("Bearer ") {
            &auth_header[7..]
        } else {
            return Outcome::Error((
                Status::Unauthorized,
                Json(AppError::Unauthorized("Invalid Authorization header format".to_string()).envelope()),
            ));
        };

        let auth_service = match request.rocket().state::<AuthService>() {
            Some(svc) => svc,
            None => {
                return Outcome::Error((
                    Status::InternalServerError,
                    Json(AppError::Internal("Auth service unavailable".to_string()).envelope()),
                ));
            }
        };

        match auth_service.validate_token(token).await {
            Ok(claims) => Outcome::Success(AuthenticatedUser {
                user_id: claims.sub.clone(),
                session_id: claims.session_id.clone(),
                org_id: claims.org_id.clone(),
                claims,
            }),
            Err(e) => Outcome::Error((
                Status::Unauthorized,
                Json(e.envelope()),
            )),
        }
    }
}

/// Request guard that checks if the authenticated user has a specific permission.
/// Usage: Add `RequirePermission<"api.users.read">` as a route parameter.
/// Since Rocket doesn't support const generics in guards easily,
/// we use a helper macro approach instead.
///
/// For route-level authorization, use the `require_permission` helper function.
pub async fn require_permission(
    perm_cache: &PermissionCache,
    user_id: &str,
    permission_code: &str,
) -> Result<(), AppError> {
    match perm_cache.has_permission(user_id, permission_code).await {
        Ok(true) => Ok(()),
        Ok(false) => Err(AppError::Forbidden(format!(
            "Missing required permission: {}", permission_code
        ))),
        Err(e) => Err(AppError::Internal(format!(
            "Permission check failed: {}", e
        ))),
    }
}

/// Check data-scope access for a specific org/department/project.
pub async fn require_data_scope(
    perm_cache: &PermissionCache,
    user_id: &str,
    org_id: &str,
    department_id: Option<&str>,
    project_id: Option<&str>,
    access_level: &str,
) -> Result<(), AppError> {
    match perm_cache.check_data_scope(user_id, org_id, department_id, project_id, access_level).await {
        Ok(true) => Ok(()),
        Ok(false) => Err(AppError::Forbidden(
            "Insufficient data scope access".to_string(),
        )),
        Err(e) => Err(AppError::Internal(format!(
            "Scope check failed: {}", e
        ))),
    }
}
