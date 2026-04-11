/// Domain types for quality scoring and evaluation workflow.
/// Covers templates, questions, evaluations, answers, second reviews,
/// report queries, and export requests.

use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};

// ============================================================
// Database row types
// ============================================================

#[derive(Debug, Serialize, Clone, sqlx::FromRow)]
pub struct ScoringTemplateRow {
    pub id: String,
    pub org_id: String,
    pub name: String,
    pub description: Option<String>,
    pub rounding_interval: f64,
    pub max_score: f64,
    pub is_active: bool,
    pub created_by: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Clone, sqlx::FromRow)]
pub struct EvaluationQuestionRow {
    pub id: String,
    pub template_id: String,
    pub question_text: String,
    pub question_type: String,
    pub weight: f64,
    pub max_points: f64,
    pub correct_answer: Option<String>,
    pub sort_order: i32,
    pub is_required: bool,
    pub is_active: bool,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Clone, sqlx::FromRow)]
pub struct EvaluationRow {
    pub id: String,
    pub org_id: String,
    pub delivery_entry_id: String,
    pub template_id: String,
    pub evaluator_id: String,
    pub status: String,
    pub prior_final_score: Option<f64>,
    pub raw_score: Option<f64>,
    pub weighted_score: Option<f64>,
    pub final_score: Option<f64>,
    pub requires_second_review: bool,
    pub score_delta: Option<f64>,
    pub second_reviewer_id: Option<String>,
    pub second_reviewed_at: Option<NaiveDateTime>,
    pub overall_comment: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Clone, sqlx::FromRow)]
pub struct EvaluationAnswerRow {
    pub id: String,
    pub evaluation_id: String,
    pub question_id: String,
    pub answer_text: Option<String>,
    pub auto_score: f64,
    pub manual_score: f64,
    pub partial_credit_fraction: f64,
    pub final_score: f64,
    pub comment: Option<String>,
    pub graded_by: Option<String>,
    pub graded_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Clone, sqlx::FromRow)]
pub struct ScoreReviewRow {
    pub id: String,
    pub evaluation_id: String,
    pub reviewer_id: String,
    pub score_before_review: f64,
    pub score_delta: f64,
    pub review_status: String,
    pub revised_score: Option<f64>,
    pub review_comment: Option<String>,
    pub reviewed_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Clone, sqlx::FromRow)]
pub struct ExportAuditLogRow {
    pub id: String,
    pub org_id: String,
    pub exported_by: String,
    pub export_type: String,
    pub filters_json: Option<String>,
    pub row_count: i32,
    pub masked: bool,
    pub permission_used: Option<String>,
    pub created_at: NaiveDateTime,
}

// ============================================================
// Request types
// ============================================================

#[derive(Debug, Deserialize)]
pub struct CreateTemplateRequest {
    pub name: String,
    pub description: Option<String>,
    pub rounding_interval: Option<f64>,
    pub max_score: Option<f64>,
    pub questions: Vec<CreateQuestionRequest>,
}

