/// Scoring page: evaluation workflow with template picker, question-level grading,
/// partial credit input, progress indicator, and second-review status cues.
/// QA Reviewers can process pending second reviews inline.

use dioxus::prelude::*;

use crate::models::{
    EvaluationDetail, EvaluationQuestionRow, PaginatedEvaluations, PaginatedReviews,
    SecondReviewRequest, ScoringTemplateRow, StartEvaluationRequest,
    SubmitAnswerRequest, SubmitEvaluationRequest, TemplateDetail,
};
use crate::services::ApiClient;
use crate::state::AuthState;

#[derive(Debug, Clone, PartialEq)]
enum ScoringTab {
    Evaluations,
    NewEvaluation,
    PendingReviews,
}

#[component]
pub fn Scoring() -> Element {
    let auth = use_context::<Signal<AuthState>>();
    let mut active_tab = use_signal(|| ScoringTab::Evaluations);

    let can_read = auth.read().has_permission("api.scoring.read");
    let can_write = auth.read().has_permission("api.scoring.write");

    if auth.read().token.is_none() {
        return rsx! {
            div { class: "page-header",
                h1 { "Quality Scoring" }
                p { style: "color: var(--color-text-secondary);",
                    "Your session has expired. Please log in again."
                }
            }
        };
    }

    if !can_read {
        return rsx! {
            div { class: "page-header",
                h1 { "Quality Scoring" }
                p { style: "color: var(--color-text-secondary);",
                    "You do not have permission to view scoring data."
                }
            }
        };
    }

    rsx! {
        div {
            div { class: "page-header",
                h1 { "Quality Scoring" }
                p { "Evaluate service delivery quality, review auto-scored and manual responses, and manage second-review escalations." }
            }

            div { style: "display: flex; gap: 4px; border-bottom: 2px solid var(--color-border); margin-bottom: 24px;",
                TabButton {
                    label: "Evaluations",
                    active: *active_tab.read() == ScoringTab::Evaluations,
                    onclick: move |_| { *active_tab.write() = ScoringTab::Evaluations; }
                }
                if can_write {
                    TabButton {
                        label: "New Evaluation",
                        active: *active_tab.read() == ScoringTab::NewEvaluation,
                        onclick: move |_| { *active_tab.write() = ScoringTab::NewEvaluation; }
                    }
                }
                TabButton {
                    label: "Pending Reviews",
                    active: *active_tab.read() == ScoringTab::PendingReviews,
                    onclick: move |_| { *active_tab.write() = ScoringTab::PendingReviews; }
                }
            }

            match *active_tab.read() {
                ScoringTab::Evaluations => rsx! { EvaluationsTab {} },
                ScoringTab::NewEvaluation => rsx! { NewEvaluationTab {} },
                ScoringTab::PendingReviews => rsx! { PendingReviewsTab { can_write: can_write } },
            }
        }
    }
}

// ============================================================
// Evaluations list tab
// ============================================================

