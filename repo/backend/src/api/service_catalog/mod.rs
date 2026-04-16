use rocket::serde::json::Json;
use rocket::{Route, State};

use crate::api::guards::{self, AuthenticatedUser};
use crate::application::catalog_service::CatalogService;
use crate::domain::auth_policy;
use crate::domain::catalog_types::*;
use crate::domain::error::AppError;
use crate::infrastructure::permission_cache::PermissionCache;

#[rocket::get("/?<category>&<active_only>")]
pub async fn list_services(
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    catalog: &State<CatalogService>,
    category: Option<String>,
    active_only: Option<bool>,
) -> Result<Json<Vec<ServiceItemRow>>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::CATALOG_READ).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "read").await?;
    let items = catalog.list_items(&user.org_id, category.as_deref(), active_only.unwrap_or(true)).await?;
    Ok(Json(items))
}

#[rocket::get("/<id>")]
pub async fn get_service(
    id: String,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    catalog: &State<CatalogService>,
) -> Result<Json<ServiceItemRow>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::CATALOG_READ).await?;
    let item = catalog.get_item(&id).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &item.org_id, None, None, "read").await?;
    Ok(Json(item))
}

#[rocket::post("/", data = "<body>")]
pub async fn create_service(
    body: Json<CreateServiceItemRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    catalog: &State<CatalogService>,
) -> Result<Json<ServiceItemRow>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::CATALOG_WRITE).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "write").await?;
    let item = catalog.create_item(&user.org_id, &body.into_inner(), &user.user_id).await?;
    Ok(Json(item))
}

#[rocket::put("/<id>", data = "<body>")]
pub async fn update_service(
    id: String,
    body: Json<UpdateServiceItemRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    catalog: &State<CatalogService>,
) -> Result<Json<ServiceItemRow>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::CATALOG_WRITE).await?;
    let item = catalog.get_item(&id).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &item.org_id, None, None, "write").await?;
    let updated = catalog.update_item(&id, &body.into_inner(), &user.user_id).await?;
    Ok(Json(updated))
}

pub fn routes() -> Vec<Route> {
    rocket::routes![list_services, get_service, create_service, update_service]
}

#[cfg(test)]
mod tests;
