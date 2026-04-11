use rocket::serde::json::Json;
use rocket::{Route, State};

use crate::api::guards::{self, AuthenticatedUser};
use crate::application::role_service::RoleService;
use crate::application::user_service::UserService;
use crate::domain::auth_policy;
use crate::domain::auth_types::*;
use crate::domain::error::AppError;
use crate::infrastructure::permission_cache::PermissionCache;

// --- Users ---

#[rocket::get("/?<page>&<per_page>&<search>&<sort_by>&<sort_order>")]
pub async fn list_users(
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    user_service: &State<UserService>,
    page: Option<i64>,
    per_page: Option<i64>,
    search: Option<String>,
    sort_by: Option<String>,
    sort_order: Option<String>,
) -> Result<Json<PaginatedResponse<UserRow>>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::USERS_READ).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "read").await?;
    let params = PaginationParams { page, per_page, search, sort_by, sort_order };
    let result = user_service.list_users(&user.org_id, &params).await?;
    Ok(Json(result))
}

#[rocket::get("/<id>")]
pub async fn get_user(
    id: String,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    user_service: &State<UserService>,
) -> Result<Json<UserRow>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::USERS_READ).await?;
    let target = user_service.get_user(&id).await?;
    // Data-scope: user must have access to target's org
    guards::require_data_scope(perm_cache, &user.user_id, &target.org_id, None, None, "read").await?;
    Ok(Json(target))
}

#[rocket::post("/", data = "<body>")]
pub async fn create_user(
    body: Json<CreateUserRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    user_service: &State<UserService>,
) -> Result<Json<UserRow>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::USERS_WRITE).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &body.org_id, None, None, "admin").await?;
    let created = user_service.create_user(&body.into_inner(), &user.user_id).await?;
    Ok(Json(created))
}

#[rocket::put("/<id>", data = "<body>")]
pub async fn update_user(
    id: String,
    body: Json<UpdateUserRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    user_service: &State<UserService>,
) -> Result<Json<UserRow>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::USERS_WRITE).await?;
    let target = user_service.get_user(&id).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &target.org_id, None, None, "write").await?;
    let updated = user_service.update_user(&id, &body.into_inner(), &user.user_id).await?;
    Ok(Json(updated))
}

// --- User roles ---

#[rocket::get("/<id>/roles")]
pub async fn get_user_roles(
    id: String,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    user_service: &State<UserService>,
) -> Result<Json<Vec<RoleRow>>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::ROLES_READ).await?;
    // SECURITY: enforce org-boundary — actor must have read scope for target user's org
    let target = user_service.get_user(&id).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &target.org_id, None, None, "read").await?;
    let roles = user_service.get_user_roles(&id).await?;
    Ok(Json(roles))
}

#[rocket::post("/<id>/roles", data = "<body>")]
pub async fn assign_role(
    id: String,
    body: Json<AssignRoleRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    user_service: &State<UserService>,
) -> Result<rocket::http::Status, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::action::ASSIGN_ROLE).await?;
    // SECURITY: enforce org-boundary — actor must have admin scope for target user's org
    let target = user_service.get_user(&id).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &target.org_id, None, None, "admin").await?;
    user_service.assign_role(&id, &body.role_id, &user.user_id).await?;
    Ok(rocket::http::Status::NoContent)
}

#[rocket::delete("/<id>/roles/<role_id>")]
pub async fn revoke_role(
    id: String,
    role_id: String,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    user_service: &State<UserService>,
) -> Result<rocket::http::Status, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::action::ASSIGN_ROLE).await?;
    // SECURITY: enforce org-boundary — actor must have admin scope for target user's org
    let target = user_service.get_user(&id).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &target.org_id, None, None, "admin").await?;
    user_service.revoke_role(&id, &role_id, &user.user_id).await?;
    Ok(rocket::http::Status::NoContent)
}

// --- User scopes ---

#[rocket::get("/<id>/scopes")]
pub async fn get_user_scopes(
    id: String,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    user_service: &State<UserService>,
) -> Result<Json<Vec<crate::application::user_service::UserScopeRow>>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::action::MANAGE_SCOPES).await?;
    // SECURITY: enforce org-boundary — actor must have read scope for target user's org
    let target = user_service.get_user(&id).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &target.org_id, None, None, "read").await?;
    let scopes = user_service.get_user_scopes(&id).await?;
    Ok(Json(scopes))
}