#[component]
fn EvaluationsTab() -> Element {
    let auth = use_context::<Signal<AuthState>>();
    let mut status_filter = use_signal(|| "".to_string());

    let evaluations = use_resource(move || {
        let t = auth.read().token.clone();
        let st = status_filter.read().clone();
        async move {
            let mut url = "/scoring/evaluations?limit=50".to_string();
            if !st.is_empty() {
                url.push_str(&format!("&status={}", st));
            }
            ApiClient::get::<PaginatedEvaluations>(&url, t.as_deref()).await.ok()
        }
    });

    rsx! {
        div {
            div { style: "display: flex; gap: 12px; margin-bottom: 16px; flex-wrap: wrap; align-items: center;",
                label { style: "font-size: 0.875rem; font-weight: 500;", "Status filter:" }
                select {
                    style: "padding: 6px 12px; border: 1px solid var(--color-border); border-radius: 4px; background: var(--color-surface);",
                    onchange: move |e| { *status_filter.write() = e.value().clone(); },
                    option { value: "", "All statuses" }
                    option { value: "draft", "Draft" }
                    option { value: "submitted", "Submitted" }
                    option { value: "second_review_required", "Needs Second Review" }
                    option { value: "finalized", "Finalized" }
                }
            }

            div { class: "card", style: "overflow-x: auto;",
                table { style: "width: 100%; border-collapse: collapse;",
                    thead {
                        tr { style: "border-bottom: 1px solid var(--color-border);",
                            th { style: "padding: 10px 12px; text-align: left; font-size: 0.875rem;", "ID" }
                            th { style: "padding: 10px 12px; text-align: left; font-size: 0.875rem;", "Delivery Entry" }
                            th { style: "padding: 10px 12px; text-align: left; font-size: 0.875rem;", "Template" }
                            th { style: "padding: 10px 12px; text-align: right; font-size: 0.875rem;", "Final Score" }
                            th { style: "padding: 10px 12px; text-align: left; font-size: 0.875rem;", "Status" }
                            th { style: "padding: 10px 12px; text-align: left; font-size: 0.875rem;", "Created" }
                        }
                    }
                    tbody {
                        match &*evaluations.read() {
                            Some(Some(resp)) if !resp.data.is_empty() => rsx! {
                                for eval in &resp.data {
                                    tr { style: "border-bottom: 1px solid var(--color-border);",
                                        td { style: "padding: 8px 12px; font-size: 0.8rem; font-family: monospace;",
                                            "{&eval.id[..8]}..."
                                        }
                                        td { style: "padding: 8px 12px; font-size: 0.8rem; font-family: monospace;",
                                            "{&eval.delivery_entry_id[..8]}..."
                                        }
                                        td { style: "padding: 8px 12px; font-size: 0.8rem; font-family: monospace;",
                                            "{&eval.template_id[..8]}..."
                                        }
                                        td { style: "padding: 8px 12px; text-align: right; font-size: 0.875rem; font-weight: 600;",
                                            {eval.final_score.map(|s| format!("{:.1}", s)).unwrap_or_else(|| "\u{2014}".to_string())}
                                        }
                                        td { style: "padding: 8px 12px; font-size: 0.8rem;",
                                            span {
                                                style: "padding: 2px 8px; border-radius: 10px; font-size: 0.75rem; font-weight: 600; background: {status_color(&eval.status)}22; color: {status_color(&eval.status)};",
                                                "{eval.status}"
                                            }
                                        }
                                        td { style: "padding: 8px 12px; font-size: 0.8rem; color: var(--color-text-secondary);",
                                            "{&eval.created_at[..10]}"
                                        }
                                    }
                                }
                            },
                            Some(Some(_)) => rsx! {
                                tr {
                                    td { colspan: "6", style: "padding: 24px; text-align: center; color: var(--color-text-secondary); font-size: 0.875rem;",
                                        "No evaluations found."
                                    }
                                }
                            },
                            Some(None) => rsx! {
                                tr {
                                    td { colspan: "6", style: "padding: 24px; text-align: center; color: var(--color-error);",
                                        "Failed to load evaluations."
                                    }
                                }
                            },
                            None => rsx! {
                                tr {
                                    td { colspan: "6", style: "padding: 24px; text-align: center; color: var(--color-text-secondary);",
                                        "Loading..."
                                    }
                                }
                            },
                        }
                    }
                }
            }
        }
    }
}

// ============================================================
// New evaluation tab
// ============================================================

