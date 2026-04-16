# Test Coverage Audit

## Scope, method, and constraints

- Audit mode: **static inspection only** (no command execution, no tests run, no containers started).
- Inspected scope:
  - Backend route declarations: `repo/backend/src/bootstrap/mod.rs`, `repo/backend/src/api/**/mod.rs`
  - API tests: `repo/API_tests/*.sh`, `repo/unit_tests/backend/test_health.sh`
  - Unit tests: `repo/backend/src/**/tests.rs`, inline `#[cfg(test)]` modules, `repo/frontend/src/**/*test*.rs`
  - Test runner design: `repo/run_tests.sh`
- Project type declaration: `repo/README.md:1` => `Project Type: fullstack`.

---

## Backend Endpoint Inventory

Resolved from Rocket mounts in `repo/backend/src/bootstrap/mod.rs` + route annotations in each module.

Total resolved endpoints: **91**

1. GET `/api/health/live`
2. GET `/api/health/ready`
3. GET `/api/health/metrics`
4. GET `/api/health/alerts`
5. GET `/api/health/chaos`
6. POST `/api/auth/login`
7. POST `/api/auth/logout`
8. GET `/api/auth/me`
9. GET `/api/admin/org/`
10. GET `/api/admin/org/:id`
11. POST `/api/admin/org/`
12. PUT `/api/admin/org/:id`
13. GET `/api/admin/org/:org_id/departments`
14. POST `/api/admin/org/:org_id/departments`
15. GET `/api/admin/org/:org_id/projects`
16. POST `/api/admin/org/:org_id/projects`
17. GET `/api/users/`
18. GET `/api/users/:id`
19. POST `/api/users/`
20. PUT `/api/users/:id`
21. GET `/api/users/:id/roles`
22. POST `/api/users/:id/roles`
23. DELETE `/api/users/:id/roles/:role_id`
24. GET `/api/users/:id/scopes`
25. POST `/api/users/:id/scopes`
26. DELETE `/api/users/:target_user_id/scopes/:scope_id`
27. GET `/api/roles/`
28. GET `/api/roles/:id`
29. POST `/api/roles/`
30. GET `/api/roles/:id/permissions`
31. POST `/api/roles/:id/permissions`
32. DELETE `/api/roles/:id/permissions/:perm_id`
33. GET `/api/roles/all`
34. GET `/api/catalog/`
35. GET `/api/catalog/:id`
36. POST `/api/catalog/`
37. PUT `/api/catalog/:id`
38. GET `/api/packages/`
39. GET `/api/packages/:id`
40. POST `/api/packages/`
41. PUT `/api/packages/:id`
42. GET `/api/packages/:id/rules`
43. GET `/api/plans/`
44. GET `/api/plans/:id`
45. POST `/api/plans/`
46. PUT `/api/plans/:id`
47. POST `/api/plans/:id/packages`
48. GET `/api/plans/:id/packages`
49. GET `/api/delivery/`
50. GET `/api/delivery/:id`
51. POST `/api/delivery/`
52. PUT `/api/delivery/:id`
53. GET `/api/delivery/:id/notes`
54. POST `/api/delivery/:id/notes`
55. POST `/api/billing/charges/generate`
56. GET `/api/billing/charges`
57. GET `/api/billing/charges/:charge_id`
58. POST `/api/billing/charges/:charge_id/adjustments`
59. POST `/api/billing/invoices/generate`
60. GET `/api/billing/invoices`
61. GET `/api/billing/invoices/:invoice_id`
62. PUT `/api/billing/invoices/:invoice_id/status`
63. GET `/api/payments/reason-codes`
64. POST `/api/payments/`
65. GET `/api/payments/`
66. GET `/api/payments/:payment_id`
67. POST `/api/payments/refunds`
68. GET `/api/payments/refunds`
69. GET `/api/payments/refunds/:refund_id`
70. GET `/api/payments/transactions`
71. POST `/api/payments/reconciliation`
72. GET `/api/payments/reconciliation`
73. GET `/api/payments/reconciliation/:run_id`
74. POST `/api/scoring/templates`
75. GET `/api/scoring/templates`
76. GET `/api/scoring/templates/:template_id`
77. POST `/api/scoring/evaluations`
78. GET `/api/scoring/evaluations`
79. GET `/api/scoring/evaluations/:eval_id`
80. POST `/api/scoring/evaluations/:eval_id/submit`
81. GET `/api/scoring/reviews/pending`
82. POST `/api/scoring/reviews/:eval_id`
83. GET `/api/reports/order-volume`
84. GET `/api/reports/revenue`
85. GET `/api/reports/utilization`
86. GET `/api/reports/kpi`
87. POST `/api/reports/export`
88. GET `/api/ops/flags`
89. POST `/api/ops/flags/:key/enable`
90. POST `/api/ops/flags/:key/disable`
91. GET `/api/audit/`

