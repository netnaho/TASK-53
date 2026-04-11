# Requirements Traceability Matrix

Maps task requirements to implementation modules, files, tests, and documentation.

---

## Phase 1: Infrastructure Requirements

| Requirement | Impl | Test | Doc |
|------------|------|------|-----|
| One-command startup (`docker compose up`) | `docker-compose.yml` | Manual | `README.md` |
| All services in Docker Compose | `docker-compose.yml` | Manual | `README.md` |
| No manual migrations/seeds | `infrastructure/database/`, `application/seed_service.rs` | `unit_tests/` | `docs/architecture.md` |
| Rocket backend | `backend/` | cargo test | `docs/architecture.md` |
| Dioxus frontend | `frontend/` | cargo check | `docs/architecture.md` |
| MySQL database | `docker-compose.yml`, `migrations/` | `API_tests/` | `docs/architecture.md` |

## Phase 2: Security & Auth Requirements

### Authentication

| Requirement | Impl | Test | Doc |
|------------|------|------|-----|
| Local username/password auth | `application/auth_service.rs` (login) | `API_tests/test_auth.sh` [1,2,3] | `docs/security_model.md` S1 |
| Argon2id password hashing | `application/auth_service.rs` (hash_password) | `auth_service::tests` | `docs/security_model.md` S1 |
| JWT session tokens | `application/auth_service.rs` (login, validate_token) | `API_tests/test_auth.sh` [1,5] | `docs/security_model.md` S1 |
| Session revocation on logout | `application/auth_service.rs` (logout) | `API_tests/test_auth.sh` [10] | `docs/security_model.md` S1 |
| Account lockout (5 fails) | `application/auth_service.rs` (login) | Implicit in auth_service | `docs/security_model.md` S1 |
| Protected routes return 401 | `api/guards/mod.rs` (AuthenticatedUser) | `API_tests/test_smoke.sh`, `test_auth.sh` [4] | `docs/security_model.md` S1 |

### RBAC

| Requirement | Impl | Test | Doc |
|------------|------|------|-----|
| Fine-grained permissions | `domain/auth_policy.rs` (55+ permission codes) | `API_tests/test_auth.sh` [7,8,9] | `docs/security_model.md` S2 |
| Menu visibility permissions | `frontend/src/components/sidebar.rs` | Frontend inspection | `docs/security_model.md` S2 |
| Button/action permissions | `frontend/src/pages/users/mod.rs`, `admin/mod.rs` | Frontend inspection | `docs/security_model.md` S2 |
| API authorization | `api/guards/mod.rs` (require_permission) | `API_tests/test_auth.sh` [7,9] | `docs/security_model.md` S2 |
| Backend enforces independently | All API handlers use guards | `API_tests/test_auth.sh` | `docs/security_model.md` S2 |
| 6 default roles with permissions | `domain/auth_policy.rs` (default_role_permissions) | `application/seed_service.rs` | `README.md`, `docs/security_model.md` S2 |
| Role CRUD | `application/role_service.rs`, `api/users_roles_permissions/` | `API_tests/test_auth.sh` | `README.md` |
| Permission list/read | `application/role_service.rs` (list_permissions) | `API_tests/test_auth.sh` | `README.md` |
| Role assignment/revocation | `application/user_service.rs` (assign_role, revoke_role) | `API_tests/test_auth.sh` | `README.md` |

### Data-Scope

| Requirement | Impl | Test | Doc |
|------------|------|------|-----|
| Organization-level scope | `infrastructure/permission_cache/mod.rs` (check_data_scope) | `API_tests/test_auth.sh` [9] | `docs/security_model.md` S3 |
| Department-level scope | `permission_cache/mod.rs`, `user_data_scopes` table | Code-level | `docs/security_model.md` S3 |
| Project-level scope | `permission_cache/mod.rs`, `user_data_scopes` table | Code-level | `docs/security_model.md` S3 |
| Scope CRUD APIs | `application/user_service.rs`, `api/users_roles_permissions/` | API tests | `README.md` |
| Scope enforcement at API level | `api/guards/mod.rs` (require_data_scope), `api/admin_org/mod.rs` | `API_tests/test_auth.sh` | `docs/security_model.md` S3 |
| **TASK-53**: Cross-org scope enforcement on role/scope mgmt APIs | `api/users_roles_permissions/mod.rs` (get_user_roles, assign_role, revoke_role, get_user_scopes, assign_scope, revoke_scope) — added `require_data_scope` after target-user lookup; `application/user_service.rs` (get_scope) | `API_tests/test_auth.sh` [12-21] | This table |

### Permission Cache