#[component]
fn NewEvaluationTab() -> Element {
    let auth = use_context::<Signal<AuthState>>();
    let mut delivery_entry_id = use_signal(|| "".to_string());
    let mut template_id = use_signal(|| "".to_string());
    let mut overall_comment = use_signal(|| "".to_string());
    let mut error_msg = use_signal(|| String::new());
    let mut success_msg = use_signal(|| String::new());
    let mut eval_detail = use_signal(|| None::<EvaluationDetail>);
    let mut selected_template_id = use_signal(|| String::new());

    let templates = use_resource(move || {
        let t = auth.read().token.clone();
        async move {
            ApiClient::get::<Vec<ScoringTemplateRow>>("/scoring/templates?active_only=true", t.as_deref()).await.ok()
        }
    });

    let on_start = {
        let delivery_entry_id = delivery_entry_id.clone();
        let template_id = template_id.clone();
        let overall_comment = overall_comment.clone();
        let mut error_msg = error_msg.clone();
        let mut success_msg = success_msg.clone();
        let mut eval_detail = eval_detail.clone();
        let mut selected_template_id = selected_template_id.clone();
        move |_| {
            let d = delivery_entry_id.read().clone();
            let t_id = template_id.read().clone();
            let comment = overall_comment.read().clone();
            if d.is_empty() || t_id.is_empty() {
                *error_msg.write() = "Delivery entry ID and template are required.".to_string();
                return;
            }
            *selected_template_id.write() = t_id.clone();
            let t = auth.read().token.clone();
            spawn(async move {
                let body = StartEvaluationRequest {
                    delivery_entry_id: d,
                    template_id: t_id,
                    overall_comment: if comment.is_empty() { None } else { Some(comment) },
                };
                match ApiClient::post::<EvaluationDetail, _>("/scoring/evaluations", &body, t.as_deref()).await {
                    Ok(detail) => {
                        *success_msg.write() = format!("Evaluation started (ID: {}). Grade each question below.", &detail.evaluation.id[..8]);
                        *error_msg.write() = String::new();
                        *eval_detail.write() = Some(detail);
                    }
                    Err(e) => {
                        *error_msg.write() = e;
                        *success_msg.write() = String::new();
                    }
                }
            });
        }
    };

    rsx! {
        div {
            if !error_msg.read().is_empty() {
                div { class: "alert alert-error", "{error_msg}" }
            }
            if !success_msg.read().is_empty() {
                div { class: "alert alert-success", "{success_msg}" }
            }

            if eval_detail.read().is_none() {
                div { class: "card", style: "max-width: 640px;",
                    h3 { style: "margin-bottom: 16px;", "Start New Evaluation" }
                    div { style: "display: flex; flex-direction: column; gap: 14px;",
                        div {
                            label { style: "display: block; font-size: 0.875rem; font-weight: 500; margin-bottom: 4px;",
                                "Delivery Entry ID *"
                            }
                            input {
                                r#type: "text",
                                placeholder: "Enter delivery entry ID",
                                style: "width: 100%; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: 4px; font-size: 0.875rem;",
                                value: "{delivery_entry_id}",
                                oninput: move |e| { *delivery_entry_id.write() = e.value().clone(); },
                            }
                        }
                        div {
                            label { style: "display: block; font-size: 0.875rem; font-weight: 500; margin-bottom: 4px;",
                                "Scoring Template *"
                            }
                            match &*templates.read() {
                                Some(Some(tpls)) if !tpls.is_empty() => rsx! {
                                    select {
                                        style: "width: 100%; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: 4px; font-size: 0.875rem;",
                                        onchange: move |e| { *template_id.write() = e.value().clone(); },
                                        option { value: "", "Select a template..." }
                                        for tpl in tpls {
                                            option { value: "{tpl.id}", "{tpl.name} (max {tpl.max_score})" }
                                        }
                                    }
                                },
                                Some(Some(_)) => rsx! {
                                    p { style: "color: var(--color-text-secondary); font-size: 0.875rem;", "No active templates found." }
                                },
                                _ => rsx! {
                                    p { style: "color: var(--color-text-secondary); font-size: 0.875rem;", "Loading templates..." }
                                },
                            }
                        }
                        div {
                            label { style: "display: block; font-size: 0.875rem; font-weight: 500; margin-bottom: 4px;",
                                "Overall Comment (optional)"
                            }
                            textarea {
                                rows: "3",
                                placeholder: "Evaluator notes...",
                                style: "width: 100%; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: 4px; font-size: 0.875rem; resize: vertical;",
                                value: "{overall_comment}",
                                oninput: move |e| { *overall_comment.write() = e.value().clone(); },
                            }
                        }
                        button {
                            class: "btn btn-primary",
                            style: "align-self: flex-start;",
                            onclick: on_start,
                            "Start Evaluation"
                        }
                    }
                }
            }

            if let Some(detail) = &*eval_detail.read() {
                SubmitEvaluationForm {
                    eval_detail: detail.clone(),
                    template_id: selected_template_id.read().clone(),
                }
            }
        }
    }
}

// ============================================================
// Per-question editable state
// ============================================================

/// Mutable state for one question's answer, held in a Vec<Signal<...>>.
#[derive(Debug, Clone)]
struct QuestionDraft {
    question_id: String,
    question_text: String,
    question_type: String, // "objective" or "subjective"
    max_points: f64,
    weight: f64,
    correct_answer: Option<String>,
    is_required: bool,
    sort_order: i32,
    // Editable fields
    answer_text: String,
    manual_score: String,
    partial_credit: String,
    comment: String,
}

impl QuestionDraft {
    fn from_question(q: &EvaluationQuestionRow) -> Self {
        Self {
            question_id: q.id.clone(),
            question_text: q.question_text.clone(),
            question_type: q.question_type.clone(),
            max_points: q.max_points,
            weight: q.weight,
            correct_answer: q.correct_answer.clone(),
            is_required: q.is_required,
            sort_order: q.sort_order,
            answer_text: String::new(),
            manual_score: "0".to_string(),
            partial_credit: "0".to_string(),
            comment: String::new(),
        }
    }

    fn is_touched(&self) -> bool {
        !self.answer_text.is_empty()
            || self.manual_score_f64() != 0.0
            || self.partial_credit_f64() != 0.0
            || !self.comment.is_empty()
    }

    fn manual_score_f64(&self) -> f64 {
        self.manual_score.parse::<f64>().unwrap_or(0.0)
    }

    fn partial_credit_f64(&self) -> f64 {
        self.partial_credit.parse::<f64>().unwrap_or(0.0)
    }

    fn validate(&self) -> Option<String> {
        let ms = self.manual_score.parse::<f64>();
        if let Ok(v) = ms {
            if v < 0.0 || v > self.max_points {
                return Some(format!(
                    "Manual score must be between 0 and {:.1}", self.max_points
                ));
            }
        } else if !self.manual_score.is_empty() {
            return Some("Manual score must be a number".to_string());
        }

        let pc = self.partial_credit.parse::<f64>();
        if let Ok(v) = pc {
            if v < 0.0 || v > 1.0 {
                return Some("Partial credit must be between 0.0 and 1.0".to_string());
            }
        } else if !self.partial_credit.is_empty() {
            return Some("Partial credit must be a number".to_string());
        }

        None
    }

