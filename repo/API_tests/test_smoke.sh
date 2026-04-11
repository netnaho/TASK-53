#!/usr/bin/env bash
# API smoke tests: verify all registered routes respond with correct HTTP status codes
# Requires the full stack running via docker-compose
# Note: Most endpoints now require authentication, so they return 401 instead of 501

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

echo "API Smoke Tests"
echo "==============="
echo "Target: $BASE_URL"
echo ""

# Health endpoints (public, no auth)
check_endpoint GET "/api/health/live" "200"
check_endpoint GET "/api/health/ready" "200"

# Auth endpoints
check_endpoint GET "/api/auth/me" "401"

# Protected endpoints (expect 401 without token)
# Each path targets a concrete mounted handler so the auth guard fires.
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

echo ""
echo "Results: $PASSED passed, $FAILED failed"

if [ "$FAILED" -gt 0 ]; then
    echo "(Failures may indicate backend is not running)"
    exit 1
fi
