# Audit Report 2 — Fix Verification Check

Date: 2026-04-11  
Method: Static code/docs/tests inspection (no runtime execution in this pass)

## Overall Disposition

- **All previously reported issues from `.tmp/audit_report-2.md` appear fixed** based on current static evidence.
- No remaining open findings were identified in this recheck.

## Issue-by-Issue Status

### 1) Blocker — Deliveries export SQL mismatch (`service_items` vs `service_catalog_items`)

**Status:** ✅ **Fixed**

**Evidence:**

- `repo/backend/src/application/export_service.rs`
  - Deliveries export query uses `JOIN service_catalog_items si ON si.id = de.service_item_id`.
- `repo/API_tests/test_reports.sh`
  - Regression gate present: `Deliveries export HTTP 200 (not 500 — SQL join is valid)`.

**Conclusion:** The table-name mismatch that previously caused delivery export failure is addressed and guarded by dedicated regression assertions.

---

### 2) High — Missing service-route reporting dimension

**Status:** ✅ **Fixed**

**Evidence:**

- Schema:
  - `repo/backend/migrations/20240109000000_add_service_route.sql` exists.
- Domain/API contract:
  - `repo/backend/src/domain/scoring_types.rs` includes `service_route` in report/export filters.
  - `repo/backend/src/api/reports_exports/mod.rs` exposes `service_route` query parameters across report endpoints.
- Service/query layer:
  - `repo/backend/src/application/report_service.rs` validates `service_route` and applies route clauses/binds.
- Test coverage:
  - `repo/API_tests/test_reports.sh` includes dedicated `service_route` tests `[20]..[25]` (with/without route, validation, export behavior).
- Documentation:
  - `repo/docs/scoring_and_reporting.md` documents `service_route` semantics and examples.

**Conclusion:** Route dimension is now implemented and traceable end-to-end.

---

### 3) Medium — Idempotency documentation drift (stated 1h vs implemented 5m)

**Status:** ✅ **Fixed**

**Evidence:**

- `repo/README.md`
  - Security section now states a **5-minute duplicate rejection window** and references `payment_idempotency_keys` semantics.

**Conclusion:** Documentation now matches the implemented idempotency model.

---

### 4) Medium — Tenant isolation test depth (cross-org denial scenarios)

**Status:** ✅ **Fixed**

**Evidence:**

- `repo/API_tests/test_auth.sh`
  - Includes deterministic cross-org isolation section (`tests 22-26`) with explicit 403 checks for cross-org access operations.

**Conclusion:** Cross-org negative-path coverage is now explicit and deterministic in auth API tests.

---

### 5) Low — Delivery UI manual internal ID entry (`Plan Package ID`, `Service Item ID`)

**Status:** ✅ **Fixed**

**Evidence:**

- `repo/frontend/src/pages/delivery/mod.rs`
  - Delivery form now uses dependent selectors:
    - Plan selector
    - Package selector (depends on plan)
    - Service selector (depends on package)
  - File comments and implementation explicitly state/select flow: `Plan → Package → Service Item`.
  - Validation now requires these selections while preserving payload IDs (`plan_package_id`, `service_item_id`) on submit.
  - No manual text inputs with labels `Plan Package ID *` / `Service Item ID *` remain.

**Conclusion:** The usability gap from manual internal-ID entry has been removed.

## Final Matrix

- **Fixed:** #1 Export SQL mismatch, #2 Service-route dimension, #3 Idempotency docs drift, #4 Tenant-isolation test depth, #5 Delivery UI manual-ID ergonomics
- **Open:** None identified in this static recheck

## Confidence and Limits

- Confidence is high for static alignment across code/docs/test intent.
- This pass did **not** execute runtime/API tests; runtime verification can be added if you want an execution-backed closure report.
