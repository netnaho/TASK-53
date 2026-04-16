Project Type: fullstack

# CareOps Service Billing & Quality Management Portal

A locally-deployed portal for managing care service delivery, billing, and quality scoring. Built with Dioxus (Rust/WASM frontend), Rocket (Rust backend), and MySQL.

## Tech Stack

| Component | Technology | Version |
|-----------|-----------|---------|
| Frontend | Dioxus (Web/WASM) | 0.6 |
| Backend | Rocket | 0.5 |
| Database | MySQL | 8.0 |
| ORM/Migrations | sqlx | 0.7 |
| Auth | Argon2id + JWT | -- |
| Encryption | AES-256-GCM | -- |
| Container | Docker Compose | -- |

## Quick Start

```bash
docker-compose up
```

> **Also accepted:** `docker compose up` (Docker Compose V2 syntax — both commands start the full stack identically)

That's it. This single command:
1. Starts MySQL and waits for it to be healthy
2. Builds and starts the Rocket backend (runs migrations and seeds automatically)
3. Builds and serves the Dioxus frontend via nginx

## Exposed Ports

| Service | Port | URL |
|---------|------|-----|
| Frontend | 3000 | http://localhost:3000 |
| Backend API | 8000 | http://localhost:8000 |
| MySQL | 3306 | `mysql://careops_user:careops_pass@localhost:3306/careops` |

## Verification

After running `docker-compose up`, verify the system is working correctly:

### Step 1 — Health check

```bash
curl http://localhost:8000/api/health/live
# Expected: {"status":"ok"}  (HTTP 200)

curl http://localhost:8000/api/health/ready
# Expected: {"status":"ready"}  (HTTP 200)
```

### Step 2 — API login and token

```bash
curl -s -X POST http://localhost:8000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"Admin123!"}'
# Expected: JSON with "token" field and "user" object containing "permissions" array
```

Save the token for subsequent calls (uses only standard shell tools — no python3 needed):

```bash
TOKEN=$(curl -s -X POST http://localhost:8000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"Admin123!"}' \
  | grep -o '"token":"[^"]*"' | cut -d'"' -f4)
```

### Step 3 — Fetch protected data

```bash
curl -s -H "Authorization: Bearer $TOKEN" \
  http://localhost:8000/api/catalog/
# Expected: JSON array (may be empty on first run before test data is created)

curl -s -H "Authorization: Bearer $TOKEN" \
  http://localhost:8000/api/admin/org/
# Expected: JSON array with at least one org (the seeded "CareOps Demo" org)
```

### Step 4 — Unauthenticated request is rejected

```bash
curl -s -o /dev/null -w "%{http_code}" http://localhost:8000/api/catalog/
# Expected: 401
```

### Step 5 — Web UI verification

1. Open **http://localhost:3000** in your browser.
2. You should see the CareOps login page.
3. Log in with `admin` / `Admin123!`.
4. Expected outcome: redirect to the Dashboard showing the sidebar with all navigation items.
5. Navigate to **Admin → Organizations** — the seeded "CareOps Demo" org should appear.
6. Navigate to **Catalog** — service items are shown (or an empty state before test data creation).
7. Log out — you should be redirected back to the login page.

### Step 6 — Run all automated tests

```bash
bash run_tests.sh
# Expected: All test suites PASS — no local Rust or Python needed.
# Success message: "All test suites passed."
```

## Authentication

### Demo Accounts

All demo accounts are seeded automatically on first startup:

| Username | Password | Role | Permissions |
|----------|----------|------|-------------|
| `admin` | `Admin123!` | System Administrator | Full access |
| `ops_manager` | `OpsManager123!` | Operations Manager | Catalog, plans, delivery, billing, scoring |
| `billing_staff` | `Billing123!` | Billing Specialist | Billing, payments, reports |
| `coach` | `Coach123!` | Coach/Clinician | Plans (read), delivery (read/write) |
| `qa_reviewer` | `QAReview123!` | QA Reviewer | Delivery (read), scoring, reports |
| `auditor` | `Auditor123!` | Auditor | Audit logs, reports (read-only) |

### Auth Flow

1. `POST /api/auth/login` with `{"username": "...", "password": "..."}` returns JWT token + user profile
2. Include token as `Authorization: Bearer <token>` on all subsequent requests
3. `POST /api/auth/logout` to invalidate session
4. `GET /api/auth/me` to fetch current user profile with permissions

