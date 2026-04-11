# CareOps Audit Recheck (Static) — Follow-up to `audit_report-1.md`

- **Date:** 2026-04-10
- **Method:** Static-only inspection (no runtime/test/docker execution)
- **Base report reviewed:** `.tmp/audit_report-1.md`

## Executive Result

All previously listed issues in `audit_report-1.md` are now **fixed in code** based on static evidence.

## Issue-by-Issue Status

| Prior Issue                                                                    | Previous Status         | Current Status | Evidence                                                                                                                                                                                                                                                                                                                                                |
| ------------------------------------------------------------------------------ | ----------------------- | -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Inconsistent route-level data-scope enforcement in payments/refunds APIs       | High                    | **Fixed**      | `require_data_scope(...)` now present throughout `repo/backend/src/api/payments_refunds/mod.rs` at `:53`, `:79`, `:94`, `:111`, `:137`, `:152`, `:169`, `:197`, `:227`, `:241`, `:256`.                                                                                                                                                                 |
| Scoring UI lacked clear question-level manual/partial grading workflow         | High                    | **Fixed**      | Question-level grading structures/components now implemented in `repo/frontend/src/pages/scoring/mod.rs`: `QuestionDraft` (`:341+`), template question load (`:440-463`), progress tracking (`:473-476`, `:597`), question cards (`:610+`, `QuestionCard` at `:674+`), manual/partial fields (`:757+`), submit payload from edited drafts (`:416-419`). |
| Export request lacked project-level scope propagation                          | Medium                  | **Fixed**      | `project_id` added to `ExportRequest` in `repo/backend/src/domain/scoring_types.rs:190`; export scope guard now passes both department+project in `repo/backend/src/api/reports_exports/mod.rs:130`.                                                                                                                                                    |
| Ops API tests had endpoint drift (`/api/ops/toggles` vs `/api/ops/flags`)      | Medium                  | **Fixed**      | `repo/API_tests/test_ops.sh` now uses `/api/ops/flags` paths (e.g., `:158`, `:187`, `:199`, `:220`, `:295`, `:298`, `:309`) and includes note confirming flags endpoint (`:288`).                                                                                                                                                                       |
| Login-failure audit logged raw username                                        | Medium (Suspected Risk) | **Fixed**      | `repo/backend/src/application/auth_service.rs` now hashes attempted username and logs hash prefix (`:73-78`, `:86-87`) instead of raw username for `user_not_found`.                                                                                                                                                                                    |
| Smoke test used ambiguous root route checks (`/api/scoring/`, `/api/reports/`) | Medium                  | **Fixed**      | `repo/API_tests/test_smoke.sh` now checks concrete endpoints (`/api/scoring/templates`, `/api/reports/export`) at `:52-53` instead of ambiguous roots.                                                                                                                                                                                                  |

## Updated Acceptance View (for previously reported defects only)

- **Remediation status for prior findings:** **Complete (static evidence)**
- **Boundary note:** This report confirms source-level fixes only; runtime behavior remains **manual verification required**.

## Notes

- No source code was modified during this recheck.
- This artifact only verifies issues from `audit_report-1.md`, not a full new end-to-end audit scope expansion.
