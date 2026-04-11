use rocket::serde::json::Json;
use rocket::{Route, State};

use crate::api::guards::{self, AuthenticatedUser};
use crate::application::delivery_service::DeliveryService;
use crate::domain::auth_policy;
use crate::domain::catalog_types::*;
use crate::domain::error::AppError;
use crate::infrastructure::permission_cache::PermissionCache;

#[rocket::get("/?<plan_id>&<provider_id>&<status>&<limit>&<offset>")]
pub async fn list_entries(
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    delivery_svc: &State<DeliveryService>,
    plan_id: Option<String>,
    provider_id: Option<String>,
    status: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::DELIVERY_READ).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "read").await?;

    let (entries, total) = delivery_svc.list_entries(
        &user.org_id,
        plan_id.as_deref(),
        provider_id.as_deref(),
        status.as_deref(),
        limit.unwrap_or(50).min(200),
        offset.unwrap_or(0),
    ).await?;

    Ok(Json(serde_json::json!({
        "data": entries,
        "total": total,
    })))
}

#[rocket::get("/<id>")]
pub async fn get_entry(
    id: String,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    delivery_svc: &State<DeliveryService>,
) -> Result<Json<DeliveryEntryRow>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::DELIVERY_READ).await?;
    let entry = delivery_svc.get_entry(&id).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &entry.org_id, None, None, "read").await?;
    Ok(Json(entry))
}

#[rocket::post("/", data = "<body>")]
pub async fn create_entry(
    body: Json<CreateDeliveryEntryRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    delivery_svc: &State<DeliveryService>,
) -> Result<Json<DeliveryEntryRow>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::DELIVERY_WRITE).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "write").await?;
    let entry = delivery_svc.create_entry(&user.org_id, &body.into_inner(), &user.user_id).await?;
    Ok(Json(entry))
}

#[rocket::put("/<id>", data = "<body>")]
pub async fn update_entry(
    id: String,
    body: Json<UpdateDeliveryEntryRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    delivery_svc: &State<DeliveryService>,
) -> Result<Json<DeliveryEntryRow>, AppError> {
    // Verify delivery entry for update: provider can update own drafts, managers can verify
    let entry = delivery_svc.get_entry(&id).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &entry.org_id, None, None, "write").await?;

    // Status transitions require specific permissions
    if let Some(ref new_status) = body.status {
        if new_status == "verified" {
            guards::require_permission(perm_cache, &user.user_id, auth_policy::action::VERIFY_DELIVERY).await?;
        }
    }
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::DELIVERY_WRITE).await?;

    let updated = delivery_svc.update_entry(&id, &body.into_inner(), &user.user_id).await?;
    Ok(Json(updated))
}

// --- Eligibility Notes ---

#[rocket::get("/<id>/notes")]
pub async fn list_notes(
    id: String,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    delivery_svc: &State<DeliveryService>,
) -> Result<Json<Vec<EligibilityNoteRow>>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::DELIVERY_READ).await?;
    let entry = delivery_svc.get_entry(&id).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &entry.org_id, None, None, "read").await?;
    let notes = delivery_svc.list_notes(None, Some(&id)).await?;
    Ok(Json(notes))
}

#[rocket::post("/<id>/notes", data = "<body>")]
pub async fn create_note(
    id: String,
    body: Json<CreateEligibilityNoteRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    delivery_svc: &State<DeliveryService>,
) -> Result<Json<EligibilityNoteRow>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, auth_policy::api::DELIVERY_WRITE).await?;
    let entry = delivery_svc.get_entry(&id).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &entry.org_id, None, None, "write").await?;
    let note = delivery_svc.create_note(&user.org_id, &body.into_inner(), &user.user_id).await?;
    Ok(Json(note))
}

pub fn routes() -> Vec<Route> {
    rocket::routes![list_entries, get_entry, create_entry, update_entry, list_notes, create_note]
}