    fn to_submit_request(&self) -> SubmitAnswerRequest {
        SubmitAnswerRequest {
            question_id: self.question_id.clone(),
            answer_text: if self.answer_text.is_empty() { None } else { Some(self.answer_text.clone()) },
            manual_score: Some(self.manual_score_f64()),
            partial_credit_fraction: Some(self.partial_credit_f64()),
            comment: if self.comment.is_empty() { None } else { Some(self.comment.clone()) },
        }
    }
}

// ============================================================
// Submit evaluation form (question-level grading)
// ============================================================

#[component]
fn SubmitEvaluationForm(eval_detail: EvaluationDetail, template_id: String) -> Element {
    let auth = use_context::<Signal<AuthState>>();
    let mut error_msg = use_signal(|| String::new());
    let mut success_msg = use_signal(|| String::new());
    let mut submitted = use_signal(|| false);
    let mut submit_result = use_signal(|| None::<EvaluationDetail>);
    let eval_id = eval_detail.evaluation.id.clone();

    let mut submit_comment = use_signal(|| String::new());

    // Fetch the template questions so we can build the grading form.
    // A brand-new draft evaluation has 0 answers — the questions come from the template.
    let questions_resource = use_resource(move || {
        let t = auth.read().token.clone();
        let tid = template_id.clone();
        async move {
            if tid.is_empty() {
                return None;
            }
            ApiClient::get::<TemplateDetail>(
                &format!("/scoring/templates/{}", tid),
                t.as_deref(),
            ).await.ok()
        }
    });

    // Build editable drafts once questions are loaded
    let mut drafts: Signal<Vec<QuestionDraft>> = use_signal(Vec::new);
    let mut drafts_initialized = use_signal(|| false);

    // Initialize drafts from fetched questions (run once)
    if !*drafts_initialized.read() {
        if let Some(Some(tpl_detail)) = &*questions_resource.read() {
            let mut new_drafts: Vec<QuestionDraft> = tpl_detail.questions
                .iter()
                .map(QuestionDraft::from_question)
                .collect();
            new_drafts.sort_by_key(|d| d.sort_order);
            *drafts.write() = new_drafts;
            *drafts_initialized.write() = true;
        }
    }

    let total_questions = drafts.read().len();
    let touched_count = drafts.read().iter().filter(|d| d.is_touched()).count();
    let progress_pct = if total_questions > 0 {
        (touched_count as f64 / total_questions as f64 * 100.0) as u32
    } else {
        0
    };

    // Collect validation errors across all drafts
    let validation_errors: Vec<(usize, String)> = drafts.read()
        .iter()
        .enumerate()
        .filter_map(|(i, d)| d.validate().map(|msg| (i, msg)))
        .collect();
    let has_validation_errors = !validation_errors.is_empty();

    let on_submit = {
        let eval_id = eval_id.clone();
        let mut error_msg = error_msg.clone();
        let mut success_msg = success_msg.clone();
        let mut submitted = submitted.clone();
        let mut submit_result = submit_result.clone();
        let submit_comment = submit_comment.clone();
        let drafts = drafts.clone();
        move |_| {
            // Client-side validation
            let errs: Vec<String> = drafts.read()
                .iter()
                .enumerate()
                .filter_map(|(i, d)| d.validate().map(|msg| format!("Q{}: {}", i + 1, msg)))
                .collect();
            if !errs.is_empty() {
                *error_msg.write() = errs.join("; ");
                return;
            }

            let t = auth.read().token.clone();
            let eid = eval_id.clone();
            let answers: Vec<SubmitAnswerRequest> = drafts.read()
                .iter()
                .map(|d| d.to_submit_request())
                .collect();
            let comment = submit_comment.read().clone();

            spawn(async move {
                let body = SubmitEvaluationRequest {
                    answers,
                    overall_comment: if comment.is_empty() { None } else { Some(comment) },
                };

                match ApiClient::post::<EvaluationDetail, _>(
                    &format!("/scoring/evaluations/{}/submit", eid),
                    &body,
                    t.as_deref(),
                ).await {
                    Ok(result) => {
                        let status = result.evaluation.status.clone();
                        let score = result.evaluation.final_score.map(|s| format!("{:.1}", s)).unwrap_or_default();
                        let review_msg = if result.evaluation.requires_second_review {
                            " A second review is required because the score delta exceeds the threshold."
                        } else {
                            ""
                        };
                        *success_msg.write() = format!(
                            "Evaluation submitted. Final score: {}. Status: {}.{}",
                            score, status, review_msg,
                        );
                        *error_msg.write() = String::new();
                        *submit_result.write() = Some(result);
                        *submitted.write() = true;
                    }
                    Err(e) => {
                        *error_msg.write() = e;
                        *success_msg.write() = String::new();
                    }
                }
            });
        }
    };

    rsx! {
        div { class: "card", style: "margin-top: 16px;",
            // Header with evaluation ID and status
            div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; flex-wrap: wrap; gap: 8px;",
                h3 { style: "margin: 0;",
                    "Evaluation: {&eval_id[..8]}..."
                }
                span {
                    style: "padding: 2px 10px; border-radius: 10px; font-size: 0.75rem; font-weight: 600; background: {status_color(&eval_detail.evaluation.status)}22; color: {status_color(&eval_detail.evaluation.status)};",
                    "{eval_detail.evaluation.status}"
                }
            }

            if !error_msg.read().is_empty() {
                div { class: "alert alert-error", "{error_msg}" }
            }
            if !success_msg.read().is_empty() {
                div { class: "alert alert-success", "{success_msg}" }
            }

            // Second-review cue on the submit result
            if let Some(result) = &*submit_result.read() {
                if result.evaluation.requires_second_review {
                    div { style: "padding: 10px 14px; background: #fef3c7; border-left: 3px solid #f59e0b; border-radius: 4px; font-size: 0.875rem; margin-bottom: 12px;",
                        strong { "Second review required. " }
                        {
                            let delta = result.evaluation.score_delta.map(|d| format!("{:.1}", d)).unwrap_or_default();
                            format!("Score delta: {}. A QA reviewer must approve or revise before finalization.", delta)
                        }
                    }
                }
            }

            if *submitted.read() {
                p { style: "color: var(--color-text-secondary); font-size: 0.875rem;",
                    "Evaluation has been submitted. Check the Evaluations tab for updated status."
                }
            } else {
                match &*questions_resource.read() {
                    Some(Some(_)) if *drafts_initialized.read() && total_questions > 0 => rsx! {
                        // ---- Progress bar ----
                        div { style: "margin-bottom: 16px;",
                            div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 4px;",
                                span { style: "font-size: 0.8rem; font-weight: 500; color: var(--color-text-secondary);",
                                    "Progress: {touched_count} / {total_questions} questions touched"
                                }
                                span { style: "font-size: 0.8rem; font-weight: 600; color: var(--color-primary);",
                                    "{progress_pct}%"
                                }
                            }
                            div { style: "width: 100%; height: 6px; background: var(--color-border); border-radius: 3px; overflow: hidden;",
                                div {
                                    style: "width: {progress_pct}%; height: 100%; background: var(--color-primary); border-radius: 3px; transition: width 0.3s ease;",
                                }
                            }
                        }

                        // ---- Question cards ----
                        div { style: "display: flex; flex-direction: column; gap: 12px; margin-bottom: 16px;",
                            for (idx, _draft) in drafts.read().iter().enumerate() {
                                QuestionCard {
                                    index: idx,
                                    drafts: drafts,
                                }
                            }
                        }

                        // ---- Overall comment + submit ----
                        div { style: "border-top: 1px solid var(--color-border); padding-top: 16px;",
                            div { style: "margin-bottom: 12px;",
                                label { style: "display: block; font-size: 0.875rem; font-weight: 500; margin-bottom: 4px;",
                                    "Overall Comment (optional)"
                                }
                                textarea {
                                    rows: "2",
                                    style: "width: 100%; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: 4px; font-size: 0.875rem; resize: vertical;",
                                    value: "{submit_comment}",
                                    oninput: move |e| { *submit_comment.write() = e.value().clone(); },
                                }
                            }
                            if has_validation_errors {
                                div { style: "color: var(--color-error); font-size: 0.8rem; margin-bottom: 8px;",
                                    for (qi, msg) in &validation_errors {
                                        p { style: "margin: 2px 0;", "Q{qi + 1}: {msg}" }
                                    }
                                }
                            }
                            button {
                                class: "btn btn-primary",
                                disabled: has_validation_errors,
                                onclick: on_submit,
                                "Submit Evaluation ({touched_count}/{total_questions} answered)"
                            }
                        }
                    },
                    Some(Some(_)) => rsx! {
                        p { style: "color: var(--color-text-secondary); font-size: 0.875rem;",
                            "This template has no active questions."
                        }
                    },
                    Some(None) => rsx! {
                        p { style: "color: var(--color-error); font-size: 0.875rem;",
                            "Failed to load template questions. Check that the template exists and you have access."
                        }
                    },
                    None => rsx! {
                        p { style: "color: var(--color-text-secondary); font-size: 0.875rem;",
                            "Loading questions..."
                        }
                    },
                }
            }
        }
    }
}