---

## API Test Mapping Table

Legend:

- Test type values: `true no-mock HTTP`, `HTTP with mocking`, `unit-only/indirect`
- Route coverage criterion: exact method + resolved path observed in test script request lines.

| Endpoint                                             | Covered | Test type         | Test files                                                                                                           | Evidence                                                                         |
| ---------------------------------------------------- | ------- | ----------------- | -------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| GET `/api/health/live`                               | yes     | true no-mock HTTP | `API_tests/test_smoke.sh`, `API_tests/test_ops.sh`, `unit_tests/backend/test_health.sh`                              | `check_body_contains GET "/api/health/live"`; `curl "$BASE_URL/api/health/live"` |
| GET `/api/health/ready`                              | yes     | true no-mock HTTP | `API_tests/test_smoke.sh`, `API_tests/test_ops.sh`                                                                   | `check_body_contains GET "/api/health/ready"`; ops `[2]`                         |
| GET `/api/health/metrics`                            | yes     | true no-mock HTTP | `API_tests/test_ops.sh`                                                                                              | ops `[3] GET /health/metrics`                                                    |
| GET `/api/health/alerts`                             | yes     | true no-mock HTTP | `API_tests/test_ops.sh`                                                                                              | ops `[4] GET /health/alerts`                                                     |
| GET `/api/health/chaos`                              | yes     | true no-mock HTTP | `API_tests/test_ops.sh`                                                                                              | ops `[5] GET /health/chaos`                                                      |
| POST `/api/auth/login`                               | yes     | true no-mock HTTP | `API_tests/test_smoke.sh`, `API_tests/test_auth.sh`, `API_tests/test_e2e.sh`, others setup                           | smoke bad-login block; auth `[1-3]`; e2e `[E2E-1]`                               |
| POST `/api/auth/logout`                              | yes     | true no-mock HTTP | `API_tests/test_auth.sh`                                                                                             | auth `[10] Logout invalidates token`                                             |
| GET `/api/auth/me`                                   | yes     | true no-mock HTTP | `API_tests/test_smoke.sh`, `API_tests/test_auth.sh`, `API_tests/test_e2e.sh`, `API_tests/test_gaps.sh`               | smoke unauth check; auth `[6]`; e2e `[E2E-2]`                                    |
| GET `/api/admin/org/`                                | yes     | true no-mock HTTP | `API_tests/test_smoke.sh`, `API_tests/test_auth.sh`, `API_tests/test_billing.sh`, `API_tests/test_reports.sh`        | smoke protected 401 list; auth `[7],[9]`; setup org discovery                    |
| GET `/api/admin/org/:id`                             | yes     | true no-mock HTTP | `API_tests/test_gaps.sh`                                                                                             | gaps `[7] GET /api/admin/org/:id`                                                |
| POST `/api/admin/org/`                               | yes     | true no-mock HTTP | `API_tests/test_auth.sh`                                                                                             | auth cross-org fixture create org-B                                              |
| PUT `/api/admin/org/:id`                             | yes     | true no-mock HTTP | `API_tests/test_gaps.sh`                                                                                             | gaps `[8] PUT /api/admin/org/:id`                                                |
| GET `/api/admin/org/:org_id/departments`             | yes     | true no-mock HTTP | `API_tests/test_reports.sh`                                                                                          | reports cross-project setup department fetch                                     |
| POST `/api/admin/org/:org_id/departments`            | yes     | true no-mock HTTP | `API_tests/test_gaps.sh`                                                                                             | gaps `[9] POST .../departments`                                                  |
| GET `/api/admin/org/:org_id/projects`                | yes     | true no-mock HTTP | `API_tests/test_gaps.sh`                                                                                             | gaps `[11] GET .../projects`                                                     |
| POST `/api/admin/org/:org_id/projects`               | yes     | true no-mock HTTP | `API_tests/test_gaps.sh`                                                                                             | gaps `[10] POST .../projects`                                                    |
| GET `/api/users/`                                    | yes     | true no-mock HTTP | `API_tests/test_smoke.sh`, `API_tests/test_auth.sh`                                                                  | smoke protected 401; auth `[4],[5]`                                              |
| GET `/api/users/:id`                                 | yes     | true no-mock HTTP | `API_tests/test_auth.sh`                                                                                             | auth `[22],[23]` cross-org/same-org checks                                       |
| POST `/api/users/`                                   | yes     | true no-mock HTTP | `API_tests/test_auth.sh`, `API_tests/test_billing.sh`, `API_tests/test_reports.sh`, `API_tests/test_gaps.sh`         | auth `[7]`; setup user creation                                                  |
| PUT `/api/users/:id`                                 | yes     | true no-mock HTTP | `API_tests/test_gaps.sh`                                                                                             | gaps `[27] PUT /api/users/:id`                                                   |
| GET `/api/users/:id/roles`                           | yes     | true no-mock HTTP | `API_tests/test_auth.sh`                                                                                             | auth `[12],[13],[24]`                                                            |
| POST `/api/users/:id/roles`                          | yes     | true no-mock HTTP | `API_tests/test_auth.sh`, `API_tests/test_billing.sh`, `API_tests/test_reports.sh`                                   | auth `[15],[19],[21]`; setup role assign                                         |
| DELETE `/api/users/:id/roles/:role_id`               | yes     | true no-mock HTTP | `API_tests/test_auth.sh`                                                                                             | auth `[16]`                                                                      |
| GET `/api/users/:id/scopes`                          | yes     | true no-mock HTTP | `API_tests/test_auth.sh`                                                                                             | auth `[14],[17],[25]`                                                            |
| POST `/api/users/:id/scopes`                         | yes     | true no-mock HTTP | `API_tests/test_auth.sh`, `API_tests/test_reports.sh`                                                                | auth `[18],[26]`; reports scoped user setup                                      |
| DELETE `/api/users/:target_user_id/scopes/:scope_id` | yes     | true no-mock HTTP | `API_tests/test_auth.sh`                                                                                             | auth `[20]`                                                                      |
| GET `/api/roles/`                                    | yes     | true no-mock HTTP | `API_tests/test_smoke.sh`, `API_tests/test_auth.sh`, `API_tests/test_billing.sh`, `API_tests/test_reports.sh`        | smoke protected; fixtures for role lookup                                        |
| GET `/api/roles/:id`                                 | yes     | true no-mock HTTP | `API_tests/test_gaps.sh`                                                                                             | gaps `[3] GET /api/roles/:id`                                                    |
| POST `/api/roles/`                                   | yes     | true no-mock HTTP | `API_tests/test_gaps.sh`                                                                                             | gaps `[2] POST /api/roles/`                                                      |
| GET `/api/roles/:id/permissions`                     | yes     | true no-mock HTTP | `API_tests/test_gaps.sh`                                                                                             | gaps `[5]`                                                                       |
| POST `/api/roles/:id/permissions`                    | yes     | true no-mock HTTP | `API_tests/test_gaps.sh`                                                                                             | gaps `[4]`                                                                       |
| DELETE `/api/roles/:id/permissions/:perm_id`         | yes     | true no-mock HTTP | `API_tests/test_gaps.sh`                                                                                             | gaps `[6]`                                                                       |
| GET `/api/roles/all`                                 | yes     | true no-mock HTTP | `API_tests/test_gaps.sh`                                                                                             | gaps `[1]`                                                                       |
| GET `/api/catalog/`                                  | yes     | true no-mock HTTP | `API_tests/test_smoke.sh`, `API_tests/test_catalog_delivery.sh`, `API_tests/test_e2e.sh`                             | catalog `[4],[15]`; e2e `[E2E-3]`                                                |
| GET `/api/catalog/:id`                               | yes     | true no-mock HTTP | `API_tests/test_gaps.sh`                                                                                             | gaps `[12]`                                                                      |
| POST `/api/catalog/`                                 | yes     | true no-mock HTTP | `API_tests/test_catalog_delivery.sh`, `API_tests/test_billing.sh`, `API_tests/test_e2e.sh`, `API_tests/test_gaps.sh` | catalog `[1],[6]`; setups                                                        |
| PUT `/api/catalog/:id`                               | yes     | true no-mock HTTP | `API_tests/test_gaps.sh`                                                                                             | gaps `[13]`                                                                      |
| GET `/api/packages/`                                 | yes     | true no-mock HTTP | `API_tests/test_smoke.sh`, `API_tests/test_gaps.sh`                                                                  | smoke protected 401; gaps `[14]`                                                 |
| GET `/api/packages/:id`                              | yes     | true no-mock HTTP | `API_tests/test_gaps.sh`                                                                                             | gaps `[15]`                                                                      |
| POST `/api/packages/`                                | yes     | true no-mock HTTP | `API_tests/test_catalog_delivery.sh`, `API_tests/test_billing.sh`, `API_tests/test_e2e.sh`, `API_tests/test_gaps.sh` | catalog `[7]`; setups                                                            |
| PUT `/api/packages/:id`                              | yes     | true no-mock HTTP | `API_tests/test_gaps.sh`                                                                                             | gaps `[17]`                                                                      |
| GET `/api/packages/:id/rules`                        | yes     | true no-mock HTTP | `API_tests/test_gaps.sh`                                                                                             | gaps `[16]`                                                                      |
| GET `/api/plans/`                                    | yes     | true no-mock HTTP | `API_tests/test_smoke.sh`, `API_tests/test_gaps.sh`                                                                  | smoke protected; gaps `[18]`                                                     |
| GET `/api/plans/:id`                                 | yes     | true no-mock HTTP | `API_tests/test_gaps.sh`                                                                                             | gaps `[19]`                                                                      |
| POST `/api/plans/`                                   | yes     | true no-mock HTTP | `API_tests/test_catalog_delivery.sh`, `API_tests/test_billing.sh`, `API_tests/test_e2e.sh`, `API_tests/test_gaps.sh` | catalog `[9]`; setups                                                            |
| PUT `/api/plans/:id`                                 | yes     | true no-mock HTTP | `API_tests/test_catalog_delivery.sh`, `API_tests/test_billing.sh`                                                    | plan activation/update status                                                    |
| POST `/api/plans/:id/packages`                       | yes     | true no-mock HTTP | `API_tests/test_catalog_delivery.sh`, `API_tests/test_billing.sh`, `API_tests/test_e2e.sh`                           | catalog `[11]`; setups                                                           |
| GET `/api/plans/:id/packages`                        | yes     | true no-mock HTTP | `API_tests/test_gaps.sh`                                                                                             | gaps `[20]`                                                                      |
| GET `/api/delivery/`                                 | yes     | true no-mock HTTP | `API_tests/test_smoke.sh`, `API_tests/test_catalog_delivery.sh`, `API_tests/test_scoring.sh`                         | catalog `[16]`; scoring delivery lookup                                          |
| GET `/api/delivery/:id`                              | yes     | true no-mock HTTP | `API_tests/test_gaps.sh`, `API_tests/test_e2e.sh`                                                                    | gaps `[21]`; e2e `[E2E-5]`                                                       |
| POST `/api/delivery/`                                | yes     | true no-mock HTTP | `API_tests/test_catalog_delivery.sh`, `API_tests/test_billing.sh`, `API_tests/test_e2e.sh`, `API_tests/test_gaps.sh` | catalog `[12]`; billing setup                                                    |
| PUT `/api/delivery/:id`                              | yes     | true no-mock HTTP | `API_tests/test_billing.sh`                                                                                          | billing setup verify entry                                                       |
| GET `/api/delivery/:id/notes`                        | yes     | true no-mock HTTP | `API_tests/test_gaps.sh`                                                                                             | gaps `[23]`                                                                      |
| POST `/api/delivery/:id/notes`                       | yes     | true no-mock HTTP | `API_tests/test_gaps.sh`                                                                                             | gaps `[22]`                                                                      |
| POST `/api/billing/charges/generate`                 | yes     | true no-mock HTTP | `API_tests/test_billing.sh`                                                                                          | billing `[2-4]`                                                                  |
| GET `/api/billing/charges`                           | yes     | true no-mock HTTP | `API_tests/test_billing.sh`                                                                                          | billing `[1],[5]`                                                                |
| GET `/api/billing/charges/:charge_id`                | yes     | true no-mock HTTP | `API_tests/test_billing.sh`                                                                                          | billing `[6]`                                                                    |
| POST `/api/billing/charges/:charge_id/adjustments`   | yes     | true no-mock HTTP | `API_tests/test_billing.sh`                                                                                          | billing `[7],[8],[25]`                                                           |
| POST `/api/billing/invoices/generate`                | yes     | true no-mock HTTP | `API_tests/test_billing.sh`                                                                                          | billing `[9],[10]`                                                               |
| GET `/api/billing/invoices`                          | yes     | true no-mock HTTP | `API_tests/test_smoke.sh`, `API_tests/test_billing.sh`                                                               | smoke protected 401; billing `[11]`                                              |
| GET `/api/billing/invoices/:invoice_id`              | yes     | true no-mock HTTP | `API_tests/test_gaps.sh`                                                                                             | gaps `[24]`                                                                      |
| PUT `/api/billing/invoices/:invoice_id/status`       | yes     | true no-mock HTTP | `API_tests/test_billing.sh`                                                                                          | billing `[12],[13]`                                                              |
| GET `/api/payments/reason-codes`                     | yes     | true no-mock HTTP | `API_tests/test_billing.sh`                                                                                          | billing `[14],[32]`                                                              |
| POST `/api/payments/`                                | yes     | true no-mock HTTP | `API_tests/test_billing.sh`                                                                                          | billing `[15],[16],[17],[21],[26]`                                               |
| GET `/api/payments/`                                 | yes     | true no-mock HTTP | `API_tests/test_smoke.sh`, `API_tests/test_billing.sh`                                                               | smoke protected 401; billing `[27]`                                              |
| GET `/api/payments/:payment_id`                      | yes     | true no-mock HTTP | `API_tests/test_gaps.sh`                                                                                             | included in gaps endpoint set (payments detail check block)                      |
| POST `/api/payments/refunds`                         | yes     | true no-mock HTTP | `API_tests/test_billing.sh`                                                                                          | billing `[18],[19],[20],[30]`                                                    |
| GET `/api/payments/refunds`                          | yes     | true no-mock HTTP | `API_tests/test_billing.sh`                                                                                          | billing `[29]`                                                                   |
| GET `/api/payments/refunds/:refund_id`               | yes     | true no-mock HTTP | `API_tests/test_gaps.sh`                                                                                             | gaps `[26]`                                                                      |
| GET `/api/payments/transactions`                     | yes     | true no-mock HTTP | `API_tests/test_billing.sh`                                                                                          | billing `[22],[31]`                                                              |
| POST `/api/payments/reconciliation`                  | yes     | true no-mock HTTP | `API_tests/test_billing.sh`                                                                                          | billing `[23],[24],[33]`                                                         |
| GET `/api/payments/reconciliation`                   | yes     | true no-mock HTTP | `API_tests/test_billing.sh`                                                                                          | billing `[34]`                                                                   |
| GET `/api/payments/reconciliation/:run_id`           | yes     | true no-mock HTTP | `API_tests/test_gaps.sh`                                                                                             | gaps `[25]`                                                                      |
| POST `/api/scoring/templates`                        | yes     | true no-mock HTTP | `API_tests/test_scoring.sh`                                                                                          | scoring `[3],[4]`                                                                |
| GET `/api/scoring/templates`                         | yes     | true no-mock HTTP | `API_tests/test_smoke.sh`, `API_tests/test_scoring.sh`                                                               | smoke protected; scoring `[2],[5]`                                               |
| GET `/api/scoring/templates/:template_id`            | yes     | true no-mock HTTP | `API_tests/test_scoring.sh`                                                                                          | scoring template detail section                                                  |
| POST `/api/scoring/evaluations`                      | yes     | true no-mock HTTP | `API_tests/test_scoring.sh`                                                                                          | scoring `[6] start evaluation`                                                   |
| GET `/api/scoring/evaluations`                       | yes     | true no-mock HTTP | `API_tests/test_scoring.sh`                                                                                          | scoring `[9] list evaluations`                                                   |
| GET `/api/scoring/evaluations/:eval_id`              | yes     | true no-mock HTTP | `API_tests/test_scoring.sh`                                                                                          | scoring detail/recheck blocks                                                    |
| POST `/api/scoring/evaluations/:eval_id/submit`      | yes     | true no-mock HTTP | `API_tests/test_scoring.sh`                                                                                          | scoring `[7] submit`                                                             |
| GET `/api/scoring/reviews/pending`                   | yes     | true no-mock HTTP | `API_tests/test_scoring.sh`                                                                                          | scoring `[8] pending reviews`                                                    |
| POST `/api/scoring/reviews/:eval_id`                 | yes     | true no-mock HTTP | `API_tests/test_scoring.sh`                                                                                          | scoring invalid/nonexistent/self-review/qa-review blocks                         |
| GET `/api/reports/order-volume`                      | yes     | true no-mock HTTP | `API_tests/test_reports.sh`, `API_tests/test_ops.sh`                                                                 | reports `[5]` and additional filter variants; ops analytics-disable check        |
| GET `/api/reports/revenue`                           | yes     | true no-mock HTTP | `API_tests/test_reports.sh`, `API_tests/test_ops.sh`                                                                 | reports `[6]`; ops analytics-disable check                                       |
| GET `/api/reports/utilization`                       | yes     | true no-mock HTTP | `API_tests/test_reports.sh`, `API_tests/test_ops.sh`                                                                 | reports `[7]`; ops analytics-disable check                                       |
| GET `/api/reports/kpi`                               | yes     | true no-mock HTTP | `API_tests/test_reports.sh`, `API_tests/test_ops.sh`                                                                 | reports `[2-4]`; ops analytics-disable check                                     |
| POST `/api/reports/export`                           | yes     | true no-mock HTTP | `API_tests/test_smoke.sh`, `API_tests/test_reports.sh`, `API_tests/test_ops.sh`                                      | smoke protected 401; reports `[8-17+]`; ops `[8]`                                |
| GET `/api/ops/flags`                                 | yes     | true no-mock HTTP | `API_tests/test_smoke.sh`, `API_tests/test_ops.sh`                                                                   | smoke protected; ops `[6]`                                                       |
| POST `/api/ops/flags/:key/enable`                    | yes     | true no-mock HTTP | `API_tests/test_ops.sh`                                                                                              | ops `[7],[8],[9],[10],[11]`                                                      |
| POST `/api/ops/flags/:key/disable`                   | yes     | true no-mock HTTP | `API_tests/test_ops.sh`                                                                                              | ops `[7],[8],[9],[10],[11]`                                                      |
| GET `/api/audit/`                                    | yes     | true no-mock HTTP | `API_tests/test_smoke.sh`, `API_tests/test_auth.sh`                                                                  | smoke protected; auth `[8]`                                                      |

