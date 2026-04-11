# CareOps Design Document (Implementation-Based)

## 1. Purpose and Scope

This document describes the **implemented design** of the CareOps Service Billing & Quality Management Portal in `repo/`, including architecture, domain workflows, data model boundaries, security controls, and operational resilience.

Scope covered:

- Local-network deployment model (offline-first)
- Frontend and backend architecture
- Business workflows (catalog → plan → delivery → billing → payment/refund → scoring → reporting)
- RBAC, data-scope, encryption, and audit controls
- Observability, degradation toggles, and chaos drill controls

Out of scope:

- Future roadmap items not reflected in the current codebase
- External integrations (none are required for this implementation)

---

## 2. Product Context and Design Drivers

CareOps is designed for providers operating entirely inside a local network, with strict compliance and auditability expectations.

Primary design drivers:

1. **Offline self-sufficiency**: No internet dependency for runtime-critical workflows.
2. **Strong authorization boundaries**: Role and data-scope checks enforced at API level.
3. **Financial immutability**: Payment/refund trails preserved as append-only records.
4. **Operational resilience**: Built-in health probes, alerts, degradation toggles, and controlled chaos drills.
5. **Task-focused UX**: Dioxus screens map directly to operational roles and domain workflows.

---

## 3. High-Level System Architecture

```text
Browser
  -> Dioxus Frontend (WASM, served via nginx, :3000)
		-> Rocket Backend API (:8000)
			 -> MySQL 8.0 (:3306)
```

Deployment and startup are orchestrated via `docker-compose.yml` with automatic migration and seeding on backend startup.

### 3.1 Runtime Components

- **Frontend**: Rust + Dioxus web app, route-driven pages with permission-aware navigation and actions.
- **Backend**: Rocket REST-style API with layered module separation.
- **Database**: MySQL with SQLx-driven migrations and relational schema across security, care operations, billing, scoring, and ops domains.

### 3.2 Backend Layering

`backend/src/` is organized into:

- `api/`: HTTP handlers, request parsing, response shaping, route guards.
- `application/`: Service layer implementing business use-cases.
- `domain/`: Validation types, scoring/billing formulas, policy constants, typed errors.
- `infrastructure/`: Database, encryption, audit logging, permission cache, logging.
- `bootstrap/`: Service wiring, background workers, Rocket build/mount logic.
- `config/`: Environment-driven settings.

---

## 4. Frontend Design

## 4.1 Route Model

The current `frontend/src/router.rs` defines route-level screens:

- `/login`
- `/` (Dashboard)
- `/admin`
- `/users`
- `/catalog`
- `/plans`
- `/delivery`
- `/billing`
- `/scoring`
- `/reports`
- `/audit`
- `/ops`

All authenticated routes are wrapped by `AppLayout`, enabling shared navigation and role-aware page framing.

## 4.2 Permission-Aware UX

Frontend permission checks (menu and action level) are used for usability and discoverability:

- Sidebar and button visibility are conditioned on permission sets.
- Backend remains the authoritative enforcement point for all sensitive actions.

## 4.3 API Interaction Pattern

Frontend service/client code attaches Bearer tokens and consumes structured JSON responses from Rocket endpoints. Validation and error feedback are surfaced inline in page flows.

---

## 5. Core Domain Workflows (Implemented)

## 5.1 Identity and Access

1. User logs in with local username/password (`/api/auth/login`).
2. Backend verifies Argon2id hash, issues JWT + session record.
3. Protected routes require auth guard; permission + scope checks applied per API.
4. Logout revokes session (`/api/auth/logout`).

## 5.2 Catalog and Package Configuration

1. Ops roles create `service_catalog_items`.
2. Package definitions are created with rule types:
   - `per_visit`
   - `hourly` (quarter-hour constraints)
   - `tiered` (validated tier config)
3. Rule validation ensures billing computability and data consistency.

## 5.3 Client Plan and Delivery Capture

1. Plans are created and package assignments are attached.
2. Delivery entries are captured against active plan-package relationships.
3. Domain validation enforces:
   - quarter-hour increments
   - mileage cap (200 miles max)
   - positive/non-negative constraints
   - plan/package/service linkage correctness
4. Eligibility notes are tracked and delivery statuses progress through operational states.

## 5.4 Billing and Financial Controls

1. Verified deliveries generate charges (idempotent batch logic).
2. Charge adjustments are additive and immutable.
3. Invoices snapshot line items and enforce status transitions.
4. Recorded payments require idempotency keys with 5-minute duplicate rejection.
5. Refunds require reason codes and enforce net-paid cap.
6. Fund transactions form immutable payment/refund ledger entries.
7. Reconciliation runs produce immutable period summaries.

## 5.5 Quality Scoring and Second Review

1. Templates define objective/subjective questions, weights, and rounding interval.
2. Submission computes auto score, manual adjustments, partial credit, weighted score.
3. If score delta is >10 points vs prior finalized score, second review is required.
4. QA review approves or revises before finalization.

## 5.6 Reports and Exports

Implemented report surfaces include KPI summary, order volume, revenue, and utilization.

Filtering dimensions include date range and optional route/scope filters. Exports are:

- masked by default
- unmasked only with specific permission
- always audit logged

---

## 6. Data Model Design

