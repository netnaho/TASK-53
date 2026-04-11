# CareOps Delivery Acceptance & Project Architecture Static Audit (Round 4)

## 1. Verdict

- **Overall conclusion: Partial Pass**

The codebase is substantially aligned with the prompt and has resolved several previously critical defects (idempotent payment uniqueness, second-review ownership checks, CORS tightening, fail-closed degradation parsing, and broad scope checks in major APIs).  
However, there are still **material gaps** in prompt-fit and security completeness, especially around:

- scoring UI workflow completeness for manual/partial grading UX,
- inconsistent data-scope enforcement depth in `payments_refunds` APIs,
- incomplete project-level scope propagation in export paths,
- and test-suite reliability/coverage mismatches for critical routes and object/scope isolation.

---

## 2. Scope and Static Verification Boundary

### What was reviewed

- Docs and run/test guidance: `repo/README.md`, `repo/API_tests/README.md`, `repo/unit_tests/README.md`, `repo/run_tests.sh`
- Backend routing and bootstrapping: `repo/backend/src/bootstrap/mod.rs`, `repo/backend/src/api/mod.rs`
- Security/authn/authz/scope internals: `repo/backend/src/api/guards/mod.rs`, `repo/backend/src/application/auth_service.rs`, `repo/backend/src/infrastructure/permission_cache/mod.rs`
- Core business services: payments/refunds, scoring/review, reports/export, degradation/ops
- Frontend scoring/reports/delivery pages
- Static test scripts under `repo/API_tests/` and `repo/unit_tests/backend/`

### What was not reviewed

- Runtime behavior, live DB state, browser rendering in execution, concurrency under load

### What was intentionally not executed

- No app start, no Docker, no test execution, no external services

### Claims requiring manual verification

- End-to-end UX correctness of scoring/reporting interactions
- Data-scope behavior under real multi-department/project fixture sets
- Race/concurrency behavior for idempotency under parallel submissions

---

## 3. Repository / Requirement Mapping Summary

### Prompt core implementation targets

- Offline/local-network care ops portal with role-adaptive UI + Rocket APIs + MySQL
- Full lifecycle: catalog/packages/plans/delivery/billing/payments/refunds/scoring/review/reports/exports
- Fine-grained RBAC + org/department/project scope with 30s cache invalidation window
- Sensitive-data encryption at rest and masked-by-default export
- Idempotent closed-loop recorded payments/refunds + immutable fund transaction links
- Resilience controls: health/metrics/alerts/degradation toggles/chaos drills

### Mapped implementation areas

- Backend route/service composition: `repo/backend/src/bootstrap/mod.rs:31-153`
- Security guard model: `repo/backend/src/api/guards/mod.rs:17-99`
- Permission cache/version checks: `repo/backend/src/infrastructure/permission_cache/mod.rs:11-13`, `:47`, `:111`
- Financial/scoring/report/export logic: `payment_service.rs`, `scoring_service.rs`, `report_service.rs`, `export_service.rs`
- Role/task UI flows: `repo/frontend/src/pages/scoring/mod.rs`, `repo/frontend/src/pages/reports/mod.rs`, `repo/frontend/src/pages/delivery/mod.rs`

---

## 4. Section-by-section Review

### 4.1 Hard Gates

#### 4.1.1 Documentation and static verifiability

- **Conclusion: Pass**
- **Rationale:** Startup/auth/security/resilience instructions are present and map to concrete modules and env config.
- **Evidence:** `repo/README.md:17-24`, `repo/README.md:51-68`, `repo/README.md:97-121`, `repo/backend/src/config/mod.rs:1-18`, `repo/backend/src/bootstrap/mod.rs:109-117`
- **Manual verification note:** Runtime command success still requires execution.

#### 4.1.2 Material deviation from Prompt

- **Conclusion: Partial Pass**
- **Rationale:** Most core backend requirements are implemented, but scoring UI still appears incomplete for true question-level manual/partial grading interaction promised by prompt semantics.
- **Evidence:** Scoring submit form currently maps existing answer payloads without rendering editable per-question controls (`repo/frontend/src/pages/scoring/mod.rs:335`, `:342`, `:362-363`, `:414`), while prompt requires rich objective/manual/partial workflow feedback.
- **Manual verification note:** Actual runtime UI behavior needs browser verification.

### 4.2 Delivery Completeness

#### 4.2.1 Coverage of explicit core requirements