#[rocket::post("/<id>/scopes", data = "<body>")]
pub async fn assign_scope(
    id: String,
    body: Json<AssignScopeRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    user_service: &State<UserService>,
) -> Result<rocket::http::Status, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::action::MANAGE_SCOPES).await?;
    // SECURITY: enforce org-boundary — actor must have admin scope for target user's org
    let target = user_service.get_user(&id).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &target.org_id, None, None, "admin").await?;
    // SECURITY: also verify actor has admin scope for the org being granted in the scope
    guards::require_data_scope(perm_cache, &user.user_id, &body.org_id, None, None, "admin").await?;
    user_service.assign_scope(&id, &body.into_inner(), &user.user_id).await?;
    Ok(rocket::http::Status::NoContent)
}

#[rocket::delete("/<target_user_id>/scopes/<scope_id>")]
pub async fn revoke_scope(
    target_user_id: String,
    scope_id: String,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    user_service: &State<UserService>,
) -> Result<rocket::http::Status, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::action::MANAGE_SCOPES).await?;
    // SECURITY: enforce org-boundary — load scope to resolve org, verify actor has admin access
    let target = user_service.get_user(&target_user_id).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &target.org_id, None, None, "admin").await?;
    let scope = user_service.get_scope(&scope_id).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &scope.org_id, None, None, "admin").await?;
    user_service.revoke_scope(&scope_id, &user.user_id).await?;
    Ok(rocket::http::Status::NoContent)
}

// --- Roles ---

#[rocket::get("/", rank = 2)]
pub async fn list_roles(
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    role_service: &State<RoleService>,
) -> Result<Json<Vec<RoleRow>>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::ROLES_READ).await?;
    let roles = role_service.list_roles().await?;
    Ok(Json(roles))
}

#[rocket::get("/<id>", rank = 2)]
pub async fn get_role(
    id: String,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    role_service: &State<RoleService>,
) -> Result<Json<RoleRow>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::ROLES_READ).await?;
    let role = role_service.get_role(&id).await?;
    Ok(Json(role))
}

#[rocket::post("/", data = "<body>", rank = 2)]
pub async fn create_role(
    body: Json<CreateRoleRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    role_service: &State<RoleService>,
) -> Result<Json<RoleRow>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::ROLES_WRITE).await?;
    let role = role_service.create_role(&body.into_inner(), &user.user_id).await?;
    Ok(Json(role))
}

// --- Role permissions ---

#[rocket::get("/<id>/permissions", rank = 2)]
pub async fn get_role_permissions(
    id: String,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    role_service: &State<RoleService>,
) -> Result<Json<Vec<PermissionRow>>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::PERMISSIONS_READ).await?;
    let perms = role_service.get_role_permissions(&id).await?;
    Ok(Json(perms))
}

#[rocket::post("/<id>/permissions", data = "<body>", rank = 2)]
pub async fn assign_permission(
    id: String,
    body: Json<AssignPermissionRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    role_service: &State<RoleService>,
) -> Result<rocket::http::Status, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::action::MANAGE_PERMISSIONS).await?;
    role_service.assign_permission(&id, &body.permission_id, &user.user_id).await?;
    Ok(rocket::http::Status::NoContent)
}

#[rocket::delete("/<id>/permissions/<perm_id>", rank = 2)]
pub async fn revoke_permission(
    id: String,
    perm_id: String,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    role_service: &State<RoleService>,
) -> Result<rocket::http::Status, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::action::MANAGE_PERMISSIONS).await?;
    role_service.revoke_permission(&id, &perm_id, &user.user_id).await?;
    Ok(rocket::http::Status::NoContent)
}

// --- Permissions list ---

#[rocket::get("/all")]
pub async fn list_all_permissions(
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    role_service: &State<RoleService>,
) -> Result<Json<Vec<PermissionRow>>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::PERMISSIONS_READ).await?;
    let perms = role_service.list_permissions().await?;
    Ok(Json(perms))
}

pub fn user_routes() -> Vec<Route> {
    rocket::routes![
        list_users, get_user, create_user, update_user,
        get_user_roles, assign_role, revoke_role,
        get_user_scopes, assign_scope, revoke_scope,
    ]
}

pub fn role_routes() -> Vec<Route> {
    rocket::routes![
        list_roles, get_role, create_role,
        get_role_permissions, assign_permission, revoke_permission,
        list_all_permissions,
    ]
}

pub fn routes() -> Vec<Route> {
    // For backward compatibility with existing mount point
    user_routes()
}
