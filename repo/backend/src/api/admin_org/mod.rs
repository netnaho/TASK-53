use rocket::serde::json::Json;
use rocket::{Route, State};

use crate::api::guards::{self, AuthenticatedUser};
use crate::application::org_service::OrgService;
use crate::domain::auth_policy;
use crate::domain::auth_types::*;
use crate::domain::error::AppError;
use crate::infrastructure::permission_cache::PermissionCache;

#[rocket::get("/")]
pub async fn list_orgs(
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    org_service: &State<OrgService>,
) -> Result<Json<Vec<OrgRow>>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::ORG_READ).await?;
    let orgs = org_service.list_orgs().await?;
    Ok(Json(orgs))
}

#[rocket::get("/<id>")]
pub async fn get_org(
    id: String,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    org_service: &State<OrgService>,
) -> Result<Json<OrgRow>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::ORG_READ).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &id, None, None, "read").await?;
    let org = org_service.get_org(&id).await?;
    Ok(Json(org))
}

#[rocket::post("/", data = "<body>")]
pub async fn create_org(
    body: Json<CreateOrgRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    org_service: &State<OrgService>,
) -> Result<Json<OrgRow>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::ORG_WRITE).await?;
    let org = org_service.create_org(&body.into_inner(), &user.user_id).await?;
    Ok(Json(org))
}

#[rocket::put("/<id>", data = "<body>")]
pub async fn update_org(
    id: String,
    body: Json<UpdateOrgRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    org_service: &State<OrgService>,
) -> Result<Json<OrgRow>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::ORG_WRITE).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &id, None, None, "admin").await?;
    let org = org_service.update_org(&id, &body.into_inner(), &user.user_id).await?;
    Ok(Json(org))
}

// --- Departments ---

#[rocket::get("/<org_id>/departments")]
pub async fn list_departments(
    org_id: String,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    org_service: &State<OrgService>,
) -> Result<Json<Vec<DepartmentRow>>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::DEPT_READ).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &org_id, None, None, "read").await?;
    let depts = org_service.list_departments(&org_id).await?;
    Ok(Json(depts))
}

#[rocket::post("/<_org_id>/departments", data = "<body>")]
pub async fn create_department(
    _org_id: String,
    body: Json<CreateDepartmentRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    org_service: &State<OrgService>,
) -> Result<Json<DepartmentRow>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::DEPT_WRITE).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &body.org_id, None, None, "admin").await?;
    let dept = org_service.create_department(&body.into_inner(), &user.user_id).await?;
    Ok(Json(dept))
}

// --- Projects ---

#[rocket::get("/<org_id>/projects")]
pub async fn list_projects(
    org_id: String,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    org_service: &State<OrgService>,
) -> Result<Json<Vec<ProjectRow>>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::PROJECT_READ).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &org_id, None, None, "read").await?;
    let projects = org_service.list_projects(&org_id, None).await?;
    Ok(Json(projects))
}

#[rocket::post("/<_org_id>/projects", data = "<body>")]
pub async fn create_project(
    _org_id: String,
    body: Json<CreateProjectRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    org_service: &State<OrgService>,
) -> Result<Json<ProjectRow>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::PROJECT_WRITE).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &body.org_id, None, None, "admin").await?;
    let project = org_service.create_project(&body.into_inner(), &user.user_id).await?;
    Ok(Json(project))
}

pub fn routes() -> Vec<Route> {
    rocket::routes![
        list_orgs, get_org, create_org, update_org,
        list_departments, create_department,
        list_projects, create_project,
    ]
}
