# Scoring, Second Review, Reporting & Exports

## Overview

Phase 5 of CareOps adds quality management and operational analytics capabilities:

- **Quality scoring**: Evaluate service delivery entries using configurable templates with objective (auto-scored) and subjective (manually graded) question types, weighted aggregation, and per-template rounding intervals.
- **Second-review escalation**: Backend-enforced when score delta exceeds 10 points. Prevents finalization until a QA Reviewer approves or revises the score.
- **Reporting**: Order volume, revenue, provider utilization, and KPI analytics computed from real stored data with date-range and department/project filters.
- **Permission-aware exports**: All exports are masked by default. Unmasked exports require the `api.export.unmasked` permission and are always logged in the export audit trail.

---

## Scoring Workflow

### State Machine

```
draft
  └─→ [POST /evaluations/:id/submit]
        ├─→ finalized                     (|new_score − prior_score| ≤ 10 OR first evaluation)
        └─→ second_review_required        (|new_score − prior_score| > 10)
                └─→ [POST /reviews/:eval_id]
                      ├─→ finalized       (action = "approve")
                      └─→ finalized       (action = "revise", revised_score set by reviewer)
```

### Scoring Formulas

**Per-answer final score:**
```
final = min(auto_score + manual_score + (partial_credit_fraction × max_points), max_points)
final = max(final, 0)
```
- `auto_score`: `max_points` if answer_text matches `correct_answer` (case-insensitive, trimmed), else `0`. Only for objective questions.
- `manual_score`: Set by evaluator for subjective questions, or as override for objective.
- `partial_credit_fraction`: A 0.0–1.0 fraction of `max_points`.

**Weighted evaluation score (0–100 scale):**
```
weighted = Σ(answer_final / question_max_points × question_weight) / Σ(question_weight) × 100
```

**Final score after rounding:**
```
final_score = round(weighted / rounding_interval) × rounding_interval
```
Default `rounding_interval` = 0.5. Example: `round_to_interval(7.3, 0.5) = 7.5`.

### Second Review Rule

Second review is **required** (enforced by the backend) when:
```
|new_final_score − prior_final_score| > 10.0
```

- First evaluation against a delivery entry never triggers second review (no prior score).
- When triggered, the evaluation status transitions to `second_review_required`.
- A `score_reviews` record is created with `review_status = 'pending'`.
- An **independent** QA Reviewer (first active user with QA Reviewer role in the org, excluding the evaluator) is auto-assigned.
- If no independent QA Reviewer is available, **no review record is created** and the evaluation remains in `second_review_required`. It cannot be finalized until an eligible reviewer is assigned. An audit event (`scoring.second_review.unassigned`) and a warning log are emitted to signal the missing assignment.
- The evaluator is **never** allowed to review their own evaluation, even if manually assigned (defense-in-depth).
- Until the review is processed by an independent reviewer, the evaluation **cannot** be considered finalized.

---

## Database Tables

| Table | Purpose |
|---|---|
| `scoring_templates` | Configurable templates: name, rounding_interval, max_score, org-scoped |
| `evaluation_questions` | Questions per template: objective/subjective, weight, max_points, correct_answer |
| `evaluations` | Grading attempts: lifecycle state, computed scores, prior_final_score for delta |
| `evaluation_answers` | Per-question answers: auto_score, manual_score, partial_credit, final_score |
| `score_reviews` | Second review records: score_before_review, score_delta, pending/approved/revised |
| `export_audit_logs` | Every export event: who, what, when, masked flag, permission_used |

### One-finalized-per-delivery constraint

MySQL does not support partial/filtered unique indexes. The constraint "one finalized evaluation per delivery+template" is enforced at the application layer:

1. `start_evaluation` fetches the most recent finalized score for delta tracking.
2. Multiple drafts can exist; only one becomes finalized.
3. Application enforces correct status transitions.

---

## API Reference

### Templates

| Method | Path | Permission | Description |
|---|---|---|---|
| `POST` | `/api/scoring/templates` | `api.scoring.write` | Create a new scoring template with questions |
| `GET` | `/api/scoring/templates?active_only=true` | `api.scoring.read` | List templates |
| `GET` | `/api/scoring/templates/:id` | `api.scoring.read` | Get template detail with questions |

**Create template request:**
```json
{
  "name": "Standard QA",
  "description": "Optional description",
  "rounding_interval": 0.5,
  "max_score": 100.0,
  "questions": [
    {
      "question_text": "Was documentation complete?",
      "question_type": "objective",
      "weight": 1.5,
      "max_points": 10.0,
      "correct_answer": "yes",
      "sort_order": 0,
      "is_required": true
    },
    {
      "question_text": "Rate the service quality.",
      "question_type": "subjective",
      "weight": 1.0,
      "max_points": 10.0,
      "sort_order": 1,
      "is_required": true
    }
  ]
}
```

### Evaluations

| Method | Path | Permission | Description |
|---|---|---|---|
| `POST` | `/api/scoring/evaluations` | `api.scoring.write` | Start a new evaluation (status: draft) |
| `GET` | `/api/scoring/evaluations?status=&evaluator_id=&limit=&offset=` | `api.scoring.read` | List evaluations with filters |
| `GET` | `/api/scoring/evaluations/:id` | `api.scoring.read` | Get evaluation detail with answers |
| `POST` | `/api/scoring/evaluations/:id/submit` | `api.scoring.write` | Submit answers and compute scores |