// ============================================================
// Individual question grading card
// ============================================================

#[component]
fn QuestionCard(index: usize, drafts: Signal<Vec<QuestionDraft>>) -> Element {
    let draft = drafts.read()[index].clone();
    let is_objective = draft.question_type == "objective";
    let q_num = index + 1;
    let validation_err = draft.validate();
    let border_color = if validation_err.is_some() {
        "var(--color-error, #ef4444)"
    } else if draft.is_touched() {
        "var(--color-primary, #3b82f6)"
    } else {
        "var(--color-border)"
    };

    rsx! {
        div {
            style: "border: 1px solid {border_color}; border-radius: 6px; padding: 14px; background: var(--color-surface);",

            // Question header
            div { style: "display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 10px; gap: 8px;",
                div { style: "flex: 1;",
                    div { style: "display: flex; align-items: center; gap: 6px; margin-bottom: 4px;",
                        span { style: "font-size: 0.75rem; font-weight: 600; color: var(--color-text-secondary);",
                            "Q{q_num}"
                        }
                        span {
                            style: "padding: 1px 6px; border-radius: 8px; font-size: 0.7rem; font-weight: 600; background: {type_badge_bg(is_objective)}; color: {type_badge_fg(is_objective)};",
                            if is_objective { "OBJECTIVE" } else { "SUBJECTIVE" }
                        }
                        if draft.is_required {
                            span { style: "color: var(--color-error, #ef4444); font-size: 0.7rem; font-weight: 600;", "*" }
                        }
                    }
                    p { style: "font-size: 0.875rem; margin: 0; line-height: 1.4;",
                        "{draft.question_text}"
                    }
                }
                div { style: "text-align: right; white-space: nowrap;",
                    p { style: "font-size: 0.7rem; color: var(--color-text-secondary); margin: 0;",
                        "max {draft.max_points:.0} pts"
                    }
                    p { style: "font-size: 0.7rem; color: var(--color-text-secondary); margin: 0;",
                        "weight {draft.weight:.1}"
                    }
                }
            }

            // Editable fields
            div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 10px;",
                // Answer text (always shown)
                div { style: "grid-column: 1 / -1;",
                    label { style: "display: block; font-size: 0.75rem; font-weight: 500; margin-bottom: 2px; color: var(--color-text-secondary);",
                        if is_objective {
                            "Answer Text (matched against correct answer for auto-scoring)"
                        } else {
                            "Answer / Response Notes"
                        }
                    }
                    input {
                        r#type: "text",
                        placeholder: if is_objective {
                            draft.correct_answer.as_deref().map(|a| format!("Correct answer: {}", a)).unwrap_or_else(|| "Type answer...".to_string())
                        } else {
                            "Response notes...".to_string()
                        },
                        style: "width: 100%; padding: 6px 10px; border: 1px solid var(--color-border); border-radius: 4px; font-size: 0.8rem;",
                        value: "{draft.answer_text}",
                        oninput: move |e| {
                            drafts.write()[index].answer_text = e.value().clone();
                        },
                    }
                }

                // Manual score
                div {
                    label { style: "display: block; font-size: 0.75rem; font-weight: 500; margin-bottom: 2px; color: var(--color-text-secondary);",
                        "Manual Score (0\u{2013}{draft.max_points:.0})"
                    }
                    input {
                        r#type: "number",
                        min: "0",
                        max: "{draft.max_points}",
                        step: "0.5",
                        style: "width: 100%; padding: 6px 10px; border: 1px solid var(--color-border); border-radius: 4px; font-size: 0.8rem;",
                        value: "{draft.manual_score}",
                        oninput: move |e| {
                            drafts.write()[index].manual_score = e.value().clone();
                        },
                    }
                }

                // Partial credit
                div {
                    label { style: "display: block; font-size: 0.75rem; font-weight: 500; margin-bottom: 2px; color: var(--color-text-secondary);",
                        "Partial Credit (0.0\u{2013}1.0)"
                    }
                    input {
                        r#type: "number",
                        min: "0",
                        max: "1",
                        step: "0.1",
                        style: "width: 100%; padding: 6px 10px; border: 1px solid var(--color-border); border-radius: 4px; font-size: 0.8rem;",
                        value: "{draft.partial_credit}",
                        oninput: move |e| {
                            drafts.write()[index].partial_credit = e.value().clone();
                        },
                    }
                }

                // Comment
                div { style: "grid-column: 1 / -1;",
                    label { style: "display: block; font-size: 0.75rem; font-weight: 500; margin-bottom: 2px; color: var(--color-text-secondary);",
                        "Grader Comment"
                    }
                    input {
                        r#type: "text",
                        placeholder: "Optional note for this question...",
                        style: "width: 100%; padding: 6px 10px; border: 1px solid var(--color-border); border-radius: 4px; font-size: 0.8rem;",
                        value: "{draft.comment}",
                        oninput: move |e| {
                            drafts.write()[index].comment = e.value().clone();
                        },
                    }
                }
            }

            // Inline validation error
            if let Some(err) = validation_err {
                p { style: "color: var(--color-error, #ef4444); font-size: 0.75rem; margin: 6px 0 0 0;",
                    "{err}"
                }
            }

            // Objective question hint
            if is_objective {
                p { style: "font-size: 0.7rem; color: var(--color-text-secondary); margin: 6px 0 0 0; font-style: italic;",
                    "Auto-scored on submit: if your answer matches the correct answer (case-insensitive), you get {draft.max_points:.0} pts automatically."
                }
            }
        }
    }
}

