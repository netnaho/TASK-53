# CareOps API Specification (Implementation-Based)

This document describes the currently implemented API surface in `repo/backend/src/api/*` and the endpoint matrix documented in `repo/README.md`.

---

## 1) API Basics

### Base URL

- Local default: `http://localhost:8000`
- API prefix: `/api`

### Content Type

- Request: `Content-Type: application/json`
- Response: `application/json`

### Authentication

- Login endpoint returns a JWT token.
- Protected endpoints require:

`Authorization: Bearer <token>`

### Authorization

Permissions are enforced server-side using fine-grained permission codes (e.g. `api.users.read`, `action.billing.generate`) plus data-scope checks where applicable.

---

## 2) Common Response Contracts

### Success Envelope

The backend returns JSON payloads per endpoint (object/list/detail shapes by resource).

### Error Envelope (consistent)

```json
{
  "error": {
    "code": "FORBIDDEN",
    "message": "Missing required permission: api.users.write",
    "trace_id": "..."
  }
}
```

### Common HTTP Statuses

- `200 OK` — successful read/update
- `201 Created` — successful create (where applicable)
- `400 Bad Request` — validation/business-rule violation
- `401 Unauthorized` — missing/invalid/expired token
- `403 Forbidden` — permission or data-scope denied
- `404 Not Found` — resource missing
- `409 Conflict` — idempotency/resource-state conflict
- `503 Service Unavailable` — feature disabled by degradation toggle

---

## 3) Public Endpoints (No Auth Required)

| Method | Path                | Purpose                            |
| ------ | ------------------- | ---------------------------------- |
| GET    | `/api/health/live`  | Liveness probe                     |
| GET    | `/api/health/ready` | Readiness probe (DB connectivity)  |
| POST   | `/api/auth/login`   | Authenticate and get session token |

### 3.1 Login

`POST /api/auth/login`

Request body:

```json
{
  "username": "admin",
  "password": "Admin123!"
}
```

Response returns token + current user profile fields (including role/permission context used by the frontend).

---

## 4) Protected Endpoint Groups

All endpoints below require Bearer authentication.

## 4.1 Authentication Session Endpoints

| Method | Path               | Permission         |
| ------ | ------------------ | ------------------ |
| POST   | `/api/auth/logout` | Authenticated user |
| GET    | `/api/auth/me`     | Authenticated user |

---

## 4.2 Organization / Department / Project (Admin Org)

| Method   | Path                             | Permission                               |
| -------- | -------------------------------- | ---------------------------------------- |
| GET/POST | `/api/admin/org/`                | `api.org.read` / `api.org.write`         |
| GET/POST | `/api/admin/org/:id/departments` | `api.dept.read` / `api.dept.write`       |
| GET/POST | `/api/admin/org/:id/projects`    | `api.project.read` / `api.project.write` |

---

## 4.3 Users, Roles, Permissions, Scopes

| Method          | Path                         | Permission                           |
| --------------- | ---------------------------- | ------------------------------------ |
| GET/POST/PUT    | `/api/users/`                | `api.users.read` / `api.users.write` |
| POST/DELETE     | `/api/users/:id/roles`       | `action.roles.assign`                |
| GET/POST/DELETE | `/api/users/:id/scopes`      | `action.scopes.manage`               |
| GET/POST        | `/api/roles/`                | `api.roles.read` / `api.roles.write` |
| GET/POST/DELETE | `/api/roles/:id/permissions` | `action.permissions.manage`          |
| GET             | `/api/roles/all`             | `api.permissions.read`               |

Notes:

- Role/scope mutation endpoints also enforce cross-org data-scope boundaries.
- Permission changes propagate via in-process cache/version invalidation logic.

---

## 4.4 Audit

| Method | Path          | Permission       |
| ------ | ------------- | ---------------- |
| GET    | `/api/audit/` | `api.audit.read` |

Typical query usage includes action/resource/user filters plus pagination.

---

## 4.5 Service Catalog

| Method       | Path            | Permission                               |
| ------------ | --------------- | ---------------------------------------- |
| GET/POST/PUT | `/api/catalog/` | `api.catalog.read` / `api.catalog.write` |

Used to manage billable service items (category, unit type, default rate).

---

## 4.6 Packages

| Method       | Path                      | Permission                               |
| ------------ | ------------------------- | ---------------------------------------- |
| GET/POST/PUT | `/api/packages/`          | `api.catalog.read` / `api.catalog.write` |
| GET          | `/api/packages/:id/rules` | `api.catalog.read`                       |