**Submit evaluation request:**
```json
{
  "answers": [
    {
      "question_id": "...",
      "answer_text": "yes",
      "manual_score": null,
      "partial_credit_fraction": null,
      "comment": "Objective auto-scored"
    },
    {
      "question_id": "...",
      "answer_text": null,
      "manual_score": 7.5,
      "partial_credit_fraction": null,
      "comment": "Good but room for improvement"
    }
  ],
  "overall_comment": "Satisfactory delivery"
}
```

### Second Reviews

| Method | Path | Permission | Description |
|---|---|---|---|
| `GET` | `/api/scoring/reviews/pending?reviewer_id=&limit=&offset=` | `api.scoring.read` | List pending second reviews |
| `POST` | `/api/scoring/reviews/:eval_id` | `api.scoring.write` | Process a pending review (approve or revise) |

**Review request:**
```json
{
  "action": "approve",
  "review_comment": "Score delta justified by documentation errors"
}
```
```json
{
  "action": "revise",
  "revised_score": 72.5,
  "review_comment": "Adjusted for partial documentation"
}
```

---

## Reports API Reference

All report endpoints require `api.reports.read` permission.

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/reports/kpi` | KPI summary for date range |
| `GET` | `/api/reports/order-volume` | Delivery counts by week |
| `GET` | `/api/reports/revenue` | Invoice/payment/refund aggregates by week |
| `GET` | `/api/reports/utilization` | Provider visits/units/mileage by week |
| `POST` | `/api/reports/export` | Permission-aware data export |

**Common query parameters:** `from_date=YYYY-MM-DD&to_date=YYYY-MM-DD&department_id=&project_id=&service_route=`

All filter parameters except date range are optional. Omitting them returns unfiltered data (backward compatible).

**`service_route`** — Optional label filter for the service route dimension (e.g. `north-metro`, `client-to-clinic`). Maps to `client_plans.service_route`. When omitted, all routes are included. When provided, must be a non-empty string (max 100 chars); whitespace-only values return 400.

### KPI Definitions

| KPI | Formula | Source |
|---|---|---|
| Attendance rate | `verified / (submitted + verified) × 100` | `delivery_entries.status` |
| Repurchase rate | `plans with ≥2 invoice periods / total plans × 100` | `invoices` grouped by plan |
| Staff utilization | `(avg deliveries / provider / week) / 20 × 100` | `delivery_entries`, verified only |
| Avg quality score | `AVG(final_score)` of finalized evaluations | `evaluations.final_score` |
| Second review rate | `evaluations needing review / total evaluations × 100` | `evaluations.requires_second_review` |

---

## Export Masking

### Default behavior (masked)

All exports mask identifying fields by default:
- `client_name` → `"****"`
- `plan_id` → `"<first-8-chars>-****"`
- `provider_id` → `"<first-8-chars>-****"`
- `evaluator_id` → `"<first-8-chars>-****"`

### Requesting unmasked exports

Set `"unmasked": true` in the export request body **and** hold the `api.export.unmasked` permission. Without the permission, the request succeeds but identifiers remain masked.

### Export types

| Type | Content |
|---|---|
| `deliveries` | Delivery entries with service name, units, mileage, status |
| `evaluations` | Evaluations with scores, template, status, evaluator |
| `revenue` | Invoices with amounts, billing periods, status |

### Audit trail

Every export event is recorded in `export_audit_logs`:
- `org_id`, `exported_by`, `export_type`
- `filters_json` — JSON snapshot of applied filters
- `row_count` — number of rows returned
- `masked` — 1 (default) or 0 (unmasked)
- `permission_used` — `api.export.unmasked` if unmasked, NULL otherwise

---

## Role Access Matrix

| Role | Scoring Read | Scoring Write | Reports | Export | Unmasked Export |
|---|---|---|---|---|---|
| System Administrator | ✓ | ✓ | ✓ | ✓ | ✓ |
| Operations Manager | ✓ | — | ✓ | — | — |
| QA Reviewer | ✓ | ✓ | ✓ | — | — |
| Billing Specialist | — | — | ✓ | — | — |
| Coach/Clinician | ✓ (read) | — | — | — | — |
| Auditor | — | — | ✓ | ✓ | — |

---

## Unit Tests

Pure scoring logic functions are unit-tested in `backend/src/domain/scoring_types.rs`:

- `round_to_interval`: Half-point rounding, unit rounding, zero-interval passthrough
- `compute_auto_score`: Case-insensitive match, trim, mismatch returns 0
- `compute_answer_final_score`: Auto-only, manual-only, partial credit, capped at max, no negatives
- `compute_weighted_score`: Equal weights → expected percentage, unequal weights → weighted average, empty
- `requires_second_review`: Delta exactly 10 does NOT trigger, delta > 10 triggers, decrease > 10 triggers, first evaluation never triggers

Run with: `cargo test --lib scoring_types`