**Endpoint mapping verdict:** all 91 resolved endpoints have at least one HTTP request test evidence.

---

## API Test Classification

### 1) True No-Mock HTTP

Evidence profile:

- Test scripts use real HTTP via `curl` against `http://localhost:8000` or configured backend URL.
- Requests target mounted route paths (`/api/...`) and validate status/body.
- No stubs/mocks of transport/controller/service observed.

Files:

- `repo/API_tests/test_smoke.sh`
- `repo/API_tests/test_auth.sh`
- `repo/API_tests/test_catalog_delivery.sh`
- `repo/API_tests/test_billing.sh`
- `repo/API_tests/test_scoring.sh`
- `repo/API_tests/test_reports.sh`
- `repo/API_tests/test_ops.sh`
- `repo/API_tests/test_gaps.sh`
- `repo/API_tests/test_e2e.sh`
- `repo/unit_tests/backend/test_health.sh` (HTTP probe style)

### 2) HTTP with Mocking

- **None detected** by static scan in API test scripts.

### 3) Non-HTTP (unit/integration without HTTP transport)

Backend examples:

- `repo/backend/src/api/auth/tests.rs`
- `repo/backend/src/api/billing/tests.rs`
- `repo/backend/src/application/auth_service.rs` (`#[cfg(test)] mod tests`)
- `repo/backend/src/application/scoring_service.rs` (`#[cfg(test)] mod tests`)
- `repo/backend/src/infrastructure/encryption/mod.rs` (`#[cfg(test)] mod tests`)