| Requirement | Impl | Test | Doc |
|------------|------|------|-----|
| In-process cache | `infrastructure/permission_cache/mod.rs` (PermissionCache) | `permission_cache::tests` | `docs/security_model.md` S4 |
| <= 30 second TTL | `permission_cache/mod.rs` (Duration::from_secs(ttl.min(30))) | Code inspection | `docs/security_model.md` S4 |
| Version-based invalidation | `permission_cache/mod.rs` (check_version) | `permission_cache::tests` | `docs/security_model.md` S4 |
| Invalidate on permission change | `user_service.rs`, `role_service.rs` (perm_cache.invalidate()) | Integration tests | `docs/security_model.md` S4 |
| permission_version table | `migrations/20240102000000_security_rbac_audit.sql` | Migration | `docs/security_model.md` S4 |

### Encryption

| Requirement | Impl | Test | Doc |
|------------|------|------|-----|
| AES-256-GCM encryption | `infrastructure/encryption/mod.rs` (EncryptionService) | `encryption::tests` (4 tests) | `docs/security_model.md` S5 |
| Environment key material | `config/mod.rs` (encryption_key), `docker-compose.yml` | Config test | `docs/security_model.md` S5 |
| Client identifier encryption | `migrations/` (client_identifier_enc column) | Schema | `docs/security_model.md` S5 |
| Sensitive notes encryption | `migrations/` (notes_enc column) | Schema | `docs/security_model.md` S5 |
| Masking for safe display | `infrastructure/encryption/mod.rs` (mask) | `encryption::tests::test_mask_values` | `docs/security_model.md` S5 |
| No raw secrets in logs | `EncryptionService` Debug impl redacts key | Code inspection | `docs/security_model.md` S5 |

### Audit Logging

| Requirement | Impl | Test | Doc |
|------------|------|------|-----|
| Audit log table | `migrations/20240102000000_security_rbac_audit.sql` (audit_logs) | Migration | `docs/security_model.md` S6 |
| Auth event logging | `application/auth_service.rs` (login success/failure, logout) | `API_tests/test_auth.sh` [1,2,10] | `docs/security_model.md` S6 |
| User/role/permission change logging | `application/user_service.rs`, `role_service.rs` | API tests | `docs/security_model.md` S6 |
| Org/dept/project change logging | `application/org_service.rs` | API tests | `docs/security_model.md` S6 |
| Audit query API | `api/audit_api/mod.rs` | `API_tests/test_auth.sh` [8] | `docs/security_model.md` S6 |
| No password logging | `infrastructure/audit/mod.rs` design | Code inspection | `docs/security_model.md` S6 |
| Immutable records | audit_logs has no UPDATE/DELETE endpoints | Code inspection | `docs/security_model.md` S6 |

## Domain Module Requirements

| Domain | Backend Module | Frontend Page | API Path | Auth Required |
|--------|---------------|--------------|----------|---------------|
| Authentication | `api/auth/` | `pages/login/` | `/api/auth` | Public (login) |
| Organization Admin | `api/admin_org/` | `pages/admin/` | `/api/admin/org` | `api.org.*` |
| Users & Roles | `api/users_roles_permissions/` | `pages/users/` | `/api/users`, `/api/roles` | `api.users.*`, `api.roles.*` |
| Audit | `api/audit_api/` | `pages/audit/` | `/api/audit` | `api.audit.read` |
| Service Catalog | `api/service_catalog/` | `pages/catalog/` | `/api/catalog` | Authenticated |
| Packages | `api/packages/` | `pages/catalog/` | `/api/packages` | Authenticated |
| Client Plans | `api/client_plans/` | `pages/plans/` | `/api/plans` | Authenticated |
| Delivery Entries | `api/delivery_entries/` | `pages/delivery/` | `/api/delivery` | Authenticated |
| Billing | `api/billing/` | `pages/billing/` | `/api/billing` | Authenticated |
| Payments & Refunds | `api/payments_refunds/` | `pages/billing/` | `/api/payments` | Authenticated |
| Quality Scoring | `api/scoring_reviews/` | `pages/scoring/` | `/api/scoring` | Authenticated |
| Reports & Exports | `api/reports_exports/` | `pages/reports/` | `/api/reports` | Authenticated |
| Observability | `api/observability/` | `pages/ops/` (health/metrics/alerts) | `/api/health` | Public (live/ready), Bearer + api.ops.read (metrics/alerts/chaos) |
| Ops Controls | `api/ops/` | `pages/ops/` (toggles tab) | `/api/ops` | api.ops.read (list), api.ops.write (toggle) |

## Database Schema Requirements