- **Conclusion: Partial Pass**
- **Rationale:** Most backend requirements are covered (RBAC, idempotent payments, second-review >10, masking defaults, fail-closed degradation, CORS allowlist), but scope-depth and scoring UX requirements are not fully evidenced.
- **Evidence (implemented):**
  - Atomic payment idempotency uniqueness migration: `repo/backend/migrations/20240107000000_payment_idempotency_unique.sql:1-18`
  - Duplicate-key conflict handling: `repo/backend/src/application/payment_service.rs:83-84`, `:111-115`
  - Second-review trigger >10: `repo/backend/src/domain/scoring_types.rs:455`, `:457`
  - Assigned reviewer enforcement: `repo/backend/src/application/scoring_service.rs:463-464`
  - Export masked by default and unmask permission gate: `repo/backend/src/application/export_service.rs:55-63`
  - Quarter-hour/mileage validations: `repo/backend/src/domain/catalog_types.rs:246-269`
- **Evidence (gaps):**
  - No `require_data_scope(...)` in payments/refunds routes: permission-only checks at `repo/backend/src/api/payments_refunds/mod.rs:26`, `:50`, `:64`, `:104`, `:160`, `:188`.

#### 4.2.2 End-to-end 0→1 deliverable vs partial/demo

- **Conclusion: Partial Pass**
- **Rationale:** The repo is full-stack and substantial, but certain operator-critical UI workflows still rely on IDs/manual payload assumptions and incomplete interactive grading controls.
- **Evidence:**
  - Delivery form still requires manual `plan_package_id` and `service_item_id`: `repo/frontend/src/pages/delivery/mod.rs:234-252`
  - Scoring submit form does not expose question-level editing controls despite messaging implying it: `repo/frontend/src/pages/scoring/mod.rs:335-444`

### 4.3 Engineering and Architecture Quality

#### 4.3.1 Structure and module decomposition

- **Conclusion: Pass**
- **Rationale:** Clear separation of API/application/domain/infrastructure; route mounts and services are modularized.
- **Evidence:** `repo/backend/src/api/mod.rs:1-16`, `repo/backend/src/bootstrap/mod.rs:31-153`

#### 4.3.2 Maintainability and extensibility

- **Conclusion: Partial Pass**
- **Rationale:** Architecture is extensible, but test/API drift and uneven scope enforcement lower maintainability confidence.
- **Evidence:**
  - Strong cache/version design: `repo/backend/src/infrastructure/permission_cache/mod.rs:11-13`, `:47`, `:111`
  - Test drift example: `repo/API_tests/test_ops.sh:294`, `:299`, `:310` uses `/api/ops/toggles` while API defines `/ops/flags...` (`repo/backend/src/api/ops/mod.rs:7-9`, `:24`, `:39`, `:59`).

### 4.4 Engineering Details and Professionalism

#### 4.4.1 Error handling, logging, validation, API design

- **Conclusion: Partial Pass**
- **Rationale:** Most modules show strong typed errors, validation, and audit logs; one notable security hygiene concern remains around login-failure details.
- **Evidence:**
  - Guarded auth and typed errors: `repo/backend/src/api/guards/mod.rs:17-99`
  - Input validation: `repo/backend/src/application/payment_service.rs:39-56`, `repo/backend/src/domain/catalog_types.rs:246-269`
  - Security-hygiene concern: user-not-found audit stores raw username: `repo/backend/src/application/auth_service.rs:79`

#### 4.4.2 Product-like deliverable vs demo

- **Conclusion: Partial Pass**
- **Rationale:** Broadly product-shaped backend and UI shells exist, but scoring interaction depth still reads partially implemented for the required grading experience.
- **Evidence:** `repo/frontend/src/pages/scoring/mod.rs:335-444`

### 4.5 Prompt Understanding and Requirement Fit

#### 4.5.1 Business goal and constraints fit

- **Conclusion: Partial Pass**
- **Rationale:** Strong overall fit (offline stack, RBAC, encryption, observability), but prompt-specific route/region reporting semantics and full project-level scope in export are only partially modeled.
- **Evidence:**
  - Route/region semantics mapped only via `department_id` comment: `repo/backend/src/domain/scoring_types.rs:171`
  - Export request omits `project_id`: `repo/backend/src/domain/scoring_types.rs:180-184`
  - Export scope check passes department but hardcodes project `None`: `repo/backend/src/api/reports_exports/mod.rs:130`