Frontend examples:

- `repo/frontend/src/app_test.rs`
- `repo/frontend/src/state/state_test.rs`
- `repo/frontend/src/features/features_test.rs`
- `repo/frontend/src/pages/login/login_test.rs`
- `repo/frontend/src/pages/delivery/mod.rs` inline `#[cfg(test)] mod tests`

---

## Mock Detection

Static pattern scan targets: `jest.mock`, `vi.mock`, `sinon.stub`, mock/stub/override/fake usage.

### Findings

- No framework mocking constructs detected in test code (`jest.mock`, `vi.mock`, `sinon.stub`: none).
- Occurrences of `fake` are test data identifiers (e.g., `fake-role-id`, `invoice_id:"fake"`) in shell requests, not mocking infrastructure.
- No DI override pattern was identified in route-level tests.

### Classification impact

- API shell suites remain classified as **true no-mock HTTP**.

---

## Coverage Summary

- Total endpoints: **91**
- Endpoints with HTTP tests: **91**
- Endpoints with true no-mock HTTP tests: **91**

Computed:

- HTTP coverage = `91 / 91 = 100.0%`
- True API coverage = `91 / 91 = 100.0%`

> Strict caveat: percentages are based on static method+path matching evidence, not executed pass/fail results.

---

## Unit Test Summary

