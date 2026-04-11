#!/usr/bin/env bash
# API tests: Operational Resilience & Observability
# Tests: health probes, metrics snapshot, alert state, chaos status,
#        degradation toggle enable/disable (admin only),
#        unauthorized toggle access (401/403), exports-disabled 503,
#        analytics-disabled 503, unknown flag key 400.

set -euo pipefail

BASE_URL="${BACKEND_URL:-http://localhost:8000}"
PASSED=0
FAILED=0

check() {
    local name="$1"
    local expected="$2"
    local actual="$3"
    if [ "$actual" = "$expected" ]; then
        echo "  PASS  $name (got $actual)"
        PASSED=$((PASSED + 1))
    else
        echo "  FAIL  $name (expected $expected, got $actual)"
        FAILED=$((FAILED + 1))
    fi
}

check_contains() {
    local name="$1"
    local needle="$2"
    local haystack="$3"
    if echo "$haystack" | grep -qF "$needle"; then
        echo "  PASS  $name (contains '$needle')"
        PASSED=$((PASSED + 1))
    else
        echo "  FAIL  $name (expected to contain '$needle')"
        echo "        Got: $(echo "$haystack" | head -c 300)"
        FAILED=$((FAILED + 1))
    fi
}

echo "======================================="
echo " Ops Controls & Observability API Tests"
echo "======================================="
echo "Target: $BASE_URL"
echo ""

# =============================================================
# Auth setup
# =============================================================

ADMIN_RESP=$(curl -sf -X POST "$BASE_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"admin","password":"Admin123!"}' 2>/dev/null || echo "FAIL")
ADMIN_TOKEN=$(echo "$ADMIN_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['token'])" 2>/dev/null || echo "")

OPS_MGR_RESP=$(curl -sf -X POST "$BASE_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"ops_manager","password":"OpsManager123!"}' 2>/dev/null || echo "FAIL")
OPS_MGR_TOKEN=$(echo "$OPS_MGR_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['token'])" 2>/dev/null || echo "")

COACH_RESP=$(curl -sf -X POST "$BASE_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"coach","password":"Coach123!"}' 2>/dev/null || echo "FAIL")
COACH_TOKEN=$(echo "$COACH_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['token'])" 2>/dev/null || echo "")

echo "[1] Auth setup"
check "Admin token obtained" "1" "$([ -n "$ADMIN_TOKEN" ] && echo 1 || echo 0)"
check "Ops manager token obtained" "1" "$([ -n "$OPS_MGR_TOKEN" ] && echo 1 || echo 0)"

# =============================================================
# Health probes (no auth required)
# =============================================================

echo ""
echo "[2] Health probes — no auth required"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/api/health/live")
check "GET /health/live → 200" "200" "$STATUS"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/api/health/ready")
check "GET /health/ready → 200" "200" "$STATUS"

READY_RESP=$(curl -sf "$BASE_URL/api/health/ready" 2>/dev/null || echo "FAIL")
check_contains "/health/ready has status" "status" "$READY_RESP"
check_contains "/health/ready has db_ok" "db_ok" "$READY_RESP"

# =============================================================
# Metrics — requires api.ops.read
# =============================================================

echo ""
echo "[3] Metrics endpoint"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/api/health/metrics")
check "GET /health/metrics — no token → 401" "401" "$STATUS"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    "$BASE_URL/api/health/metrics" \
    -H "Authorization: Bearer $COACH_TOKEN")
check "GET /health/metrics — coach (no ops.read) → 403" "403" "$STATUS"

METRICS_RESP=$(curl -sf \
    "$BASE_URL/api/health/metrics" \
    -H "Authorization: Bearer $ADMIN_TOKEN" 2>/dev/null || echo "FAIL")
check_contains "Metrics has total_requests" "total_requests" "$METRICS_RESP"
check_contains "Metrics has window_error_rate_pct" "window_error_rate_pct" "$METRICS_RESP"
check_contains "Metrics has threshold_pct" "threshold_pct" "$METRICS_RESP"
check_contains "Metrics has alert_rule" "alert_rule" "$METRICS_RESP"

# =============================================================
# Alerts — requires api.ops.read
# =============================================================

echo ""
echo "[4] Alerts endpoint"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/api/health/alerts")
check "GET /health/alerts — no token → 401" "401" "$STATUS"

ALERTS_RESP=$(curl -sf \
    "$BASE_URL/api/health/alerts" \
    -H "Authorization: Bearer $ADMIN_TOKEN" 2>/dev/null || echo "FAIL")
check_contains "Alerts has status field" "status" "$ALERTS_RESP"
check_contains "Alerts has since field" "since" "$ALERTS_RESP"
check_contains "Alerts has current_error_rate_pct" "current_error_rate_pct" "$ALERTS_RESP"

# Ops manager can read alerts (has api.ops.read)
STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    "$BASE_URL/api/health/alerts" \
    -H "Authorization: Bearer $OPS_MGR_TOKEN")
check "GET /health/alerts — ops manager → 200" "200" "$STATUS"

# =============================================================
# Chaos status — requires api.ops.read
# =============================================================

echo ""
echo "[5] Chaos status endpoint"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/api/health/chaos")
check "GET /health/chaos — no token → 401" "401" "$STATUS"

CHAOS_RESP=$(curl -sf \
    "$BASE_URL/api/health/chaos" \
    -H "Authorization: Bearer $ADMIN_TOKEN" 2>/dev/null || echo "FAIL")
check_contains "Chaos has chaos_enabled" "chaos_enabled" "$CHAOS_RESP"
check_contains "Chaos has drill_active" "drill_active" "$CHAOS_RESP"
check_contains "Chaos has in_drill_window" "in_drill_window" "$CHAOS_RESP"
check_contains "Chaos has guardrails" "guardrails" "$CHAOS_RESP"

# =============================================================
# Ops flags — list (api.ops.read)
# =============================================================

echo ""
echo "[6] List degradation flags"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/api/ops/flags")
check "GET /ops/flags — no token → 401" "401" "$STATUS"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    "$BASE_URL/api/ops/flags" \
    -H "Authorization: Bearer $COACH_TOKEN")