### Phase 1 Tables

| Table | Migration | Domain |
|-------|-----------|--------|
| `organizations` | `001_initial_schema.sql` | Admin/Org |
| `users` | `001_initial_schema.sql` | Users |
| `service_catalog` | `001_initial_schema.sql` | Catalog |
| `packages`, `package_services` | `001_initial_schema.sql` | Packages |
| `client_plans` | `001_initial_schema.sql` | Plans |
| `delivery_entries` | `001_initial_schema.sql` | Delivery |
| `invoices` | `001_initial_schema.sql` | Billing |
| `payments` | `001_initial_schema.sql` | Payments |
| `quality_scores` | `001_initial_schema.sql` | Scoring |
| `_seed_history` | `001_initial_schema.sql` | Infrastructure |

### Phase 2 Tables

| Table | Migration | Domain |
|-------|-----------|--------|
| `departments` | `002_security_rbac_audit.sql` | Org structure |
| `projects` | `002_security_rbac_audit.sql` | Org structure |
| `user_credentials` | `002_security_rbac_audit.sql` | Auth |
| `sessions` | `002_security_rbac_audit.sql` | Auth |
| `roles` | `002_security_rbac_audit.sql` | RBAC |
| `permissions` | `002_security_rbac_audit.sql` | RBAC |
| `role_permissions` | `002_security_rbac_audit.sql` | RBAC |
| `user_roles` | `002_security_rbac_audit.sql` | RBAC |
| `user_data_scopes` | `002_security_rbac_audit.sql` | Data scope |
| `permission_version` | `002_security_rbac_audit.sql` | Cache |
| `audit_logs` | `002_security_rbac_audit.sql` | Audit |

## Test Coverage

| Suite | Location | Scope | Status |
|-------|----------|-------|--------|
| Backend unit tests | `backend/src/` (cargo test) | Password hashing, encryption, cache logic | Active |
| Security unit tests | `unit_tests/backend/test_password_hashing.sh` | Runs cargo test --lib | Active |
| Health smoke test | `unit_tests/backend/test_health.sh` | Health endpoint | Active |
| API smoke tests | `API_tests/test_smoke.sh` | Auth boundary (401 for protected) | Active |
| API auth tests | `API_tests/test_auth.sh` | Login, logout, 401, 403, role-based access | Active |
| Test runner | `run_tests.sh` | Orchestrates all 6 suites | Active |

## Documentation

| Document | Location | Status |
|----------|----------|--------|
| Project README | `README.md` | Active |
| Architecture | `docs/architecture.md` | Active |
| Security Model | `docs/security_model.md` | Active |
| Catalog & Delivery | `docs/catalog_and_delivery_workflows.md` | Active |
| Requirements Traceability | `docs/requirements_traceability.md` | Active (this file) |

## Phase 3: Catalog, Packages, Plans, Delivery

### Service Catalog

| Requirement | Impl | Test | Doc |
|------------|------|------|-----|
| Service item CRUD | `application/catalog_service.rs`, `api/service_catalog/` | `API_tests/test_catalog_delivery.sh` [1-5] | `docs/catalog_and_delivery_workflows.md` S1 |
| Category validation | `domain/catalog_types.rs` (validate_category) | `test_catalog_delivery.sh` [3] | Workflow doc S1 |
| Code uniqueness per org | `catalog_service.rs` (create_item) | `test_catalog_delivery.sh` [2] | Workflow doc S1 |
| Data-scope enforcement | `api/service_catalog/mod.rs` (require_data_scope) | `test_catalog_delivery.sh` [5] | Workflow doc S5 |

### Package Rules

| Requirement | Impl | Test | Doc |
|------------|------|------|-----|
| Package CRUD with rules | `application/package_service.rs`, `api/packages/` | `test_catalog_delivery.sh` [7-8] | Workflow doc S2 |
| per_visit rule type | `domain/catalog_types.rs`, `package_service.rs` | `test_catalog_delivery.sh` [7] | Workflow doc S2 |
| hourly rule type | `catalog_types.rs` (validate_quarter_hour) | `catalog_types::tests` | Workflow doc S2 |
| tiered rule type | `catalog_types.rs` (validate_tier_config) | `catalog_types::tests` | Workflow doc S2 |
| Rule validation | `catalog_types.rs` (validate_package_rule) | `catalog_types::tests`, `test_catalog_delivery.sh` [8] | Workflow doc S2 |
| Service-org cross-check | `package_service.rs` (create_package) | `test_catalog_delivery.sh` | Workflow doc S2 |

### Client Plans

