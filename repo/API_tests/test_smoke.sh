#!/usr/bin/env bash
# API smoke tests: verify all registered routes respond with correct HTTP status codes
# and that public endpoints return expected response structure.
# Requires the full stack running via docker-compose.

set -euo pipefail

BASE_URL="${BACKEND_URL:-http://localhost:8000}"
PASSED=0
FAILED=0

check_endpoint() {
    local method="$1"
    local path="$2"
    local expected_status="$3"
    local url="$BASE_URL$path"

    STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X "$method" "$url")

    if [ "$STATUS" = "$expected_status" ]; then
        echo "  PASS  $method $path -> $STATUS"
        PASSED=$((PASSED + 1))
    else
        echo "  FAIL  $method $path -> $STATUS (expected $expected_status)"
        FAILED=$((FAILED + 1))
    fi
}

# Check both status AND a response body field
check_body_contains() {
    local method="$1"
    local path="$2"
    local expected_status="$3"
    local expected_field="$4"
    local url="$BASE_URL$path"

    BODY=$(curl -s -X "$method" "$url")
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X "$method" "$url")

    if [ "$STATUS" != "$expected_status" ]; then
        echo "  FAIL  $method $path -> status $STATUS (expected $expected_status)"
        FAILED=$((FAILED + 1))
        return
    fi

    if echo "$BODY" | grep -q "\"$expected_field\""; then
        echo "  PASS  $method $path -> $STATUS (body has '$expected_field')"
        PASSED=$((PASSED + 1))
    else
        echo "  FAIL  $method $path -> $STATUS but body missing '$expected_field': $BODY"
        FAILED=$((FAILED + 1))
    fi
}

echo "API Smoke Tests"
echo "==============="
echo "Target: $BASE_URL"
echo ""

# Health endpoints (public, no auth) — check status AND response body structure
echo "--- Public health endpoints (status + body) ---"
check_body_contains GET "/api/health/live" "200" "status"
check_body_contains GET "/api/health/ready" "200" "db_ok"

# Verify liveness response contains the expected "ok" value
echo ""
echo "--- Liveness response content ---"
LIVE_BODY=$(curl -s "$BASE_URL/api/health/live")
if echo "$LIVE_BODY" | grep -q '"status"'; then
    echo "  PASS  GET /api/health/live -> has 'status' field"
    PASSED=$((PASSED + 1))
else
    echo "  FAIL  GET /api/health/live -> missing 'status' field in: $LIVE_BODY"
    FAILED=$((FAILED + 1))
fi

if echo "$LIVE_BODY" | grep -q '"ok"'; then
    echo "  PASS  GET /api/health/live -> status value is \"ok\""
    PASSED=$((PASSED + 1))
else
    echo "  FAIL  GET /api/health/live -> status value is not \"ok\": $LIVE_BODY"
    FAILED=$((FAILED + 1))
fi

# Verify readiness response contains expected fields
echo ""
echo "--- Readiness response content ---"
READY_BODY=$(curl -s "$BASE_URL/api/health/ready")
for field in "status" "db_ok" "chaos_active"; do
    if echo "$READY_BODY" | grep -q "\"$field\""; then
        echo "  PASS  GET /api/health/ready -> has '$field' field"
        PASSED=$((PASSED + 1))
    else
        echo "  FAIL  GET /api/health/ready -> missing '$field' field in: $READY_BODY"
        FAILED=$((FAILED + 1))
    fi
done

# Auth endpoint — check unauthenticated access returns 401
# Note: Rocket request-guard rejections return a plain HTTP 401 (not our JSON envelope).
# The JSON envelope is only present for handler-level errors (AppError). Status check suffices.
echo ""
echo "--- Auth endpoint (unauthenticated -> 401) ---"
AUTH_ME_STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/api/auth/me")
if [ "$AUTH_ME_STATUS" = "401" ]; then
    echo "  PASS  GET /api/auth/me -> 401"
    PASSED=$((PASSED + 1))
else
    echo "  FAIL  GET /api/auth/me -> $AUTH_ME_STATUS (expected 401)"
    FAILED=$((FAILED + 1))
fi

# Protected endpoints (expect 401 without token)
echo ""
echo "--- Protected endpoints (unauthenticated -> 401) ---"
check_endpoint GET  "/api/admin/org/" "401"
check_endpoint GET  "/api/users/" "401"
check_endpoint GET  "/api/roles/" "401"
check_endpoint GET  "/api/catalog/" "401"
check_endpoint GET  "/api/packages/" "401"
check_endpoint GET  "/api/plans/" "401"
check_endpoint GET  "/api/delivery/" "401"
check_endpoint GET  "/api/billing/invoices" "401"
check_endpoint GET  "/api/payments/" "401"
check_endpoint GET  "/api/scoring/templates" "401"
check_endpoint POST "/api/reports/export" "401"
check_endpoint GET  "/api/audit/" "401"
check_endpoint GET  "/api/ops/flags" "401"

# Invalid login returns 401 with error body
echo ""
echo "--- Login with bad credentials returns 401 with error body ---"
BAD_LOGIN_BODY=$(curl -s -X POST "$BASE_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"notauser","password":"wrongpass"}')
BAD_LOGIN_STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"notauser","password":"wrongpass"}')
if [ "$BAD_LOGIN_STATUS" = "401" ]; then
    echo "  PASS  POST /api/auth/login (bad creds) -> 401"
    PASSED=$((PASSED + 1))
else
    echo "  FAIL  POST /api/auth/login (bad creds) -> $BAD_LOGIN_STATUS"
    FAILED=$((FAILED + 1))
fi
if echo "$BAD_LOGIN_BODY" | grep -q '"error"'; then
    echo "  PASS  POST /api/auth/login (bad creds) -> error envelope present"
    PASSED=$((PASSED + 1))
else
    echo "  FAIL  POST /api/auth/login (bad creds) -> missing error envelope: $BAD_LOGIN_BODY"
    FAILED=$((FAILED + 1))
fi

echo ""
echo "Results: $PASSED passed, $FAILED failed"

if [ "$FAILED" -gt 0 ]; then
    echo "(Failures may indicate backend is not running)"
    exit 1
fi