## 6.1 Security and Access Tables

- `users`, `user_credentials`, `sessions`
- `roles`, `permissions`, `role_permissions`, `user_roles`
- `user_data_scopes`
- `permission_version`
- `audit_logs`

## 6.2 Operations and Care Domain Tables

- `organizations`, `departments`, `projects`
- `service_catalog_items`
- `package_definitions`, `package_rule_definitions`
- `client_plans`, `client_plan_packages`
- `delivery_entries`, `eligibility_notes`

## 6.3 Billing and Finance Tables

- `charges`, `charge_adjustments`
- `invoices`, `invoice_line_items`
- `recorded_payments`, `recorded_refunds`
- `refund_reason_codes`
- `fund_transactions`
- `reconciliation_runs`
- `payment_idempotency_keys` (idempotency window behavior)

## 6.4 Scoring and Reporting Tables

- `scoring_templates`, `evaluation_questions`
- `evaluations`, `evaluation_answers`
- `score_reviews`
- `export_audit_logs`

## 6.5 Ops/Resilience Tables

- `ops_config` (degradation flags)
- `ops_events` (toggle/alarm/chaos events)

---

## 7. Security Design

## 7.1 Authentication and Session Security

- Password hashing: Argon2id with salted hashes.
- Session model: JWT + server-side revocation tracking.
- Account lockout: failed-attempt guardrails.

## 7.2 Authorization Model

Fine-grained permission families:

- `menu.*`
- `action.*`
- `api.*`

Data-scope constraints are enforced by org/department/project access logic. Backend guards (`AuthenticatedUser`, `require_permission`, `require_data_scope`) are mandatory gates for protected operations.

## 7.3 Permission Cache Consistency

In-process permission cache combines TTL (capped at 30s), version checks, and invalidation on permission/scope mutations to meet propagation requirements while reducing DB load.

## 7.4 Data Protection and Compliance

- Sensitive fields encrypted at rest with AES-256-GCM.
- Export masking enabled by default.
- Immutable audit trail for auth, role, org, financial, and operational changes.

---

## 8. Observability and Resilience Design

## 8.1 Logging and Tracing

- Structured tracing-based logs.
- Request/response instrumentation via Rocket fairing.
- Correlation-friendly trace identifiers.

## 8.2 Metrics and Alerting

- In-process metrics service maintains 10-minute sliding window.
- Alert rule: error rate > 2% over window transitions alarm to ALERTING.
- Background evaluator runs every 30 seconds.

## 8.3 Health Endpoints

- Public liveness/readiness probes.
- Auth-protected metrics/alerts/chaos status endpoints.

## 8.4 Degradation Toggles

Runtime flags support controlled feature shedding:

- `exports_enabled`
- `analytics_enabled`

Flag changes are persisted, audited, and surfaced operationally.

## 8.5 Chaos Drill Framework

Controlled fault injection is guarded by:

1. explicit environment arming
2. strict schedule window
3. bounded fault intensity

This validates resilience behavior without external dependencies.

---

## 9. API Design Principles

1. **REST-style domain grouping** by module (`/api/auth`, `/api/billing`, `/api/scoring`, etc.).
2. **Consistent error envelope** with machine-readable code and trace context.
3. **Permission-first handler flow**: authenticate → authorize → validate → execute.
4. **Idempotency and immutability** for financial side effects.
5. **Auditability by default** for security and compliance-sensitive operations.

---

## 10. Deployment and Environment Design

## 10.1 Local Compose Topology

`docker compose up` starts:

- MySQL
- Rocket backend (migrations + seed)
- Dioxus frontend served by nginx

## 10.2 Configuration Sources

Environment-driven settings include DB URL, JWT secret, encryption key, session TTL, and chaos arming options.

## 10.3 Operational Assumptions

- Primarily trusted local network environment
- No internet required for primary workflows
- Backup/restore and secret-hardening are environment responsibilities outside app runtime logic

---

## 11. Key Design Tradeoffs

1. **In-process metrics/alerts vs external stack**
   - Chosen for offline simplicity and reduced ops burden.
   - Tradeoff: less long-term observability depth than full external telemetry platforms.

2. **JWT + session revocation hybrid**
   - Balances stateless request auth with explicit logout control.

3. **Strict immutability in finance/audit flows**
   - Supports compliance and forensic traceability.
   - Tradeoff: corrections are additive, requiring reconciliation-aware reporting logic.

4. **Permission cache with bounded staleness**
   - Reduces database pressure while meeting <30s propagation requirement.

---

## 12. Validation Evidence from Current Repo

The design in this document is aligned with current implementation artifacts in:

- `repo/README.md`
- `repo/docs/architecture.md`
- `repo/docs/security_model.md`
- `repo/docs/catalog_and_delivery_workflows.md`
- `repo/docs/billing_and_financial_controls.md`
- `repo/docs/scoring_and_reporting.md`
- `repo/docs/observability_and_resilience.md`
- `repo/docs/requirements_traceability.md`

---

## 13. Summary

The implemented CareOps design is a modular, offline-capable, role-secured operations platform that couples care-service workflows with strict financial controls and quality-review governance. The architecture emphasizes local reliability, clear authorization boundaries, immutable compliance trails, and built-in operational safety mechanisms.
