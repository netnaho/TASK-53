# Business Logic Questions Log (TASK-53 CareOps)

This log captures key business-level ambiguities identified while interpreting the TASK-53 prompt, along with the working hypothesis and the implemented solution reflected in the current repository.

---

## 1) Role-adaptive UI vs backend enforcement scope

**Question:** The prompt says UI adapts by role (System Administrator, Operations Manager, Billing Specialist, Coach/Clinician, QA Reviewer, Auditor). Is role gating only a frontend concern, or must backend enforce the same restrictions on every API route?

**My Understanding/Hypothesis:** Frontend visibility is advisory for UX; backend must be the source of truth to prevent privilege bypass.

**Solution:** Implemented fine-grained backend authorization across menu/action/API permissions and route guards, with role-permission mappings and per-endpoint checks. Frontend checks remain convenience-only.

---

## 2) Data scope boundaries (org/department/project)

**Question:** The prompt requires data-scope rules by organization, department, and project, but does not explicitly define whether user/role management APIs must also respect target-user scope boundaries.

**My Understanding/Hypothesis:** Scope should be enforced consistently, including role/scope assignment endpoints, not just business data endpoints.

**Solution:** Added and enforced scope validation for user-role and user-scope management operations so cross-org privilege manipulation is blocked.

---

## 3) Permission change propagation time

**Question:** Prompt states permission changes must take effect within 30 seconds; should cache be pure TTL, event invalidation, or both?

**My Understanding/Hypothesis:** TTL alone can leave stale grants/revocations; safest is capped TTL + version checks + explicit invalidation on mutation.

**Solution:** Implemented in-process permission cache with TTL capped at 30 seconds, version-based invalidation, and explicit cache invalidation after role/permission updates.

---

## 4) Sensitive data encryption boundaries

**Question:** Prompt says client identifiers and notes must be encrypted at rest, but does not define exact fields.

**My Understanding/Hypothesis:** At minimum: client identifiers in plans and sensitive notes in plans/delivery should be encrypted field-level, while keeping searchable metadata plaintext.

**Solution:** Implemented AES-256-GCM field encryption for sensitive columns (e.g., client identifiers and notes), plus masking utilities for safe display and exports.

---

## 5) Export masking default behavior

**Question:** Should exports be unmasked when a privileged user requests data, or masked by default unless explicitly elevated?

**My Understanding/Hypothesis:** Compliance-first default should always be masked; unmasked is an explicit, permission-gated exception.

**Solution:** Export APIs default to masked output. Unmasked export requires explicit request intent and `api.export.unmasked` permission; every export is audit-logged.

---

## 6) Hourly unit validation precision

**Question:** Prompt requires labor hours in 0.25-hour increments but doesn’t clarify floating-point tolerance near boundaries (e.g., 1.249999).

**My Understanding/Hypothesis:** Validation should accept quarter-hour multiples with a small numeric tolerance to avoid false rejections.

**Solution:** Implemented quarter-hour validation logic in domain validators and enforced it during package and delivery workflows.

---

## 7) Mileage cap boundary condition

**Question:** Prompt says mileage reimbursement “caps at 200 miles per visit”; is 200 valid and only values greater than 200 invalid?

**My Understanding/Hypothesis:** Cap means inclusive upper bound; reject only values above 200.

**Solution:** Implemented delivery mileage validation with 200 as allowed maximum and >200 as invalid.

---

## 8) Tiered billing rule completeness

**Question:** Prompt requires tiered rules but does not specify whether tiers must include a final open-ended bracket.

**My Understanding/Hypothesis:** To avoid unbillable overflow units, the final tier should be unbounded.

**Solution:** Implemented tier configuration validation requiring non-empty ordered tiers and a last unbounded tier (`up_to: null`).

---

## 9) Charge generation for unmatched rules

**Question:** If a verified delivery entry has no matching package billing rule, should generation fail the whole batch or skip that entry?

**My Understanding/Hypothesis:** Batch should be resilient; skip unmatched entries, report skipped count, and continue processing valid entries.

**Solution:** Billing generation performs idempotent processing and skips entries without matching rules instead of failing the full request.

---

## 10) Adjustment model: overwrite vs additive

**Question:** Prompt says Billing Specialist can post adjustments; should adjustments mutate original charge amounts directly?

**My Understanding/Hypothesis:** Financial traceability requires additive immutable adjustment records, with recomputed net amount.