### 4.6 Aesthetics (frontend)

#### 4.6.1 Visual and interaction quality

- **Conclusion: Partial Pass (Cannot Confirm Fully Statistically)**
- **Rationale:** UI hierarchy/tabs/cards/feedback states are present; full rendering quality and interaction polish require runtime review.
- **Evidence:**
  - Scoring/report pages include tabbed structure and load/error states: `repo/frontend/src/pages/scoring/mod.rs:42-74`, `repo/frontend/src/pages/reports/mod.rs:49-87`
  - Delivery page has inline validation messaging: `repo/frontend/src/pages/delivery/mod.rs:142-154`, `:190-198`

---

## 5. Issues / Suggestions (Severity-Rated)

### High

1. **Severity:** High  
   **Title:** Inconsistent route-level data-scope enforcement in payments/refunds APIs  
   **Conclusion:** Partial Fail  
   **Evidence:** `repo/backend/src/api/payments_refunds/mod.rs:26`, `:50`, `:64`, `:104`, `:118`, `:160`, `:188` show permission checks only; no `require_data_scope(...)` matches in this route module.  
   **Impact:** Access control depends mainly on org ownership in service queries; department/project scope policy can be bypassed for payment/refund/reconciliation reads/writes where prompt expects org+dept+project scope consistency.  
   **Minimum actionable fix:** Add `require_data_scope(...)` to payments/refunds/reconciliation endpoints and propagate optional department/project filters where applicable.

2. **Severity:** High  
   **Title:** Scoring UI does not statically demonstrate full question-level manual/partial grading workflow  
   **Conclusion:** Partial Fail  
   **Evidence:** Submit flow serializes existing `eval_detail.answers` (`repo/frontend/src/pages/scoring/mod.rs:362-363`) with no visible per-question answer/score editors in `SubmitEvaluationForm` (`:335-444`), despite UI text indicating manual adjustments (`:414`).  
   **Impact:** Prompt-critical UX requirement (objective + subjective + partial credit grading with progress) may be only partially delivered in frontend behavior.  
   **Minimum actionable fix:** Render question list with editable answer/manual/partial fields bound to local form state and display progress metrics before submit.

### Medium

3. **Severity:** Medium  
   **Title:** Export scope request model does not include project-level filter, reducing policy granularity  
   **Conclusion:** Partial Fail  
   **Evidence:** `ExportRequest` includes `department_id` but no `project_id` (`repo/backend/src/domain/scoring_types.rs:180-184`); export scope guard passes `None` for project (`repo/backend/src/api/reports_exports/mod.rs:130`).  
   **Impact:** Project-level scope isolation in exports is not fully expressible at API contract level.  
   **Minimum actionable fix:** Extend export request with optional `project_id` and enforce both department/project scope checks.

4. **Severity:** Medium  
   **Title:** Ops API tests contain endpoint drift (`/api/ops/toggles` vs `/api/ops/flags`)  
   **Conclusion:** Fail  
   **Evidence:** Test uses `/api/ops/toggles` at `repo/API_tests/test_ops.sh:294`, `:299`, `:310`, while server exposes `/ops/flags` routes (`repo/backend/src/api/ops/mod.rs:7-9`, `:24`, `:39`, `:59`).  
   **Impact:** Test reliability degrades; false negatives can mask actual regressions or create noise in acceptance gating.  
   **Minimum actionable fix:** Update tests to use `/api/ops/flags` endpoints consistently.

5. **Severity:** Medium  
   **Title:** Login-failure audit details include raw username (enumeration-sensitive telemetry)  
   **Conclusion:** Suspected Risk  
   **Evidence:** `repo/backend/src/application/auth_service.rs:79` logs `{"reason":"user_not_found","username":...}`.  
   **Impact:** Internal logs may retain identifiable failed-attempt metadata that can amplify account enumeration intelligence if log access is broad.  
   **Minimum actionable fix:** Hash or redact username in failure logs, keep only normalized signal counters.

6. **Severity:** Medium  
   **Title:** Smoke test expects auth on undefined root paths (possible route expectation mismatch)  
   **Conclusion:** Partial Fail  
   **Evidence:** `repo/API_tests/test_smoke.sh:45-46` expects `401` on `/api/scoring/` and `/api/reports/`; route modules define specific subpaths, not guaranteed root handlers (`repo/backend/src/api/scoring_reviews/mod.rs`, `repo/backend/src/api/reports_exports/mod.rs`).  
   **Impact:** Test intent (“all registered routes”) may not match actual route registration behavior, reducing signal quality.  
   **Minimum actionable fix:** Verify actual mounted route list and update smoke endpoints to concrete handlers.