Rules support `per_visit`, `hourly`, and `tiered` configurations.

---

## 4.7 Client Plans

| Method       | Path                      | Permission                           |
| ------------ | ------------------------- | ------------------------------------ |
| GET/POST/PUT | `/api/plans/`             | `api.plans.read` / `api.plans.write` |
| POST/GET     | `/api/plans/:id/packages` | `api.plans.write` / `api.plans.read` |

Plans are package-assignment containers for delivery and billing workflows.

---

## 4.8 Delivery Entries

| Method       | Path                      | Permission                                 |
| ------------ | ------------------------- | ------------------------------------------ |
| GET/POST/PUT | `/api/delivery/`          | `api.delivery.read` / `api.delivery.write` |
| GET/POST     | `/api/delivery/:id/notes` | `api.delivery.read` / `api.delivery.write` |

Business validations include quarter-hour increments and mileage max rules.

---

## 4.9 Billing (Charges + Invoices)

| Method | Path                                   | Permission                |
| ------ | -------------------------------------- | ------------------------- |
| POST   | `/api/billing/charges/generate`        | `action.billing.generate` |
| GET    | `/api/billing/charges`                 | `api.billing.read`        |
| GET    | `/api/billing/charges/:id`             | `api.billing.read`        |
| POST   | `/api/billing/charges/:id/adjustments` | `action.billing.generate` |
| POST   | `/api/billing/invoices/generate`       | `action.billing.generate` |
| GET    | `/api/billing/invoices`                | `api.billing.read`        |
| GET    | `/api/billing/invoices/:id`            | `api.billing.read`        |
| PUT    | `/api/billing/invoices/:id/status`     | `action.billing.approve`  |

### 4.9.1 Generate Charges (example)

`POST /api/billing/charges/generate`

Request body (representative):

```json
{
  "plan_id": "<uuid>"
}
```

Behavior:

- generates charges from verified delivery entries
- idempotently skips already-charged entries

### 4.9.2 Post Charge Adjustment (example)

`POST /api/billing/charges/:id/adjustments`

Request body (representative):

```json
{
  "amount": -15.0,
  "reason": "Manual correction"
}
```

---

## 4.10 Payments, Refunds, Reconciliation

| Method | Path                               | Permission               |
| ------ | ---------------------------------- | ------------------------ |
| GET    | `/api/payments/reason-codes`       | `api.payments.read`      |
| POST   | `/api/payments/`                   | `action.payments.record` |
| GET    | `/api/payments/`                   | `api.payments.read`      |
| GET    | `/api/payments/:id`                | `api.payments.read`      |
| POST   | `/api/payments/refunds`            | `action.payments.refund` |
| GET    | `/api/payments/refunds`            | `api.payments.read`      |
| GET    | `/api/payments/refunds/:id`        | `api.payments.read`      |
| GET    | `/api/payments/transactions`       | `api.payments.read`      |
| POST   | `/api/payments/reconciliation`     | `api.billing.read`       |
| GET    | `/api/payments/reconciliation`     | `api.billing.read`       |
| GET    | `/api/payments/reconciliation/:id` | `api.billing.read`       |

### 4.10.1 Record Payment (example)

`POST /api/payments/`

Request body (representative):

```json
{
  "invoice_id": "<uuid>",
  "amount": 250.0,
  "payment_method": "recorded_payment",
  "idempotency_key": "2f84d09a-0efd-4d8e-bcc6-2a3d71191d7f"
}
```

Idempotency behavior:

- duplicate `(org_id, idempotency_key)` within 5 minutes -> `409 Conflict`
- expired window allows reuse

### 4.10.2 Record Refund (example)

`POST /api/payments/refunds`

Request body (representative):

```json
{
  "invoice_id": "<uuid>",
  "amount": 40.0,
  "reason_code": "BILLING_ERROR",
  "notes": "Overcharge correction"
}
```

Rules:

- reason code required
- refund amount cannot exceed net paid

---

## 4.11 Scoring and Reviews

