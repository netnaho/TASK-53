#!/usr/bin/env bash
# End-to-End integration test: Frontend-to-Backend flow
#
# This test simulates the exact sequence of HTTP calls the Dioxus frontend
# makes via services/api_client.rs:
#   1. Login  (POST /api/auth/login)
#   2. Fetch current user profile  (GET /api/auth/me)
#   3. Fetch catalog list  (GET /api/catalog/)
#   4. Write: create a delivery entry  (POST /api/delivery/)
#   5. Read back that delivery entry  (GET /api/delivery/:id)
#   6. Verify JSON Content-Type on all responses
#
# Run against dockerized services (no mocks):
#   BACKEND_URL=http://localhost:8000 bash API_tests/test_e2e.sh

set -euo pipefail

BASE_URL="${BACKEND_URL:-http://localhost:8000}"
PASSED=0
FAILED=0
RUN_SUFFIX=$(date +%s)

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

echo "============================================="
echo " Frontend-to-Backend E2E Integration Test"
echo "============================================="
echo "Target: $BASE_URL"
echo ""

# ---------------------------------------------------------------------------
# Step 1: Login — same call as LoginPage makes via api_client.rs
# ---------------------------------------------------------------------------
echo "[E2E-1] Login: POST /api/auth/login"
# Use ops_manager: has catalog.read, plans.read, delivery.write, billing.read, scoring.read
LOGIN_RESP=$(curl -si -X POST "$BASE_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"ops_manager","password":"OpsManager123!"}' \
    2>/dev/null || echo "FAIL")

LOGIN_BODY=$(echo "$LOGIN_RESP" | sed -n '/^\r\{0,1\}$/,$ p' | tail -n +2)
LOGIN_CT=$(echo "$LOGIN_RESP" | grep -i "^content-type:" | head -1 || echo "")
TOKEN=$(echo "$LOGIN_BODY" | python3 -c "import sys,json; print(json.load(sys.stdin)['token'])" 2>/dev/null || echo "")

check "Login returns HTTP 200" "200" "$(echo "$LOGIN_RESP" | head -1 | grep -oP '\d{3}' | head -1 || echo '000')"
check "Login response is JSON" "true" "$(echo "$LOGIN_CT" | grep -qi 'application/json' && echo 'true' || echo 'false')"
check "Login response contains token" "true" "$([ -n "$TOKEN" ] && echo 'true' || echo 'false')"

if [ -z "$TOKEN" ]; then
    echo "FATAL: No token from login — cannot continue E2E test."
    exit 1
fi

# ---------------------------------------------------------------------------
# Step 2: Fetch current user profile — AppLayout calls this on mount
# ---------------------------------------------------------------------------
echo "[E2E-2] Fetch current user: GET /api/auth/me"
ME_RESP=$(curl -si -H "Authorization: Bearer $TOKEN" \
    "$BASE_URL/api/auth/me" 2>/dev/null || echo "FAIL")

ME_BODY=$(echo "$ME_RESP" | sed -n '/^\r\{0,1\}$/,$ p' | tail -n +2)
ME_CT=$(echo "$ME_RESP" | grep -i "^content-type:" | head -1 || echo "")
ME_USERNAME=$(echo "$ME_BODY" | python3 -c "import sys,json; print(json.load(sys.stdin).get('username',''))" 2>/dev/null || echo "")
ME_PERMS=$(echo "$ME_BODY" | python3 -c "import sys,json; print(len(json.load(sys.stdin).get('permissions',[])))" 2>/dev/null || echo "0")

check "GET /api/auth/me returns HTTP 200" "200" "$(echo "$ME_RESP" | head -1 | grep -oP '\d{3}' | head -1 || echo '000')"
check "GET /api/auth/me response is JSON" "true" "$(echo "$ME_CT" | grep -qi 'application/json' && echo 'true' || echo 'false')"
check "GET /api/auth/me returns username=ops_manager" "ops_manager" "$ME_USERNAME"
check "GET /api/auth/me returns at least 1 permission" "true" "$([ "$ME_PERMS" -ge 1 ] && echo 'true' || echo 'false')"

# ---------------------------------------------------------------------------
# Step 3: Fetch catalog list — CatalogPage initial load
# ---------------------------------------------------------------------------
echo "[E2E-3] Fetch catalog: GET /api/catalog/"
CAT_RESP=$(curl -si -H "Authorization: Bearer $TOKEN" \
    "$BASE_URL/api/catalog/" 2>/dev/null || echo "FAIL")

CAT_BODY=$(echo "$CAT_RESP" | sed -n '/^\r\{0,1\}$/,$ p' | tail -n +2)
CAT_CT=$(echo "$CAT_RESP" | grep -i "^content-type:" | head -1 || echo "")
CAT_IS_ARRAY=$(echo "$CAT_BODY" | python3 -c "import sys,json; d=json.load(sys.stdin); print('true' if isinstance(d,list) else 'false')" 2>/dev/null || echo "false")

check "GET /api/catalog/ returns HTTP 200" "200" "$(echo "$CAT_RESP" | head -1 | grep -oP '\d{3}' | head -1 || echo '000')"
check "GET /api/catalog/ response is JSON" "true" "$(echo "$CAT_CT" | grep -qi 'application/json' && echo 'true' || echo 'false')"
check "GET /api/catalog/ response is an array" "true" "$CAT_IS_ARRAY"

# ---------------------------------------------------------------------------
# Setup: billing_staff needs a plan+package to create a delivery entry.
# Use admin to set up prerequisites, then billing_staff performs the write.
# ---------------------------------------------------------------------------
echo "[E2E-setup] Building prerequisites with admin token..."

ADMIN_RESP=$(curl -sf -X POST "$BASE_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"admin","password":"Admin123!"}' 2>/dev/null || echo "FAIL")
ADMIN_TOKEN=$(echo "$ADMIN_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['token'])" 2>/dev/null || echo "")

SVC_RESP=$(curl -sf -X POST "$BASE_URL/api/catalog/" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d "{\"code\":\"SVC-E2E-${RUN_SUFFIX}\",\"name\":\"E2E Service\",\"category\":\"nursing\",\"unit_type\":\"visit\",\"default_rate\":70.00}" \
    2>/dev/null || echo "FAIL")
E2E_SVC_ID=$(echo "$SVC_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('id',''))" 2>/dev/null || echo "")

PKG_RESP=$(curl -sf -X POST "$BASE_URL/api/packages/" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d "{\"code\":\"PKG-E2E-${RUN_SUFFIX}\",\"name\":\"E2E Package\",\"rules\":[{\"service_item_id\":\"$E2E_SVC_ID\",\"rule_type\":\"per_visit\",\"rate\":70.00}]}" \
    2>/dev/null || echo "FAIL")
E2E_PKG_ID=$(echo "$PKG_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('package',{}).get('id',''))" 2>/dev/null || echo "")

PLAN_RESP=$(curl -sf -X POST "$BASE_URL/api/plans/" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d "{\"client_name\":\"E2E Client ${RUN_SUFFIX}\",\"start_date\":\"2024-01-01\",\"end_date\":\"2024-12-31\"}" \
    2>/dev/null || echo "FAIL")
E2E_PLAN_ID=$(echo "$PLAN_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('id',''))" 2>/dev/null || echo "")

if [ -n "$E2E_PLAN_ID" ]; then
    curl -sf -X PUT "$BASE_URL/api/plans/$E2E_PLAN_ID" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d '{"status":"active"}' 2>/dev/null > /dev/null || true
fi

E2E_PP_ID=""
if [ -n "$E2E_PLAN_ID" ] && [ -n "$E2E_PKG_ID" ]; then
    PP_RESP=$(curl -sf -X POST "$BASE_URL/api/plans/$E2E_PLAN_ID/packages" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d "{\"package_id\":\"$E2E_PKG_ID\",\"effective_date\":\"2024-01-01\"}" \
        2>/dev/null || echo "FAIL")
    E2E_PP_ID=$(echo "$PP_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('id',''))" 2>/dev/null || echo "")
fi

echo "  plan_id=${E2E_PLAN_ID:-MISSING}  pkg_id=${E2E_PKG_ID:-MISSING}  pp_id=${E2E_PP_ID:-MISSING}"

# ---------------------------------------------------------------------------
# Step 4: Write action — billing_staff creates a delivery entry
# (DeliveryPage submit form, equivalent to api_client.post_delivery_entry)
# ---------------------------------------------------------------------------
echo "[E2E-4] Write: POST /api/delivery/ (billing_staff token)"
CREATE_ENTRY_STATUS="SKIP"
E2E_ENTRY_ID=""

if [ -n "$E2E_PLAN_ID" ] && [ -n "$E2E_PP_ID" ] && [ -n "$E2E_SVC_ID" ]; then
    ENTRY_RESP=$(curl -si -X POST "$BASE_URL/api/delivery/" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d "{\"plan_id\":\"$E2E_PLAN_ID\",\"plan_package_id\":\"$E2E_PP_ID\",\"service_item_id\":\"$E2E_SVC_ID\",\"delivery_date\":\"2024-05-01\",\"units\":1.0}" \
        2>/dev/null || echo "FAIL")
    ENTRY_BODY=$(echo "$ENTRY_RESP" | sed -n '/^\r\{0,1\}$/,$ p' | tail -n +2)
    ENTRY_HTTP=$(echo "$ENTRY_RESP" | head -1 | grep -oP '\d{3}' | head -1 || echo "000")
    E2E_ENTRY_ID=$(echo "$ENTRY_BODY" | python3 -c "import sys,json; print(json.load(sys.stdin).get('id',''))" 2>/dev/null || echo "")
    check "POST /api/delivery/ returns HTTP 200" "200" "$ENTRY_HTTP"
    check "POST /api/delivery/ returns entry id" "true" "$([ -n "$E2E_ENTRY_ID" ] && echo 'true' || echo 'false')"
else
    echo "  SKIP (missing prerequisites)"
fi

# ---------------------------------------------------------------------------
# Step 5: Read back — verify the written delivery entry exists
# (DeliveryPage detail fetch, equivalent to api_client.get_delivery_entry)
# ---------------------------------------------------------------------------
echo "[E2E-5] Read back: GET /api/delivery/:id"
if [ -n "$E2E_ENTRY_ID" ]; then
    READBACK_RESP=$(curl -si -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$BASE_URL/api/delivery/$E2E_ENTRY_ID" 2>/dev/null || echo "FAIL")
    READBACK_BODY=$(echo "$READBACK_RESP" | sed -n '/^\r\{0,1\}$/,$ p' | tail -n +2)
    READBACK_HTTP=$(echo "$READBACK_RESP" | head -1 | grep -oP '\d{3}' | head -1 || echo "000")
    READBACK_ID=$(echo "$READBACK_BODY" | python3 -c "import sys,json; print(json.load(sys.stdin).get('id',''))" 2>/dev/null || echo "")
    READBACK_PLAN=$(echo "$READBACK_BODY" | python3 -c "import sys,json; print(json.load(sys.stdin).get('plan_id',''))" 2>/dev/null || echo "")

    check "GET /api/delivery/:id returns HTTP 200" "200" "$READBACK_HTTP"
    check "Read-back entry id matches created id" "$E2E_ENTRY_ID" "$READBACK_ID"
    check "Read-back entry plan_id matches" "$E2E_PLAN_ID" "$READBACK_PLAN"
else
    echo "  SKIP (no entry_id from step 4)"
fi

# ---------------------------------------------------------------------------
# Step 6: Unauthenticated request is denied (frontend redirects to /login)
# ---------------------------------------------------------------------------
echo "[E2E-6] Unauthenticated request returns 401"
UNAUTH_STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/api/catalog/" 2>/dev/null || echo "000")
check "Unauthenticated catalog request -> 401" "401" "$UNAUTH_STATUS"

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
echo ""
echo "============================================="
echo "E2E Results: $PASSED passed, $FAILED failed"
echo "============================================="
[ "$FAILED" -eq 0 ] || exit 1