---

## 6. Security Review Summary

### Authentication entry points

- **Conclusion: Pass**
- **Evidence:** JWT bearer guard and session revocation checks: `repo/backend/src/api/guards/mod.rs:17-58`, `repo/backend/src/application/auth_service.rs:232-261`; login/logout/me endpoints: `repo/backend/src/api/auth/mod.rs:10-36`.

### Route-level authorization

- **Conclusion: Partial Pass**
- **Evidence:** Strong permission checks across modules (e.g., billing/reports/scoring/ops) in route handlers; but payments/refunds routes are permission-only without explicit scope checks (`repo/backend/src/api/payments_refunds/mod.rs:26`, `:50`, `:64`, `:104`).

### Object-level authorization

- **Conclusion: Partial Pass**
- **Evidence:** Second-review assigned-reviewer ownership enforced (`repo/backend/src/application/scoring_service.rs:463-464`); however broader object-level tests for cross-scope financial data are limited in static test suite.

### Function-level authorization

- **Conclusion: Pass**
- **Evidence:** Action/API permission checks are explicit and specific (`repo/backend/src/api/billing/mod.rs:25`, `repo/backend/src/api/payments_refunds/mod.rs:50`, `repo/backend/src/api/ops/mod.rs:46`, `:66`).

### Tenant / user data isolation

- **Conclusion: Partial Pass**
- **Evidence:** Org scoping is pervasive in services and route guard input; dept/project checks present in several domains (`repo/backend/src/api/reports_exports/mod.rs:38`, `:61`, `:84`, `:105`) but inconsistent in `payments_refunds` route layer.

### Admin / internal / debug protection

- **Conclusion: Pass**
- **Evidence:** Public-only health live/ready (`repo/backend/src/api/observability/mod.rs:31-52`); metrics/alerts/chaos protected by `api.ops.read` (`:70-72`, `:98`, `:112`); ops mutation protected by `api.ops.write` (`repo/backend/src/api/ops/mod.rs:46`, `:66`).

---

## 7. Tests and Logging Review

### Unit tests

- **Conclusion: Basically covered**
- **Rationale:** Unit tests exist for auth hashing, scoring review validation logic, rounding helpers, permission cache helper logic.
- **Evidence:** `repo/backend/src/application/auth_service.rs` tests section, `repo/backend/src/application/scoring_service.rs` tests section, `repo/backend/src/domain/scoring_types.rs:351-357`, `repo/unit_tests/backend/test_password_hashing.sh:1-22`.

### API / integration tests

- **Conclusion: Partial Pass**
- **Rationale:** Good breadth across auth/catalog/billing/scoring/reports/ops, improved nonzero exits, but route drift and missing deep scope/object tests remain.
- **Evidence:** suite files under `repo/API_tests/`; fail-on-fail endings (`test_billing.sh:405`, `test_reports.sh:273`, `test_scoring.sh:300`, `test_ops.sh:327`, `test_catalog_delivery.sh:244`); route drift in `test_ops.sh:294`/`:299`/`:310`.

### Logging categories / observability

- **Conclusion: Pass**
- **Rationale:** Structured health/metrics/alert/chaos endpoints and alert thresholds are implemented with operational guardrails.
- **Evidence:** `repo/backend/src/api/observability/mod.rs:1-121`, `repo/backend/src/application/alert_engine.rs` (mounted in `bootstrap`).

### Sensitive-data leakage risk in logs / responses

- **Conclusion: Partial Pass (Suspected Risk)**
- **Rationale:** Export masking defaults are strong, but login-failure logs include raw usernames.
- **Evidence:** masking defaults `repo/backend/src/application/export_service.rs:55-63`; raw username in audit details `repo/backend/src/application/auth_service.rs:79`.

---

## 8. Test Coverage Assessment (Static Audit)

### 8.1 Test Overview

- **Unit tests exist:** Yes (`cargo test --lib` + focused shell wrappers)  
  Evidence: `repo/run_tests.sh:17-26`, `repo/unit_tests/backend/test_password_hashing.sh:13-19`
