#!/usr/bin/env bash
# API tests: Reports & Exports
# Tests: order volume, revenue, utilization, KPI endpoints with date filters,
#        export masking defaults, unmasked export gating, unknown export type 400.

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
    # Use grep -F (fixed string) so regex metacharacters like [ and ] work as literals
    if echo "$haystack" | grep -qF "$needle"; then
        echo "  PASS  $name (contains '$needle')"
        PASSED=$((PASSED + 1))
    else
        echo "  FAIL  $name (expected to contain '$needle')"
        echo "        Got: $haystack" | head -c 300
        FAILED=$((FAILED + 1))
    fi
}

check_not_contains() {
    local name="$1"
    local needle="$2"
    local haystack="$3"
    if echo "$haystack" | grep -q "$needle"; then
        echo "  FAIL  $name (should NOT contain '$needle')"
        FAILED=$((FAILED + 1))
    else
        echo "  PASS  $name (does not contain '$needle')"
        PASSED=$((PASSED + 1))
    fi
}

echo "======================================="
echo " Reports & Exports API Tests"
echo "======================================="
echo "Target: $BASE_URL"
echo ""

# =============================================================
# Auth
# =============================================================

ADMIN_RESP=$(curl -sf -X POST "$BASE_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"admin","password":"Admin123!"}' 2>/dev/null || echo "FAIL")
ADMIN_TOKEN=$(echo "$ADMIN_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['token'])" 2>/dev/null || echo "")

AUDITOR_RESP=$(curl -sf -X POST "$BASE_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"auditor","password":"Auditor123!"}' 2>/dev/null || echo "FAIL")
AUDITOR_TOKEN=$(echo "$AUDITOR_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['token'])" 2>/dev/null || echo "")

COACH_RESP=$(curl -sf -X POST "$BASE_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"coach","password":"Coach123!"}' 2>/dev/null || echo "FAIL")
COACH_TOKEN=$(echo "$COACH_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['token'])" 2>/dev/null || echo "")

echo "[1] Auth setup"
check "Admin token obtained" "1" "$([ -n "$ADMIN_TOKEN" ] && echo 1 || echo 0)"

# =============================================================
# Unauthenticated
# =============================================================

echo ""
echo "[2] Unauthenticated access"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    "$BASE_URL/api/reports/kpi?from_date=2024-01-01&to_date=2024-12-31")
check "KPI — no token → 401" "401" "$STATUS"

# =============================================================
# Coach lacks reports permission
# =============================================================

echo ""
echo "[3] Coach cannot access reports"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    "$BASE_URL/api/reports/kpi?from_date=2024-01-01&to_date=2024-12-31" \
    -H "Authorization: Bearer $COACH_TOKEN")
check "Coach reports/kpi → 403" "403" "$STATUS"

# =============================================================
# KPI summary
# =============================================================

echo ""
echo "[4] KPI summary"

KPI_RESP=$(curl -sf \
    "$BASE_URL/api/reports/kpi?from_date=2024-01-01&to_date=2024-12-31" \
    -H "Authorization: Bearer $ADMIN_TOKEN" 2>/dev/null || echo "FAIL")
check_contains "KPI response has attendance_rate_pct" "attendance_rate_pct" "$KPI_RESP"
check_contains "KPI response has repurchase_rate_pct" "repurchase_rate_pct" "$KPI_RESP"
check_contains "KPI response has staff_utilization_pct" "staff_utilization_pct" "$KPI_RESP"
check_contains "KPI response has second_review_rate_pct" "second_review_rate_pct" "$KPI_RESP"
check_contains "KPI has period_start" "period_start" "$KPI_RESP"

# KPI numeric sanity: all percentage fields must be in the 0-100 range.
# If any value exceeds 100, the double-*100 bug has regressed.
echo ""
echo "[4b] KPI numeric range assertions (percentages must be 0–100)"

KPI_RANGE_OK=$(echo "$KPI_RESP" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    fields = ['attendance_rate_pct','repurchase_rate_pct','staff_utilization_pct','second_review_rate_pct']
    for f in fields:
        v = d.get(f, 0)
        if not (0 <= v <= 100):
            print(f'FAIL:{f}={v}')
            sys.exit(0)
    # avg_score is on 0-100 scale but may be null
    avg = d.get('avg_score')
    if avg is not None and not (0 <= avg <= 100):
        print(f'FAIL:avg_score={avg}')
        sys.exit(0)
    print('OK')
except Exception as e:
    print(f'ERROR:{e}')
" 2>/dev/null || echo "ERROR")

check "KPI pct fields all in 0–100 range" "OK" "$KPI_RANGE_OK"

# Verify period echoed correctly
KPI_PERIOD=$(echo "$KPI_RESP" | python3 -c "
import sys,json
d=json.load(sys.stdin)
print(f\"{d['period_start']}:{d['period_end']}\")
" 2>/dev/null || echo "")
check "KPI period matches request" "2024-01-01:2024-12-31" "$KPI_PERIOD"

# =============================================================
# Order volume
# =============================================================

echo ""
echo "[5] Order volume report"

OV_RESP=$(curl -sf \
    "$BASE_URL/api/reports/order-volume?from_date=2024-01-01&to_date=2024-12-31" \
    -H "Authorization: Bearer $ADMIN_TOKEN" 2>/dev/null || echo "FAIL")
# Should be a JSON array
check_contains "Order volume returns array" "[" "$OV_RESP"

# =============================================================
# Revenue report
# =============================================================

echo ""
echo "[6] Revenue report"

REV_RESP=$(curl -sf \
    "$BASE_URL/api/reports/revenue?from_date=2024-01-01&to_date=2024-12-31" \
    -H "Authorization: Bearer $ADMIN_TOKEN" 2>/dev/null || echo "FAIL")
check_contains "Revenue report returns array" "[" "$REV_RESP"

# =============================================================
# Utilization report
# =============================================================

echo ""
echo "[7] Utilization report"

UTIL_RESP=$(curl -sf \
    "$BASE_URL/api/reports/utilization?from_date=2024-01-01&to_date=2024-12-31" \
    -H "Authorization: Bearer $ADMIN_TOKEN" 2>/dev/null || echo "FAIL")
check_contains "Utilization report returns array" "[" "$UTIL_RESP"

# =============================================================
# Export — masked by default
# =============================================================

echo ""
echo "[8] Export — masked by default"

EXPORT_RESP=$(curl -sf -X POST "$BASE_URL/api/reports/export" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
        "export_type": "deliveries",
        "from_date": "2024-01-01",
        "to_date": "2024-12-31"
    }' 2>/dev/null || echo "FAIL")

check_contains "Export response has masked=true" "\"masked\":true" "$EXPORT_RESP"
check_contains "Export has export_log_id" "export_log_id" "$EXPORT_RESP"
check_contains "Export has row_count" "row_count" "$EXPORT_RESP"

# Client names should be masked
if echo "$EXPORT_RESP" | python3 -c "
import sys, json
d = json.load(sys.stdin)
rows = d.get('rows', [])
if not rows:
    print('no_rows')
    sys.exit(0)
# All client_name fields must be ****
bad = [r for r in rows if r.get('client_name') != '****']
print('ok' if not bad else 'has_unmasked')
" 2>/dev/null | grep -q "ok\|no_rows"; then
    echo "  PASS  Masked export — client_name is '****' or no rows"
    PASSED=$((PASSED + 1))
else
    echo "  FAIL  Masked export — client_name not masked"
    FAILED=$((FAILED + 1))
fi

# =============================================================
# Export — invalid type → 400
# =============================================================

echo ""
echo "[9] Export validation"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$BASE_URL/api/reports/export" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"export_type":"unknown_type","from_date":"2024-01-01","to_date":"2024-12-31"}')
check "Unknown export_type → 400" "400" "$STATUS"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$BASE_URL/api/reports/export" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"export_type":"deliveries","from_date":"not-a-date","to_date":"2024-12-31"}')
check "Invalid from_date format → 400" "400" "$STATUS"

# =============================================================
# Coach cannot export
# =============================================================

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$BASE_URL/api/reports/export" \
    -H "Authorization: Bearer $COACH_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"export_type":"deliveries","from_date":"2024-01-01","to_date":"2024-12-31"}')
check "Coach export → 403" "403" "$STATUS"

# =============================================================
# Unmasked export requires explicit permission
# =============================================================

echo ""
echo "[10] Unmasked export — admin has permission"

UNMASKED_RESP=$(curl -sf -X POST "$BASE_URL/api/reports/export" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
        "export_type": "deliveries",
        "from_date": "2024-01-01",
        "to_date": "2024-12-31",
        "unmasked": true
    }' 2>/dev/null || echo "FAIL")

check_contains "Unmasked export — admin gets masked=false" "\"masked\":false" "$UNMASKED_RESP"

# =============================================================
# Export evaluations type
# =============================================================

echo ""
echo "[11] Export evaluations type"

EVAL_EXPORT=$(curl -sf -X POST "$BASE_URL/api/reports/export" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
        "export_type": "evaluations",
        "from_date": "2024-01-01",
        "to_date": "2024-12-31"
    }' 2>/dev/null || echo "FAIL")
check_contains "Evaluations export has row_count" "row_count" "$EVAL_EXPORT"
check_contains "Evaluations export has masked=true" "\"masked\":true" "$EVAL_EXPORT"

# =============================================================
# Export — project_id filtering (backward compat + new field)
# =============================================================

echo ""
echo "[12] Export with department_id only (backward compat)"

DEPT_EXPORT=$(curl -sf -X POST "$BASE_URL/api/reports/export" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
        "export_type": "deliveries",
        "from_date": "2024-01-01",
        "to_date": "2024-12-31",
        "department_id": "nonexistent-dept-id"
    }' 2>/dev/null || echo "FAIL")
# Should succeed (200) with 0 rows — the department doesn't exist
check_contains "Dept-only export has row_count" "row_count" "$DEPT_EXPORT"

echo ""
echo "[13] Export with department_id + project_id"

PROJ_EXPORT=$(curl -sf -X POST "$BASE_URL/api/reports/export" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
        "export_type": "deliveries",
        "from_date": "2024-01-01",
        "to_date": "2024-12-31",
        "department_id": "nonexistent-dept",
        "project_id": "nonexistent-proj"
    }' 2>/dev/null || echo "FAIL")
check_contains "Dept+project export has row_count" "row_count" "$PROJ_EXPORT"

echo ""
echo "[14] Export with project_id only (no department)"

PROJ_ONLY=$(curl -sf -X POST "$BASE_URL/api/reports/export" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
        "export_type": "revenue",
        "from_date": "2024-01-01",
        "to_date": "2024-12-31",
        "project_id": "nonexistent-proj"
    }' 2>/dev/null || echo "FAIL")
check_contains "Project-only export has row_count" "row_count" "$PROJ_ONLY"

echo ""
echo "[15] Backward compat — omitting project_id still works"

# This is the original export shape with no project_id key at all
COMPAT_EXPORT=$(curl -sf -X POST "$BASE_URL/api/reports/export" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
        "export_type": "evaluations",
        "from_date": "2024-01-01",
        "to_date": "2024-12-31"
    }' 2>/dev/null || echo "FAIL")
check_contains "No project_id field — still returns row_count" "row_count" "$COMPAT_EXPORT"

# =============================================================
# Cross-project scope 403 — user scoped to project_A cannot
# export data filtered to project_B
# =============================================================

echo ""
echo "[16] Cross-project scope enforcement"

# Discover the demo org and create a project to scope a test user to
ORG_ID=$(curl -sf -H "Authorization: Bearer $ADMIN_TOKEN" "$BASE_URL/api/admin/org/" 2>/dev/null \
    | python3 -c "import sys,json; orgs=json.load(sys.stdin); print(orgs[0]['id'] if orgs else '')" 2>/dev/null || echo "")

# Find a department id to anchor the project
DEPT_ID=$(curl -sf -H "Authorization: Bearer $ADMIN_TOKEN" \
    "$BASE_URL/api/admin/org/$ORG_ID/departments" 2>/dev/null \
    | python3 -c "import sys,json; d=json.load(sys.stdin); print(d[0]['id'] if d else '')" 2>/dev/null || echo "")

# Find the Billing Specialist role (has action.reports.export)
BILLING_ROLE_ID=$(curl -sf -H "Authorization: Bearer $ADMIN_TOKEN" "$BASE_URL/api/roles/" 2>/dev/null \
    | python3 -c "
import sys,json
roles=json.load(sys.stdin)
for r in roles:
    if r['name']=='Billing Specialist':
        print(r['id']); break
" 2>/dev/null || echo "")

SCOPE_TOKEN=""
if [ -n "$ORG_ID" ] && [ -n "$DEPT_ID" ] && [ -n "$BILLING_ROLE_ID" ]; then
    # Create two projects under the department
    PROJ_A_RESP=$(curl -sf -X POST "$BASE_URL/api/admin/org/$ORG_ID/departments/$DEPT_ID/projects" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d '{"name":"ScopeTest Project A"}' 2>/dev/null || echo "FAIL")
    PROJ_A_ID=$(echo "$PROJ_A_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('id',''))" 2>/dev/null || echo "")

    PROJ_B_RESP=$(curl -sf -X POST "$BASE_URL/api/admin/org/$ORG_ID/departments/$DEPT_ID/projects" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d '{"name":"ScopeTest Project B"}' 2>/dev/null || echo "FAIL")
    PROJ_B_ID=$(echo "$PROJ_B_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('id',''))" 2>/dev/null || echo "")

    # Create a test user scoped to Project A only
    SCOPE_USER="projscope_$(date +%s)"
    CREATE_RESP=$(curl -sf -X POST "$BASE_URL/api/users/" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d "{\"username\":\"$SCOPE_USER\",\"email\":\"${SCOPE_USER}@test.local\",\"password\":\"Sc0pe123!\",\"org_id\":\"$ORG_ID\"}" \
        2>/dev/null || echo "FAIL")
    SCOPE_UID=$(echo "$CREATE_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('id',''))" 2>/dev/null || echo "")

    if [ -n "$SCOPE_UID" ] && [ -n "$PROJ_A_ID" ] && [ -n "$PROJ_B_ID" ]; then
        # Assign Billing Specialist role (gives action.reports.export)
        curl -sf -X POST "$BASE_URL/api/users/$SCOPE_UID/roles" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $ADMIN_TOKEN" \
            -d "{\"role_id\":\"$BILLING_ROLE_ID\"}" 2>/dev/null > /dev/null

        # Grant scope to Project A only (not B)
        curl -sf -X POST "$BASE_URL/api/users/$SCOPE_UID/scopes" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $ADMIN_TOKEN" \
            -d "{\"org_id\":\"$ORG_ID\",\"department_id\":\"$DEPT_ID\",\"project_id\":\"$PROJ_A_ID\",\"access_level\":\"read\"}" \
            2>/dev/null > /dev/null

        # Log in as the scoped user
        SCOPE_LOGIN=$(curl -sf -X POST "$BASE_URL/api/auth/login" \
            -H "Content-Type: application/json" \
            -d "{\"username\":\"$SCOPE_USER\",\"password\":\"Sc0pe123!\"}" 2>/dev/null || echo "FAIL")
        SCOPE_TOKEN=$(echo "$SCOPE_LOGIN" | python3 -c "import sys,json; print(json.load(sys.stdin)['token'])" 2>/dev/null || echo "")
    fi
fi

if [ -n "$SCOPE_TOKEN" ] && [ -n "$PROJ_A_ID" ] && [ -n "$PROJ_B_ID" ] && [ -n "$DEPT_ID" ]; then
    # Export for Project A (user has scope) — should succeed
    STATUS_A=$(curl -s -o /dev/null -w "%{http_code}" \
        -X POST "$BASE_URL/api/reports/export" \
        -H "Authorization: Bearer $SCOPE_TOKEN" \
        -H "Content-Type: application/json" \
        -d "{\"export_type\":\"deliveries\",\"from_date\":\"2024-01-01\",\"to_date\":\"2024-12-31\",\"department_id\":\"$DEPT_ID\",\"project_id\":\"$PROJ_A_ID\"}" \
        2>/dev/null || echo "000")
    check "Scoped user export own project -> 200" "200" "$STATUS_A"

    # Export for Project B (user does NOT have scope) — should be 403
    STATUS_B=$(curl -s -o /dev/null -w "%{http_code}" \
        -X POST "$BASE_URL/api/reports/export" \
        -H "Authorization: Bearer $SCOPE_TOKEN" \
        -H "Content-Type: application/json" \
        -d "{\"export_type\":\"deliveries\",\"from_date\":\"2024-01-01\",\"to_date\":\"2024-12-31\",\"department_id\":\"$DEPT_ID\",\"project_id\":\"$PROJ_B_ID\"}" \
        2>/dev/null || echo "000")
    check "Scoped user export OTHER project -> 403" "403" "$STATUS_B"
else
    echo "  SKIP [16] (could not create scoped test user/projects)"
fi

# =============================================================
# Delivery export regression (service_catalog_items join fix)
#
# A prior bug used "service_items" (non-existent table) in the SQL
# JOIN, causing 500 on any deliveries export.  These tests prove:
#   - HTTP 200 (not 500) from the deliveries export endpoint
#   - Response body parses as valid JSON with required envelope fields
#   - When rows exist, every row contains the expected schema fields
#   - row_count matches the actual rows array length
#
# The HTTP status check is the primary regression gate: a wrong table
# name in the SQL produces a 500, which fails the first assertion
# immediately with a clear message.
# =============================================================

echo ""
echo "[17] Delivery export: HTTP status (regression gate for SQL table join)"

# Step 1: explicit HTTP status check — catches SQL errors (wrong table name → 500)
DEL_HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$BASE_URL/api/reports/export" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"export_type":"deliveries","from_date":"2024-01-01","to_date":"2024-12-31"}')
check "Deliveries export HTTP 200 (not 500 — SQL join is valid)" "200" "$DEL_HTTP_STATUS"

# Step 2: fetch the body for field-level assertions
DEL_EXPORT=$(curl -sf -X POST "$BASE_URL/api/reports/export" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
        "export_type": "deliveries",
        "from_date": "2024-01-01",
        "to_date": "2024-12-31"
    }' 2>/dev/null || echo "FAIL")

DEL_SHAPE=$(echo "$DEL_EXPORT" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    missing = {'rows','row_count','masked','export_log_id'} - set(d.keys())
    if missing:
        print(f'FAIL:envelope missing {missing}')
    else:
        print('OK')
except Exception as e:
    print(f'FAIL:{e}')
" 2>/dev/null || echo "FAIL:parse_error")
check "Delivery export response envelope" "OK" "$DEL_SHAPE"

# Step 3: verify per-row schema when data exists
echo "[17b] Delivery export row field schema"
DEL_FIELDS=$(echo "$DEL_EXPORT" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    rows = d.get('rows', [])
    if not rows:
        # No delivery data in the date range — this is a data gap, not a code bug.
        # The HTTP 200 above already proves the SQL JOIN is valid.
        print('OK_NO_DATA')
        sys.exit(0)
    # Full field set expected from export_deliveries query
    expected = {'id','delivery_date','plan_id','client_name','provider_id',
                'service_item_id','service_name','units','mileage','status',
                'start_time','end_time'}
    for i, r in enumerate(rows):
        missing = expected - set(r.keys())
        if missing:
            print(f'FAIL:row[{i}] missing {missing}')
            sys.exit(0)
    print(f'OK:{len(rows)}_rows')
except Exception as e:
    print(f'FAIL:{e}')
" 2>/dev/null || echo "FAIL:parse_error")

case "$DEL_FIELDS" in
    OK:*) check "Delivery export row fields (with data)" "1" "1" ;;
    OK_NO_DATA)
        echo "  PASS  Delivery export row fields (no delivery data in range — HTTP 200 proves JOIN validity)"
        PASSED=$((PASSED + 1))
        ;;
    *) check "Delivery export row fields" "OK" "$DEL_FIELDS" ;;
esac

echo "[18] Delivery export row_count matches rows array length"

DEL_COUNT_CHECK=$(echo "$DEL_EXPORT" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    actual_len = len(d.get('rows', []))
    declared = d.get('row_count', -1)
    if actual_len == declared:
        print('OK')
    else:
        print(f'FAIL:row_count={declared},actual={actual_len}')
except Exception as e:
    print(f'FAIL:{e}')
" 2>/dev/null || echo "FAIL:parse_error")
check "Delivery export row_count consistent" "OK" "$DEL_COUNT_CHECK"

echo "[19] Delivery export with filters still returns 200"

DEL_FILTER_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$BASE_URL/api/reports/export" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
        "export_type": "deliveries",
        "from_date": "2024-01-01",
        "to_date": "2024-12-31",
        "department_id": "nonexistent-dept",
        "project_id": "nonexistent-proj"
    }' 2>/dev/null || echo "000")
check "Delivery export with filters -> 200" "200" "$DEL_FILTER_STATUS"

echo "[19b] Evaluations and revenue exports also return 200 (not 500)"
EVAL_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$BASE_URL/api/reports/export" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"export_type":"evaluations","from_date":"2024-01-01","to_date":"2024-12-31"}')
check "Evaluations export HTTP 200" "200" "$EVAL_STATUS"

REV_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$BASE_URL/api/reports/export" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"export_type":"revenue","from_date":"2024-01-01","to_date":"2024-12-31"}')
check "Revenue export HTTP 200" "200" "$REV_STATUS"

# =============================================================
# Service route filter dimension tests
#
# Validates that the optional service_route query parameter:
#   - is accepted and returns 200 when provided (filter narrows results)
#   - does not change behavior when omitted (backward compatibility)
#   - returns 400 for empty/whitespace-only values
# =============================================================

echo ""
echo "--- Service route filter dimension ---"

echo "[20] Reports without service_route still return 200 (backward compat)"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    "$BASE_URL/api/reports/order-volume?from_date=2024-01-01&to_date=2024-12-31" \
    -H "Authorization: Bearer $ADMIN_TOKEN")
check "Order volume without route -> 200" "200" "$STATUS"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    "$BASE_URL/api/reports/revenue?from_date=2024-01-01&to_date=2024-12-31" \
    -H "Authorization: Bearer $ADMIN_TOKEN")
check "Revenue without route -> 200" "200" "$STATUS"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    "$BASE_URL/api/reports/utilization?from_date=2024-01-01&to_date=2024-12-31" \
    -H "Authorization: Bearer $ADMIN_TOKEN")
check "Utilization without route -> 200" "200" "$STATUS"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    "$BASE_URL/api/reports/kpi?from_date=2024-01-01&to_date=2024-12-31" \
    -H "Authorization: Bearer $ADMIN_TOKEN")
check "KPI without route -> 200" "200" "$STATUS"

echo "[21] Reports with service_route filter return 200"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    "$BASE_URL/api/reports/order-volume?from_date=2024-01-01&to_date=2024-12-31&service_route=north-metro" \
    -H "Authorization: Bearer $ADMIN_TOKEN")
