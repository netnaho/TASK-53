/// Quality scoring service: evaluation lifecycle, objective/subjective grading,
/// weighted aggregation, rounding to nearest 0.5, and second-review enforcement.
///
/// State machine:
///   draft -> submitted -> (delta > 10) second_review_required -> reviewed -> finalized
///                      -> (delta <= 10) finalized

use sqlx::MySqlPool;
use uuid::Uuid;

use crate::domain::error::AppError;
use crate::domain::scoring_types::*;
use crate::infrastructure::audit::{AuditEntry, AuditService};

#[derive(Clone)]
pub struct ScoringService {
    pool: MySqlPool,
    audit: AuditService,
}

impl ScoringService {
    pub fn new(pool: MySqlPool, audit: AuditService) -> Self {
        Self { pool, audit }
    }

    // ------------------------------------------------------------------
    // Template management
    // ------------------------------------------------------------------

    pub async fn create_template(
        &self,
        org_id: &str,
        user_id: &str,
        req: &CreateTemplateRequest,
    ) -> Result<TemplateDetail, AppError> {
        if req.name.trim().is_empty() {
            return Err(AppError::BadRequest("Template name is required".to_string()));
        }
        if req.questions.is_empty() {
            return Err(AppError::BadRequest("Template must have at least one question".to_string()));
        }

        let rounding = req.rounding_interval.unwrap_or(0.5);
        if rounding <= 0.0 {
            return Err(AppError::BadRequest("rounding_interval must be positive".to_string()));
        }

        let template_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO scoring_templates
             (id, org_id, name, description, rounding_interval, max_score, created_by)
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&template_id)
        .bind(org_id)
        .bind(req.name.trim())
        .bind(&req.description)
        .bind(rounding)
        .bind(req.max_score.unwrap_or(100.0))
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        // Insert questions
        for (idx, q) in req.questions.iter().enumerate() {
            if q.question_text.trim().is_empty() {
                return Err(AppError::BadRequest(format!("Question {} text is empty", idx + 1)));
            }
            let qtype = &q.question_type;
            if !matches!(qtype.as_str(), "objective" | "subjective") {
                return Err(AppError::BadRequest(format!(
                    "Invalid question_type '{}' — must be 'objective' or 'subjective'", qtype
                )));
            }

            let q_id = Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO evaluation_questions
                 (id, template_id, question_text, question_type, weight, max_points, correct_answer, sort_order, is_required)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&q_id)
            .bind(&template_id)
            .bind(q.question_text.trim())
            .bind(qtype)
            .bind(q.weight.unwrap_or(1.0))
            .bind(q.max_points.unwrap_or(10.0))
            .bind(&q.correct_answer)
            .bind(q.sort_order.unwrap_or(idx as i32))
            .bind(q.is_required.unwrap_or(true))
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        }

        self.audit.log(AuditEntry {
            user_id: Some(user_id.to_string()),
            action: "scoring.template.created".to_string(),
            resource_type: "scoring_template".to_string(),
            resource_id: Some(template_id.clone()),
            org_id: Some(org_id.to_string()),
            details: Some(serde_json::json!({ "name": req.name, "question_count": req.questions.len() })),
            ip_address: None,
            trace_id: None,
        }).await;

        self.get_template_detail(&template_id, org_id).await
    }

    pub async fn list_templates(
        &self,
        org_id: &str,
        active_only: bool,
    ) -> Result<Vec<ScoringTemplateRow>, AppError> {
        let q = if active_only {
            "SELECT id, org_id, name, description, rounding_interval, max_score, is_active, created_by, created_at, updated_at
             FROM scoring_templates WHERE org_id = ? AND is_active = 1 ORDER BY name ASC"
        } else {
            "SELECT id, org_id, name, description, rounding_interval, max_score, is_active, created_by, created_at, updated_at
             FROM scoring_templates WHERE org_id = ? ORDER BY name ASC"
        };
        sqlx::query_as::<_, ScoringTemplateRow>(q)
            .bind(org_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    pub async fn get_template_detail(
        &self,
        template_id: &str,
        org_id: &str,
    ) -> Result<TemplateDetail, AppError> {
        let template = sqlx::query_as::<_, ScoringTemplateRow>(
            "SELECT id, org_id, name, description, rounding_interval, max_score, is_active, created_by, created_at, updated_at
             FROM scoring_templates WHERE id = ? AND org_id = ?"
        )
        .bind(template_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Scoring template not found".to_string()))?;

        let questions = sqlx::query_as::<_, EvaluationQuestionRow>(
            "SELECT id, template_id, question_text, question_type, weight, max_points, correct_answer, sort_order, is_required, is_active, created_at
             FROM evaluation_questions WHERE template_id = ? AND is_active = 1
             ORDER BY sort_order ASC, created_at ASC"
        )
        .bind(template_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(TemplateDetail { template, questions })
    }

    // ------------------------------------------------------------------
    // Evaluation lifecycle
    // ------------------------------------------------------------------

    pub async fn start_evaluation(
        &self,
        org_id: &str,
        user_id: &str,
        req: &StartEvaluationRequest,
    ) -> Result<EvaluationDetail, AppError> {
        // Validate delivery entry belongs to org
        let entry: Option<(String, String)> = sqlx::query_as(
            "SELECT id, org_id FROM delivery_entries WHERE id = ? AND org_id = ?"
        )
        .bind(&req.delivery_entry_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        if entry.is_none() {
            return Err(AppError::NotFound("Delivery entry not found".to_string()));
        }

        // Validate template
        let tpl: Option<(String, String)> = sqlx::query_as(
            "SELECT id, org_id FROM scoring_templates WHERE id = ? AND org_id = ? AND is_active = 1"
        )
        .bind(&req.template_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        if tpl.is_none() {
            return Err(AppError::NotFound("Scoring template not found or inactive".to_string()));
        }

        // Get prior finalized score for delta detection
        let prior_score: Option<(Option<f64>,)> = sqlx::query_as(
            "SELECT final_score FROM evaluations
             WHERE delivery_entry_id = ? AND template_id = ? AND status = 'finalized'
             ORDER BY updated_at DESC LIMIT 1"
        )
        .bind(&req.delivery_entry_id)
        .bind(&req.template_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let prior_final = prior_score.and_then(|(s,)| s);

        let eval_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO evaluations
             (id, org_id, delivery_entry_id, template_id, evaluator_id, status, prior_final_score, overall_comment)
             VALUES (?, ?, ?, ?, ?, 'draft', ?, ?)"
        )
        .bind(&eval_id)
        .bind(org_id)
        .bind(&req.delivery_entry_id)
        .bind(&req.template_id)
        .bind(user_id)
        .bind(prior_final)
        .bind(&req.overall_comment)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        self.get_evaluation_detail(&eval_id, org_id).await
    }

    pub async fn submit_evaluation(
        &self,
        eval_id: &str,
        org_id: &str,
        user_id: &str,
        req: &SubmitEvaluationRequest,
    ) -> Result<EvaluationDetail, AppError> {
        // Load evaluation and verify ownership
        let eval = self.load_eval(eval_id, org_id).await?;

        if eval.evaluator_id != user_id {
            return Err(AppError::Forbidden("Only the original evaluator can submit this evaluation".to_string()));
        }
        if !matches!(eval.status.as_str(), "draft") {
            return Err(AppError::BadRequest(format!(
                "Cannot submit evaluation with status '{}'", eval.status
            )));
        }

        // Load template for rounding and question details
        let template = sqlx::query_as::<_, ScoringTemplateRow>(
            "SELECT id, org_id, name, description, rounding_interval, max_score, is_active, created_by, created_at, updated_at
             FROM scoring_templates WHERE id = ?"
        )
        .bind(&eval.template_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let questions = sqlx::query_as::<_, EvaluationQuestionRow>(
            "SELECT id, template_id, question_text, question_type, weight, max_points, correct_answer, sort_order, is_required, is_active, created_at
             FROM evaluation_questions WHERE template_id = ? AND is_active = 1"
        )
        .bind(&eval.template_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        // Process and persist each answer
        let mut answer_tuples: Vec<(f64, f64, f64)> = Vec::new(); // (final_score, weight, max_points)

        for q in &questions {
            let submitted = req.answers.iter().find(|a| a.question_id == q.id);

            let (answer_text, manual_score, partial_fraction, comment) = match submitted {
                Some(a) => (
                    a.answer_text.clone(),
                    a.manual_score.unwrap_or(0.0),
                    a.partial_credit_fraction.unwrap_or(0.0).clamp(0.0, 1.0),
                    a.comment.clone(),
                ),
                None => (None, 0.0, 0.0, None),
            };

            // Auto-score for objective questions
            let auto_sc = if q.question_type == "objective" {
                if let (Some(txt), Some(correct)) = (&answer_text, &q.correct_answer) {
                    compute_auto_score(txt, correct, q.max_points)
                } else {
                    0.0
                }
            } else {
                0.0
            };

            let final_sc = compute_answer_final_score(auto_sc, manual_score, partial_fraction, q.max_points);

            // Upsert answer
            let existing: Option<(String,)> = sqlx::query_as(
                "SELECT id FROM evaluation_answers WHERE evaluation_id = ? AND question_id = ?"
            )
            .bind(eval_id)
            .bind(&q.id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

            if let Some((ans_id,)) = existing {
                sqlx::query(
                    "UPDATE evaluation_answers
                     SET answer_text=?, auto_score=?, manual_score=?, partial_credit_fraction=?,
                         final_score=?, comment=?, graded_by=?, graded_at=NOW(), updated_at=NOW()
                     WHERE id=?"
                )
                .bind(&answer_text).bind(auto_sc).bind(manual_score).bind(partial_fraction)
                .bind(final_sc).bind(&comment).bind(user_id).bind(&ans_id)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            } else {
                let ans_id = Uuid::new_v4().to_string();
                sqlx::query(
                    "INSERT INTO evaluation_answers
                     (id, evaluation_id, question_id, answer_text, auto_score, manual_score,
                      partial_credit_fraction, final_score, comment, graded_by, graded_at)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NOW())"
                )
                .bind(&ans_id).bind(eval_id).bind(&q.id)
                .bind(&answer_text).bind(auto_sc).bind(manual_score)
                .bind(partial_fraction).bind(final_sc).bind(&comment).bind(user_id)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            }

            answer_tuples.push((final_sc, q.weight, q.max_points));
        }

        // Compute aggregate scores
        let (raw_score, weighted_score) = compute_weighted_score(&answer_tuples);
        let final_score = round_to_interval(weighted_score, template.rounding_interval);

        // Detect second-review requirement
        let needs_review = requires_second_review(eval.prior_final_score, final_score);
        let score_delta = eval.prior_final_score.map(|p| (final_score - p).abs());

        let new_status = if needs_review {
            "second_review_required"
        } else {
            "finalized"
        };

        // Update evaluation
        sqlx::query(
            "UPDATE evaluations
             SET status=?, raw_score=?, weighted_score=?, final_score=?, requires_second_review=?,
                 score_delta=?, overall_comment=?, updated_at=NOW()
             WHERE id=?"
        )
        .bind(new_status)
        .bind(raw_score)
        .bind(weighted_score)
        .bind(final_score)
        .bind(needs_review)
        .bind(score_delta)
        .bind(&req.overall_comment)
        .bind(eval_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        // If second review required, try to assign an independent QA reviewer.
        // SECURITY: never fall back to the evaluator — that would create a self-review
        // path, allowing the same person who scored to approve their own delta.
        if needs_review {
            // Find an independent reviewer: first active QA Reviewer in the org
            // who is NOT the evaluator (prevents self-review assignment).
            let reviewer: Option<(String,)> = sqlx::query_as(
                "SELECT u.id FROM users u
                 JOIN user_roles ur ON ur.user_id = u.id
                 JOIN roles r ON r.id = ur.role_id
                 WHERE u.org_id = ? AND r.name = 'QA Reviewer' AND u.status = 'active'
                   AND u.id != ?
                 LIMIT 1"
            )
            .bind(org_id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

            if let Some((reviewer_id,)) = reviewer {
                // Independent reviewer found — create pending review record
                let review_id = Uuid::new_v4().to_string();
                sqlx::query(
                    "INSERT INTO score_reviews (id, evaluation_id, reviewer_id, score_before_review, score_delta, review_status)
                     VALUES (?, ?, ?, ?, ?, 'pending')"
                )
                .bind(&review_id)
                .bind(eval_id)
                .bind(&reviewer_id)
                .bind(final_score)
                .bind(score_delta.unwrap_or(0.0))
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            } else {
                // No independent QA reviewer available — evaluation stays in
                // second_review_required with no pending review record.  It cannot
                // be finalized until a reviewer is assigned.  Log a clear signal.
                tracing::warn!(
                    eval_id = eval_id,
                    org_id = org_id,
                    evaluator_id = user_id,
                    "Second review required but no independent QA Reviewer available — \
                     evaluation blocked in second_review_required until a reviewer is assigned"
                );
                self.audit.log(AuditEntry {
                    user_id: Some(user_id.to_string()),
                    action: "scoring.second_review.unassigned".to_string(),
                    resource_type: "evaluation".to_string(),
                    resource_id: Some(eval_id.to_string()),
                    org_id: Some(org_id.to_string()),
                    details: Some(serde_json::json!({
                        "reason": "no_independent_qa_reviewer",
                        "score_delta": score_delta,
                        "final_score": final_score,
                    })),
                    ip_address: None,
                    trace_id: None,
                }).await;
            }
        }

        let answered_count = answer_tuples.len();
        let total_questions = questions.len();
        let progress_pct = if total_questions > 0 {
            (answered_count as f64 / total_questions as f64) * 100.0
        } else {
            0.0
        };

        self.audit.log(AuditEntry {
            user_id: Some(user_id.to_string()),
            action: "scoring.evaluation.submitted".to_string(),
            resource_type: "evaluation".to_string(),
            resource_id: Some(eval_id.to_string()),
            org_id: Some(org_id.to_string()),
            details: Some(serde_json::json!({
                "final_score": final_score,
                "requires_second_review": needs_review,
                "score_delta": score_delta,
                "progress_pct": progress_pct,
            })),
            ip_address: None,
            trace_id: None,
        }).await;

        self.get_evaluation_detail(eval_id, org_id).await
    }

    pub async fn process_second_review(
        &self,
        eval_id: &str,
        org_id: &str,
        reviewer_id: &str,
        req: &SecondReviewRequest,
    ) -> Result<EvaluationDetail, AppError> {
        // Load evaluation
        let eval = self.load_eval(eval_id, org_id).await?;

        if eval.status != "second_review_required" {
            return Err(AppError::BadRequest(format!(
                "Evaluation is not pending second review (status: '{}')", eval.status
            )));
        }

        // Find the pending review record
        let review: Option<ScoreReviewRow> = sqlx::query_as(
            "SELECT id, evaluation_id, reviewer_id, score_before_review, score_delta,
                    review_status, revised_score, review_comment, reviewed_at, created_at
             FROM score_reviews
             WHERE evaluation_id = ? AND review_status = 'pending'
             ORDER BY created_at DESC LIMIT 1"
        )
        .bind(eval_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let review = review.ok_or_else(|| AppError::NotFound("No pending review found".to_string()))?;

        // Object-level authorization: only the assigned reviewer may act on this review
        if review.reviewer_id != reviewer_id {
            return Err(AppError::Forbidden(
                "You are not the assigned reviewer for this evaluation".to_string(),
            ));
        }

        // SECURITY: defense-in-depth — even if a review record somehow has the
        // evaluator as reviewer (e.g. manual DB edit), block self-review.
        if reviewer_id == eval.evaluator_id {
            return Err(AppError::Forbidden(
                "Evaluator cannot review their own evaluation".to_string(),
            ));
        }

        match req.action.as_str() {
            "approve" => {
                // Approve: finalize with current score
                sqlx::query(
                    "UPDATE score_reviews SET review_status='approved', review_comment=?, reviewed_at=NOW() WHERE id=?"
                )
                .bind(&req.review_comment)
                .bind(&review.id)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

                sqlx::query(
                    "UPDATE evaluations SET status='finalized', second_reviewer_id=?, second_reviewed_at=NOW() WHERE id=?"
                )
                .bind(reviewer_id)
                .bind(eval_id)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            }
            "revise" => {
                let revised = req.revised_score.ok_or_else(|| {
                    AppError::BadRequest("revised_score is required for action='revise'".to_string())
                })?;
                if !(0.0..=100.0).contains(&revised) {
                    return Err(AppError::BadRequest(
                        "revised_score must be between 0 and 100".to_string(),
                    ));
                }

                sqlx::query(
                    "UPDATE score_reviews SET review_status='revised', revised_score=?, review_comment=?, reviewed_at=NOW() WHERE id=?"
                )
                .bind(revised)
                .bind(&req.review_comment)
                .bind(&review.id)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

                sqlx::query(
                    "UPDATE evaluations SET status='finalized', final_score=?, second_reviewer_id=?, second_reviewed_at=NOW() WHERE id=?"
                )
                .bind(revised)
                .bind(reviewer_id)
                .bind(eval_id)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            }
            _ => {
                return Err(AppError::BadRequest(format!(
                    "Invalid action '{}' — must be 'approve' or 'revise'", req.action
                )));
            }
        }

        self.audit.log(AuditEntry {
            user_id: Some(reviewer_id.to_string()),
            action: "scoring.second_review.completed".to_string(),
            resource_type: "evaluation".to_string(),
            resource_id: Some(eval_id.to_string()),
            org_id: Some(org_id.to_string()),
            details: Some(serde_json::json!({
                "action": req.action,
                "revised_score": req.revised_score,
            })),
            ip_address: None,
            trace_id: None,
        }).await;

        self.get_evaluation_detail(eval_id, org_id).await
    }

    pub async fn list_evaluations(
        &self,
        org_id: &str,
        delivery_entry_id: Option<&str>,
        status_filter: Option<&str>,
        evaluator_id: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<EvaluationRow>, i64), AppError> {
        let mut where_clause = "WHERE org_id = ?".to_string();
        let mut binds: Vec<String> = vec![org_id.to_string()];

        if let Some(d) = delivery_entry_id {
            where_clause.push_str(" AND delivery_entry_id = ?");
            binds.push(d.to_string());
        }
        if let Some(s) = status_filter {
            where_clause.push_str(" AND status = ?");
            binds.push(s.to_string());
        }
        if let Some(e) = evaluator_id {
            where_clause.push_str(" AND evaluator_id = ?");
            binds.push(e.to_string());
        }

        let count_q = format!("SELECT COUNT(*) FROM evaluations {}", where_clause);
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_q);
        for b in &binds { cq = cq.bind(b); }
        let (total,) = cq.fetch_one(&self.pool).await.map_err(|e| AppError::Internal(e.to_string()))?;

        let data_q = format!(
            "SELECT id, org_id, delivery_entry_id, template_id, evaluator_id, status,
                    prior_final_score, raw_score, weighted_score, final_score,
                    requires_second_review, score_delta, second_reviewer_id, second_reviewed_at,
                    overall_comment, created_at, updated_at
             FROM evaluations {} ORDER BY created_at DESC LIMIT ? OFFSET ?",
            where_clause
        );
        let mut dq = sqlx::query_as::<_, EvaluationRow>(&data_q);
        for b in &binds { dq = dq.bind(b); }
        let rows = dq.bind(limit).bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok((rows, total))
    }

    pub async fn get_evaluation_detail(
        &self,
        eval_id: &str,
        org_id: &str,
    ) -> Result<EvaluationDetail, AppError> {
        let evaluation = self.load_eval(eval_id, org_id).await?;

        let answers = sqlx::query_as::<_, EvaluationAnswerRow>(
            "SELECT id, evaluation_id, question_id, answer_text, auto_score, manual_score,
                    partial_credit_fraction, final_score, comment, graded_by, graded_at, created_at, updated_at
             FROM evaluation_answers WHERE evaluation_id = ?
             ORDER BY created_at ASC"
        )
        .bind(eval_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let pending_review = sqlx::query_as::<_, ScoreReviewRow>(
            "SELECT id, evaluation_id, reviewer_id, score_before_review, score_delta,
                    review_status, revised_score, review_comment, reviewed_at, created_at
             FROM score_reviews
             WHERE evaluation_id = ? AND review_status = 'pending'
             ORDER BY created_at DESC LIMIT 1"
        )
        .bind(eval_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(EvaluationDetail { evaluation, answers, pending_review })
    }

    pub async fn list_pending_reviews(
        &self,
        org_id: &str,
        reviewer_id: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<ScoreReviewRow>, i64), AppError> {
        let mut where_clause = "WHERE e.org_id = ? AND sr.review_status = 'pending'".to_string();
        let mut binds: Vec<String> = vec![org_id.to_string()];

        if let Some(r) = reviewer_id {
            where_clause.push_str(" AND sr.reviewer_id = ?");
            binds.push(r.to_string());
        }

        let count_q = format!(
            "SELECT COUNT(*) FROM score_reviews sr JOIN evaluations e ON e.id = sr.evaluation_id {}",
            where_clause
        );
        let mut cq = sqlx::query_as::<_, (i64,)>(&count_q);
        for b in &binds { cq = cq.bind(b); }
        let (total,) = cq.fetch_one(&self.pool).await.map_err(|e| AppError::Internal(e.to_string()))?;

        let data_q = format!(
            "SELECT sr.id, sr.evaluation_id, sr.reviewer_id, sr.score_before_review, sr.score_delta,
                    sr.review_status, sr.revised_score, sr.review_comment, sr.reviewed_at, sr.created_at
             FROM score_reviews sr JOIN evaluations e ON e.id = sr.evaluation_id
             {} ORDER BY sr.created_at ASC LIMIT ? OFFSET ?",
            where_clause
        );
        let mut dq = sqlx::query_as::<_, ScoreReviewRow>(&data_q);
        for b in &binds { dq = dq.bind(b); }
        let rows = dq.bind(limit).bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok((rows, total))
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    async fn load_eval(&self, eval_id: &str, org_id: &str) -> Result<EvaluationRow, AppError> {
        sqlx::query_as::<_, EvaluationRow>(
            "SELECT id, org_id, delivery_entry_id, template_id, evaluator_id, status,
                    prior_final_score, raw_score, weighted_score, final_score,
                    requires_second_review, score_delta, second_reviewer_id, second_reviewed_at,
                    overall_comment, created_at, updated_at
             FROM evaluations WHERE id = ? AND org_id = ?"
        )
        .bind(eval_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Evaluation not found".to_string()))
    }
}

/// Validate that the caller is the assigned reviewer and that the action is
/// well-formed.  Returns `Ok(())` on success or the appropriate `AppError`.
///
/// Extracted as a pure function so the authorization + validation logic can be
/// unit-tested without a database.
fn validate_review_request(
    caller_id: &str,
    assigned_reviewer_id: &str,
    evaluator_id: &str,
    eval_status: &str,
    action: &str,
    revised_score: Option<f64>,
) -> Result<(), AppError> {
    // Status gate
    if eval_status != "second_review_required" {
        return Err(AppError::BadRequest(format!(
            "Evaluation is not pending second review (status: '{}')", eval_status
        )));
    }

    // Object-level authorization
    if caller_id != assigned_reviewer_id {
        return Err(AppError::Forbidden(
            "You are not the assigned reviewer for this evaluation".to_string(),
        ));
    }

    // SECURITY: prevent self-review — the evaluator cannot review their own work,
    // even if somehow assigned as reviewer (defense-in-depth).
    if caller_id == evaluator_id {
        return Err(AppError::Forbidden(
            "Evaluator cannot review their own evaluation".to_string(),
        ));
    }

    // Action validation
    match action {
        "approve" => Ok(()),
        "revise" => {
            let score = revised_score.ok_or_else(|| {
                AppError::BadRequest("revised_score is required for action='revise'".to_string())
            })?;
            if !(0.0..=100.0).contains(&score) {
                return Err(AppError::BadRequest(
                    "revised_score must be between 0 and 100".to_string(),
                ));
            }
            Ok(())
        }
        _ => Err(AppError::BadRequest(format!(
            "Invalid action '{}' — must be 'approve' or 'revise'", action
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- assigned independent reviewer can approve ----------------------

    #[test]
    fn assigned_reviewer_can_approve() {
        let result = validate_review_request(
            "reviewer-1",
            "reviewer-1",
            "evaluator-1",  // different person — independent review
            "second_review_required",
            "approve",
            None,
        );
        assert!(result.is_ok());
    }

    // -- assigned independent reviewer can revise with valid score ------

    #[test]
    fn assigned_reviewer_can_revise_with_valid_score() {
        let result = validate_review_request(
            "reviewer-1",
            "reviewer-1",
            "evaluator-1",
            "second_review_required",
            "revise",
            Some(85.0),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn assigned_reviewer_can_revise_boundary_zero() {
        let result = validate_review_request(
            "reviewer-1",
            "reviewer-1",
            "evaluator-1",
            "second_review_required",
            "revise",
            Some(0.0),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn assigned_reviewer_can_revise_boundary_hundred() {
        let result = validate_review_request(
            "reviewer-1",
            "reviewer-1",
            "evaluator-1",
            "second_review_required",
            "revise",
            Some(100.0),
        );
        assert!(result.is_ok());
    }

    // -- self-review prevention (TASK-53) --------------------------------

    #[test]
    fn evaluator_as_reviewer_gets_forbidden_on_approve() {
        // Evaluator is also the assigned reviewer — must be blocked
        let result = validate_review_request(
            "evaluator-1",
            "evaluator-1",
            "evaluator-1",
            "second_review_required",
            "approve",
            None,
        );
        match result {
            Err(AppError::Forbidden(msg)) => {
                assert!(msg.contains("cannot review their own"), "msg: {}", msg);
            }
            other => panic!("expected Forbidden for self-review, got {:?}", other),
        }
    }

    #[test]
    fn evaluator_as_reviewer_gets_forbidden_on_revise() {
        let result = validate_review_request(
            "evaluator-1",
            "evaluator-1",
            "evaluator-1",
            "second_review_required",
            "revise",
            Some(75.0),
        );
        match result {
            Err(AppError::Forbidden(msg)) => {
                assert!(msg.contains("cannot review their own"), "msg: {}", msg);
            }
            other => panic!("expected Forbidden for self-review, got {:?}", other),
        }
    }

    // -- unassigned scorer gets Forbidden --------------------------------

    #[test]
    fn unassigned_scorer_gets_forbidden_on_approve() {
        let result = validate_review_request(
            "attacker-99",
            "reviewer-1",
            "evaluator-1",
            "second_review_required",
            "approve",
            None,
        );
        match result {
            Err(AppError::Forbidden(msg)) => {
                assert!(msg.contains("not the assigned reviewer"), "msg: {}", msg);
            }
            other => panic!("expected Forbidden, got {:?}", other),
        }
    }

    #[test]
    fn unassigned_scorer_gets_forbidden_on_revise() {
        let result = validate_review_request(
            "attacker-99",
            "reviewer-1",
            "evaluator-1",
            "second_review_required",
            "revise",
            Some(50.0),
        );
        match result {
            Err(AppError::Forbidden(msg)) => {
                assert!(msg.contains("not the assigned reviewer"), "msg: {}", msg);
            }
            other => panic!("expected Forbidden, got {:?}", other),
        }
    }

    // Verify that authorization is checked before action validation:
    // an unassigned user with an invalid action still gets Forbidden, not BadRequest.
    #[test]
    fn unassigned_scorer_forbidden_before_action_validation() {
        let result = validate_review_request(
            "attacker-99",
            "reviewer-1",
            "evaluator-1",
            "second_review_required",
            "bogus_action",
            None,
        );
        match result {
            Err(AppError::Forbidden(_)) => {} // correct: authz checked first
            other => panic!("expected Forbidden, got {:?}", other),
        }
    }

    // -- invalid action still fails as before ----------------------------

    #[test]
    fn invalid_action_returns_bad_request() {
        let result = validate_review_request(
            "reviewer-1",
            "reviewer-1",
            "evaluator-1",
            "second_review_required",
            "reject",
            None,
        );
        match result {
            Err(AppError::BadRequest(msg)) => {
                assert!(msg.contains("Invalid action"), "msg: {}", msg);
            }
            other => panic!("expected BadRequest, got {:?}", other),
        }
    }

    // -- status gate ------------------------------------------------------

    #[test]
    fn wrong_status_returns_bad_request() {
        let result = validate_review_request(
            "reviewer-1",
            "reviewer-1",
            "evaluator-1",
            "finalized",
            "approve",
            None,
        );
        match result {
            Err(AppError::BadRequest(msg)) => {
                assert!(msg.contains("not pending second review"), "msg: {}", msg);
            }
            other => panic!("expected BadRequest, got {:?}", other),
        }
    }

    // -- revise validation edge cases ------------------------------------

    #[test]
    fn revise_without_score_returns_bad_request() {
        let result = validate_review_request(
            "reviewer-1",
            "reviewer-1",
            "evaluator-1",
            "second_review_required",
            "revise",
            None,
        );
        match result {
            Err(AppError::BadRequest(msg)) => {
                assert!(msg.contains("revised_score is required"), "msg: {}", msg);
            }
            other => panic!("expected BadRequest, got {:?}", other),
        }
    }

    #[test]
    fn revise_with_negative_score_returns_bad_request() {
        let result = validate_review_request(
            "reviewer-1",
            "reviewer-1",
            "evaluator-1",
            "second_review_required",
            "revise",
            Some(-1.0),
        );
        match result {
            Err(AppError::BadRequest(msg)) => {
                assert!(msg.contains("between 0 and 100"), "msg: {}", msg);
            }
            other => panic!("expected BadRequest, got {:?}", other),
        }
    }

    #[test]
    fn revise_with_over_100_returns_bad_request() {
        let result = validate_review_request(
            "reviewer-1",
            "reviewer-1",
            "evaluator-1",
            "second_review_required",
            "revise",
            Some(100.01),
        );
        match result {
            Err(AppError::BadRequest(msg)) => {
                assert!(msg.contains("between 0 and 100"), "msg: {}", msg);
            }
            other => panic!("expected BadRequest, got {:?}", other),
        }
    }
}
