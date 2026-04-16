/// Payments & Refunds API: recorded payment creation, refund processing,
/// fund transaction ledger (read-only), refund reason codes, and reconciliation.
///
/// ## Authorization model (two-gate)
///
/// Every handler enforces **both** checks, in order:
///
///   1. **Permission check** (`require_permission`) — Does the user's role carry
///      the required RBAC permission code?  This is a coarse filter.
///
///   2. **Data-scope check** (`require_data_scope`) — Is the user's data-scope
///      entry (org / department / project + access level) sufficient for the
///      requested operation?  This prevents a user who has the permission in one
///      org from reaching data that belongs to another org, or data that is
///      scoped to a department or project they have not been granted access to.
///
/// Passing permission alone is **not** enough — an attacker who obtains a valid
/// token for Org-A must still be denied access to Org-B's payments even if the
/// token carries `api.payments.read`.  The scope check closes that gap.
///
/// Because payment/refund/reconciliation resources do not currently carry a
/// department_id or project_id on the route input, we enforce org-level scope
/// with `department_id = None, project_id = None`.  This is least-privilege
/// at the resolution available today; if finer-grained scoping is added later
/// the department/project IDs can be threaded through without changing the
/// handler signatures.

use rocket::serde::json::Json;
use rocket::{Route, State};
use serde::Serialize;

use crate::api::guards::{self, AuthenticatedUser};
use crate::application::payment_service::PaymentService;
use crate::application::reconciliation_service::ReconciliationService;
use crate::domain::auth_policy::{action, api};
use crate::domain::billing_types::*;
use crate::domain::error::AppError;
use crate::infrastructure::permission_cache::PermissionCache;

// ============================================================
// Refund reason codes
// ============================================================

#[rocket::get("/reason-codes")]
pub async fn list_reason_codes(
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    payment: &State<PaymentService>,
) -> Result<Json<Vec<RefundReasonCodeRow>>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::PAYMENTS_READ).await?;
    // Reason codes are org-independent reference data, but we still gate on
    // read-level org scope to prevent unenrolled users from probing the list.
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "read").await?;
    let codes = payment.list_refund_reason_codes().await?;
    Ok(Json(codes))
}

// ============================================================
// Payments
// ============================================================

#[derive(Serialize)]
pub struct PaginatedPayments {
    pub data: Vec<RecordedPaymentRow>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[rocket::post("/", data = "<body>")]
pub async fn record_payment(
    body: Json<RecordPaymentRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    payment: &State<PaymentService>,
) -> Result<Json<RecordedPaymentRow>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, action::RECORD_PAYMENT).await?;
    // Write-level scope: recording a payment mutates financial state.
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "write").await?;
    let row = payment.record_payment(&user.org_id, &user.user_id, &body).await?;
    Ok(Json(row))
}

#[rocket::get("/?<invoice_id>&<limit>&<offset>")]
pub async fn list_payments(
    invoice_id: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    payment: &State<PaymentService>,
) -> Result<Json<PaginatedPayments>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::PAYMENTS_READ).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "read").await?;
    let limit = limit.unwrap_or(50).clamp(1, 200);
    let offset = offset.unwrap_or(0).max(0);
    let (data, total) = payment
        .list_payments(&user.org_id, invoice_id.as_deref(), limit, offset)
        .await?;
    Ok(Json(PaginatedPayments { data, total, limit, offset }))
}

#[rocket::get("/<payment_id>")]
pub async fn get_payment(
    payment_id: String,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    payment: &State<PaymentService>,
) -> Result<Json<RecordedPaymentRow>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::PAYMENTS_READ).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "read").await?;
    let row = payment.get_payment(&payment_id, &user.org_id).await?;
    Ok(Json(row))
}

// ============================================================
// Refunds
// ============================================================

#[derive(Serialize)]
pub struct PaginatedRefunds {
    pub data: Vec<RecordedRefundRow>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[rocket::post("/refunds", data = "<body>")]
pub async fn record_refund(
    body: Json<RecordRefundRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    payment: &State<PaymentService>,
) -> Result<Json<RecordedRefundRow>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, action::PROCESS_REFUND).await?;
    // Write-level scope: issuing a refund mutates financial state.
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "write").await?;
    let row = payment.record_refund(&user.org_id, &user.user_id, &body).await?;
    Ok(Json(row))
}