| Requirement | Impl | Test | Doc |
|------------|------|------|-----|
| Plan CRUD | `application/plan_service.rs`, `api/client_plans/` | `test_catalog_delivery.sh` [9-10] | Workflow doc S3 |
| Package assignment | `plan_service.rs` (assign_package) | `test_catalog_delivery.sh` [11] | Workflow doc S3 |
| Encrypted client ID | `plan_service.rs` (create_plan, encryption.encrypt) | Code inspection | Workflow doc S3 |
| Encrypted notes | `plan_service.rs` (create_plan, update_plan) | Code inspection | Workflow doc S3 |
| Date validation | `plan_service.rs` (NaiveDate parse) | `test_catalog_delivery.sh` [10] | Workflow doc S3 |

### Delivery Entries

| Requirement | Impl | Test | Doc |
|------------|------|------|-----|
| Delivery entry CRUD | `application/delivery_service.rs`, `api/delivery_entries/` | `test_catalog_delivery.sh` [12, 16] | Workflow doc S4 |
| Quarter-hour validation | `catalog_types.rs` (validate_quarter_hour) | `catalog_types::tests`, `test_catalog_delivery.sh` [13] | Workflow doc S4 |
| Mileage cap (200 mi) | `catalog_types.rs` (validate_mileage) | `catalog_types::tests`, `test_catalog_delivery.sh` [14] | Workflow doc S4 |
| Plan-package linkage | `delivery_service.rs` (create_entry) | `test_catalog_delivery.sh` [12] | Workflow doc S4 |
| Service-in-package check | `delivery_service.rs` (create_entry) | `test_catalog_delivery.sh` | Workflow doc S4 |
| Encrypted delivery notes | `delivery_service.rs` (encryption.encrypt) | Code inspection | Workflow doc S4 |
| Eligibility notes | `delivery_service.rs` (create_note, list_notes) | API endpoints | Workflow doc S4 |
| Verify permission check | `api/delivery_entries/mod.rs` (action::VERIFY_DELIVERY) | Code inspection | Workflow doc S4 |
| Billed entries immutable | `delivery_service.rs` (update_entry) | Code inspection | Workflow doc S4 |

### Database Schema (Phase 3)

| Table | Migration | Domain |
|-------|-----------|--------|
| `service_catalog_items` | `003_catalog_packages_plans_delivery.sql` | Catalog |
| `package_definitions` | `003_catalog_packages_plans_delivery.sql` | Packages |
| `package_rule_definitions` | `003_catalog_packages_plans_delivery.sql` | Package rules |
| `client_plans` (enhanced) | `003_catalog_packages_plans_delivery.sql` | Plans |
| `client_plan_packages` | `003_catalog_packages_plans_delivery.sql` | Plan assignments |
| `delivery_entries` (enhanced) | `003_catalog_packages_plans_delivery.sql` | Delivery |
| `eligibility_notes` | `003_catalog_packages_plans_delivery.sql` | Clinical notes |

### Phase 3 Test Coverage

| Suite | Location | Scope |
|-------|----------|-------|
| Validation unit tests | `backend/src/domain/catalog_types.rs` (tests) | Quarter-hour, mileage, tier config, rule validation |
| Catalog & delivery API tests | `API_tests/test_catalog_delivery.sh` (16 tests) | CRUD, validation, auth, scope |
| Updated smoke tests | `API_tests/test_smoke.sh` | Auth boundary verification |

---

## Phase 4: Billing Engine, Invoices, Payments, Refunds, Reconciliation

### Charge Generation

| Requirement | Impl | Test | Doc |
|------------|------|------|-----|
| Generate charges from verified delivery entries | `application/billing_service.rs` (generate_charges) | `API_tests/test_billing.sh` [3,4] | `docs/billing_and_financial_controls.md` S2 |
| Per-visit, hourly, tiered rate computation | `billing_service.rs` (compute_tiered_amount) | `test_billing.sh` [3] | Billing doc S2 |
| Skip already-charged entries (idempotent) | `billing_service.rs` (LEFT JOIN charges) | `test_billing.sh` [4] | Billing doc S2 |
| Charge adjustments (additive, immutable) | `billing_service.rs` (post_adjustment) | `test_billing.sh` [7,8] | Billing doc S3 |
| Adjustment blocked on invoiced/voided charge | `billing_service.rs` (post_adjustment status check) | `test_billing.sh` [8] | Billing doc S3 |

### Invoice Generation