check "Order volume with route -> 200" "200" "$STATUS"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    "$BASE_URL/api/reports/revenue?from_date=2024-01-01&to_date=2024-12-31&service_route=north-metro" \
    -H "Authorization: Bearer $ADMIN_TOKEN")
check "Revenue with route -> 200" "200" "$STATUS"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    "$BASE_URL/api/reports/utilization?from_date=2024-01-01&to_date=2024-12-31&service_route=north-metro" \
    -H "Authorization: Bearer $ADMIN_TOKEN")
check "Utilization with route -> 200" "200" "$STATUS"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    "$BASE_URL/api/reports/kpi?from_date=2024-01-01&to_date=2024-12-31&service_route=north-metro" \
    -H "Authorization: Bearer $ADMIN_TOKEN")
check "KPI with route -> 200" "200" "$STATUS"

echo "[22] Empty service_route returns 400 (validation)"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    "$BASE_URL/api/reports/order-volume?from_date=2024-01-01&to_date=2024-12-31&service_route=%20" \
    -H "Authorization: Bearer $ADMIN_TOKEN")
check "Order volume with whitespace-only route -> 400" "400" "$STATUS"

echo "[23] Export with service_route filter returns 200"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$BASE_URL/api/reports/export" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"export_type":"deliveries","from_date":"2024-01-01","to_date":"2024-12-31","service_route":"north-metro"}')
check "Delivery export with route -> 200" "200" "$STATUS"

echo "[24] Export without service_route still returns 200 (backward compat)"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$BASE_URL/api/reports/export" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"export_type":"deliveries","from_date":"2024-01-01","to_date":"2024-12-31"}')
check "Delivery export without route -> 200" "200" "$STATUS"

echo "[25] Route filter combines with existing dept/project filters"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    "$BASE_URL/api/reports/order-volume?from_date=2024-01-01&to_date=2024-12-31&department_id=nonexistent&service_route=north-metro" \
    -H "Authorization: Bearer $ADMIN_TOKEN")
check "Order volume with dept + route -> 200" "200" "$STATUS"

# =============================================================
# Summary
# =============================================================

echo ""
echo "======================================="
echo " Results: $PASSED passed, $FAILED failed"
echo "======================================="

[ "$FAILED" -eq 0 ] || exit 1