#[rocket::get("/refunds?<invoice_id>&<limit>&<offset>")]
pub async fn list_refunds(
    invoice_id: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    payment: &State<PaymentService>,
) -> Result<Json<PaginatedRefunds>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::PAYMENTS_READ).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "read").await?;
    let limit = limit.unwrap_or(50).clamp(1, 200);
    let offset = offset.unwrap_or(0).max(0);
    let (data, total) = payment
        .list_refunds(&user.org_id, invoice_id.as_deref(), limit, offset)
        .await?;
    Ok(Json(PaginatedRefunds { data, total, limit, offset }))
}

#[rocket::get("/refunds/<refund_id>")]
pub async fn get_refund(
    refund_id: String,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    payment: &State<PaymentService>,
) -> Result<Json<RecordedRefundRow>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::PAYMENTS_READ).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "read").await?;
    let row = payment.get_refund(&refund_id, &user.org_id).await?;
    Ok(Json(row))
}

// ============================================================
// Fund Transactions (immutable ledger — read-only)
// ============================================================

#[derive(Serialize)]
pub struct PaginatedTransactions {
    pub data: Vec<FundTransactionRow>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[rocket::get("/transactions?<invoice_id>&<limit>&<offset>")]
pub async fn list_fund_transactions(
    invoice_id: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    payment: &State<PaymentService>,
) -> Result<Json<PaginatedTransactions>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::PAYMENTS_READ).await?;
    // The fund-transaction ledger is read-only; read scope is sufficient.
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "read").await?;
    let limit = limit.unwrap_or(100).clamp(1, 500);
    let offset = offset.unwrap_or(0).max(0);
    let (data, total) = payment
        .list_fund_transactions(&user.org_id, invoice_id.as_deref(), limit, offset)
        .await?;
    Ok(Json(PaginatedTransactions { data, total, limit, offset }))
}

// ============================================================
// Reconciliation
// ============================================================

#[derive(Serialize)]
pub struct PaginatedReconciliation {
    pub data: Vec<ReconciliationRunRow>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[rocket::post("/reconciliation", data = "<body>")]
pub async fn generate_reconciliation(
    body: Json<ReconciliationRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    recon: &State<ReconciliationService>,
) -> Result<Json<ReconciliationRunRow>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::BILLING_READ).await?;
    // Reconciliation generation creates a snapshot row — write-level scope.
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "write").await?;
    let row = recon.generate_reconciliation(&user.org_id, &user.user_id, &body).await?;
    Ok(Json(row))
}

#[rocket::get("/reconciliation?<limit>&<offset>")]
pub async fn list_reconciliation(
    limit: Option<i64>,
    offset: Option<i64>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    recon: &State<ReconciliationService>,
) -> Result<Json<PaginatedReconciliation>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::BILLING_READ).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "read").await?;
    let limit = limit.unwrap_or(20).clamp(1, 100);
    let offset = offset.unwrap_or(0).max(0);
    let (data, total) = recon.list_reconciliation_runs(&user.org_id, limit, offset).await?;
    Ok(Json(PaginatedReconciliation { data, total, limit, offset }))
}

#[rocket::get("/reconciliation/<run_id>")]
pub async fn get_reconciliation(
    run_id: String,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    recon: &State<ReconciliationService>,
) -> Result<Json<ReconciliationRunRow>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::BILLING_READ).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "read").await?;
    let row = recon.get_reconciliation_run(&run_id, &user.org_id).await?;
    Ok(Json(row))
}

pub fn routes() -> Vec<Route> {
    rocket::routes![
        list_reason_codes,
        record_payment,
        list_payments,
        get_payment,
        record_refund,
        list_refunds,
        get_refund,
        list_fund_transactions,
        generate_reconciliation,
        list_reconciliation,
        get_reconciliation,
    ]
}

#[cfg(test)]
mod tests;