| Requirement | Impl | Test | Doc |
|------------|------|------|-----|
| Invoice from pending charges with period filter | `billing_service.rs` (generate_invoice) | `test_billing.sh` [9,10] | Billing doc S4 |
| Invoice number generation (INV-YYYYMM-suffix) | `billing_service.rs` (invoice_number) | `test_billing.sh` [9] | Billing doc S4 |
| Immutable line items snapshot | `invoice_line_items` table, `billing_service.rs` | `test_billing.sh` [9] | Billing doc S4 |
| Charges linked to invoice on generation | `charges.invoice_id`, `status = 'invoiced'` | Code inspection | Billing doc S4 |
| Status transition enforcement | `domain/billing_types.rs` (validate_invoice_status_transition) | `test_billing.sh` [12,13] | Billing doc S4 |
| Status validation unit tests | `billing_types::tests` | `cargo test --lib` | Billing doc |

### Recorded Payments

| Requirement | Impl | Test | Doc |
|------------|------|------|-----|
| Record payment with idempotency key | `application/payment_service.rs` (record_payment) | `test_billing.sh` [15] | Billing doc S5 |
| 5-minute duplicate rejection window | `payment_service.rs` (INSERT ON DUPLICATE KEY UPDATE into `payment_idempotency_keys`; rows_affected=0 → 409), `migrations/20240108000000_restore_idempotency_window.sql` | `test_billing.sh` [16, 26] | Billing doc S5 |
| Different key accepted | `payment_service.rs` | `test_billing.sh` [17] | Billing doc S5 |
| Fund transaction created on payment | `fund_transactions` (credit, immutable) | `test_billing.sh` [22] | Billing doc S7 |
| Invoice status auto-updated after payment | `payment_service.rs` (reconcile_invoice_payment_status) | Code inspection | Billing doc |

### Recorded Refunds

| Requirement | Impl | Test | Doc |
|------------|------|------|-----|
| Mandatory reason code | `payment_service.rs` (record_refund reason_code validation) | `test_billing.sh` [20] | Billing doc S6 |
| Net-paid cap enforcement | `payment_service.rs` (net_paid > amount check) | `test_billing.sh` [19] | Billing doc S6 |
| Refund reason codes seeded | `migrations/20240104000000_billing_payments_refunds.sql` | `test_billing.sh` [14] | Billing doc S6 |
| Fund transaction created on refund | `fund_transactions` (debit, immutable) | `test_billing.sh` [22] | Billing doc S7 |
| Role check: only billing staff/admin can refund | `api/payments_refunds/mod.rs` (action::PROCESS_REFUND) | `test_billing.sh` [21] | Billing doc |

### Immutable Fund Transactions

| Requirement | Impl | Test | Doc |
|------------|------|------|-----|
| fund_transactions table (no UPDATE/DELETE) | `migrations/20240104000000_billing_payments_refunds.sql` | Schema inspection | Billing doc S7 |
| Every payment creates fund_transaction | `payment_service.rs` (INSERT fund_transactions) | `test_billing.sh` [22] | Billing doc S7 |
| Every refund creates fund_transaction | `payment_service.rs` (INSERT fund_transactions) | `test_billing.sh` [22] | Billing doc S7 |
| Read-only API for transactions | `api/payments_refunds/mod.rs` (GET only) | `test_billing.sh` [22] | Billing doc S7 |

### Reconciliation

| Requirement | Impl | Test | Doc |
|------------|------|------|-----|
| Reconciliation snapshot generation | `application/reconciliation_service.rs` | `test_billing.sh` [23] | Billing doc S8 |
| Period validation (end >= start) | `reconciliation_service.rs` | `test_billing.sh` [24] | Billing doc S8 |
| Immutable reconciliation runs | `reconciliation_runs` (no updated_at) | Schema inspection | Billing doc S8 |

### Database Schema (Phase 4)

| Table | Migration | Domain |
|-------|-----------|--------|
| `refund_reason_codes` | `004_billing_payments_refunds.sql` | Lookup |
| `charges` | `004_billing_payments_refunds.sql` | Billing |
| `charge_adjustments` | `004_billing_payments_refunds.sql` | Billing |
| `invoices` | `004_billing_payments_refunds.sql` | Billing |
| `invoice_line_items` | `004_billing_payments_refunds.sql` | Billing |
| `fund_transactions` | `004_billing_payments_refunds.sql` | Financial ledger |
| `recorded_payments` | `004_billing_payments_refunds.sql` | Payments |
| `recorded_refunds` | `004_billing_payments_refunds.sql` | Refunds |
| `reconciliation_runs` | `004_billing_payments_refunds.sql` | Reconciliation |

### Phase 4 Test Coverage