#[derive(Debug, Deserialize)]
pub struct CreateQuestionRequest {
    pub question_text: String,
    pub question_type: String,
    pub weight: Option<f64>,
    pub max_points: Option<f64>,
    pub correct_answer: Option<String>,
    pub sort_order: Option<i32>,
    pub is_required: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct StartEvaluationRequest {
    pub delivery_entry_id: String,
    pub template_id: String,
    pub overall_comment: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitAnswerRequest {
    pub question_id: String,
    /// Raw text answer (for objective matching or subjective context)
    pub answer_text: Option<String>,
    /// Manual score override (QA grading for subjective or objective override)
    pub manual_score: Option<f64>,
    /// Partial credit 0.0–1.0 fraction of max_points
    pub partial_credit_fraction: Option<f64>,
    pub comment: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitEvaluationRequest {
    pub answers: Vec<SubmitAnswerRequest>,
    pub overall_comment: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SecondReviewRequest {
    /// Approve (keep score as-is) or revise (set new score)
    pub action: String,  // "approve" | "revise"
    pub revised_score: Option<f64>,
    pub review_comment: Option<String>,
}

// ============================================================
// Report request types
// ============================================================

#[derive(Debug, Deserialize)]
pub struct ReportFilters {
    pub from_date: String,
    pub to_date: String,
    /// Optional: filter by department_id
    pub department_id: Option<String>,
    /// Optional: filter by project_id
    pub project_id: Option<String>,
    /// Optional: filter by service route label (e.g. "north-metro", "client-to-clinic").
    /// When omitted, reports include all routes — backward compatible.
    pub service_route: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    pub export_type: String,
    pub from_date: String,
    pub to_date: String,
    pub department_id: Option<String>,
    /// Optional project-level scope filter.  When provided alongside
    /// `department_id`, the export is restricted to rows whose owning
    /// plan belongs to both the department *and* the project.
    /// Omitting this field (or passing `null`) preserves the pre-existing
    /// department-only filtering behaviour — backward compatible.
    pub project_id: Option<String>,
    /// Optional service route filter.  When provided, only rows whose
    /// owning plan has a matching `service_route` label are included.
    /// Omitting this field preserves existing behavior.
    pub service_route: Option<String>,
    /// If true and caller has EXPORT_UNMASKED permission, identifiers are unmasked.
    /// Default: false (masked).
    pub unmasked: Option<bool>,
}

// ============================================================
// Response types
// ============================================================

#[derive(Debug, Serialize)]
pub struct TemplateDetail {
    pub template: ScoringTemplateRow,
    pub questions: Vec<EvaluationQuestionRow>,
}

#[derive(Debug, Serialize)]
pub struct EvaluationDetail {
    pub evaluation: EvaluationRow,
    pub answers: Vec<EvaluationAnswerRow>,
    pub pending_review: Option<ScoreReviewRow>,
}

/// Computed scoring stats for a single evaluation submission
#[derive(Debug, Serialize)]
pub struct ScoringResult {
    pub raw_score: f64,
    pub weighted_score: f64,
    pub final_score: f64,
    pub requires_second_review: bool,
    pub score_delta: Option<f64>,
    pub progress_pct: f64,
    pub answered_count: usize,
    pub total_questions: usize,
}

// ============================================================
// Report response shapes
// ============================================================

#[derive(Debug, Serialize)]
pub struct OrderVolumeRow {
    pub period: String,
    pub delivery_count: i64,
    pub unique_plans: i64,
    pub unique_providers: i64,
}

#[derive(Debug, Serialize)]
pub struct RevenueReportRow {
    pub period: String,
    pub gross_charges: f64,
    pub net_charges: f64,
    pub total_invoiced: f64,
    pub total_paid: f64,
    pub total_refunded: f64,
    pub refund_rate_pct: f64,
}

#[derive(Debug, Serialize)]
pub struct UtilizationRow {
    pub provider_id: String,
    pub period: String,
    pub total_visits: i64,
    pub total_units: f64,
    pub total_mileage: f64,
}

#[derive(Debug, Serialize)]
pub struct KpiSummary {
    pub period_start: String,
    pub period_end: String,
    pub attendance_rate_pct: f64,     // verified / (submitted + verified)
    pub repurchase_rate_pct: f64,     // plans with 2+ invoice periods / total plans
    pub staff_utilization_pct: f64,   // avg deliveries per provider vs capacity proxy
    pub avg_score: Option<f64>,
    pub second_review_rate_pct: f64,  // evaluations requiring second review / total
}

#[derive(Debug, Serialize)]
pub struct ExportResult {
    pub rows: Vec<serde_json::Value>,
    pub row_count: usize,
    pub masked: bool,
    pub export_log_id: String,
}

// ============================================================
// Scoring computation helpers (pure functions — unit-testable)
// ============================================================

/// Round a score to the nearest interval (e.g. 0.5).
/// Examples:
///   round_to_interval(7.3, 0.5) = 7.5
///   round_to_interval(7.2, 0.5) = 7.0
///   round_to_interval(7.25, 0.5) = 7.5
pub fn round_to_interval(score: f64, interval: f64) -> f64 {
    if interval <= 0.0 {
        return score;
    }
    let rounded = (score / interval).round() * interval;
    // Avoid floating-point drift: round to 4 decimal places
    (rounded * 10000.0).round() / 10000.0
}

/// Compute auto-score for an objective question.
/// Returns max_points if answer matches correct_answer (case-insensitive trim), else 0.
pub fn compute_auto_score(answer_text: &str, correct_answer: &str, max_points: f64) -> f64 {
    if answer_text.trim().to_lowercase() == correct_answer.trim().to_lowercase() {
        max_points
    } else {
        0.0
    }
}

/// Compute the final score for a single answer:
///   final = min(auto_score + manual_score + partial_credit_fraction * max_points, max_points)
pub fn compute_answer_final_score(
    auto_score: f64,
    manual_score: f64,
    partial_credit_fraction: f64,
    max_points: f64,
) -> f64 {
    let raw = auto_score + manual_score + (partial_credit_fraction * max_points);
    raw.min(max_points).max(0.0)
}

/// Compute weighted evaluation score (0–100 scale) from answers and their questions.
/// Questions are paired with their weights. Returns (raw_sum, weighted_score_0_to_100).
pub fn compute_weighted_score(
    answers: &[(f64, f64, f64)], // (answer_final_score, question_weight, question_max_points)
) -> (f64, f64) {
    if answers.is_empty() {
        return (0.0, 0.0);
    }

    let raw_sum: f64 = answers.iter().map(|(s, _, _)| s).sum();
    let total_max: f64 = answers.iter().map(|(_, _, m)| m).sum();
    let total_weight: f64 = answers.iter().map(|(_, w, _)| w).sum();

    if total_weight <= 0.0 || total_max <= 0.0 {
        return (raw_sum, 0.0);
    }

    // Weighted score: each answer contributes (final_score / max_points) * weight
    let weighted_sum: f64 = answers
        .iter()
        .map(|(s, w, m)| {
            if *m > 0.0 { (s / m) * w } else { 0.0 }
        })
        .sum();

    // Normalize to 0–100
    let weighted_pct = (weighted_sum / total_weight) * 100.0;
    (raw_sum, (weighted_pct * 100.0).round() / 100.0)
}

// ============================================================
// Unit tests for scoring logic
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_to_interval_half() {
        assert_eq!(round_to_interval(7.3, 0.5), 7.5);
        assert_eq!(round_to_interval(7.2, 0.5), 7.0);
        assert_eq!(round_to_interval(7.25, 0.5), 7.5);
        assert_eq!(round_to_interval(0.0, 0.5), 0.0);
        assert_eq!(round_to_interval(100.0, 0.5), 100.0);
        assert_eq!(round_to_interval(85.3, 0.5), 85.5);
        assert_eq!(round_to_interval(85.1, 0.5), 85.0);
    }

    #[test]
    fn test_round_to_interval_unit() {
        assert_eq!(round_to_interval(7.6, 1.0), 8.0);
        assert_eq!(round_to_interval(7.4, 1.0), 7.0);
    }

    #[test]
    fn test_round_to_interval_zero_interval_passthrough() {
        // zero interval returns score unchanged
        assert_eq!(round_to_interval(7.3, 0.0), 7.3);
    }

    #[test]
    fn test_auto_score_match() {
        assert_eq!(compute_auto_score("A", "A", 10.0), 10.0);
        assert_eq!(compute_auto_score("a", "A", 10.0), 10.0); // case-insensitive
        assert_eq!(compute_auto_score(" a ", "a", 10.0), 10.0); // trim
    }

    #[test]
    fn test_auto_score_mismatch() {
        assert_eq!(compute_auto_score("B", "A", 10.0), 0.0);
        assert_eq!(compute_auto_score("", "A", 10.0), 0.0);
    }

    #[test]
    fn test_answer_final_score_basic() {
        // auto only
        assert_eq!(compute_answer_final_score(10.0, 0.0, 0.0, 10.0), 10.0);
        // manual only
        assert_eq!(compute_answer_final_score(0.0, 7.5, 0.0, 10.0), 7.5);
        // partial credit: 50% of 10 = 5
        assert_eq!(compute_answer_final_score(0.0, 0.0, 0.5, 10.0), 5.0);
        // combined, capped at max
        assert_eq!(compute_answer_final_score(5.0, 5.0, 0.5, 10.0), 10.0); // capped
    }

    #[test]
    fn test_answer_final_score_no_negative() {
        assert_eq!(compute_answer_final_score(-5.0, 0.0, 0.0, 10.0), 0.0);
    }

    #[test]
    fn test_weighted_score_equal_weights() {
        // 2 questions, both worth 10 pts, both scored 8 → weighted = 80%
        let answers = vec![(8.0, 1.0, 10.0), (8.0, 1.0, 10.0)];
        let (raw, weighted) = compute_weighted_score(&answers);
        assert_eq!(raw, 16.0);
        assert!((weighted - 80.0).abs() < 0.01, "weighted={}", weighted);
    }

    #[test]
    fn test_weighted_score_unequal_weights() {
        // Q1: 10/10 pts, weight=2; Q2: 5/10 pts, weight=1
        // weighted = (10/10 * 2 + 5/10 * 1) / (2+1) * 100 = (2 + 0.5) / 3 * 100 = 83.33
        let answers = vec![(10.0, 2.0, 10.0), (5.0, 1.0, 10.0)];
        let (_, weighted) = compute_weighted_score(&answers);
        assert!((weighted - 83.33).abs() < 0.01, "weighted={}", weighted);
    }

    #[test]
    fn test_weighted_score_empty() {
        let (raw, weighted) = compute_weighted_score(&[]);
        assert_eq!(raw, 0.0);
        assert_eq!(weighted, 0.0);
    }

    #[test]
    fn test_second_review_trigger_threshold() {
        // Delta of exactly 10 should NOT trigger
        let prior = 80.0_f64;
        let new_score = 90.0_f64;
        let delta = (new_score - prior).abs();
        assert_eq!(delta, 10.0);
        assert!(!requires_second_review(prior, new_score));

        // Delta > 10 SHOULD trigger
        let new_score2 = 91.0_f64;
        let delta2 = (new_score2 - prior).abs();
        assert_eq!(delta2, 11.0);
        assert!(requires_second_review(prior, new_score2));

        // Decrease > 10 also triggers
        assert!(requires_second_review(90.0, 75.0));
    }

    /// Helper used in tests to check if second review is required.
    /// Also exposed for use in service.
    fn requires_second_review(prior: f64, new_score: f64) -> bool {
        (new_score - prior).abs() > 10.0
    }
}

/// Returns true if the score change from prior to new exceeds 10 points.
/// This is the backend-enforced second-review trigger.
pub fn requires_second_review(prior_score: Option<f64>, new_score: f64) -> bool {
    match prior_score {
        Some(prior) => (new_score - prior).abs() > 10.0,
        None => false, // First evaluation never requires second review
    }
}
