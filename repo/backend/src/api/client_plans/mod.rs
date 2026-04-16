use rocket::serde::json::Json;
use rocket::{Route, State};

use crate::api::guards::{self, AuthenticatedUser};
use crate::application::plan_service::PlanService;
use crate::domain::auth_policy;
use crate::domain::catalog_types::*;
use crate::domain::error::AppError;
use crate::infrastructure::permission_cache::PermissionCache;

#[rocket::get("/?<status>")]
pub async fn list_plans(
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    plan_svc: &State<PlanService>,
    status: Option<String>,
) -> Result<Json<Vec<ClientPlanRow>>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::PLANS_READ).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "read").await?;
    let plans = plan_svc.list_plans(&user.org_id, status.as_deref()).await?;
    Ok(Json(plans))
}

#[rocket::get("/<id>")]
pub async fn get_plan(
    id: String,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    plan_svc: &State<PlanService>,
) -> Result<Json<ClientPlanRow>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::PLANS_READ).await?;
    let plan = plan_svc.get_plan(&id).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &plan.org_id, plan.department_id.as_deref(), plan.project_id.as_deref(), "read").await?;
    Ok(Json(plan))
}

#[rocket::post("/", data = "<body>")]
pub async fn create_plan(
    body: Json<CreateClientPlanRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    plan_svc: &State<PlanService>,
) -> Result<Json<ClientPlanRow>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::PLANS_WRITE).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "write").await?;
    let plan = plan_svc.create_plan(&user.org_id, &body.into_inner(), &user.user_id).await?;
    Ok(Json(plan))
}

#[rocket::put("/<id>", data = "<body>")]
pub async fn update_plan(
    id: String,
    body: Json<UpdateClientPlanRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    plan_svc: &State<PlanService>,
) -> Result<Json<ClientPlanRow>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::PLANS_WRITE).await?;
    let plan = plan_svc.get_plan(&id).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &plan.org_id, None, None, "write").await?;
    let updated = plan_svc.update_plan(&id, &body.into_inner(), &user.user_id).await?;
    Ok(Json(updated))
}

#[rocket::post("/<id>/packages", data = "<body>")]
pub async fn assign_package(
    id: String,
    body: Json<AssignPackageRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    plan_svc: &State<PlanService>,
) -> Result<Json<PlanPackageRow>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::PLANS_WRITE).await?;
    let plan = plan_svc.get_plan(&id).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &plan.org_id, None, None, "write").await?;
    let assignment = plan_svc.assign_package(&id, &body.into_inner(), &user.user_id).await?;
    Ok(Json(assignment))
}

#[rocket::get("/<id>/packages")]
pub async fn get_plan_packages(
    id: String,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    plan_svc: &State<PlanService>,
) -> Result<Json<Vec<PlanPackageRow>>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::PLANS_READ).await?;
    let plan = plan_svc.get_plan(&id).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &plan.org_id, plan.department_id.as_deref(), plan.project_id.as_deref(), "read").await?;
    let pkgs = plan_svc.get_plan_packages(&id).await?;
    Ok(Json(pkgs))
}

pub fn routes() -> Vec<Route> {
    rocket::routes![list_plans, get_plan, create_plan, update_plan, assign_package, get_plan_packages]
}

#[cfg(test)]
mod tests;
