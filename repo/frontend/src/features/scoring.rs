/// Pure scoring/evaluation logic: score display, status labelling, threshold checks.
/// No browser APIs — compiles and tests on native targets.

/// Format a final score for display (rounds to 1 decimal, appends %).
/// Returns "--" when score is absent.
pub fn format_score(score: Option<f64>) -> String {
    match score {
        Some(s) => format!("{:.1}%", s),
        None => "--".to_string(),
    }
}

/// Returns a human-readable label for an evaluation status string.
pub fn evaluation_status_label(status: &str) -> &'static str {
    match status {
        "draft" => "Draft",
        "submitted" => "Submitted",
        "second_review_required" => "Awaiting Second Review",
        "reviewed" => "Reviewed",
        "finalized" => "Finalized",
        _ => "Unknown",
    }
}

/// Returns true if the evaluation status indicates it requires a second review.
pub fn requires_reviewer(status: &str) -> bool {
    status == "second_review_required"
}

/// Returns true if the evaluation can still be edited (draft or submitted).
pub fn is_editable(status: &str) -> bool {
    matches!(status, "draft" | "submitted")
}

/// Determine whether a score delta warrants a second review.
/// Backend rule: delta > 10 points triggers second review.
pub fn score_delta_requires_review(prior: f64, new_score: f64) -> bool {
    (new_score - prior).abs() > 10.0
}

/// Returns a CSS class name for score badge styling.
pub fn score_badge_class(score: f64) -> &'static str {
    if score >= 90.0 {
        "badge-excellent"
    } else if score >= 75.0 {
        "badge-good"
    } else if score >= 60.0 {
        "badge-fair"
    } else {
        "badge-poor"
    }
}