| Method | Path                                  | Permission          |
| ------ | ------------------------------------- | ------------------- |
| POST   | `/api/scoring/templates`              | `api.scoring.write` |
| GET    | `/api/scoring/templates`              | `api.scoring.read`  |
| GET    | `/api/scoring/templates/:id`          | `api.scoring.read`  |
| POST   | `/api/scoring/evaluations`            | `api.scoring.write` |
| GET    | `/api/scoring/evaluations`            | `api.scoring.read`  |
| GET    | `/api/scoring/evaluations/:id`        | `api.scoring.read`  |
| POST   | `/api/scoring/evaluations/:id/submit` | `api.scoring.write` |
| GET    | `/api/scoring/reviews/pending`        | `api.scoring.read`  |
| POST   | `/api/scoring/reviews/:eval_id`       | `api.scoring.write` |

### 4.11.1 Submit Evaluation (example)

`POST /api/scoring/evaluations/:id/submit`

Request body (representative):

```json
{
  "answers": [
    {
      "question_id": "<uuid>",
      "answer_text": "yes",
      "manual_score": null,
      "partial_credit_fraction": null,
      "comment": "Auto-scored"
    }
  ],
  "overall_comment": "Reviewed"
}
```

Backend enforces second review when score delta exceeds 10 points.

---

## 4.12 Reports and Exports

| Method | Path                        | Permission              |
| ------ | --------------------------- | ----------------------- |
| GET    | `/api/reports/kpi`          | `api.reports.read`      |
| GET    | `/api/reports/order-volume` | `api.reports.read`      |
| GET    | `/api/reports/revenue`      | `api.reports.read`      |
| GET    | `/api/reports/utilization`  | `api.reports.read`      |
| POST   | `/api/reports/export`       | `action.reports.export` |

Common filters include `from_date`, `to_date`, optional scope filters, and optional `service_route`.

### 4.12.1 Export (example)

`POST /api/reports/export`

Request body (representative):

```json
{
  "type": "deliveries",
  "from_date": "2024-01-01",
  "to_date": "2024-01-31",
  "department_id": null,
  "project_id": null,
  "service_route": "client-to-clinic",
  "unmasked": false
}
```

Export policy:

- masked by default
- unmasked requires `api.export.unmasked`
- every export is audited

---

## 4.13 Observability Health APIs

| Method | Path                  | Permission     |
| ------ | --------------------- | -------------- |
| GET    | `/api/health/metrics` | `api.ops.read` |
| GET    | `/api/health/alerts`  | `api.ops.read` |
| GET    | `/api/health/chaos`   | `api.ops.read` |

---

## 4.14 Operational Toggles (Ops)

| Method | Path                          | Permission      |
| ------ | ----------------------------- | --------------- |
| GET    | `/api/ops/flags`              | `api.ops.read`  |
| POST   | `/api/ops/flags/:key/enable`  | `api.ops.write` |
| POST   | `/api/ops/flags/:key/disable` | `api.ops.write` |

Supported keys are currently `exports_enabled` and `analytics_enabled`.

---

## 5) Permission Summary by Domain

Representative permission families used across endpoints:

- Org/Admin: `api.org.*`, `api.dept.*`, `api.project.*`
- User/RBAC: `api.users.*`, `api.roles.*`, `api.permissions.read`, `action.roles.assign`, `action.permissions.manage`, `action.scopes.manage`
- Catalog/Plans/Delivery: `api.catalog.*`, `api.plans.*`, `api.delivery.*`
- Billing/Payments: `api.billing.read`, `action.billing.generate`, `action.billing.approve`, `api.payments.read`, `action.payments.record`, `action.payments.refund`
- Scoring/Reports: `api.scoring.*`, `api.reports.read`, `action.reports.export`, `api.export.unmasked`
- Operations: `api.ops.read`, `api.ops.write`

---

## 6) Testing and Verification Sources

Endpoint behavior in this spec is corroborated by the implementation and test suites:

- `repo/API_tests/test_smoke.sh`
- `repo/API_tests/test_auth.sh`
- `repo/API_tests/test_catalog_delivery.sh`
- `repo/API_tests/test_billing.sh`
- `repo/API_tests/test_scoring.sh`
- `repo/API_tests/test_reports.sh`
- `repo/API_tests/test_ops.sh`

---

## 7) Notes

1. This is an **implementation-based** API spec for the current codebase, not a future-state contract.
2. Payload examples are representative shapes aligned with implemented domain behavior and tests.
3. For detailed business rules behind each endpoint family, see:
   - `repo/docs/security_model.md`
   - `repo/docs/catalog_and_delivery_workflows.md`
   - `repo/docs/billing_and_financial_controls.md`
   - `repo/docs/scoring_and_reporting.md`
   - `repo/docs/observability_and_resilience.md`
