# API Tests

Integration tests that verify API endpoints against a running backend with seeded data.

## Prerequisites

The full stack must be running with default seed data:
```bash
docker compose up
```

The tests depend on:
- **Seeded demo users**: `admin`, `ops_manager`, `billing_staff`, `coach`, `qa_reviewer`, `auditor` (created by `seed_service.rs` on first startup)
- **Seeded RBAC**: Six default roles with permission assignments
- **Seeded demo org**: "CareOps Demo Org" with departments
- **MySQL available**: Tests create transient fixture data (orgs, users) via the admin API

## Running

All test suites run inside Docker containers via `run_tests.sh` — no local `python3`, `cargo`, or
other runtime tools are required on the host.

```bash
# Run all suites (unit + API) via the project test runner (fully Docker-contained)
./run_tests.sh

# Run an individual suite inside the api-test-runner container
docker compose --profile test run --rm api-test-runner bash API_tests/test_auth.sh
docker compose --profile test run --rm api-test-runner bash API_tests/test_billing.sh
docker compose --profile test run --rm api-test-runner bash API_tests/test_scoring.sh

# Against a custom backend URL (inside container, backend service name resolves automatically)
docker compose --profile test run --rm -e BACKEND_URL=http://backend:8000 \
  api-test-runner bash API_tests/test_auth.sh
```

## Test Suites

### `test_smoke.sh`
Verifies all registered routes respond with expected HTTP status codes (401 for protected, 200 for public).

### `test_auth.sh`
Authentication, authorization, and tenant isolation:
- Login (valid/invalid/nonexistent), logout, session revocation
- Permission enforcement (auditor cannot write, coach blocked from admin)
- Target-user validation (404 for nonexistent users)
- **Cross-org tenant isolation** (tests 22-26): Creates a second org and user via the admin API, then proves a scoped Operations Manager actor in org-A cannot read or manage a user in org-B (403 from data-scope check). Includes a positive control (same-org read → 200).

### `test_catalog_delivery.sh`
Service catalog CRUD, package rules, client plan lifecycle, delivery entry creation and verification, scope enforcement.

### `test_billing.sh`
Charge generation, adjustments, invoice lifecycle, payment recording with idempotency, refund cap enforcement, reconciliation, data-scope enforcement for billing endpoints.

### `test_scoring.sh`
Template creation, evaluation lifecycle, auto/manual/partial scoring, second-review enforcement, independent reviewer validation.

### `test_reports.sh`
KPI, order volume, revenue, utilization reports. Export masking defaults, unmasked gating, cross-project scope enforcement. **Delivery export regression gate** (test 17): explicit HTTP 200 check catches SQL table-name mismatches that produce 500 errors.

### `test_ops.sh`
Health probes, metrics, alert state, chaos drill status, degradation toggle enable/disable, exports-disabled/analytics-disabled 503 behavior.

## Fixture Strategy

Tests that need data beyond the seed set create it inline via admin API calls (e.g., creating a second org for cross-org tests). This keeps tests self-contained and deterministic without requiring custom database scripts. Fixture creation failures are reported as test failures, not silent skips.

## Error Handling

- `set -euo pipefail` — scripts abort on unexpected errors
- `curl -sf` — HTTP 4xx/5xx cause empty response, handled by `|| echo "FAIL"` fallback
- Fixture failures in setup blocks increment `FAILED` counter with specific diagnostic messages
- Silent `SKIP` only appears for the initial login test where the token is used by all later tests