- **API/integration tests exist:** Yes (`repo/API_tests/*.sh`)  
  Evidence: `repo/run_tests.sh:51-167`
- **Framework/tools:** shell + `curl` + `python3` JSON parsing; Rust unit tests via Cargo.
- **Test entry points:** `repo/run_tests.sh`, individual scripts under `repo/API_tests` and `repo/unit_tests/backend`.
- **Docs provide commands:** Yes  
  Evidence: `repo/API_tests/README.md:9-21`, `repo/unit_tests/README.md:9-14`, `repo/README.md` test/quickstart sections.

### 8.2 Coverage Mapping Table

| Requirement / Risk Point                     | Mapped Test Case(s)                                                                                       | Key Assertion / Fixture / Mock                                           | Coverage Assessment | Gap                                                                         | Minimum Test Addition                                                     |
| -------------------------------------------- | --------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ | ------------------- | --------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| Auth happy path + 401/logout revocation      | `repo/API_tests/test_auth.sh:30-62`, `:154-163`                                                           | Valid login token, invalid password 401, revoked token 401               | basically covered   | No token-expiry boundary test                                               | Add expired-token and malformed JWT matrix                                |
| Role-based 403 route authz                   | `test_auth.sh:95-142`, `test_billing.sh:338-383`, `test_reports.sh:92-101,217-225`, `test_ops.sh:111-200` | Coach/auditor/ops-manager denied writes                                  | sufficient          | None major for route-level permission gates                                 | Keep role matrix synced with seeded roles                                 |
| Billing idempotency and refund constraints   | `test_billing.sh:276-329`, `:386-404`                                                                     | Duplicate key 409, refund cap 400, reason code checks                    | basically covered   | No explicit parallel/concurrency duplicate submit check                     | Add concurrent same-key submit test harness                               |
| Scoring second review and invalid actions    | `test_scoring.sh:248-287`                                                                                 | Pending reviews endpoint + invalid action/revise checks                  | insufficient        | No direct API test for non-assigned reviewer blocked on real pending review | Add cross-user review process test with assigned vs non-assigned reviewer |
| Data-scope isolation (org/dept/project)      | Not clearly present in API tests                                                                          | N/A                                                                      | missing             | No cross-dept/project forbidden assertions for billing/reports/payments     | Add fixtures for two scopes and assert 403/empty data boundaries          |
| Reports/export masking and permission gating | `test_reports.sh:154-262`                                                                                 | Masked default true, unmasked permission behavior, export validation 400 | basically covered   | Lacks project-scope filter coverage                                         | Add export/report tests with dept+project scoped fixtures                 |
| Ops toggles and observability protections    | `test_ops.sh:72-280`                                                                                      | 401/403 checks, 503 when disabled                                        | insufficient        | Contains endpoint drift `/ops/toggles` not implemented                      | Replace with `/ops/flags` only and assert response schema                 |
| Smoke route sanity                           | `test_smoke.sh:28-47`                                                                                     | Public health + protected route status checks                            | insufficient        | Uses root paths likely not mounted (`/api/scoring/`, `/api/reports/`)       | Use explicit concrete route probes                                        |

### 8.3 Security Coverage Audit

- **Authentication:** basically covered (login failures, no-token, revoked session).
- **Route authorization:** basically covered (many 403 checks), but some scripts use drifted endpoints reducing trust in those sections.
- **Object-level authorization:** insufficient at API-test level (service-level unit tests exist for reviewer ownership, but limited end-to-end API coverage).
- **Tenant/data isolation:** missing/insufficient (no robust cross-scope fixture-driven tests for dept/project boundaries).
- **Admin/internal protection:** basically covered (ops/health protections tested), with caveat on endpoint drift in one ops sub-block.

### 8.4 Final Coverage Judgment

- **Final Coverage Judgment: Partial Pass**

Major permission and billing flows are covered, but severe defects could still pass undetected due to:

1. weak cross-scope/object-level isolation testing,
2. endpoint drift in ops tests,
3. and limited coverage of prompt-specific route/region semantics in reports.

---

## 9. Final Notes

- This is a **static-only** audit; runtime correctness is not asserted.
- The project has matured significantly versus earlier states and now resembles a production-leaning implementation.
- Remaining acceptance risk is concentrated in **scope-depth consistency**, **scoring UI workflow completeness**, and **test trustworthiness for critical isolation controls**.
