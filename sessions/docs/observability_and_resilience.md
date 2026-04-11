# Observability & Resilience — CareOps Backend

This document describes the structured logging, in-process metrics, alert system, degradation toggles, chaos drill framework, and operational event log introduced in Phase 6.

---

## Table of Contents

1. [Structured Logs & Tracing](#1-structured-logs--tracing)
2. [In-Process Metrics](#2-in-process-metrics)
3. [Alert Engine — 2% Error Rate Rule](#3-alert-engine--2-error-rate-rule)
4. [Health Endpoints](#4-health-endpoints)
5. [Degradation Toggles](#5-degradation-toggles)
6. [Chaos Drill Framework](#6-chaos-drill-framework)
7. [ops_events Table](#7-ops_events-table)
8. [Role & Permission Matrix](#8-role--permission-matrix)
9. [Unit Tests](#9-unit-tests)

---

## 1. Structured Logs & Tracing

CareOps uses the [`tracing`](https://docs.rs/tracing) crate for structured, leveled logging. All log entries are emitted as key-value pairs.

### TracingFairing

`backend/src/api/tracing_fairing.rs` is a Rocket fairing that instruments every HTTP request/response:

- **`on_request`**: emits `tracing::info!` with `method`, `uri`.
- **`on_response`**: emits `tracing::info!` with `method`, `uri`, `status`; also calls `MetricsService::record(is_server_error)` where `is_server_error = status.code >= 500`.

### Key log sites

| Location | Level | Fields |
|---|---|---|
| `TracingFairing::on_response` | info | method, uri, status |
| `DegradationService::set_flag` | warn | key, old_value, new_value, actor_id |
| `AlertEngine::evaluate` (OK→ALERTING) | warn | error_rate_pct, threshold_pct, window_requests |
| `AlertEngine::evaluate` (ALERTING→OK) | info | error_rate_pct, window_requests |
| `ChaosService::log_drill_started` | warn | drill_start_utc, window_duration_minutes |
| `ChaosService::log_drill_stopped` | info | drill_stop_utc |
| `seed_service::run_seeds` | info | seed_name, status |

---

## 2. In-Process Metrics

**File**: `backend/src/application/metrics_service.rs`

### Architecture

```
MetricsService (Arc-cloned, managed by Rocket)
  └── MetricsInner (Mutex)
        ├── observations: VecDeque<Observation>   ← sliding window
        ├── total_requests: u64                    ← all-time counter
        └── total_errors: u64                      ← all-time counter

Observation { timestamp: Instant, is_error: bool }
```

### Window

Default window: **10 minutes** (`WINDOW_DURATION = Duration::from_secs(600)`).

On every call to `record()` or any read method, stale observations older than the window are pruned from the front of the deque.

### Public API

| Method | Description |
|---|---|
| `record(is_error: bool)` | Append observation; prune stale entries; increment lifetime counters |
| `window_error_rate() -> f64` | `window_error_count / window_request_count` (0.0 if no requests) |
| `window_request_count() -> u64` | Requests in the last 10 minutes |
| `window_error_count() -> u64` | 5xx responses in the last 10 minutes |
| `total_requests() -> u64` | All-time request count since process start |
| `total_errors() -> u64` | All-time error count since process start |

### No external dependencies

All metrics are stored in-process memory. There is no Prometheus endpoint, no StatsD push, and no external time-series DB. The metrics are purely for the local alert engine and the `/api/health/metrics` snapshot endpoint.

---

## 3. Alert Engine — 2% Error Rate Rule

**File**: `backend/src/application/alert_engine.rs`

### Rule

> An alarm transitions to **ALERTING** when the 10-minute sliding window error rate is **strictly greater than 2%** (`> 0.02`).
> It transitions back to **OK** when the rate drops to **2% or below** (`<= 0.02`).

### Evaluation cycle

A background Tokio task in `bootstrap/mod.rs` calls `AlertEngine::evaluate()` every **30 seconds**:

```rust
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    loop {
        interval.tick().await;
        engine.evaluate().await;
    }
});
```

### Edge-triggered DB writes

`evaluate()` writes to `ops_events` **only on state transitions**. If the alarm has been ALERTING for 10 minutes with no change, no DB write occurs until it flips back to OK. This prevents log spam and unnecessary DB load.

### AlarmState

```json
{
  "status": "ok",            // "ok" | "alerting"
  "since": 1712345678,       // Unix timestamp of last transition
  "current_error_rate_pct": 0.0,
  "window_requests": 42,
  "message": "System is operating normally"
}
```

---

## 4. Health Endpoints

**File**: `backend/src/api/observability/mod.rs`

All endpoints are mounted at `/api/health`.

### `GET /api/health/live`

No authentication required. Returns `{"status":"ok"}` with HTTP 200 as long as the process is running.

### `GET /api/health/ready`

No authentication required. Performs a lightweight DB ping (`SELECT 1`). Returns HTTP 200:

```json
{
  "status": "ok",
  "db_ok": true,
  "chaos_active": false
}
```

Returns HTTP 200 with `"status":"degraded"` and `"db_ok":false` if the DB ping fails.

### `GET /api/health/metrics`

**Requires authentication + `api.ops.read` permission.** Returns the current in-process metrics snapshot:

```json
{
  "total_requests": 1500,
  "total_errors": 12,
  "window_requests": 85,
  "window_errors": 2,
  "window_error_rate_pct": 2.35,
  "alert_rule": "ALERTING when window error rate > 2.0% over 10 minutes",
  "threshold_pct": 2.0
}
```

### `GET /api/health/alerts`

**Requires authentication + `api.ops.read` permission.** Returns the current alarm state from the `AlertEngine`.

### `GET /api/health/chaos`

**Requires authentication + `api.ops.read` permission.** Returns the current chaos drill status:

```json
{
  "chaos_enabled": false,
  "in_drill_window": false,
  "drill_active": false,
  "next_window_utc": "Sunday 02:00 UTC",
  "guardrails": [
    "CHAOS_ENABLED env var must be 'true'",
    "Time window: Sunday 02:00-02:15 UTC only",
    "Max DB latency: 200ms",
    "Max timeout injection rate: 5%"
  ]
}
```

---

## 5. Degradation Toggles

**File**: `backend/src/application/degradation_service.rs`

### Purpose

Toggles allow an operator to instantly disable expensive or risky operations (exports, heavy analytics) without restarting the service or deploying new code. This is the primary mechanism for shedding load during peak windows or before a major migration.

### Known toggles

| Key | Default | Effect when disabled |
|---|---|---|
| `exports_enabled` | `true` | `POST /api/reports/export` returns 503 |
| `analytics_enabled` | `true` | All report endpoints (`/kpi`, `/order-volume`, `/revenue`, `/utilization`) return 503 |

### Fail-open semantics

If the `ops_config` DB read fails or the key is not found, `get_flag()` returns `true` (enabled). This ensures a DB glitch cannot accidentally disable exports or analytics system-wide.

### Cache layer

Toggle values are cached in an in-memory `Arc<RwLock<HashMap<String, bool>>>`. On `set_flag()`, the cache entry for that key is invalidated (removed), forcing the next `get_flag()` call to re-read from DB.

### Change audit

Every `set_flag()` call:

1. Validates the key is in `KNOWN_TOGGLES` (400 if unknown).
2. Writes to `ops_config` via `INSERT ... ON DUPLICATE KEY UPDATE`.
3. Invalidates the in-memory cache.
4. Writes an `ops_events` row with `event_type = "toggle_change"`, `old_value`, `new_value`, `actor_id`.
5. Emits a `tracing::warn!` log line.
6. Writes to the `audit_log` table with `resource_type = "ops_toggle"`.

### Seeding defaults

`seed_service::seed_ops_config()` is called at startup (after users are seeded) and inserts default values for both toggles using the admin user's ID as `updated_by` (to satisfy the FK constraint on `ops_config`).

### API

```
GET  /api/ops/flags                      — list all flags (api.ops.read)
POST /api/ops/flags/:key/enable          — enable a flag (api.ops.write)
POST /api/ops/flags/:key/disable         — disable a flag (api.ops.write)
```

---

## 6. Chaos Drill Framework

**File**: `backend/src/application/chaos_service.rs`

### Purpose

Chaos drills verify that the system's alerting, degradation paths, and error-rate metrics respond correctly to induced faults — without requiring a real incident.

### Three-layer guard

All three conditions must be true for faults to activate:

1. **`CHAOS_ENABLED=true`** env var (defaults to `false`)
2. **Time window**: UTC day-of-week = Sunday (6), hour = 2, minute < 15
3. **Checked at call time**: `drill_active()` is a pure function checked inline

### Fault types

| Fault | Trigger function | Behaviour |
|---|---|---|
| DB latency simulation | `maybe_inject_latency()` | `tokio::time::sleep(200ms)` if `drill_active()` |
| Request timeout simulation | `should_inject_timeout()` | Returns `true` for ~5% of calls (`subsec_nanos < threshold`) |

`maybe_inject_latency()` is called at the start of `ReportService` and `ExportService` methods so the latency shows up in the 10-minute metrics window and can trigger the 2% alarm.

### Drill monitor task

If `ChaosService::is_chaos_armed()`, a background Tokio task checks every 60 seconds whether the time window transitioned and writes `chaos_drill_started` / `chaos_drill_stopped` events to `ops_events`.

### Constants

```rust
DRILL_DAY_OF_WEEK       = 6     // Sunday (chrono weekday index)
DRILL_START_HOUR        = 2     // 02:00 UTC
DRILL_DURATION_MINUTES  = 15    // ends at 02:14:59
SIMULATED_DB_LATENCY_MS = 200   // ms per injected sleep
SIMULATED_TIMEOUT_FRACTION = 0.05  // 5% of requests
```

---

## 7. ops_events Table

**Migration**: `backend/migrations/20240106000000_ops_config.sql`

```sql
CREATE TABLE ops_events (
    id          CHAR(36)     PRIMARY KEY,
    event_type  VARCHAR(64)  NOT NULL,
    key_name    VARCHAR(64),
    old_value   VARCHAR(255),
    new_value   VARCHAR(255),
    actor_id    VARCHAR(36),
    note        TEXT,
    created_at  TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

The table is **append-only** — no `UPDATE` or `DELETE` should ever target it.

### Event types

| event_type | Written by | When |
|---|---|---|
| `toggle_change` | `DegradationService::set_flag` | Any flag is enabled or disabled |
| `alarm_alerting` | `AlertEngine::evaluate` | Error rate crosses 2% threshold upward |
| `alarm_ok` | `AlertEngine::evaluate` | Error rate drops back to ≤ 2% |
| `chaos_drill_started` | `ChaosService::log_drill_started` | Sunday 02:00 UTC window entered (when armed) |
| `chaos_drill_stopped` | `ChaosService::log_drill_stopped` | Sunday 02:15 UTC window exited (when armed) |

---

## 8. Role & Permission Matrix

| Permission | System Administrator | Operations Manager | Auditor | Others |
|---|---|---|---|---|
| `api.ops.read` | ✓ | ✓ | ✓ | ✗ |
| `api.ops.write` | ✓ | ✗ | ✗ | ✗ |

Only System Administrators can modify degradation toggles. Operations Managers and Auditors can view the current state via `GET /api/ops/flags` and the health endpoints.

---

## 9. Unit Tests

### MetricsService (`metrics_service.rs`)

| Test | What it verifies |
|---|---|
| `test_empty_metrics` | Zero counts, 0.0 error rate on fresh instance |
| `test_all_success` | 10 success requests → 0.0% error rate |
| `test_error_rate` | 2 errors in 10 requests → 20.0% error rate |
| `test_window_expiry` | Observations outside window (100ms) don't count |
| `test_lifetime_totals` | `total_requests` / `total_errors` count across window resets |

### AlertEngine (`alert_engine.rs`)

| Test | What it verifies |
|---|---|
| `test_alert_threshold_constant` | `ALERT_THRESHOLD == 0.02` |
| `test_alarm_state_construction` | `AlarmState::new()` defaults |
| `test_status_transitions` | OK/ALERTING string serialization |

### DegradationService (`degradation_service.rs`)

| Test | What it verifies |
|---|---|
| `test_known_toggle_keys` | `KNOWN_TOGGLES` contains both keys |
| `test_unknown_flag_validation` | `set_flag("bad_key", ...)` returns `BadRequest` |
| `test_bool_parse` | "true"/"false" string→bool conversion |

### ChaosService (`chaos_service.rs`)

| Test | What it verifies |
|---|---|
| `test_chaos_armed_default` | `is_chaos_armed()` returns false without env var |
| `test_constants` | Day=6, hour=2, duration=15, latency=200, fraction=0.05 |
| `test_no_timeout_when_not_armed` | `should_inject_timeout()` always false when not armed |
| `test_window_format` | `next_window_utc` string contains "Sunday" |
| `test_latency_bound` | Simulated latency ≤ 1 second |
| `test_fraction_bound` | Timeout fraction ≤ 0.10 (10% max safety) |
| `test_chaos_status_structure` | `status()` returns all required fields |