### Security Features

- Password hashing: Argon2id with random salts
- Account lockout: 5 failed attempts -> 15-minute lock
- Session revocation: logout immediately invalidates the JWT
- Permission cache: changes take effect within 30 seconds
- Encryption at rest: AES-256-GCM for sensitive fields
- Audit logging: all auth events and security-sensitive mutations
- CORS origin allowlist: explicit origins only (no wildcard with credentials). Configure via `CORS_ALLOWED_ORIGINS` env var (comma-separated). Defaults to localhost ports in local mode.
- Payment idempotency: 5-minute duplicate rejection window per `(org_id, idempotency_key)`. Same key within 5 minutes returns 409 Conflict; after 5 minutes the key is accepted for a new payment. Race-safe via `INSERT … ON DUPLICATE KEY UPDATE` on the `payment_idempotency_keys` table (see migration `20240108000000_restore_idempotency_window.sql` and [Billing & Financial Controls, Section 5](docs/billing_and_financial_controls.md#5-idempotency-model) for full details).

See [docs/security_model.md](docs/security_model.md) for the full security architecture.

## RBAC Summary

Six permission categories enforce access control at every layer:

| Category | Enforcement | Example |
|----------|-------------|---------|
| `menu.*` | Frontend sidebar visibility | `menu.dashboard`, `menu.admin` |
| `action.*` | Button/action visibility + backend check | `action.users.create` |
| `api.*` | Backend route authorization | `api.users.read`, `api.billing.write` |
| Data scope | Backend org/dept/project filtering | `user_data_scopes` table |

Frontend permission checks are advisory. Backend enforcement is mandatory.

## Observability & Resilience

### Startup

```bash
docker compose up                          # default: chaos drills off
CHAOS_ENABLED=true docker compose up       # arm chaos drills (Sunday 02:00 UTC only)
```

### Degradation toggles

Two in-process toggles persist in the `ops_config` DB table and can be flipped without restart:

| Toggle key | Default | Effect when disabled |
|---|---|---|
| `exports_enabled` | `true` | Export endpoint returns 503 |
| `analytics_enabled` | `true` | All report endpoints return 503 |

**Safety:** Toggle parsing is fail-closed. If a toggle value in the database is missing or malformed (not `"true"`/`"false"`), it defaults to `false` (feature disabled) and emits a warning log.

Toggle via `POST /api/ops/flags/:key/enable|disable` (requires `api.ops.write`, System Administrator only).
All changes are written to `ops_events` and `audit_log`.

### Error-rate alarm

A background task evaluates the 10-minute sliding window error rate every 30 seconds.
When the rate exceeds **2%**, the alarm transitions to ALERTING and an `ops_events` row is written.
Visible at `GET /api/health/alerts` and `GET /api/health/metrics`.

### Chaos drills

Weekly, Sunday 02:00–02:15 UTC. Guarded by `CHAOS_ENABLED=true` env var (off by default).
Simulates 200 ms DB latency and 5% request timeouts — bounded, time-limited faults.
See [docs/observability_and_resilience.md](docs/observability_and_resilience.md) for full details.

## Migrations & Seeds

- **Migrations** run automatically when the backend starts (embedded via sqlx)
- **Seeds** are idempotent: checked via `_seed_history` table
- Seeds include: permissions, roles with permission mappings, demo org, demo users
- No manual database setup is required

## Repository Structure

```
repo/
├── docker-compose.yml
├── README.md
├── run_tests.sh
├── .gitignore / .dockerignore
│
├── backend/
│   ├── Cargo.toml
│   ├── Dockerfile
│   ├── migrations/
│   │   ├── 20240101000000_initial_schema.sql
│   │   └── 20240102000000_security_rbac_audit.sql
│   └── src/
│       ├── main.rs
│       ├── bootstrap/          # Rocket wiring, service initialization
│       ├── config/             # Environment configuration
│       ├── api/
│       │   ├── guards/         # JWT auth guard, permission/scope checkers
│       │   ├── auth/           # Login, logout, current user
│       │   ├── admin_org/      # Org, department, project CRUD
│       │   ├── users_roles_permissions/  # User, role, permission, scope CRUD
│       │   ├── audit_api/      # Audit log querying
│       │   ├── service_catalog/
│       │   ├── packages/
│       │   ├── client_plans/
│       │   ├── delivery_entries/
│       │   ├── billing/
│       │   ├── payments_refunds/
│       │   ├── scoring_reviews/
│       │   ├── reports_exports/
│       │   ├── observability/  # Health probes (public), metrics/alerts/chaos (api.ops.read)
│       │   ├── ops/            # Degradation toggle controls (api.ops.read/write)
│       │   └── tracing_fairing.rs
│       ├── application/        # Service layer
│       │   ├── auth_service.rs
│       │   ├── user_service.rs
│       │   ├── role_service.rs
│       │   ├── org_service.rs
│       │   ├── seed_service.rs
│       │   ├── metrics_service.rs
│       │   ├── alert_engine.rs
│       │   ├── degradation_service.rs
│       │   └── chaos_service.rs
│       ├── domain/
│       │   ├── error.rs        # Typed errors -> HTTP status + JSON envelope
│       │   ├── auth_types.rs   # JWT claims, request/response types
│       │   └── auth_policy.rs  # Permission codes, role-permission matrix
│       └── infrastructure/
│           ├── database/       # Pool, migrations
│           ├── logging/        # Structured JSON logging
│           ├── encryption/     # AES-256-GCM encrypt/decrypt/mask
│           ├── audit/          # Immutable audit log service
│           └── permission_cache/  # In-process cache with version invalidation
│
├── frontend/
│   ├── Cargo.toml
│   ├── Dockerfile / Dioxus.toml / nginx.conf
│   ├── assets/main.css
│   └── src/
│       ├── main.rs / app.rs / router.rs
│       ├── layouts/app_layout.rs
│       ├── components/         # Sidebar (permission-aware), topbar, state components
│       ├── pages/
│       │   ├── login/          # Real auth form with demo credentials
│       │   ├── dashboard/
│       │   ├── admin/          # Org/dept/role management
│       │   ├── users/          # User listing and management
│       │   ├── audit/          # Live audit log viewer
│       │   └── ...             # Other domain pages
│       ├── services/api_client.rs  # HTTP client with auth header injection
│       ├── state/mod.rs            # AuthState with permission checking
│       └── models/mod.rs           # Shared API types
│
├── docs/
│   ├── architecture.md
│   ├── security_model.md
│   ├── catalog_and_delivery_workflows.md
│   └── requirements_traceability.md
│
├── unit_tests/backend/
│   ├── test_health.sh
│   └── test_password_hashing.sh
│
└── API_tests/
    ├── test_smoke.sh
    ├── test_auth.sh
    └── test_catalog_delivery.sh
```

## API Endpoints

### Public (no auth)

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/api/health/live` | Liveness check |
| GET | `/api/health/ready` | Readiness check (DB connectivity) |
| POST | `/api/auth/login` | Authenticate and get JWT |

### Protected (require Bearer token)

| Method | Path | Permission | Status |
|--------|------|-----------|--------|
| POST | `/api/auth/logout` | (authenticated) | Implemented |
| GET | `/api/auth/me` | (authenticated) | Implemented |
| GET/POST | `/api/admin/org/` | `api.org.read/write` | Implemented |
| GET/POST | `/api/admin/org/:id/departments` | `api.dept.read/write` | Implemented |
| GET/POST | `/api/admin/org/:id/projects` | `api.project.read/write` | Implemented |
| GET/POST/PUT | `/api/users/` | `api.users.read/write` | Implemented |
| POST/DELETE | `/api/users/:id/roles` | `action.roles.assign` | Implemented |
| GET/POST/DELETE | `/api/users/:id/scopes` | `action.scopes.manage` | Implemented |
| GET/POST | `/api/roles/` | `api.roles.read/write` | Implemented |
| GET/POST/DELETE | `/api/roles/:id/permissions` | `action.permissions.manage` | Implemented |
| GET | `/api/roles/all` | `api.permissions.read` | Implemented |
| GET | `/api/audit/` | `api.audit.read` | Implemented |
| GET/POST/PUT | `/api/catalog/` | `api.catalog.read/write` | Implemented |
| GET/POST/PUT | `/api/packages/` | `api.catalog.read/write` | Implemented |
| GET | `/api/packages/:id/rules` | `api.catalog.read` | Implemented |
| GET/POST/PUT | `/api/plans/` | `api.plans.read/write` | Implemented |
| POST/GET | `/api/plans/:id/packages` | `api.plans.write/read` | Implemented |
| GET/POST/PUT | `/api/delivery/` | `api.delivery.read/write` | Implemented |
| GET/POST | `/api/delivery/:id/notes` | `api.delivery.read/write` | Implemented |
| POST | `/api/billing/charges/generate` | `action.billing.generate` | Implemented |
| GET | `/api/billing/charges` | `api.billing.read` | Implemented |
| GET | `/api/billing/charges/:id` | `api.billing.read` | Implemented |
| POST | `/api/billing/charges/:id/adjustments` | `action.billing.generate` | Implemented |
| POST | `/api/billing/invoices/generate` | `action.billing.generate` | Implemented |
| GET | `/api/billing/invoices` | `api.billing.read` | Implemented |
| GET | `/api/billing/invoices/:id` | `api.billing.read` | Implemented |
| PUT | `/api/billing/invoices/:id/status` | `action.billing.approve` | Implemented |
| GET | `/api/payments/reason-codes` | `api.payments.read` | Implemented |
| POST | `/api/payments/` | `action.payments.record` | Implemented |
| GET | `/api/payments/` | `api.payments.read` | Implemented |
| GET | `/api/payments/:id` | `api.payments.read` | Implemented |
| POST | `/api/payments/refunds` | `action.payments.refund` | Implemented |
| GET | `/api/payments/refunds` | `api.payments.read` | Implemented |
| GET | `/api/payments/refunds/:id` | `api.payments.read` | Implemented |
| GET | `/api/payments/transactions` | `api.payments.read` | Implemented |
| POST | `/api/payments/reconciliation` | `api.billing.read` | Implemented |
| GET | `/api/payments/reconciliation` | `api.billing.read` | Implemented |
| GET | `/api/payments/reconciliation/:id` | `api.billing.read` | Implemented |
| POST | `/api/scoring/templates` | `api.scoring.write` | Implemented |
| GET | `/api/scoring/templates` | `api.scoring.read` | Implemented |
| GET | `/api/scoring/templates/:id` | `api.scoring.read` | Implemented |
| POST | `/api/scoring/evaluations` | `api.scoring.write` | Implemented |
| GET | `/api/scoring/evaluations` | `api.scoring.read` | Implemented |
| GET | `/api/scoring/evaluations/:id` | `api.scoring.read` | Implemented |
| POST | `/api/scoring/evaluations/:id/submit` | `api.scoring.write` | Implemented |
| GET | `/api/scoring/reviews/pending` | `api.scoring.read` | Implemented |
| POST | `/api/scoring/reviews/:eval_id` | `api.scoring.write` | Implemented |
| GET | `/api/reports/kpi` | `api.reports.read` | Implemented |
| GET | `/api/reports/order-volume` | `api.reports.read` | Implemented |
| GET | `/api/reports/revenue` | `api.reports.read` | Implemented |
| GET | `/api/reports/utilization` | `api.reports.read` | Implemented |
| POST | `/api/reports/export` | `action.reports.export` | Implemented |
| GET | `/api/health/metrics` | `api.ops.read` | Implemented |
| GET | `/api/health/alerts` | `api.ops.read` | Implemented |
| GET | `/api/health/chaos` | `api.ops.read` | Implemented |
| GET | `/api/ops/flags` | `api.ops.read` | Implemented |
| POST | `/api/ops/flags/:key/enable` | `api.ops.write` | Implemented |
| POST | `/api/ops/flags/:key/disable` | `api.ops.write` | Implemented |

### Error Format

All errors return a consistent JSON envelope:
```json
{"error": {"code": "FORBIDDEN", "message": "Missing required permission: api.users.write", "trace_id": "..."}}
```

## Running Tests

**Requires only:** Docker (with `docker compose` v2 plugin) and `curl`. No local Rust, Python, or Node installation needed.

### What to run

```bash
# 1. Start the application stack
docker-compose up -d

# 2. Run all test suites (fully Docker-contained — no local Rust/Python needed)
bash run_tests.sh
```

`run_tests.sh` waits for the backend to be ready, then executes every suite inside
Docker containers. The only host-side dependency is `docker`, `docker compose`, and `curl`.

### How tests are containerized

| Steps | How it runs |
|-------|-------------|
| `[1]` Backend unit tests | `docker compose --profile test run backend-unit-tests` (rust:1.88-bookworm image) |
| `[2]` Frontend unit tests | `docker compose --profile test run frontend-unit-tests` (rust:1.88-bookworm image) |
| `[4]`–`[13]` API test suites | `docker compose --profile test run api-test-runner` (python:3.12-slim + curl image) |

No `cargo`, `python3`, or other runtime tools are required on the host.

### What success looks like

```
[1/13] Backend Unit Tests (cargo test --lib — in Docker)   PASS
[2/13] Frontend Unit Tests (cargo test --lib — in Docker)  PASS
[3/13] Security Unit Tests (in Docker)                     PASS
[4/13] Health Endpoint Unit Test                           PASS
[5/13] API Smoke Tests                                     PASS
[6/13] API Auth & Security Tests                           PASS
[7/13] API Catalog & Delivery Tests                        PASS
[8/13] API Billing Engine Tests                            PASS
[9/13] API Scoring & Review Tests                          PASS
[10/13] API Reports & Exports Tests                        PASS
[11/13] API Ops Controls Tests                             PASS
[12/13] API Gap Coverage Tests (27 endpoints)              PASS
[13/13] FE↔BE E2E Integration Test                        PASS
All test suites passed.
Coverage:
  API endpoints:  90/90 (100%)
  Backend units:  cargo test --lib
  Frontend units: cargo test --lib  (state, models, url_utils, features)
  E2E:            login → profile → catalog → write → read-back
```

### Troubleshooting common failures

| Symptom | Likely cause | Fix |
|---------|-------------|-----|
| `Backend did not become ready` | Container still starting | Wait 30 s and re-run |
| `401` on API tests | Token not obtained at login | Check `docker compose logs backend` for seed errors |
| `Connection refused` | Stack not started | Run `docker-compose up -d` first |
| Docker image build fails | Network issue fetching base images | Check `docker compose build --no-cache` |
| `api-test-runner` not found | Profile not built yet | Run `docker compose --profile test build api-test-runner` |

### Test suites

1. **Backend unit tests** — auth, billing types, catalog validation, scoring formulas, payment methods, metrics, alert threshold, degradation flags, chaos guardrails, all controller request/response types and error mapping
2. **Frontend unit tests** — `AuthState` permission logic + sidebar/topbar display helpers, model type serialization, URL construction, login validation, api_client contracts
3. **Security unit tests** — password hashing, encryption
4. **Health smoke test** — liveness probe
5. **API smoke tests** — auth boundaries across all endpoint groups
6. **Auth & security** — login/logout, token validity, role/scope boundary enforcement, cross-org isolation
7. **Catalog & delivery** — service CRUD, package rules, plan assignment, delivery entry validation
8. **Billing engine** — charge generation, adjustments, invoice lifecycle, payments, refunds, idempotency, reconciliation
9. **Scoring & review** — template creation, evaluation lifecycle, auto/manual/partial scoring, second-review enforcement
10. **Reports & exports** — KPI, order volume, revenue, utilization, masking, unmasked permission gating
11. **Ops controls** — health probes, metrics, alert state, chaos status, toggle enable/disable, 503 degradation, unknown flag rejection
12. **Gap coverage** — all 27 previously uncovered endpoints now have direct HTTP tests with request + response body assertions
13. **E2E integration** — login → profile fetch → catalog read → delivery write → read-back verification (proves FE↔BE integration)

## Documentation

- [Architecture](docs/architecture.md) - System design, data flow, module structure
- [Security Model](docs/security_model.md) - Auth, RBAC, data scope, encryption, audit
- [Catalog & Delivery Workflows](docs/catalog_and_delivery_workflows.md) - Service catalog, package rules, plan assignment, delivery validation
- [Billing & Financial Controls](docs/billing_and_financial_controls.md) - Charge generation, invoice lifecycle, idempotency model, refund controls, immutable fund ledger, reconciliation
- [Scoring & Reporting](docs/scoring_and_reporting.md) - Scoring formulas, second-review rule, report definitions, export masking rules, KPI definitions
- [Observability & Resilience](docs/observability_and_resilience.md) - Structured logs, in-process metrics, 2% error-rate alarm, degradation toggles, chaos drill guardrails, ops_events log
- [Requirements Traceability](docs/requirements_traceability.md) - Requirement-to-code mapping
