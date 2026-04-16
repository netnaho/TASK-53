/// Billing API: charge generation, charge adjustments, and invoice management.

use rocket::serde::json::Json;
use rocket::{Route, State};
use serde::Serialize;

use crate::api::guards::{self, AuthenticatedUser};
use crate::application::billing_service::BillingService;
use crate::domain::auth_policy::{action, api};
use crate::domain::billing_types::*;
use crate::domain::error::AppError;
use crate::infrastructure::permission_cache::PermissionCache;

// ============================================================
// Charge generation
// ============================================================

#[rocket::post("/charges/generate", data = "<body>")]
pub async fn generate_charges(
    body: Json<GenerateChargesRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    billing: &State<BillingService>,
) -> Result<Json<GenerateChargesResponse>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, action::GENERATE_INVOICE).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "write").await?;
    let result = billing.generate_charges(&user.org_id, &user.user_id, &body).await?;
    Ok(Json(result))
}

// ============================================================
// Charge list / detail
// ============================================================

#[derive(Serialize)]
pub struct PaginatedCharges {
    pub data: Vec<ChargeRow>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[rocket::get("/charges?<plan_id>&<status>&<limit>&<offset>")]
pub async fn list_charges(
    plan_id: Option<String>,
    status: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    billing: &State<BillingService>,
) -> Result<Json<PaginatedCharges>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::BILLING_READ).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "read").await?;
    let limit = limit.unwrap_or(50).clamp(1, 200);
    let offset = offset.unwrap_or(0).max(0);
    let (data, total) = billing
        .list_charges(&user.org_id, plan_id.as_deref(), status.as_deref(), limit, offset)
        .await?;
    Ok(Json(PaginatedCharges { data, total, limit, offset }))
}

#[rocket::get("/charges/<charge_id>")]
pub async fn get_charge(
    charge_id: String,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    billing: &State<BillingService>,
) -> Result<Json<ChargeDetail>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::BILLING_READ).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "read").await?;
    let detail = billing.get_charge_detail(&charge_id, &user.org_id).await?;
    Ok(Json(detail))
}

// ============================================================
// Charge adjustments
// ============================================================

#[rocket::post("/charges/<charge_id>/adjustments", data = "<body>")]
pub async fn post_adjustment(
    charge_id: String,
    body: Json<PostAdjustmentRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    billing: &State<BillingService>,
) -> Result<Json<ChargeAdjustmentRow>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, action::GENERATE_INVOICE).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "write").await?;
    let adj = billing.post_adjustment(&charge_id, &user.org_id, &user.user_id, &body).await?;
    Ok(Json(adj))
}

// ============================================================
// Invoice generation
// ============================================================

#[rocket::post("/invoices/generate", data = "<body>")]
pub async fn generate_invoice(
    body: Json<GenerateInvoiceRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    billing: &State<BillingService>,
) -> Result<Json<InvoiceDetail>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, action::GENERATE_INVOICE).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "write").await?;
    let detail = billing.generate_invoice(&user.org_id, &user.user_id, &body).await?;
    Ok(Json(detail))
}

// ============================================================
// Invoice list / detail / status update
// ============================================================

#[derive(Serialize)]
pub struct PaginatedInvoices {
    pub data: Vec<InvoiceRow>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[rocket::get("/invoices?<plan_id>&<status>&<limit>&<offset>")]
pub async fn list_invoices(
    plan_id: Option<String>,
    status: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    billing: &State<BillingService>,
) -> Result<Json<PaginatedInvoices>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::BILLING_READ).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "read").await?;
    let limit = limit.unwrap_or(50).clamp(1, 200);
    let offset = offset.unwrap_or(0).max(0);
    let (data, total) = billing
        .list_invoices(&user.org_id, plan_id.as_deref(), status.as_deref(), limit, offset)
        .await?;
    Ok(Json(PaginatedInvoices { data, total, limit, offset }))
}

#[rocket::get("/invoices/<invoice_id>")]
pub async fn get_invoice(
    invoice_id: String,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    billing: &State<BillingService>,
) -> Result<Json<InvoiceDetail>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::BILLING_READ).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "read").await?;
    let detail = billing.get_invoice_detail(&invoice_id, &user.org_id).await?;
    Ok(Json(detail))
}

#[rocket::put("/invoices/<invoice_id>/status", data = "<body>")]
pub async fn update_invoice_status(
    invoice_id: String,
    body: Json<UpdateInvoiceStatusRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    billing: &State<BillingService>,
) -> Result<Json<InvoiceRow>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, action::APPROVE_INVOICE).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "write").await?;
    let updated = billing
        .update_invoice_status(&invoice_id, &user.org_id, &user.user_id, &body)
        .await?;
    Ok(Json(updated))
}

pub fn routes() -> Vec<Route> {
    rocket::routes![
        generate_charges,
        list_charges,
        get_charge,
        post_adjustment,
        generate_invoice,
        list_invoices,
        get_invoice,
        update_invoice_status,
    ]
}

#[cfg(test)]
mod tests;