| Suite | Location | Scope |
|-------|----------|-------|
| Billing type unit tests | `backend/src/domain/billing_types.rs` (tests) | Payment method validation, invoice status transitions |
| Billing API tests | `API_tests/test_billing.sh` (25 tests) | Charge generation, adjustments, invoice lifecycle, payment idempotency, refund cap, 409/400/403, reconciliation |

## Phase 5: Quality Scoring, Second Review, Reporting, Exports

### Scoring Requirements

| Requirement | Impl | Test | Doc |
|------------|------|------|-----|
| Configurable evaluation templates | `application/scoring_service.rs` (create_template) | `API_tests/test_scoring.sh` [4] | `docs/scoring_and_reporting.md` |
| Objective auto-scoring (case-insensitive match) | `domain/scoring_types.rs` (compute_auto_score) | `scoring_types::tests::test_auto_score_*` | `docs/scoring_and_reporting.md` |
| Subjective manual grading | `scoring_service.rs` (submit_evaluation) | `API_tests/test_scoring.sh` [7] | `docs/scoring_and_reporting.md` |
| Partial credit (0–1 fraction of max_points) | `domain/scoring_types.rs` (compute_answer_final_score) | `scoring_types::tests::test_answer_final_score_*` | `docs/scoring_and_reporting.md` |
| Configurable question weights | `domain/scoring_types.rs` (compute_weighted_score) | `scoring_types::tests::test_weighted_score_*` | `docs/scoring_and_reporting.md` |
| Rounding to nearest interval | `domain/scoring_types.rs` (round_to_interval) | `scoring_types::tests::test_round_to_interval_*` | `docs/scoring_and_reporting.md` |
| Progress indicator (answered/total) | `scoring_service.rs` (submit_evaluation, progress_pct) | Audit detail | `docs/scoring_and_reporting.md` |
| Second review enforced when delta > 10 | `domain/scoring_types.rs` (requires_second_review) + `scoring_service.rs` | `scoring_types::tests::test_second_review_trigger_threshold` | `docs/scoring_and_reporting.md` |
| QA Reviewer approve/revise | `scoring_service.rs` (process_second_review) | `API_tests/test_scoring.sh` [8] | `docs/scoring_and_reporting.md` |

### Reporting Requirements

| Requirement | Impl | Test | Doc |
|------------|------|------|-----|
| Order volume report by week | `application/report_service.rs` (order_volume) | `API_tests/test_reports.sh` [5] | `docs/scoring_and_reporting.md` |
| Revenue report by week | `application/report_service.rs` (revenue_report) | `API_tests/test_reports.sh` [6] | `docs/scoring_and_reporting.md` |
| Provider utilization by week | `application/report_service.rs` (utilization_report) | `API_tests/test_reports.sh` [7] | `docs/scoring_and_reporting.md` |
| KPI analytics (attendance, repurchase, utilization, avg score) | `application/report_service.rs` (kpi_summary, round_2dp helper) | `API_tests/test_reports.sh` [4, 4b] | `docs/scoring_and_reporting.md` |
| Date range + department/project filters | `domain/scoring_types.rs` (ReportFilters), all report methods | `API_tests/test_reports.sh` | `docs/scoring_and_reporting.md` |
| Service route dimension filter | `domain/scoring_types.rs` (ReportFilters.service_route, ExportRequest.service_route), `report_service.rs` (route_clause in order_volume, revenue, utilization), `export_service.rs` (route clause in all 3 export methods), `api/reports_exports/mod.rs` (service_route query param), `migrations/20240109000000_add_service_route.sql` (client_plans.service_route column), `frontend/src/pages/reports/mod.rs` (DateRangeFilter service_route input) | `API_tests/test_reports.sh` [20-25] | `docs/scoring_and_reporting.md` |
| Real DB queries, no mocks | All report methods join live tables | Integration tests | `docs/scoring_and_reporting.md` |

### Export Requirements

| Requirement | Impl | Test | Doc |
|------------|------|------|-----|
| Masked by default | `application/export_service.rs` (masked=true default) | `API_tests/test_reports.sh` [8] | `docs/scoring_and_reporting.md` |
| Unmasked requires `api.export.unmasked` | `export_service.rs` (has_unmasked_perm check) | `API_tests/test_reports.sh` [10] | `docs/scoring_and_reporting.md` |
| Export audit log on every export | `export_service.rs` (INSERT export_audit_logs) | `API_tests/test_reports.sh` [8] | `docs/scoring_and_reporting.md` |
| Three export types | `export_service.rs` (deliveries, evaluations, revenue) | `API_tests/test_reports.sh` [11] | `docs/scoring_and_reporting.md` |

### Phase 5 Schema