**Solution:** Implemented immutable charge-adjustment records; parent charge net totals are recomputed, and adjustments are blocked once charge is invoiced/voided.

---

## 11) Recorded payment idempotency semantics

**Question:** Prompt says duplicate submissions within 5 minutes are rejected; should idempotency key be globally unique or scoped?

**My Understanding/Hypothesis:** Key should be scoped at least by organization to prevent cross-tenant collisions.

**Solution:** Implemented idempotency keyed by `(org_id, idempotency_key)` with 5-minute duplicate rejection window and conflict response for active duplicates.

---

## 12) Partial refund cap definition

**Question:** Prompt allows partial refunds up to net paid amount; does cap compare against invoice total or payment-minus-refund net?

**My Understanding/Hypothesis:** Correct cap is dynamic net paid: `sum(payments) - sum(prior_refunds)`.

**Solution:** Implemented refund validation against net paid balance and rejects over-cap refund attempts with explicit validation errors.

---

## 13) Refund reason-code governance

**Question:** Prompt requires mandatory refund reason codes but does not define source of truth.

**My Understanding/Hypothesis:** Reason codes should be controlled lookup values (seeded and queryable), not arbitrary free text.

**Solution:** Implemented seeded refund reason-code catalog and required reason-code validation on refund recording.

---

## 14) Second review trigger threshold interpretation

**Question:** Prompt requires second review for score change “over 10 points.” Does exactly 10 trigger review?

**My Understanding/Hypothesis:** “Over 10” means strict inequality, so delta = 10 should not trigger second review.

**Solution:** Implemented second-review trigger as `abs(delta) > 10`, with tests for threshold boundaries.

---

## 15) No independent QA reviewer available

**Question:** Prompt requires second review but does not define behavior when no eligible independent reviewer exists.

**My Understanding/Hypothesis:** Evaluation should remain blocked from finalization and surface operational signal until assignment is possible.

**Solution:** Implemented `second_review_required` hold state with pending-review workflow; evaluator self-review is blocked; missing reviewer condition is logged/audited for operator follow-up.

---

## 16) Service-route reporting dimension normalization

**Question:** Prompt references route dimensions (“client-to-clinic” or “provider region”) but does not define an enum.

**My Understanding/Hypothesis:** Use validated free-text route labels to support local naming while enforcing basic input constraints.

**Solution:** Implemented route-aware filters in reports/exports using `service_route` field with input validation and query-level filtering.

---

## 17) Degradation toggle semantics under failure

**Question:** Prompt requires one-click degradation toggles but does not specify behavior for malformed/missing config.

**My Understanding/Hypothesis:** Safety-first for malformed values (disable feature), and deterministic defaults should be seeded to avoid ambiguous runtime behavior.

**Solution:** Implemented persisted feature flags (`exports_enabled`, `analytics_enabled`) with API toggle controls, validation, and operational event/audit tracking; disabled state produces 503 for gated endpoints.

---

## 18) Chaos drill guardrails and offline constraints

**Question:** Prompt requires controlled chaos window without internet dependencies; what hard guardrails prevent accidental always-on fault injection?

**My Understanding/Hypothesis:** Fault injection must require explicit arming + strict schedule checks + bounded fault intensity.

**Solution:** Implemented chaos controls with env-based arming, scheduled Sunday window, bounded latency/timeout injection, and drill lifecycle observability via health/ops endpoints.

---

## 19) Audit trail mutability rules

**Question:** Prompt emphasizes immutable financial/audit lineage but does not explicitly state if mutation APIs are allowed for audit/event records.

**My Understanding/Hypothesis:** Audit and operational event stores should be append-only; corrections should be new entries.

**Solution:** Implemented append-only audit/event patterns with read/query APIs and without update/delete workflows for historical records.

---

## 20) Delivery entry edit boundary after billing

**Question:** Prompt requires linkage from delivery to billing, but does not define whether billed delivery entries can be edited later.

**My Understanding/Hypothesis:** Once financially consumed, delivery records should be immutable to preserve invoice integrity.

**Solution:** Implemented restrictions that prevent mutation of billed entries; corrections must flow through billing adjustments/refunds instead of retroactive delivery edits.

---

## Notes

- This file intentionally focuses on **business-process, business-rule, data-relationship, and boundary-condition** questions (not low-level technical refactors).
- Solutions above align with the currently implemented behavior documented in `repo/README.md`, `repo/docs/*.md`, and the requirements traceability matrix.
