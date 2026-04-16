use rocket::serde::json::Json;
use rocket::{Route, State};

use crate::api::guards::{self, AuthenticatedUser};
use crate::application::package_service::PackageService;
use crate::domain::auth_policy;
use crate::domain::catalog_types::*;
use crate::domain::error::AppError;
use crate::infrastructure::permission_cache::PermissionCache;

#[rocket::get("/?<active_only>")]
pub async fn list_packages(
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    pkg_svc: &State<PackageService>,
    active_only: Option<bool>,
) -> Result<Json<Vec<PackageRow>>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::CATALOG_READ).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "read").await?;
    let pkgs = pkg_svc.list_packages(&user.org_id, active_only.unwrap_or(true)).await?;
    Ok(Json(pkgs))
}

#[rocket::get("/<id>")]
pub async fn get_package(
    id: String,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    pkg_svc: &State<PackageService>,
) -> Result<Json<PackageDetail>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::CATALOG_READ).await?;
    let detail = pkg_svc.get_package_detail(&id).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &detail.package.org_id, None, None, "read").await?;
    Ok(Json(detail))
}

#[rocket::post("/", data = "<body>")]
pub async fn create_package(
    body: Json<CreatePackageRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    pkg_svc: &State<PackageService>,
) -> Result<Json<PackageDetail>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::CATALOG_WRITE).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "write").await?;
    let detail = pkg_svc.create_package(&user.org_id, &body.into_inner(), &user.user_id).await?;
    Ok(Json(detail))
}

#[rocket::put("/<id>", data = "<body>")]
pub async fn update_package(
    id: String,
    body: Json<UpdatePackageRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    pkg_svc: &State<PackageService>,
) -> Result<Json<PackageRow>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::CATALOG_WRITE).await?;
    let pkg = pkg_svc.get_package(&id).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &pkg.org_id, None, None, "write").await?;
    let updated = pkg_svc.update_package(&id, &body.into_inner(), &user.user_id).await?;
    Ok(Json(updated))
}

#[rocket::get("/<id>/rules")]
pub async fn get_package_rules(
    id: String,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    pkg_svc: &State<PackageService>,
) -> Result<Json<Vec<PackageRuleRow>>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::CATALOG_READ).await?;
    let pkg = pkg_svc.get_package(&id).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &pkg.org_id, None, None, "read").await?;
    let rules = pkg_svc.get_rules(&id).await?;
    Ok(Json(rules))
}

pub fn routes() -> Vec<Route> {
    rocket::routes![list_packages, get_package, create_package, update_package, get_package_rules]
}

#[cfg(test)]
mod tests;