// ============================================================
// Pending reviews tab
// ============================================================

#[component]
fn PendingReviewsTab(can_write: bool) -> Element {
    let auth = use_context::<Signal<AuthState>>();
    let mut action = use_signal(|| "approve".to_string());
    let mut revised_score = use_signal(|| "".to_string());
    let mut review_comment = use_signal(|| "".to_string());
    let mut selected_eval = use_signal(|| "".to_string());
    let mut error_msg = use_signal(|| String::new());
    let mut success_msg = use_signal(|| String::new());
    let mut refresh = use_signal(|| 0u32);

    let reviews = use_resource(move || {
        let t = auth.read().token.clone();
        let _r = refresh.read();
        async move {
            ApiClient::get::<PaginatedReviews>("/scoring/reviews/pending?limit=50", t.as_deref()).await.ok()
        }
    });

    let on_submit_review = {
        let selected_eval = selected_eval.clone();
        let action = action.clone();
        let revised_score = revised_score.clone();
        let review_comment = review_comment.clone();
        let mut error_msg = error_msg.clone();
        let mut success_msg = success_msg.clone();
        let mut refresh = refresh.clone();
        move |_| {
            let eval_id = selected_eval.read().clone();
            let act = action.read().clone();
            let score_str = revised_score.read().clone();
            let comment = review_comment.read().clone();

            if eval_id.is_empty() {
                *error_msg.write() = "Evaluation ID is required.".to_string();
                return;
            }

            let rev_score = if act == "revise" {
                match score_str.parse::<f64>() {
                    Ok(v) if (0.0..=100.0).contains(&v) => Some(v),
                    _ => {
                        *error_msg.write() = "Revised score must be a number between 0 and 100.".to_string();
                        return;
                    }
                }
            } else {
                None
            };

            let t = auth.read().token.clone();
            spawn(async move {
                let body = SecondReviewRequest {
                    action: act.clone(),
                    revised_score: rev_score,
                    review_comment: if comment.is_empty() { None } else { Some(comment) },
                };
                match ApiClient::post::<EvaluationDetail, _>(
                    &format!("/scoring/reviews/{}", eval_id),
                    &body,
                    t.as_deref(),
                ).await {
                    Ok(result) => {
                        let score = result.evaluation.final_score.map(|s| format!("{:.1}", s)).unwrap_or_default();
                        *success_msg.write() = format!("Review processed ({}). Final score: {}", act, score);
                        *error_msg.write() = String::new();
                        *refresh.write() += 1;
                    }
                    Err(e) => {
                        *error_msg.write() = e;
                        *success_msg.write() = String::new();
                    }
                }
            });
        }
    };

    rsx! {
        div {
            if !error_msg.read().is_empty() {
                div { class: "alert alert-error", "{error_msg}" }
            }
            if !success_msg.read().is_empty() {
                div { class: "alert alert-success", "{success_msg}" }
            }

            div { class: "card", style: "overflow-x: auto;",
                div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px;",
                    h3 { style: "margin: 0;", "Pending Second Reviews" }
                    span { style: "font-size: 0.875rem; color: var(--color-text-secondary);",
                        "Evaluations where |score delta| > 10 require QA Reviewer sign-off"
                    }
                }
                table { style: "width: 100%; border-collapse: collapse;",
                    thead {
                        tr { style: "border-bottom: 1px solid var(--color-border);",
                            th { style: "padding: 10px 12px; text-align: left; font-size: 0.875rem;", "Review ID" }
                            th { style: "padding: 10px 12px; text-align: left; font-size: 0.875rem;", "Evaluation ID" }
                            th { style: "padding: 10px 12px; text-align: right; font-size: 0.875rem;", "Score Before" }
                            th { style: "padding: 10px 12px; text-align: right; font-size: 0.875rem;", "Delta" }
                            th { style: "padding: 10px 12px; text-align: left; font-size: 0.875rem;", "Reviewer" }
                        }
                    }
                    tbody {
                        match &*reviews.read() {
                            Some(Some(resp)) if !resp.data.is_empty() => rsx! {
                                for review in &resp.data {
                                    tr { style: "border-bottom: 1px solid var(--color-border);",
                                        td { style: "padding: 8px 12px; font-size: 0.8rem; font-family: monospace;",
                                            "{&review.id[..8]}..."
                                        }
                                        td { style: "padding: 8px 12px; font-size: 0.8rem; font-family: monospace;",
                                            "{&review.evaluation_id[..8]}..."
                                        }
                                        td { style: "padding: 8px 12px; text-align: right; font-size: 0.875rem;",
                                            "{review.score_before_review:.1}"
                                        }
                                        td { style: "padding: 8px 12px; text-align: right; font-size: 0.875rem; font-weight: 600;",
                                            "{review.score_delta:.1}"
                                        }
                                        td { style: "padding: 8px 12px; font-size: 0.8rem; font-family: monospace;",
                                            "{&review.reviewer_id[..8]}..."
                                        }
                                    }
                                }
                            },
                            Some(Some(_)) => rsx! {
                                tr {
                                    td { colspan: "5", style: "padding: 24px; text-align: center; color: var(--color-text-secondary); font-size: 0.875rem;",
                                        "No pending reviews at this time."
                                    }
                                }
                            },
                            Some(None) => rsx! {
                                tr {
                                    td { colspan: "5", style: "padding: 24px; text-align: center; color: var(--color-error);",
                                        "Failed to load pending reviews."
                                    }
                                }
                            },
                            None => rsx! {
                                tr {
                                    td { colspan: "5", style: "padding: 24px; text-align: center; color: var(--color-text-secondary);",
                                        "Loading..."
                                    }
                                }
                            },
                        }
                    }
                }
            }

            if can_write {
                div { class: "card", style: "max-width: 480px; margin-top: 16px;",
                    h4 { style: "margin-bottom: 14px;", "Process a Review" }
                    div { style: "display: flex; flex-direction: column; gap: 12px;",
                        div {
                            label { style: "display: block; font-size: 0.875rem; font-weight: 500; margin-bottom: 4px;",
                                "Evaluation ID *"
                            }
                            input {
                                r#type: "text",
                                placeholder: "Evaluation ID to review",
                                style: "width: 100%; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: 4px; font-size: 0.875rem;",
                                value: "{selected_eval}",
                                oninput: move |e| { *selected_eval.write() = e.value().clone(); },
                            }
                        }
                        div {
                            label { style: "display: block; font-size: 0.875rem; font-weight: 500; margin-bottom: 4px;",
                                "Action"
                            }
                            select {
                                style: "width: 100%; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: 4px; font-size: 0.875rem;",
                                onchange: move |e| { *action.write() = e.value().clone(); },
                                option { value: "approve", "Approve \u{2014} keep current score" }
                                option { value: "revise", "Revise \u{2014} set new score" }
                            }
                        }
                        if *action.read() == "revise" {
                            div {
                                label { style: "display: block; font-size: 0.875rem; font-weight: 500; margin-bottom: 4px;",
                                    "Revised Score (0\u{2013}100) *"
                                }
                                input {
                                    r#type: "number",
                                    min: "0",
                                    max: "100",
                                    step: "0.5",
                                    placeholder: "e.g. 72.5",
                                    style: "width: 100%; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: 4px; font-size: 0.875rem;",
                                    value: "{revised_score}",
                                    oninput: move |e| { *revised_score.write() = e.value().clone(); },
                                }
                            }
                        }
                        div {
                            label { style: "display: block; font-size: 0.875rem; font-weight: 500; margin-bottom: 4px;",
                                "Review Comment (optional)"
                            }
                            textarea {
                                rows: "2",
                                style: "width: 100%; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: 4px; font-size: 0.875rem; resize: vertical;",
                                value: "{review_comment}",
                                oninput: move |e| { *review_comment.write() = e.value().clone(); },
                            }
                        }
                        button {
                            class: "btn btn-primary",
                            style: "align-self: flex-start;",
                            onclick: on_submit_review,
                            "Submit Review"
                        }
                    }
                }
            }
        }
    }
}

// ============================================================
// Shared helpers
// ============================================================

fn status_color(status: &str) -> &'static str {
    match status {
        "finalized" => "#22c55e",
        "second_review_required" => "#f59e0b",
        "submitted" => "#3b82f6",
        "draft" => "#94a3b8",
        _ => "#94a3b8",
    }
}

fn type_badge_bg(is_objective: bool) -> &'static str {
    if is_objective { "#dbeafe" } else { "#fef3c7" }
}

fn type_badge_fg(is_objective: bool) -> &'static str {
    if is_objective { "#1e40af" } else { "#92400e" }
}

#[component]
fn TabButton(label: &'static str, active: bool, onclick: EventHandler<MouseEvent>) -> Element {
    let style = if active {
        "padding: 8px 16px; border: none; border-bottom: 2px solid var(--color-primary); background: transparent; font-weight: 600; cursor: pointer; color: var(--color-primary);"
    } else {
        "padding: 8px 16px; border: none; border-bottom: 2px solid transparent; background: transparent; cursor: pointer; color: var(--color-text-secondary);"
    };
    rsx! {
        button { style: style, onclick: move |e| onclick.call(e), "{label}" }
    }
}
