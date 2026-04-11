# CareOps Architecture

## System Overview

CareOps is a locally-deployed service billing and quality management portal for care operations organizations. The system runs entirely within Docker Compose with no external cloud dependencies.

```
┌─────────────┐     ┌──────────────┐     ┌──────────┐
│   Browser    │────▶│   Frontend   │────▶│ Backend  │────▶│  MySQL   │
│              │     │  (Dioxus/    │     │ (Rocket) │     │  (8.0)   │
│              │     │   WASM)      │     │          │     │          │
│              │     │  :3000       │     │  :8000   │     │  :3306   │
└─────────────┘     └──────────────┘     └──────────┘     └──────────┘
                     nginx proxy          REST API         persistent
                     /api/* -> backend                     volume
```

## Bounded Contexts / Domain Modules

| Module | Responsibility | Backend Path | Frontend Route |
|--------|---------------|-------------|----------------|
| **Auth** | Login, logout, session, current user | `/api/auth` | `/login` |
| **Admin / Org** | Organization settings and configuration | `/api/admin/org` | `/admin` |
| **Users, Roles, Permissions** | User CRUD, role assignment, RBAC | `/api/users` | `/admin` |
| **Service Catalog** | Define billable services and rates | `/api/catalog` | `/catalog` |
| **Packages** | Group services into reusable bundles | `/api/packages` | `/catalog` |
| **Client Plans** | Assign packages to clients with date ranges | `/api/plans` | `/plans` |
| **Delivery Entries** | Log and verify service delivery units | `/api/delivery` | `/delivery` |
| **Billing** | Generate invoices from verified deliveries | `/api/billing` | `/billing` |
| **Payments & Refunds** | Record payments, process refunds | `/api/payments` | `/billing` |
| **Scoring & Reviews** | Quality score delivery entries | `/api/scoring` | `/scoring` |
| **Reports & Exports** | Generate and export operational reports | `/api/reports` | `/reports` |
| **Observability** | Health checks, readiness, metrics | `/api/health` | -- |

## Data Flow

1. **User authenticates** via `/api/auth/login` receiving a session token
2. **Admin configures** the organization, service catalog, and packages
3. **Care managers create** client plans by assigning packages
4. **Providers log** delivery entries against active plans
5. **Supervisors verify** delivery entries
6. **Billing staff generates** invoices from verified entries
7. **Payments are recorded** against invoices
8. **Quality reviewers score** delivery entries
9. **Reports are generated** for operational and compliance needs

## Backend Architecture (Layered)

```
src/
├── api/              # HTTP handlers, request/response types, route registration
│   ├── auth/
│   ├── admin_org/
│   ├── billing/
│   ├── ... (one module per domain)
│   ├── observability/
│   └── tracing_fairing.rs
├── application/      # Service layer: orchestrates domain + infrastructure
├── domain/           # Core business types, error envelope, auth policy
│   ├── error.rs      # Typed AppError -> HTTP status + JSON envelope
│   └── auth_policy.rs # RBAC roles, permissions, policy enforcement
├── infrastructure/   # External concerns: database, logging
│   ├── database/     # Connection pool, migrations, seeds
│   └── logging/      # Structured JSON logging with tracing
├── config/           # Environment-based configuration
├── bootstrap/        # Rocket builder: wires all modules together
└── main.rs           # Entry point
```

### Error Envelope

All API errors return a consistent JSON structure:

```json
{
  "error": {
    "code": "NOT_FOUND",
    "message": "Resource not found",
    "trace_id": "550e8400-e29b-41d4-a716-446655440000"
  }
}
```

### Request Tracing

Every request receives an `X-Trace-Id` header (generated or propagated from the client). This ID appears in structured log output for request correlation.

## Frontend Architecture

```
src/
├── app.rs            # Root component with Router
├── router.rs         # Route definitions mapping paths to pages
├── main.rs           # WASM entry point
├── layouts/          # Page layout shells (sidebar + topbar + content)
├── components/       # Reusable UI components
│   ├── sidebar.rs
│   ├── topbar.rs
│   ├── loading.rs
│   ├── empty_state.rs
│   ├── error_state.rs
│   ├── permission_denied.rs
│   └── validation_message.rs
├── pages/            # Route-level page components (one per domain)
├── features/         # Domain-specific feature logic
├── services/         # API client and data fetching
├── state/            # Global application state
└── models/           # Shared data types
```

## Authorization Architecture (Implemented)

Authorization uses a fine-grained RBAC system with 55+ permission codes across 4 categories:

- **Roles**: System Administrator, Operations Manager, Billing Specialist, Coach/Clinician, QA Reviewer, Auditor
- **Permission categories**: `menu.*` (navigation), `action.*` (buttons), `api.*` (endpoints), data scope
- **Enforcement points**:
  - Backend: `AuthenticatedUser` guard rejects unauthenticated requests (401)
  - Backend: `require_permission()` checks permission codes (403)
  - Backend: `require_data_scope()` checks org/department/project access (403)
  - Frontend: Sidebar links conditionally render based on `menu.*` permissions
  - Frontend: Action buttons check `action.*` permissions
  - Database: `user_data_scopes` table restricts data access by org/dept/project

The auth policy module (`domain/auth_policy.rs`) defines all permission codes and the role-permission matrix. The permission cache (`infrastructure/permission_cache/`) ensures changes take effect within 30 seconds.

See [docs/security_model.md](security_model.md) for the complete security architecture.

## Offline / Local-Network Assumptions

- The entire system runs on a single machine or local network via Docker Compose
- No external auth providers (OAuth, SSO) - authentication is local username/password
- No external payment gateways - payments are recorded manually
- No cloud storage - all data persists in the MySQL volume
- No external API calls - the system is fully self-contained

## Database

- MySQL 8.0 with InnoDB engine
- UTF-8 (utf8mb4) character set
- Migrations managed via sqlx (embedded, run at startup)
- Schema follows normalized relational design with foreign keys
- Multi-tenant isolation via `org_id` on all business tables