| Table | Migration | Purpose |
|-------|-----------|---------|
| `scoring_templates` | `005_scoring_reporting.sql` | Evaluation templates |
| `evaluation_questions` | `005_scoring_reporting.sql` | Per-template questions |
| `evaluations` | `005_scoring_reporting.sql` | Grading attempts |
| `evaluation_answers` | `005_scoring_reporting.sql` | Per-question scored answers |
| `score_reviews` | `005_scoring_reporting.sql` | Second review records |
| `export_audit_logs` | `005_scoring_reporting.sql` | Export compliance trail |

### Phase 5 Test Coverage

| Suite | Location | Scope |
|-------|----------|-------|
| Scoring logic unit tests | `backend/src/domain/scoring_types.rs` (11 tests) | Rounding, auto-score, partial credit, weighted score, second-review trigger |
| Scoring API tests | `API_tests/test_scoring.sh` | Template CRUD, evaluation lifecycle, submit, second-review enforcement, 401/403 |
| Reports API tests | `API_tests/test_reports.sh` | KPI, order-volume, revenue, utilization, masked export, unmasked gating, invalid type/date 400 |

---

## Phase 6: Observability, Resilience, Degradation Toggles, Chaos Drills

### Structured Logs & Metrics

| Requirement | Impl | Test | Doc |
|------------|------|------|-----|
| Structured logs via tracing crate | `api/tracing_fairing.rs`, all service modules | Cargo test (log output) | `docs/observability_and_resilience.md` S1 |
| 5xx responses recorded to metrics | `api/tracing_fairing.rs` (on_response → metrics.record) | `metrics_service::tests` | `docs/observability_and_resilience.md` S2 |
| 10-minute sliding window | `application/metrics_service.rs` (WINDOW_DURATION = 600s) | `metrics_service::tests::test_window_expiry` | `docs/observability_and_resilience.md` S2 |
| Window error rate calculation | `metrics_service.rs` (window_error_rate) | `metrics_service::tests::test_error_rate` | `docs/observability_and_resilience.md` S2 |
| No external metrics dependencies | Pure in-process VecDeque, no Prometheus/StatsD | Code inspection | `docs/observability_and_resilience.md` S2 |

### Alert Engine

| Requirement | Impl | Test | Doc |
|------------|------|------|-----|
| Error rate > 2% triggers ALERTING | `application/alert_engine.rs` (ALERT_THRESHOLD = 0.02) | `alert_engine::tests::test_alert_threshold_constant` | `docs/observability_and_resilience.md` S3 |
| Exactly 2% does NOT trigger | Alert rule is `>` not `>=` | `alert_engine::tests::test_status_transitions` | `docs/observability_and_resilience.md` S3 |
| Edge-triggered (DB write on transition only) | `alert_engine.rs` (status change check before INSERT) | Code inspection | `docs/observability_and_resilience.md` S3 |
| 30-second evaluation cycle | `bootstrap/mod.rs` (tokio::time::interval(30s)) | Integration | `docs/observability_and_resilience.md` S3 |
| Alarm state persisted in memory | `AlertEngine { state: Arc<Mutex<AlarmState>> }` | `alert_engine::tests` | `docs/observability_and_resilience.md` S3 |

### Health Endpoints

| Requirement | Impl | Test | Doc |
|------------|------|------|-----|
| `GET /api/health/live` — no auth | `api/observability/mod.rs` (live) | `unit_tests/test_health.sh` | `docs/observability_and_resilience.md` S4 |
| `GET /api/health/ready` — DB ping | `api/observability/mod.rs` (ready) | `API_tests/test_ops.sh` [2] | `docs/observability_and_resilience.md` S4 |
| `GET /api/health/metrics` — Bearer + api.ops.read | `api/observability/mod.rs` (metrics) | `API_tests/test_ops.sh` [3] | `docs/observability_and_resilience.md` S4 |
| `GET /api/health/alerts` — Bearer + api.ops.read | `api/observability/mod.rs` (alerts) | `API_tests/test_ops.sh` [4] | `docs/observability_and_resilience.md` S4 |
| `GET /api/health/chaos` — Bearer + api.ops.read | `api/observability/mod.rs` (chaos) | `API_tests/test_ops.sh` [5] | `docs/observability_and_resilience.md` S4 |

### Degradation Toggles