check "GET /ops/flags — coach (no ops.read) → 403" "403" "$STATUS"

FLAGS_RESP=$(curl -sf \
    "$BASE_URL/api/ops/flags" \
    -H "Authorization: Bearer $ADMIN_TOKEN" 2>/dev/null || echo "FAIL")
check_contains "Flags list contains exports_enabled" "exports_enabled" "$FLAGS_RESP"
check_contains "Flags list contains analytics_enabled" "analytics_enabled" "$FLAGS_RESP"

# Ops manager can list flags
STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    "$BASE_URL/api/ops/flags" \
    -H "Authorization: Bearer $OPS_MGR_TOKEN")
check "GET /ops/flags — ops manager → 200" "200" "$STATUS"

# =============================================================
# Toggle enable/disable — requires api.ops.write (admin only)
# =============================================================

echo ""
echo "[7] Toggle enable/disable — admin only"

# Coach cannot toggle
STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$BASE_URL/api/ops/flags/exports_enabled/disable" \
    -H "Authorization: Bearer $COACH_TOKEN")
check "POST /flags/exports_enabled/disable — coach → 403" "403" "$STATUS"

# Ops manager can read but cannot toggle (no ops.write)
STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$BASE_URL/api/ops/flags/exports_enabled/disable" \
    -H "Authorization: Bearer $OPS_MGR_TOKEN")
check "POST /flags/exports_enabled/disable — ops manager → 403" "403" "$STATUS"

# Admin can disable exports
DISABLE_RESP=$(curl -sf \
    -X POST "$BASE_URL/api/ops/flags/exports_enabled/disable" \
    -H "Authorization: Bearer $ADMIN_TOKEN" 2>/dev/null || echo "FAIL")
check_contains "Admin disable exports — has key" "exports_enabled" "$DISABLE_RESP"
check_contains "Admin disable exports — value false" "false" "$DISABLE_RESP"

# =============================================================
# Exports disabled → 503 on export endpoint
# =============================================================

echo ""
echo "[8] Exports disabled → 503"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$BASE_URL/api/reports/export" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"export_type":"deliveries","from_date":"2024-01-01","to_date":"2024-12-31"}')
check "Export while disabled → 503" "503" "$STATUS"

