#!/usr/bin/env bash
# API tests: Catalog, Packages, Plans, and Delivery workflows
# Requires the full stack running via docker-compose

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

echo "======================================="
echo " Catalog & Delivery API Tests"
echo "======================================="
echo "Target: $BASE_URL"
echo ""

# Login as admin (has full access)
ADMIN_RESP=$(curl -sf -X POST "$BASE_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"admin","password":"Admin123!"}' 2>/dev/null || echo "FAIL")
ADMIN_TOKEN=$(echo "$ADMIN_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['token'])" 2>/dev/null || echo "")

# Login as coach (limited access)
COACH_RESP=$(curl -sf -X POST "$BASE_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"coach","password":"Coach123!"}' 2>/dev/null || echo "FAIL")
COACH_TOKEN=$(echo "$COACH_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['token'])" 2>/dev/null || echo "")

# Login as auditor (read-only)
AUDITOR_RESP=$(curl -sf -X POST "$BASE_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"auditor","password":"Auditor123!"}' 2>/dev/null || echo "FAIL")
AUDITOR_TOKEN=$(echo "$AUDITOR_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['token'])" 2>/dev/null || echo "")

if [ -z "$ADMIN_TOKEN" ]; then
    echo "FATAL: Could not get admin token. Backend may not be running."
    exit 1
fi

# Use a per-run suffix so tests are idempotent across multiple invocations
# against a persistent database.
RUN_SUFFIX=$(date +%s)

# --- Catalog Tests ---
echo "[1] Create service item"
SVC_RESP=$(curl -sf -X POST "$BASE_URL/api/catalog/" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d "{\"code\":\"SVC-TEST-${RUN_SUFFIX}\",\"name\":\"Test Nursing Visit\",\"category\":\"nursing\",\"unit_type\":\"visit\",\"default_rate\":75.00}" \
    2>/dev/null || echo "FAIL")
if echo "$SVC_RESP" | grep -q '"id"'; then
    SVC_ID=$(echo "$SVC_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['id'])" 2>/dev/null || echo "")
    check "Create service returns ID" "true" "true"
else
    check "Create service returns ID" "true" "false"
    SVC_ID=""
fi

echo "[2] Duplicate code rejected"
STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE_URL/api/catalog/" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d "{\"code\":\"SVC-TEST-${RUN_SUFFIX}\",\"name\":\"Duplicate\",\"category\":\"nursing\",\"unit_type\":\"visit\",\"default_rate\":50.00}")
check "Duplicate code -> 400" "400" "$STATUS"

echo "[3] Invalid category rejected"
STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE_URL/api/catalog/" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d '{"code":"SVC-BAD","name":"Bad","category":"invalid_cat","unit_type":"visit","default_rate":50.00}' \
    2>/dev/null || echo "000")
check "Invalid category -> 400" "400" "$STATUS"

echo "[4] List services"
STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    "$BASE_URL/api/catalog/" 2>/dev/null || echo "000")
check "List services -> 200" "200" "$STATUS"

echo "[5] Auditor cannot create services"
STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE_URL/api/catalog/" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $AUDITOR_TOKEN" \
    -d "{\"code\":\"SVC-AUD-${RUN_SUFFIX}\",\"name\":\"Auditor Service\",\"category\":\"nursing\",\"unit_type\":\"visit\",\"default_rate\":50.00}")
check "Auditor cannot create service (403)" "403" "$STATUS"

# Create hourly service for package rules
echo "[6] Create hourly service"
HOUR_RESP=$(curl -sf -X POST "$BASE_URL/api/catalog/" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d "{\"code\":\"SVC-REHAB-${RUN_SUFFIX}\",\"name\":\"Rehab Session\",\"category\":\"rehab\",\"unit_type\":\"hour\",\"default_rate\":60.00}" \
    2>/dev/null || echo "FAIL")
HOUR_SVC_ID=$(echo "$HOUR_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['id'])" 2>/dev/null || echo "")

# --- Package Tests ---
echo "[7] Create package with rules"
if [ -n "$SVC_ID" ] && [ -n "$HOUR_SVC_ID" ]; then
    PKG_RESP=$(curl -sf -X POST "$BASE_URL/api/packages/" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d "{\"code\":\"PKG-BASIC-${RUN_SUFFIX}\",\"name\":\"Basic Care Package\",\"description\":\"Standard care bundle\",\"rules\":[{\"service_item_id\":\"$SVC_ID\",\"rule_type\":\"per_visit\",\"rate\":75.00},{\"service_item_id\":\"$HOUR_SVC_ID\",\"rule_type\":\"hourly\",\"rate\":60.00,\"min_increment\":0.25,\"max_units_per_delivery\":8.0}]}" \
        2>/dev/null || echo "FAIL")
    if echo "$PKG_RESP" | grep -q '"package"'; then
        PKG_ID=$(echo "$PKG_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['package']['id'])" 2>/dev/null || echo "")
        check "Create package with rules" "true" "true"
    else
        check "Create package with rules" "true" "false"
        PKG_ID=""
    fi
else
    echo "  SKIP (missing service IDs)"
    PKG_ID=""
fi

echo "[8] Package with invalid rule rejected"
STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE_URL/api/packages/" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d '{"code":"PKG-BAD","name":"Bad Package","rules":[{"service_item_id":"nonexistent","rule_type":"invalid","rate":-10.0}]}' \
    2>/dev/null || echo "000")
check "Invalid rule -> 400" "400" "$STATUS"

# --- Client Plan Tests ---
echo "[9] Create client plan"
PLAN_RESP=$(curl -sf -X POST "$BASE_URL/api/plans/" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d '{"client_name":"Jane Doe","start_date":"2024-01-15","end_date":"2024-12-31"}' \
    2>/dev/null || echo "FAIL")
if echo "$PLAN_RESP" | grep -q '"id"'; then
    PLAN_ID=$(echo "$PLAN_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['id'])" 2>/dev/null || echo "")
    check "Create plan" "true" "true"
else
    check "Create plan" "true" "false"
    PLAN_ID=""
fi

echo "[10] Invalid date rejected"
STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE_URL/api/plans/" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d '{"client_name":"Test","start_date":"not-a-date"}' \
    2>/dev/null || echo "000")
check "Invalid date -> 400" "400" "$STATUS"

# Activate plan
if [ -n "$PLAN_ID" ]; then
    curl -sf -X PUT "$BASE_URL/api/plans/$PLAN_ID" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d '{"status":"active"}' 2>/dev/null > /dev/null
fi

# Assign package to plan
echo "[11] Assign package to plan"
if [ -n "$PLAN_ID" ] && [ -n "$PKG_ID" ]; then
    ASSIGN_RESP=$(curl -sf -X POST "$BASE_URL/api/plans/$PLAN_ID/packages" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d "{\"package_id\":\"$PKG_ID\",\"effective_date\":\"2024-01-15\"}" \
        2>/dev/null || echo "FAIL")
    if echo "$ASSIGN_RESP" | grep -q '"id"'; then
        PP_ID=$(echo "$ASSIGN_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['id'])" 2>/dev/null || echo "")
        check "Assign package to plan" "true" "true"
    else
        check "Assign package to plan" "true" "false"
        PP_ID=""
    fi
else
    echo "  SKIP (missing plan/package)"
    PP_ID=""
fi

# --- Delivery Entry Tests ---
echo "[12] Create delivery entry (coach)"
if [ -n "$PLAN_ID" ] && [ -n "$PP_ID" ] && [ -n "$HOUR_SVC_ID" ] && [ -n "$COACH_TOKEN" ]; then
    ENTRY_RESP=$(curl -sf -X POST "$BASE_URL/api/delivery/" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $COACH_TOKEN" \
        -d "{\"plan_id\":\"$PLAN_ID\",\"plan_package_id\":\"$PP_ID\",\"service_item_id\":\"$HOUR_SVC_ID\",\"delivery_date\":\"2024-03-15\",\"units\":2.5,\"start_time\":\"09:00\",\"end_time\":\"11:30\"}" \
        2>/dev/null || echo "FAIL")
    if echo "$ENTRY_RESP" | grep -q '"id"'; then
        ENTRY_ID=$(echo "$ENTRY_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['id'])" 2>/dev/null || echo "")
        check "Coach creates delivery entry" "true" "true"
    else
        check "Coach creates delivery entry" "true" "false"
    fi
else
    echo "  SKIP (missing prerequisites)"
fi

echo "[13] Invalid quarter-hour rejected"
if [ -n "$PLAN_ID" ] && [ -n "$PP_ID" ] && [ -n "$HOUR_SVC_ID" ] && [ -n "$COACH_TOKEN" ]; then
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE_URL/api/delivery/" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $COACH_TOKEN" \
        -d "{\"plan_id\":\"$PLAN_ID\",\"plan_package_id\":\"$PP_ID\",\"service_item_id\":\"$HOUR_SVC_ID\",\"delivery_date\":\"2024-03-16\",\"units\":1.3}" \
        2>/dev/null || echo "000")
    check "Non-quarter-hour rejected (400)" "400" "$STATUS"
else
    echo "  SKIP"
fi

echo "[14] Mileage over 200 rejected"
if [ -n "$PLAN_ID" ] && [ -n "$PP_ID" ] && [ -n "$SVC_ID" ] && [ -n "$COACH_TOKEN" ]; then
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE_URL/api/delivery/" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $COACH_TOKEN" \
        -d "{\"plan_id\":\"$PLAN_ID\",\"plan_package_id\":\"$PP_ID\",\"service_item_id\":\"$SVC_ID\",\"delivery_date\":\"2024-03-17\",\"units\":1.0,\"mileage\":250.0}" \
        2>/dev/null || echo "000")
    check "Mileage > 200 rejected (400)" "400" "$STATUS"
else
    echo "  SKIP"
fi

echo "[15] Unauthenticated catalog access"
STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/api/catalog/")
check "No auth -> 401" "401" "$STATUS"

echo "[16] List delivery entries"
if [ -n "$ADMIN_TOKEN" ]; then
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$BASE_URL/api/delivery/" 2>/dev/null || echo "000")
    check "List delivery entries -> 200" "200" "$STATUS"
fi

echo ""
echo "======================================="
echo "Results: $PASSED passed, $FAILED failed"
echo "======================================="
[ "$FAILED" -eq 0 ] || exit 1