### Backend Unit Tests

Evidence files (non-exhaustive key set):

- API-layer tests: `repo/backend/src/api/*/tests.rs`
- Application-layer tests: `repo/backend/src/application/{auth_service,scoring_service,payment_service,metrics_service,degradation_service,chaos_service,alert_engine}.rs`
- Domain/infra tests: `repo/backend/src/domain/{billing_types,catalog_types,scoring_types}.rs`, `repo/backend/src/infrastructure/{encryption,permission_cache}/mod.rs`, `repo/backend/src/config/mod.rs`

Modules covered by unit tests:

- Controllers/API contracts: auth, admin_org, users/roles/scopes, catalog, packages, plans, delivery, billing, payments/refunds, scoring, reports, ops, observability, audit.
- Services/application logic: auth/password hashing, scoring review rules, payment math/idempotency helpers, metrics/alerts, chaos/degradation logic.
- Infra: encryption and permission cache utility behavior.

Important backend modules not clearly unit-tested (by direct module-local tests):

- `repo/backend/src/application/org_service.rs`
- `repo/backend/src/application/user_service.rs`
- `repo/backend/src/application/role_service.rs`
- `repo/backend/src/application/catalog_service.rs`
- `repo/backend/src/application/package_service.rs`
- `repo/backend/src/application/plan_service.rs`
- `repo/backend/src/application/delivery_service.rs`
- `repo/backend/src/application/billing_service.rs`
- `repo/backend/src/application/reconciliation_service.rs`
- `repo/backend/src/application/report_service.rs`
- `repo/backend/src/application/export_service.rs`

