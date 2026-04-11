/// Scoring & Reviews API
///
/// Endpoints:
///   Templates:
///     POST   /templates              - create a new scoring template (SCORING_WRITE)
///     GET    /templates              - list templates (SCORING_READ)
///     GET    /templates/<id>         - get template detail (SCORING_READ)
///
///   Evaluations:
///     POST   /evaluations            - start an evaluation (SCORING_WRITE)
///     GET    /evaluations            - list evaluations (SCORING_READ)
///     GET    /evaluations/<id>       - get evaluation detail (SCORING_READ)
///     POST   /evaluations/<id>/submit - submit answers and finalize (SCORING_WRITE)
///
///   Second Reviews:
///     GET    /reviews/pending        - list pending reviews (SCORING_READ)
///     POST   /reviews/<eval_id>      - perform second review (SCORING_WRITE)

use rocket::serde::json::Json;
use rocket::{Route, State};
use serde::Serialize;

use crate::api::guards::{self, AuthenticatedUser};
use crate::application::scoring_service::ScoringService;
use crate::domain::auth_policy::api;
use crate::domain::error::AppError;
use crate::domain::scoring_types::*;
use crate::infrastructure::permission_cache::PermissionCache;

// ============================================================
// Templates
// ============================================================

#[rocket::post("/templates", data = "<body>")]
pub async fn create_template(
    body: Json<CreateTemplateRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    scoring: &State<ScoringService>,
) -> Result<Json<TemplateDetail>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::SCORING_WRITE).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "write").await?;
    let detail = scoring.create_template(&user.org_id, &user.user_id, &body).await?;
    Ok(Json(detail))
}

#[rocket::get("/templates?<active_only>")]
pub async fn list_templates(
    active_only: Option<bool>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    scoring: &State<ScoringService>,
) -> Result<Json<Vec<ScoringTemplateRow>>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::SCORING_READ).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "read").await?;
    let rows = scoring.list_templates(&user.org_id, active_only.unwrap_or(true)).await?;
    Ok(Json(rows))
}

#[rocket::get("/templates/<template_id>")]
pub async fn get_template(
    template_id: String,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    scoring: &State<ScoringService>,
) -> Result<Json<TemplateDetail>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::SCORING_READ).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "read").await?;
    let detail = scoring.get_template_detail(&template_id, &user.org_id).await?;
    Ok(Json(detail))
}

// ============================================================
// Evaluations
// ============================================================

#[derive(Serialize)]
pub struct PaginatedEvaluations {
    pub data: Vec<EvaluationRow>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[rocket::post("/evaluations", data = "<body>")]
pub async fn start_evaluation(
    body: Json<StartEvaluationRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    scoring: &State<ScoringService>,
) -> Result<Json<EvaluationDetail>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::SCORING_WRITE).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "write").await?;
    let detail = scoring.start_evaluation(&user.org_id, &user.user_id, &body).await?;
    Ok(Json(detail))
}

#[rocket::get("/evaluations?<delivery_entry_id>&<status>&<evaluator_id>&<limit>&<offset>")]
pub async fn list_evaluations(
    delivery_entry_id: Option<String>,
    status: Option<String>,
    evaluator_id: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    scoring: &State<ScoringService>,
) -> Result<Json<PaginatedEvaluations>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::SCORING_READ).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "read").await?;
    let limit = limit.unwrap_or(50).clamp(1, 200);
    let offset = offset.unwrap_or(0).max(0);
    let (data, total) = scoring
        .list_evaluations(
            &user.org_id,
            delivery_entry_id.as_deref(),
            status.as_deref(),
            evaluator_id.as_deref(),
            limit,
            offset,
        )
        .await?;
    Ok(Json(PaginatedEvaluations { data, total, limit, offset }))
}

#[rocket::get("/evaluations/<eval_id>")]
pub async fn get_evaluation(
    eval_id: String,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    scoring: &State<ScoringService>,
) -> Result<Json<EvaluationDetail>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::SCORING_READ).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "read").await?;
    let detail = scoring.get_evaluation_detail(&eval_id, &user.org_id).await?;
    Ok(Json(detail))
}

#[rocket::post("/evaluations/<eval_id>/submit", data = "<body>")]
pub async fn submit_evaluation(
    eval_id: String,
    body: Json<SubmitEvaluationRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    scoring: &State<ScoringService>,
) -> Result<Json<EvaluationDetail>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::SCORING_WRITE).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "write").await?;
    let detail = scoring.submit_evaluation(&eval_id, &user.org_id, &user.user_id, &body).await?;
    Ok(Json(detail))
}

// ============================================================
// Second Reviews
// ============================================================

#[derive(Serialize)]
pub struct PaginatedReviews {
    pub data: Vec<ScoreReviewRow>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[rocket::get("/reviews/pending?<reviewer_id>&<limit>&<offset>")]
pub async fn list_pending_reviews(
    reviewer_id: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    scoring: &State<ScoringService>,
) -> Result<Json<PaginatedReviews>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::SCORING_READ).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "read").await?;
    let limit = limit.unwrap_or(50).clamp(1, 200);
    let offset = offset.unwrap_or(0).max(0);
    let (data, total) = scoring
        .list_pending_reviews(&user.org_id, reviewer_id.as_deref(), limit, offset)
        .await?;
    Ok(Json(PaginatedReviews { data, total, limit, offset }))
}

#[rocket::post("/reviews/<eval_id>", data = "<body>")]
pub async fn process_review(
    eval_id: String,
    body: Json<SecondReviewRequest>,
    user: AuthenticatedUser,
    perm_cache: &State<PermissionCache>,
    scoring: &State<ScoringService>,
) -> Result<Json<EvaluationDetail>, AppError> {
    guards::require_permission(perm_cache, &user.user_id, api::SCORING_WRITE).await?;
    guards::require_data_scope(perm_cache, &user.user_id, &user.org_id, None, None, "write").await?;
    let detail = scoring.process_second_review(&eval_id, &user.org_id, &user.user_id, &body).await?;
    Ok(Json(detail))
}

pub fn routes() -> Vec<Route> {
    rocket::routes![
        create_template,
        list_templates,
        get_template,
        start_evaluation,
        list_evaluations,
        get_evaluation,
        submit_evaluation,
        list_pending_reviews,
        process_review,
    ]
}