| Requirement | Impl | Test | Doc |
|------------|------|------|-----|
| `exports_enabled` toggle | `application/degradation_service.rs` (TOGGLE_EXPORTS) | `API_tests/test_ops.sh` [7,8] | `docs/observability_and_resilience.md` S5 |
| `analytics_enabled` toggle | `degradation_service.rs` (TOGGLE_ANALYTICS) | `API_tests/test_ops.sh` [7,9] | `docs/observability_and_resilience.md` S5 |
| Fail-open (true if DB unreachable) | `degradation_service.rs` (get_flag → unwrap_or(true)) | `degradation_service::tests` | `docs/observability_and_resilience.md` S5 |
| Unknown key rejected with 400 | `degradation_service.rs` (KNOWN_TOGGLES check) | `API_tests/test_ops.sh` [10] | `docs/observability_and_resilience.md` S5 |
| Toggle change written to ops_events | `degradation_service.rs` (set_flag → INSERT ops_events) | Integration | `docs/observability_and_resilience.md` S5 |
| Toggle change written to audit_log | `degradation_service.rs` (set_flag → audit.log) | Integration | `docs/observability_and_resilience.md` S5 |
| Exports disabled → 503 on export endpoint | `export_service.rs` (check_exports_enabled) | `API_tests/test_ops.sh` [8] | `docs/observability_and_resilience.md` S5 |
| Analytics disabled → 503 on report endpoints | `report_service.rs` (check_analytics_enabled) | `API_tests/test_ops.sh` [9] | `docs/observability_and_resilience.md` S5 |
| Seeded defaults on startup | `application/seed_service.rs` (seed_ops_config) | Integration | `docs/observability_and_resilience.md` S5 |
| Only api.ops.write can toggle | `api/ops/mod.rs` (require_permission OPS_WRITE) | `API_tests/test_ops.sh` [7] | `docs/observability_and_resilience.md` S8 |

### Chaos Drills

| Requirement | Impl | Test | Doc |
|------------|------|------|-----|
| CHAOS_ENABLED env var guard | `chaos_service.rs` (is_chaos_armed) | `chaos_service::tests::test_chaos_armed_default` | `docs/observability_and_resilience.md` S6 |
| Sunday 02:00–02:15 UTC window | `chaos_service.rs` (is_drill_window) | `chaos_service::tests::test_constants` | `docs/observability_and_resilience.md` S6 |
| 200ms simulated DB latency | `chaos_service.rs` (maybe_inject_latency) | `chaos_service::tests::test_latency_bound` | `docs/observability_and_resilience.md` S6 |
| 5% request timeout simulation | `chaos_service.rs` (should_inject_timeout) | `chaos_service::tests::test_fraction_bound` | `docs/observability_and_resilience.md` S6 |
| Latency injected in report/export services | `report_service.rs`, `export_service.rs` | Integration | `docs/observability_and_resilience.md` S6 |
| Drill start/stop written to ops_events | `chaos_service.rs` (log_drill_started/stopped) | Code inspection | `docs/observability_and_resilience.md` S6 |
| 60s drill monitor background task | `bootstrap/mod.rs` (tokio::spawn chaos monitor) | Code inspection | `docs/observability_and_resilience.md` S6 |

### Phase 6 Schema

| Table | Migration | Purpose |
|-------|-----------|---------|
| `ops_config` | `006_ops_config.sql` | Degradation toggle persistence |
| `ops_events` | `006_ops_config.sql` | Immutable operational event log |

### Phase 6 Test Coverage

| Suite | Location | Scope |
|-------|----------|-------|
| MetricsService unit tests | `backend/src/application/metrics_service.rs` (5 tests) | Empty state, all-success, 20% error rate, window expiry, lifetime totals |
| AlertEngine unit tests | `backend/src/application/alert_engine.rs` (3 tests) | Threshold constant, state construction, status transitions |
| DegradationService unit tests | `backend/src/application/degradation_service.rs` (3 tests) | Known keys, unknown flag validation, bool parse |
| ChaosService unit tests | `backend/src/application/chaos_service.rs` (7 tests) | Default disabled, constants, no timeout when not armed, window format, latency bound, fraction bound, status structure |
| Ops API integration tests | `API_tests/test_ops.sh` | Health probes, metrics, alerts, chaos status, toggle enable/disable, 503 degradation, 400 unknown key, 401/403 auth |

## Documentation

| Document | Location | Status |
|----------|----------|--------|
| Project README | `README.md` | Active |
| Architecture | `docs/architecture.md` | Active |
| Security Model | `docs/security_model.md` | Active |
| Catalog & Delivery | `docs/catalog_and_delivery_workflows.md` | Active |
| Billing & Financial Controls | `docs/billing_and_financial_controls.md` | Active |
| Scoring & Reporting | `docs/scoring_and_reporting.md` | Active |
| Observability & Resilience | `docs/observability_and_resilience.md` | Active |
| Requirements Traceability | `docs/requirements_traceability.md` | Active (this file) |