### Frontend Unit Tests (STRICT REQUIREMENT)

Frontend test files detected:

- `repo/frontend/src/app_test.rs`
- `repo/frontend/src/url_utils_test.rs`
- `repo/frontend/src/state/state_test.rs`
- `repo/frontend/src/models/models_test.rs`
- `repo/frontend/src/features/features_test.rs`
- `repo/frontend/src/services/api_client_test.rs`
- `repo/frontend/src/pages/login/login_test.rs`
- `repo/frontend/src/pages/delivery/mod.rs` inline `#[cfg(test)] mod tests`

Framework/tools detected:

- Rust built-in test harness (`#[test]`, `cargo test --lib`) via module wiring in:
  - `repo/frontend/src/lib.rs`
  - `repo/frontend/src/app.rs`
  - `repo/frontend/src/features/mod.rs`
  - `repo/frontend/src/state/mod.rs`
  - `repo/frontend/src/models/mod.rs`
  - `repo/frontend/src/pages/login/mod.rs`

Components/modules covered:

- Auth/session state logic (`AuthState`, permission checks)
- URL utilities and API path construction contracts
- Login page validation/error mapping helpers
- Delivery page payload/validation pure functions
- Feature helper modules for reporting/scoring/billing/ops

Important frontend components/modules NOT tested (or not directly evidenced):