# Re-enable exports
ENABLE_RESP=$(curl -sf \
    -X POST "$BASE_URL/api/ops/flags/exports_enabled/enable" \
    -H "Authorization: Bearer $ADMIN_TOKEN" 2>/dev/null || echo "FAIL")
check_contains "Admin enable exports — value true" "true" "$ENABLE_RESP"

# Export works again
STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$BASE_URL/api/reports/export" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"export_type":"deliveries","from_date":"2024-01-01","to_date":"2024-12-31"}')
check "Export after re-enable → 200" "200" "$STATUS"

# =============================================================
# Analytics disabled → 503 on report endpoints
# =============================================================

echo ""
echo "[9] Analytics disabled → 503"

# Disable analytics
curl -sf \
    -X POST "$BASE_URL/api/ops/flags/analytics_enabled/disable" \
    -H "Authorization: Bearer $ADMIN_TOKEN" > /dev/null 2>&1 || true

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    "$BASE_URL/api/reports/kpi?from_date=2024-01-01&to_date=2024-12-31" \
    -H "Authorization: Bearer $ADMIN_TOKEN")
check "KPI while analytics disabled → 503" "503" "$STATUS"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    "$BASE_URL/api/reports/order-volume?from_date=2024-01-01&to_date=2024-12-31" \
    -H "Authorization: Bearer $ADMIN_TOKEN")
check "Order-volume while analytics disabled → 503" "503" "$STATUS"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    "$BASE_URL/api/reports/revenue?from_date=2024-01-01&to_date=2024-12-31" \
    -H "Authorization: Bearer $ADMIN_TOKEN")
check "Revenue while analytics disabled → 503" "503" "$STATUS"

# Re-enable analytics
curl -sf \
    -X POST "$BASE_URL/api/ops/flags/analytics_enabled/enable" \
    -H "Authorization: Bearer $ADMIN_TOKEN" > /dev/null 2>&1 || true

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    "$BASE_URL/api/reports/kpi?from_date=2024-01-01&to_date=2024-12-31" \
    -H "Authorization: Bearer $ADMIN_TOKEN")
check "KPI after re-enable → 200" "200" "$STATUS"

# =============================================================
# Unknown flag key → 400
# =============================================================

echo ""
echo "[10] Unknown flag key validation"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$BASE_URL/api/ops/flags/unknown_flag_xyz/enable" \
    -H "Authorization: Bearer $ADMIN_TOKEN")
check "Enable unknown flag → 400" "400" "$STATUS"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$BASE_URL/api/ops/flags/unknown_flag_xyz/disable" \
    -H "Authorization: Bearer $ADMIN_TOKEN")
check "Disable unknown flag → 400" "400" "$STATUS"

# =============================================================
# [11] Degradation toggle fail-closed validation
#      Uses the actual /api/ops/flags endpoints (not /toggles).
# =============================================================

echo ""
echo "[11] Toggle disable returns false for exports_enabled"
if [ -n "$ADMIN_TOKEN" ]; then
    # Disable exports via the real endpoint
    curl -sf -X POST "$BASE_URL/api/ops/flags/exports_enabled/disable" \
        -H "Authorization: Bearer $ADMIN_TOKEN" 2>/dev/null > /dev/null
    # Verify exports_enabled is false in the flag list
    FLAGS=$(curl -sf -H "Authorization: Bearer $ADMIN_TOKEN" "$BASE_URL/api/ops/flags" 2>/dev/null || echo "[]")
    EXPORTS_VAL=$(echo "$FLAGS" | python3 -c "
import sys,json
flags=json.load(sys.stdin)
for f in flags:
    if f['key_name']=='exports_enabled':
        print(str(f['value']).lower())
        break
" 2>/dev/null || echo "unknown")
    check "Disabled toggle returns false" "false" "$EXPORTS_VAL"
    # Re-enable via the real endpoint
    curl -sf -X POST "$BASE_URL/api/ops/flags/exports_enabled/enable" \
        -H "Authorization: Bearer $ADMIN_TOKEN" 2>/dev/null > /dev/null
else
    echo "  SKIP (no admin token)"
fi

# =============================================================
# Summary
# =============================================================

echo ""
echo "======================================="
echo " Results: $PASSED passed, $FAILED failed"
echo "======================================="

[ "$FAILED" -eq 0 ] || exit 1
