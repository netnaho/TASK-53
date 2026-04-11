-- CareOps Phase 5: Quality Scoring, Second Review, Reporting, KPI, Exports
-- Migration: 20240105000000_scoring_reporting
--
-- Implements:
--   - Evaluation templates with configurable question types and weights
--   - Evaluation questions with objective/subjective type distinction
--   - Evaluations (grading attempts) with full lifecycle states
--   - Per-question answers with auto-score, manual-score, and partial credit
--   - Score reviews for mandatory second-review workflow when delta > 10
--   - Export audit logs (records every data export with masking state)

-- ============================================================
-- Scoring Templates
-- ============================================================
CREATE TABLE scoring_templates (
    id CHAR(36) PRIMARY KEY,
    org_id CHAR(36) NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    -- Rounding interval: 0.5 means round to nearest half-point
    rounding_interval DECIMAL(4,2) NOT NULL DEFAULT 0.5,
    -- Max possible score (for percentage calculations)
    max_score DECIMAL(6,2) NOT NULL DEFAULT 100.0,
    is_active TINYINT(1) NOT NULL DEFAULT 1,
    created_by CHAR(36) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (org_id) REFERENCES organizations(id),
    FOREIGN KEY (created_by) REFERENCES users(id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- Evaluation Questions
-- Each question belongs to exactly one template.
-- question_type: 'objective' (auto-scored) | 'subjective' (manual grading)
-- weight: relative weight in final score aggregation (0.0 to 1.0, normalized at compute time)
-- ============================================================
CREATE TABLE evaluation_questions (
    id CHAR(36) PRIMARY KEY,
    template_id CHAR(36) NOT NULL,
    question_text TEXT NOT NULL,
    question_type ENUM('objective', 'subjective') NOT NULL DEFAULT 'subjective',
    -- Weight for weighted aggregation; normalized across all questions at compute time
    weight DECIMAL(5,2) NOT NULL DEFAULT 1.0,
    max_points DECIMAL(6,2) NOT NULL DEFAULT 10.0,
    -- For objective questions: the correct answer(s) as JSON ["A", "B"] or plain text
    correct_answer TEXT,
    sort_order INT NOT NULL DEFAULT 0,
    is_required TINYINT(1) NOT NULL DEFAULT 1,
    is_active TINYINT(1) NOT NULL DEFAULT 1,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (template_id) REFERENCES scoring_templates(id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- Evaluations (grading attempts)
-- One evaluation per delivery entry per template.
-- status lifecycle:
--   draft -> submitted -> (delta > 10) second_review_required -> reviewed -> finalized
--                      -> (delta <= 10) finalized directly from submitted
-- ============================================================
CREATE TABLE evaluations (
    id CHAR(36) PRIMARY KEY,
    org_id CHAR(36) NOT NULL,
    delivery_entry_id CHAR(36) NOT NULL,
    template_id CHAR(36) NOT NULL,
    evaluator_id CHAR(36) NOT NULL,
    status ENUM('draft', 'submitted', 'second_review_required', 'reviewed', 'finalized') NOT NULL DEFAULT 'draft',
    -- Prior finalized score (to detect delta on re-evaluation)
    prior_final_score DECIMAL(6,2) NULL,
    -- Computed scores at submission time
    raw_score DECIMAL(6,2) NULL,       -- simple sum before weighting
    weighted_score DECIMAL(6,2) NULL,  -- weighted average (0-100 scale)
    final_score DECIMAL(6,2) NULL,     -- after rounding to nearest interval
    -- Second-review tracking
    requires_second_review TINYINT(1) NOT NULL DEFAULT 0,
    score_delta DECIMAL(6,2) NULL,     -- |new_final_score - prior_final_score|; NULL on first eval
    second_reviewer_id CHAR(36) NULL,
    second_reviewed_at TIMESTAMP NULL,
    -- Overall evaluator comment
    overall_comment TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (org_id) REFERENCES organizations(id),
    FOREIGN KEY (delivery_entry_id) REFERENCES delivery_entries(id),
    FOREIGN KEY (template_id) REFERENCES scoring_templates(id),
    FOREIGN KEY (evaluator_id) REFERENCES users(id),
    FOREIGN KEY (second_reviewer_id) REFERENCES users(id),
    -- MySQL does not support partial/filtered unique indexes.
    -- The "one finalized evaluation per delivery+template" constraint
    -- is enforced at the application layer in scoring_service.rs.
    INDEX idx_eval_delivery_template (delivery_entry_id, template_id),
    INDEX idx_eval_org_status (org_id, status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- Evaluation Answers
-- One row per question per evaluation.
-- auto_score: computed for objective questions (0 or max_points)
-- manual_score: set by evaluator for subjective, or override for objective
-- partial_credit: additional credit (0.0 to 1.0 fraction of max_points)
-- final_score: auto_score + manual_score + (partial_credit * max_points)
-- ============================================================
CREATE TABLE evaluation_answers (
    id CHAR(36) PRIMARY KEY,
    evaluation_id CHAR(36) NOT NULL,
    question_id CHAR(36) NOT NULL,
    -- For objective: the submitted answer text
    answer_text TEXT,
    -- Objective auto-score: set automatically on submit
    auto_score DECIMAL(6,2) NOT NULL DEFAULT 0.0,
    -- Manual grading score (for subjective or objective override)
    manual_score DECIMAL(6,2) NOT NULL DEFAULT 0.0,
    -- Partial credit as a fraction (0.0 to 1.0) applied to max_points
    partial_credit_fraction DECIMAL(4,3) NOT NULL DEFAULT 0.0,
    -- Computed: auto_score + manual_score + (partial_credit_fraction * question.max_points)
    -- Capped at question.max_points
    final_score DECIMAL(6,2) NOT NULL DEFAULT 0.0,
    comment TEXT,
    graded_by CHAR(36) NULL,
    graded_at TIMESTAMP NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (evaluation_id) REFERENCES evaluations(id),
    FOREIGN KEY (question_id) REFERENCES evaluation_questions(id),
    FOREIGN KEY (graded_by) REFERENCES users(id),
    UNIQUE KEY uq_answer_per_question (evaluation_id, question_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- Score Reviews
-- Created when requires_second_review = 1 (delta > 10 points).
-- A QA Reviewer must approve or revise before finalization.
-- ============================================================
CREATE TABLE score_reviews (
    id CHAR(36) PRIMARY KEY,
    evaluation_id CHAR(36) NOT NULL,
    reviewer_id CHAR(36) NOT NULL,
    -- Score values at time of review request
    score_before_review DECIMAL(6,2) NOT NULL,
    score_delta DECIMAL(6,2) NOT NULL,
    -- Reviewer's outcome
    review_status ENUM('pending', 'approved', 'revised') NOT NULL DEFAULT 'pending',
    -- If revised: the new score the reviewer sets
    revised_score DECIMAL(6,2) NULL,
    review_comment TEXT,
    reviewed_at TIMESTAMP NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (evaluation_id) REFERENCES evaluations(id),
    FOREIGN KEY (reviewer_id) REFERENCES users(id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- Export Audit Logs
-- Records every data export event: who, what, when, masked/unmasked.
-- Separate from general audit_logs to allow targeted compliance queries.
-- ============================================================
CREATE TABLE export_audit_logs (
    id CHAR(36) PRIMARY KEY,
    org_id CHAR(36) NOT NULL,
    exported_by CHAR(36) NOT NULL,
    export_type VARCHAR(50) NOT NULL,          -- 'deliveries', 'evaluations', 'revenue', etc.
    filters_json TEXT,                          -- JSON snapshot of applied filters
    row_count INT NOT NULL DEFAULT 0,
    -- masked: 1 = identifiers were masked (default); 0 = unmasked (requires explicit permission)
    masked TINYINT(1) NOT NULL DEFAULT 1,
    permission_used VARCHAR(100),               -- which permission allowed unmasking if masked=0
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (org_id) REFERENCES organizations(id),
    FOREIGN KEY (exported_by) REFERENCES users(id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================
-- Seed: Default scoring template for the demo organization
-- (Uses a placeholder org_id that will be overwritten by the
--  seed_service; the migration just creates the table structure.
--  Actual template seeding is done in seed_service.rs.)
-- ============================================================