- UI component rendering behavior in `repo/frontend/src/components/**`
- Layout behavior in `repo/frontend/src/layouts/**` (except indirect route wiring checks)
- Full page modules: `admin`, `users`, `catalog`, `plans`, `billing`, `scoring`, `reports`, `audit`, `ops`, `dashboard`
- Router navigation behavior (runtime transitions/guards)
- Live HTTP behavior in `api_client` methods requiring WASM/browser runtime (explicitly noted as not covered by native tests)

### Mandatory Verdict

**Frontend unit tests: PRESENT**

(They exist and target frontend modules with explicit test framework evidence.)

### Cross-layer observation

- Backend API testing depth is substantially heavier than frontend runtime/UI testing.
- Frontend has meaningful pure-logic unit tests, but limited browser/UI integration evidence.
- Balance verdict: **backend-heavy; frontend runtime coverage comparatively shallow**.

---

## API Observability Check

Criteria: tests should clearly show endpoint, request input, and response content.

- Strong examples:
  - `test_e2e.sh` steps include explicit method/path, content-type checks, and field assertions.
  - `test_smoke.sh` validates status and selected response fields for health/auth error envelopes.
  - `test_reports.sh` includes structured response-shape and numeric-range assertions.
- Weak areas:
  - Several checks are status-code only (especially authorization checks), with minimal response-body assertions.

Observability verdict: **moderate** (not weak overall, but mixed depth across suites).

---

## Tests Check

### Success paths

- Broadly covered across all domains (auth, catalog, plans, delivery, billing, payments, scoring, reports, ops).

### Failure and authorization paths

- Strong coverage for 401/403/404/400 scenarios (auth boundaries, invalid transitions, invalid payloads, scope restrictions).

### Edge cases

- Present in key financial/scoring/reporting tests (idempotency duplicate key, refund cap, KPI ranges, second-review constraints).

### Integration boundaries

- Strong API-level integration evidence via HTTP scripts.
- FE↔BE E2E exists (`test_e2e.sh`) but is API-sequence simulation, not browser automation.

### Assertion quality

- Mostly meaningful assertions with named checks; some suites still include status-only checks.

### `run_tests.sh` characterization

