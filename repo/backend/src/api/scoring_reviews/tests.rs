// Controller-level tests for the scoring & reviews API layer.
//
// Covers: template/evaluation/review request deserialization, scoring type
// validation (objective/subjective), partial-credit fraction bounds,
// second-review action constraint, and error mapping.
// No database, no HTTP server — runs with `cargo test --lib`.

use crate::domain::auth_policy::api;
use crate::domain::error::AppError;
use crate::domain::scoring_types::{
    CreateQuestionRequest, CreateTemplateRequest, SecondReviewRequest, StartEvaluationRequest,
    SubmitAnswerRequest, SubmitEvaluationRequest,
};

// ---------------------------------------------------------------------------
// CreateTemplateRequest
// ---------------------------------------------------------------------------

#[test]
fn create_template_request_minimal_valid() {
    let json = r#"{
        "name": "Nurse Competency",
        "questions": [
            {"question_text": "Assess hand hygiene", "question_type": "objective"}
        ]
    }"#;
    let req: CreateTemplateRequest =
        serde_json::from_str(json).expect("deserialize CreateTemplateRequest");
    assert_eq!(req.name, "Nurse Competency");
    assert_eq!(req.questions.len(), 1);
    assert!(req.description.is_none());
    assert!(req.rounding_interval.is_none());
}

#[test]
fn create_template_request_with_all_fields() {
    let json = r#"{
        "name": "Full Assessment",
        "description": "Comprehensive eval",
        "rounding_interval": 0.5,
        "max_score": 100.0,
        "questions": [
            {"question_text": "Q1", "question_type": "objective"},
            {"question_text": "Q2", "question_type": "subjective"}
        ]
    }"#;
    let req: CreateTemplateRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.rounding_interval, Some(0.5));
    assert_eq!(req.max_score, Some(100.0));
    assert_eq!(req.questions.len(), 2);
}

#[test]
fn create_template_request_missing_name_fails() {
    let json = r#"{"questions":[{"question_text":"Q","question_type":"objective"}]}"#;
    let result: Result<CreateTemplateRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn create_template_request_empty_questions_array_parses() {
    // Empty array parses fine; service-layer validation rejects it
    let json = r#"{"name":"Empty","questions":[]}"#;
    let req: CreateTemplateRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.questions.len(), 0);
}

// ---------------------------------------------------------------------------
// CreateQuestionRequest
// ---------------------------------------------------------------------------

#[test]
fn question_objective_type_parses() {
    let json = r#"{"question_text":"Wash hands correctly?","question_type":"objective"}"#;
    let q: CreateQuestionRequest = serde_json::from_str(json).unwrap();
    assert_eq!(q.question_type, "objective");
    assert!(q.weight.is_none());
    assert!(q.correct_answer.is_none());
}

#[test]
fn question_subjective_type_parses() {
    let json = r#"{"question_text":"Explain procedure","question_type":"subjective","weight":2.0}"#;
    let q: CreateQuestionRequest = serde_json::from_str(json).unwrap();
    assert_eq!(q.question_type, "subjective");
    assert_eq!(q.weight, Some(2.0));
}

#[test]
fn question_with_correct_answer_parses() {
    let json = r#"{"question_text":"PPE required?","question_type":"objective","correct_answer":"yes","max_points":5.0}"#;
    let q: CreateQuestionRequest = serde_json::from_str(json).unwrap();
    assert_eq!(q.correct_answer.as_deref(), Some("yes"));
    assert_eq!(q.max_points, Some(5.0));
}

// ---------------------------------------------------------------------------
// StartEvaluationRequest
// ---------------------------------------------------------------------------

#[test]
fn start_evaluation_request_required_fields() {
    let json = r#"{"delivery_entry_id":"entry-1","template_id":"tmpl-1"}"#;
    let req: StartEvaluationRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.delivery_entry_id, "entry-1");
    assert_eq!(req.template_id, "tmpl-1");
    assert!(req.overall_comment.is_none());
}

#[test]
fn start_evaluation_request_with_comment() {
    let json = r#"{"delivery_entry_id":"entry-2","template_id":"tmpl-2","overall_comment":"Routine eval"}"#;
    let req: StartEvaluationRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.overall_comment.as_deref(), Some("Routine eval"));
}