- `repo/run_tests.sh` is Docker-first and orchestrates containerized test execution.
- Positive: aligns with containerized execution expectations.
- Note: scripts rely on `curl`; API scripts also use `python3` for JSON extraction, but intended execution path is within test containers.

---

## End-to-End Expectations (fullstack)

Expected: fullstack should include FE↔BE tests.

- Present: `repo/API_tests/test_e2e.sh` performs login/profile/catalog/write/read-back flow through API.
- Limitation: no browser/UI automation evidence (no DOM-level user-flow assertions).
- Compensation: very strong API coverage + backend unit testing partially compensates.

Verdict: **partial E2E fulfillment** (API-driven E2E present; UI-level E2E not evidenced).

---

## Test Coverage Score (0–100)

**Score: 92 / 100**

### Score rationale

- +40: Endpoint coverage breadth (91/91 statically mapped)
- +25: True no-mock HTTP evidence across domains
- +15: Failure/auth/scope edge coverage quality
- +7: Backend unit coverage breadth
- +5: Frontend unit presence and wiring
- -5: Limited frontend UI/runtime integration depth
- -5: Mixed assertion observability (status-only checks in parts)

---

## Key Gaps

1. **Frontend runtime/UI test gap**: no direct browser-render/component interaction tests evidenced.
2. **Some core backend services lack direct unit tests** (org/user/role/catalog/plan/delivery/billing/reconciliation/report/export service modules).
3. **Observability inconsistency**: several API checks validate only status code without payload semantics.

---

## Confidence & Assumptions

- Confidence: **high** for endpoint inventory and static mapping; **medium-high** for qualitative depth judgments.
- Assumptions:
  1. Rocket route definitions in inspected modules are the authoritative active routing surface.
  2. Shell test lines that build exact request paths are treated as coverage evidence regardless of runtime pass/fail.
  3. For dynamic IDs (`:id` etc.), normalized parameterized paths are considered exact path family coverage.

---

# README Audit

## README location check

- Required path: `repo/README.md`
- Status: **present**

---

## Project Type Detection

- Declared at top: `Project Type: fullstack` (`repo/README.md:1`)
- Inferred structure (frontend + backend + docker compose) is consistent with declaration.

---

## Hard Gate Evaluation

### Formatting

- Clean Markdown structure with headings, tables, and code blocks.
- **Result: PASS**

### Startup Instructions (fullstack)

- Includes required command: `docker-compose up` (`Quick Start` section).
- Also includes `docker compose up` variant.
- **Result: PASS**

### Access Method

- URLs/ports provided:
  - Frontend `http://localhost:3000`
  - Backend `http://localhost:8000`
  - DB connection details
- **Result: PASS**

### Verification Method

- Includes concrete verification steps:
  - health curl checks
  - login/token and protected endpoint calls
  - unauthenticated 401 check
  - web UI flow checklist
  - full test-run instruction
- **Result: PASS**

### Environment Rules (STRICT)

- No `npm install`, `pip install`, `apt-get`, runtime install instructions, or manual DB setup requirement in startup path.
- README explicitly states migrations/seeds are automatic and no manual DB setup required.
- **Result: PASS**

### Demo Credentials (auth present)

- Auth exists and README provides demo accounts with credentials and roles:
  - `admin`, `ops_manager`, `billing_staff`, `coach`, `qa_reviewer`, `auditor`
- **Result: PASS**

---

## Engineering Quality Review

### Strengths

- Clear tech stack section.
- Good operational guidance (observability, degradation toggles, chaos controls).
- Security and RBAC overview is concrete.
- Repository structure and major modules documented.

### Weaknesses (non-gate)

- Endpoint catalog in README appears summarized and not guaranteed to stay synchronized with full 91-route backend inventory.
- Some operational claims (e.g., “all suites pass”, coverage counts) are declarative and not self-verifying from README alone.

---

## High Priority Issues

- **None identified** (hard-gate criticals all passed).

## Medium Priority Issues

1. Endpoint listing drift risk: README endpoint section is concise and may become stale relative to route code.
2. Test/coverage claims in prose are not linked to generated artifacts/reports.

## Low Priority Issues

1. README is long and dense; discoverability could improve with a quick TOC.
2. Some advanced operational details may be excessive for first-time setup readers.

## Hard Gate Failures

- **None**

## README Verdict

**PASS**

---

## Final Combined Verdicts

- **Test Coverage Audit Verdict:** **STRONG with targeted gaps**
  - Endpoint HTTP coverage is complete by static mapping (91/91), but frontend runtime/UI testing depth is limited.
- **README Audit Verdict:** **PASS**
  - All strict hard gates satisfied for a fullstack project.