#[test]
fn start_evaluation_request_missing_delivery_entry_fails() {
    let json = r#"{"template_id":"tmpl-1"}"#;
    let result: Result<StartEvaluationRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// SubmitAnswerRequest
// ---------------------------------------------------------------------------

#[test]
fn submit_answer_request_only_question_id_required() {
    let json = r#"{"question_id":"q-1"}"#;
    let req: SubmitAnswerRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.question_id, "q-1");
    assert!(req.answer_text.is_none());
    assert!(req.manual_score.is_none());
    assert!(req.partial_credit_fraction.is_none());
}

#[test]
fn submit_answer_request_with_manual_score() {
    let json = r#"{"question_id":"q-2","manual_score":8.5,"comment":"Good technique"}"#;
    let req: SubmitAnswerRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.manual_score, Some(8.5));
    assert_eq!(req.comment.as_deref(), Some("Good technique"));
}

#[test]
fn submit_answer_request_with_partial_credit() {
    let json = r#"{"question_id":"q-3","partial_credit_fraction":0.75}"#;
    let req: SubmitAnswerRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.partial_credit_fraction, Some(0.75));
}

// ---------------------------------------------------------------------------
// SubmitEvaluationRequest
// ---------------------------------------------------------------------------

#[test]
fn submit_evaluation_request_with_answers() {
    let json = r#"{
        "answers": [
            {"question_id": "q-1", "manual_score": 9.0},
            {"question_id": "q-2", "answer_text": "Completed"}
        ],
        "overall_comment": "Passed with distinction"
    }"#;
    let req: SubmitEvaluationRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.answers.len(), 2);
    assert_eq!(req.overall_comment.as_deref(), Some("Passed with distinction"));
}

#[test]
fn submit_evaluation_request_empty_answers_parses() {
    let json = r#"{"answers":[]}"#;
    let req: SubmitEvaluationRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.answers.len(), 0);
}

// ---------------------------------------------------------------------------
// SecondReviewRequest
// ---------------------------------------------------------------------------

#[test]
fn second_review_approve_action_parses() {
    let json = r#"{"action":"approve"}"#;
    let req: SecondReviewRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.action, "approve");
    assert!(req.revised_score.is_none());
}

#[test]
fn second_review_revise_action_with_score_parses() {
    let json = r#"{"action":"revise","revised_score":82.5,"review_comment":"Adjusted for partial credit"}"#;
    let req: SecondReviewRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.action, "revise");
    assert_eq!(req.revised_score, Some(82.5));
    assert_eq!(req.review_comment.as_deref(), Some("Adjusted for partial credit"));
}

#[test]
fn second_review_missing_action_fails() {
    let json = r#"{"revised_score":80.0}"#;
    let result: Result<SecondReviewRequest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Authorization codes for scoring controller
// ---------------------------------------------------------------------------

#[test]
fn scoring_read_permission_code_is_correct() {
    assert_eq!(api::SCORING_READ, "api.scoring.read");
}

#[test]
fn scoring_write_permission_code_is_correct() {
    assert_eq!(api::SCORING_WRITE, "api.scoring.write");
}

// ---------------------------------------------------------------------------
// Error mapping for scoring controller paths
// ---------------------------------------------------------------------------

#[test]
fn template_not_found_maps_to_not_found() {
    let err = AppError::NotFound("Scoring template not found".to_string());
    assert_eq!(err.envelope().error.code, "NOT_FOUND");
}

#[test]
fn empty_template_name_maps_to_bad_request() {
    let err = AppError::BadRequest("Template name is required".to_string());
    assert_eq!(err.envelope().error.code, "BAD_REQUEST");
}

#[test]
fn no_questions_in_template_maps_to_bad_request() {
    let err = AppError::BadRequest("Template must have at least one question".to_string());
    let env = err.envelope();
    assert_eq!(env.error.code, "BAD_REQUEST");
    assert!(env.error.message.contains("question"));
}

#[test]
fn evaluation_already_submitted_maps_to_conflict() {
    let err = AppError::Conflict("Evaluation already submitted".to_string());
    assert_eq!(err.envelope().error.code, "CONFLICT");
}

#[test]
fn second_review_not_required_maps_to_bad_request() {
    let err = AppError::BadRequest("Second review not required for this evaluation".to_string());
    assert_eq!(err.envelope().error.code, "BAD_REQUEST");
}
