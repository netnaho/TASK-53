#!/usr/bin/env bash
# API tests: Gap coverage — all 27 previously uncovered endpoints
# Requires the full stack running via docker-compose

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

echo "==========================================="
echo " Gap Coverage API Tests (27 endpoints)"
echo "==========================================="
echo "Target: $BASE_URL"
echo ""

# ---------------------------------------------------------------------------
# Login
# ---------------------------------------------------------------------------
ADMIN_RESP=$(curl -sf -X POST "$BASE_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"admin","password":"Admin123!"}' 2>/dev/null || echo "FAIL")
ADMIN_TOKEN=$(echo "$ADMIN_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['token'])" 2>/dev/null || echo "")

if [ -z "$ADMIN_TOKEN" ]; then
    echo "FATAL: could not obtain admin token. Is the backend running?"
    exit 1
fi

# Get admin's org_id and user_id
ME_RESP=$(curl -sf -H "Authorization: Bearer $ADMIN_TOKEN" "$BASE_URL/api/auth/me" 2>/dev/null || echo "{}")
ADMIN_ORG_ID=$(echo "$ME_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('org_id',''))" 2>/dev/null || echo "")
ADMIN_USER_ID=$(echo "$ME_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('id',''))" 2>/dev/null || echo "")

if [ -z "$ADMIN_ORG_ID" ]; then
    echo "FATAL: could not resolve admin org_id"
    exit 1
fi

echo "Admin org_id: $ADMIN_ORG_ID"
echo ""

# ---------------------------------------------------------------------------
# Setup: create all prerequisites in one pass
# ---------------------------------------------------------------------------
echo "--- Setup: creating test data ---"

# Service item
SVC_RESP=$(curl -sf -X POST "$BASE_URL/api/catalog/" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d "{\"code\":\"SVC-GAP-${RUN_SUFFIX}\",\"name\":\"Gap Test Visit\",\"category\":\"nursing\",\"unit_type\":\"visit\",\"default_rate\":80.00}" \
    2>/dev/null || echo "FAIL")
SVC_ID=$(echo "$SVC_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('id',''))" 2>/dev/null || echo "")
echo "  service_item_id: ${SVC_ID:-MISSING}"

# Package
PKG_RESP=$(curl -sf -X POST "$BASE_URL/api/packages/" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d "{\"code\":\"PKG-GAP-${RUN_SUFFIX}\",\"name\":\"Gap Test Package\",\"description\":\"Test\",\"rules\":[{\"service_item_id\":\"$SVC_ID\",\"rule_type\":\"per_visit\",\"rate\":80.00}]}" \
    2>/dev/null || echo "FAIL")
PKG_ID=$(echo "$PKG_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('package',{}).get('id',''))" 2>/dev/null || echo "")
echo "  package_id: ${PKG_ID:-MISSING}"

# Plan
PLAN_RESP=$(curl -sf -X POST "$BASE_URL/api/plans/" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d "{\"client_name\":\"Gap Test Client ${RUN_SUFFIX}\",\"start_date\":\"2024-01-01\",\"end_date\":\"2024-12-31\"}" \
    2>/dev/null || echo "FAIL")
PLAN_ID=$(echo "$PLAN_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('id',''))" 2>/dev/null || echo "")
echo "  plan_id: ${PLAN_ID:-MISSING}"

# Activate plan
if [ -n "$PLAN_ID" ]; then
    curl -sf -X PUT "$BASE_URL/api/plans/$PLAN_ID" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d '{"status":"active"}' 2>/dev/null > /dev/null || true
fi

# Assign package to plan
PP_ID=""
if [ -n "$PLAN_ID" ] && [ -n "$PKG_ID" ]; then
    PP_RESP=$(curl -sf -X POST "$BASE_URL/api/plans/$PLAN_ID/packages" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d "{\"package_id\":\"$PKG_ID\",\"effective_date\":\"2024-01-01\"}" \
        2>/dev/null || echo "FAIL")
    PP_ID=$(echo "$PP_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('id',''))" 2>/dev/null || echo "")
fi
echo "  plan_package_id: ${PP_ID:-MISSING}"

# Delivery entry
ENTRY_ID=""
if [ -n "$PLAN_ID" ] && [ -n "$PP_ID" ] && [ -n "$SVC_ID" ]; then
    ENTRY_RESP=$(curl -sf -X POST "$BASE_URL/api/delivery/" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d "{\"plan_id\":\"$PLAN_ID\",\"plan_package_id\":\"$PP_ID\",\"service_item_id\":\"$SVC_ID\",\"delivery_date\":\"2024-03-10\",\"units\":1.0}" \
        2>/dev/null || echo "FAIL")
    ENTRY_ID=$(echo "$ENTRY_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('id',''))" 2>/dev/null || echo "")
fi
echo "  delivery_entry_id: ${ENTRY_ID:-MISSING}"

# Verify delivery entry
if [ -n "$ENTRY_ID" ]; then
    curl -sf -X PUT "$BASE_URL/api/delivery/$ENTRY_ID" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d '{"status":"verified"}' 2>/dev/null > /dev/null || true
fi

# Generate charges
CHARGE_ID=""
if [ -n "$PLAN_ID" ]; then
    CHRG_RESP=$(curl -sf -X POST "$BASE_URL/api/billing/charges/generate" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d "{\"plan_id\":\"$PLAN_ID\",\"from_date\":\"2024-01-01\",\"to_date\":\"2024-12-31\"}" \
        2>/dev/null || echo "FAIL")
    CHARGE_ID=$(echo "$CHRG_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['charges'][0]['id'] if d.get('charges') else '')" 2>/dev/null || echo "")
fi

# Generate invoice
INVOICE_ID=""
if [ -n "$PLAN_ID" ]; then
    INV_RESP=$(curl -sf -X POST "$BASE_URL/api/billing/invoices/generate" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d "{\"plan_id\":\"$PLAN_ID\",\"billing_period_start\":\"2024-01-01\",\"billing_period_end\":\"2024-12-31\"}" \
        2>/dev/null || echo "FAIL")
    INVOICE_ID=$(echo "$INV_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('invoice',{}).get('id',''))" 2>/dev/null || echo "")
fi
echo "  invoice_id: ${INVOICE_ID:-MISSING}"

# Record payment (prerequisite for refund)
if [ -n "$INVOICE_ID" ]; then
    IDEM_KEY="gap-pay-${RUN_SUFFIX}"
    curl -sf -X POST "$BASE_URL/api/payments/" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d "{\"invoice_id\":\"$INVOICE_ID\",\"idempotency_key\":\"$IDEM_KEY\",\"payment_method\":\"check\",\"amount\":80.00,\"payment_date\":\"2024-04-01\"}" \
        2>/dev/null > /dev/null || true
fi

# Record refund
REFUND_ID=""
if [ -n "$INVOICE_ID" ]; then
    REF_RESP=$(curl -sf -X POST "$BASE_URL/api/payments/refunds" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d "{\"invoice_id\":\"$INVOICE_ID\",\"reason_code\":\"BILLING_ERROR\",\"amount\":5.00,\"refund_method\":\"check\",\"refund_date\":\"2024-04-05\",\"reason_notes\":\"Gap test refund\"}" \
        2>/dev/null || echo "FAIL")
    REFUND_ID=$(echo "$REF_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('id',''))" 2>/dev/null || echo "")
fi
echo "  refund_id: ${REFUND_ID:-MISSING}"

# Reconciliation run
RECON_ID=""
RECON_RESP=$(curl -sf -X POST "$BASE_URL/api/payments/reconciliation" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d '{"period_start":"2024-01-01","period_end":"2024-12-31"}' \
    2>/dev/null || echo "FAIL")
RECON_ID=$(echo "$RECON_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('id',''))" 2>/dev/null || echo "")
echo "  reconciliation_run_id: ${RECON_ID:-MISSING}"

# Create a new user (for PUT /api/users/:id test)
NEW_USER_ID=""
NEW_USER_RESP=$(curl -sf -X POST "$BASE_URL/api/users/" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d "{\"username\":\"gapuser${RUN_SUFFIX}\",\"email\":\"gap${RUN_SUFFIX}@example.com\",\"password\":\"GapTest123!\",\"org_id\":\"$ADMIN_ORG_ID\"}" \
    2>/dev/null || echo "FAIL")
NEW_USER_ID=$(echo "$NEW_USER_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('id',''))" 2>/dev/null || echo "")
echo "  new_user_id: ${NEW_USER_ID:-MISSING}"

echo ""
echo "--- Tests ---"

# ===========================================================================
# [1] GET /api/roles/all — list all permission codes
# ===========================================================================
echo "[1] GET /api/roles/all"
PERMS_RESP=$(curl -sf -H "Authorization: Bearer $ADMIN_TOKEN" \
    "$BASE_URL/api/roles/all" 2>/dev/null || echo "FAIL")
PERMS_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    "$BASE_URL/api/roles/all" 2>/dev/null || echo "000")
check "GET /api/roles/all -> 200" "200" "$PERMS_STATUS"
FIRST_PERM_ID=$(echo "$PERMS_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d[0]['id'] if d else '')" 2>/dev/null || echo "")
if echo "$PERMS_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); assert isinstance(d,list) and len(d)>0" 2>/dev/null; then
    check "GET /api/roles/all returns non-empty array" "true" "true"
else
    check "GET /api/roles/all returns non-empty array" "true" "false"
fi

# ===========================================================================
# [2] POST /api/roles/ — create a new role
# ===========================================================================
echo "[2] POST /api/roles/"
ROLE_RESP=$(curl -sf -X POST "$BASE_URL/api/roles/" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d "{\"name\":\"GapTestRole-${RUN_SUFFIX}\",\"description\":\"Created by test_gaps.sh\"}" \
    2>/dev/null || echo "FAIL")
ROLE_STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE_URL/api/roles/" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d "{\"name\":\"GapTestRole2-${RUN_SUFFIX}\",\"description\":\"Second role\"}" \
    2>/dev/null || echo "000")
ROLE_ID=$(echo "$ROLE_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('id',''))" 2>/dev/null || echo "")
check "POST /api/roles/ returns id" "true" "$([ -n "$ROLE_ID" ] && echo 'true' || echo 'false')"
ROLE_NAME=$(echo "$ROLE_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('name',''))" 2>/dev/null || echo "")
check "POST /api/roles/ response body has name field" "true" "$(echo "$ROLE_NAME" | grep -q "GapTestRole" && echo 'true' || echo 'false')"

# ===========================================================================
# [3] GET /api/roles/:id — get a specific role
# ===========================================================================
echo "[3] GET /api/roles/:id"
if [ -n "$ROLE_ID" ]; then
    ROLE_DETAIL_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$BASE_URL/api/roles/$ROLE_ID" 2>/dev/null || echo "000")
    check "GET /api/roles/:id -> 200" "200" "$ROLE_DETAIL_STATUS"
    ROLE_DETAIL=$(curl -sf -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$BASE_URL/api/roles/$ROLE_ID" 2>/dev/null || echo "{}")
    ROLE_DETAIL_NAME=$(echo "$ROLE_DETAIL" | python3 -c "import sys,json; print(json.load(sys.stdin).get('name',''))" 2>/dev/null || echo "")
    check "GET /api/roles/:id body has name" "true" "$(echo "$ROLE_DETAIL_NAME" | grep -q "GapTestRole" && echo 'true' || echo 'false')"
else
    echo "  SKIP (no ROLE_ID from setup)"
fi

# ===========================================================================
# [4] POST /api/roles/:id/permissions — assign permission to role
# ===========================================================================
echo "[4] POST /api/roles/:id/permissions"
if [ -n "$ROLE_ID" ] && [ -n "$FIRST_PERM_ID" ]; then
    ASSIGN_PERM_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -X POST "$BASE_URL/api/roles/$ROLE_ID/permissions" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d "{\"permission_id\":\"$FIRST_PERM_ID\"}" \
        2>/dev/null || echo "000")
    check "POST /api/roles/:id/permissions -> 204" "204" "$ASSIGN_PERM_STATUS"
else
    echo "  SKIP (missing role_id or perm_id)"
fi

# ===========================================================================
# [5] GET /api/roles/:id/permissions — list permissions for role
# ===========================================================================
echo "[5] GET /api/roles/:id/permissions"
if [ -n "$ROLE_ID" ]; then
    ROLE_PERMS_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$BASE_URL/api/roles/$ROLE_ID/permissions" 2>/dev/null || echo "000")
    check "GET /api/roles/:id/permissions -> 200" "200" "$ROLE_PERMS_STATUS"
    ROLE_PERMS=$(curl -sf -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$BASE_URL/api/roles/$ROLE_ID/permissions" 2>/dev/null || echo "[]")
    if echo "$ROLE_PERMS" | python3 -c "import sys,json; d=json.load(sys.stdin); assert isinstance(d,list)" 2>/dev/null; then
        check "GET /api/roles/:id/permissions returns array" "true" "true"
    else
        check "GET /api/roles/:id/permissions returns array" "true" "false"
    fi
else
    echo "  SKIP (no ROLE_ID)"
fi

# ===========================================================================
# [6] DELETE /api/roles/:id/permissions/:perm_id — revoke permission from role
# ===========================================================================
echo "[6] DELETE /api/roles/:id/permissions/:perm_id"
if [ -n "$ROLE_ID" ] && [ -n "$FIRST_PERM_ID" ]; then
    DEL_PERM_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -X DELETE "$BASE_URL/api/roles/$ROLE_ID/permissions/$FIRST_PERM_ID" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        2>/dev/null || echo "000")
    check "DELETE /api/roles/:id/permissions/:perm_id -> 204" "204" "$DEL_PERM_STATUS"
else
    echo "  SKIP (missing role_id or perm_id)"
fi

# ===========================================================================
# [7] GET /api/admin/org/:id — get a specific org
# ===========================================================================
echo "[7] GET /api/admin/org/:id"
ORG_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    "$BASE_URL/api/admin/org/$ADMIN_ORG_ID" 2>/dev/null || echo "000")
check "GET /api/admin/org/:id -> 200" "200" "$ORG_STATUS"
ORG_DETAIL=$(curl -sf -H "Authorization: Bearer $ADMIN_TOKEN" \
    "$BASE_URL/api/admin/org/$ADMIN_ORG_ID" 2>/dev/null || echo "{}")
ORG_ID_FIELD=$(echo "$ORG_DETAIL" | python3 -c "import sys,json; print(json.load(sys.stdin).get('id',''))" 2>/dev/null || echo "")
check "GET /api/admin/org/:id body has id field" "true" "$([ "$ORG_ID_FIELD" = "$ADMIN_ORG_ID" ] && echo 'true' || echo 'false')"

# ===========================================================================
# [8] PUT /api/admin/org/:id — update an org
# ===========================================================================
echo "[8] PUT /api/admin/org/:id"
PUT_ORG_RESP=$(curl -sf -X PUT "$BASE_URL/api/admin/org/$ADMIN_ORG_ID" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d "{\"name\":\"CareOps Demo Org Updated ${RUN_SUFFIX}\"}" \
    2>/dev/null || echo "FAIL")
PUT_ORG_STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X PUT "$BASE_URL/api/admin/org/$ADMIN_ORG_ID" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d '{"name":"CareOps Demo Org"}' \
    2>/dev/null || echo "000")
# Use the first PUT response to check body
PUT_ORG_ID=$(echo "$PUT_ORG_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('id',''))" 2>/dev/null || echo "")
check "PUT /api/admin/org/:id body has id" "true" "$([ -n "$PUT_ORG_ID" ] && echo 'true' || echo 'false')"
check "PUT /api/admin/org/:id -> 200" "200" "$PUT_ORG_STATUS"

# ===========================================================================
# [9] POST /api/admin/org/:org_id/departments — create a department
# ===========================================================================
echo "[9] POST /api/admin/org/:org_id/departments"
DEPT_RESP=$(curl -sf -X POST "$BASE_URL/api/admin/org/$ADMIN_ORG_ID/departments" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d "{\"name\":\"Gap Test Dept ${RUN_SUFFIX}\",\"org_id\":\"$ADMIN_ORG_ID\"}" \
    2>/dev/null || echo "FAIL")
DEPT_STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE_URL/api/admin/org/$ADMIN_ORG_ID/departments" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d "{\"name\":\"Gap Test Dept 2 ${RUN_SUFFIX}\",\"org_id\":\"$ADMIN_ORG_ID\"}" \
    2>/dev/null || echo "000")
DEPT_ID=$(echo "$DEPT_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('id',''))" 2>/dev/null || echo "")
check "POST /api/admin/org/:org_id/departments returns id" "true" "$([ -n "$DEPT_ID" ] && echo 'true' || echo 'false')"
check "POST /api/admin/org/:org_id/departments -> 200" "200" "$DEPT_STATUS"

# ===========================================================================
# [10] POST /api/admin/org/:org_id/projects — create a project
# ===========================================================================
echo "[10] POST /api/admin/org/:org_id/projects"
PROJ_RESP=$(curl -sf -X POST "$BASE_URL/api/admin/org/$ADMIN_ORG_ID/projects" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d "{\"name\":\"Gap Test Project ${RUN_SUFFIX}\",\"org_id\":\"$ADMIN_ORG_ID\"}" \
    2>/dev/null || echo "FAIL")
PROJ_STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE_URL/api/admin/org/$ADMIN_ORG_ID/projects" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d "{\"name\":\"Gap Test Project 2 ${RUN_SUFFIX}\",\"org_id\":\"$ADMIN_ORG_ID\"}" \
    2>/dev/null || echo "000")
PROJ_ID=$(echo "$PROJ_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('id',''))" 2>/dev/null || echo "")
check "POST /api/admin/org/:org_id/projects returns id" "true" "$([ -n "$PROJ_ID" ] && echo 'true' || echo 'false')"
check "POST /api/admin/org/:org_id/projects -> 200" "200" "$PROJ_STATUS"

# ===========================================================================
# [11] GET /api/admin/org/:org_id/projects — list projects for org
# ===========================================================================
echo "[11] GET /api/admin/org/:org_id/projects"
PROJS_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    "$BASE_URL/api/admin/org/$ADMIN_ORG_ID/projects" 2>/dev/null || echo "000")
check "GET /api/admin/org/:org_id/projects -> 200" "200" "$PROJS_STATUS"
PROJS=$(curl -sf -H "Authorization: Bearer $ADMIN_TOKEN" \
    "$BASE_URL/api/admin/org/$ADMIN_ORG_ID/projects" 2>/dev/null || echo "[]")
if echo "$PROJS" | python3 -c "import sys,json; d=json.load(sys.stdin); assert isinstance(d,list)" 2>/dev/null; then
    check "GET /api/admin/org/:org_id/projects returns array" "true" "true"
else
    check "GET /api/admin/org/:org_id/projects returns array" "true" "false"
fi

# ===========================================================================
# [12] GET /api/catalog/:id — get a specific service item
# ===========================================================================
echo "[12] GET /api/catalog/:id"
if [ -n "$SVC_ID" ]; then
    SVC_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$BASE_URL/api/catalog/$SVC_ID" 2>/dev/null || echo "000")
    check "GET /api/catalog/:id -> 200" "200" "$SVC_STATUS"
    SVC_DETAIL=$(curl -sf -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$BASE_URL/api/catalog/$SVC_ID" 2>/dev/null || echo "{}")
    SVC_CODE=$(echo "$SVC_DETAIL" | python3 -c "import sys,json; print(json.load(sys.stdin).get('code',''))" 2>/dev/null || echo "")
    check "GET /api/catalog/:id body has code field" "true" "$(echo "$SVC_CODE" | grep -q "SVC-GAP" && echo 'true' || echo 'false')"
else
    echo "  SKIP (no SVC_ID from setup)"
fi

# ===========================================================================
# [13] PUT /api/catalog/:id — update a service item
# ===========================================================================
echo "[13] PUT /api/catalog/:id"
if [ -n "$SVC_ID" ]; then
    PUT_SVC_RESP=$(curl -sf -X PUT "$BASE_URL/api/catalog/$SVC_ID" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d '{"name":"Gap Test Visit Updated","default_rate":85.00}' \
        2>/dev/null || echo "FAIL")
    PUT_SVC_STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X PUT "$BASE_URL/api/catalog/$SVC_ID" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d '{"name":"Gap Test Visit"}' \
        2>/dev/null || echo "000")
    check "PUT /api/catalog/:id -> 200" "200" "$PUT_SVC_STATUS"
    PUT_SVC_NAME=$(echo "$PUT_SVC_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('name',''))" 2>/dev/null || echo "")
    check "PUT /api/catalog/:id body has updated name" "true" "$(echo "$PUT_SVC_NAME" | grep -q "Updated" && echo 'true' || echo 'false')"
else
    echo "  SKIP (no SVC_ID from setup)"
fi

# ===========================================================================
# [14] GET /api/packages/ — list packages
# ===========================================================================
echo "[14] GET /api/packages/"
PKG_LIST_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    "$BASE_URL/api/packages/" 2>/dev/null || echo "000")
check "GET /api/packages/ -> 200" "200" "$PKG_LIST_STATUS"
PKG_LIST=$(curl -sf -H "Authorization: Bearer $ADMIN_TOKEN" \
    "$BASE_URL/api/packages/" 2>/dev/null || echo "[]")
if echo "$PKG_LIST" | python3 -c "import sys,json; d=json.load(sys.stdin); assert isinstance(d,list)" 2>/dev/null; then
    check "GET /api/packages/ returns array" "true" "true"
else
    check "GET /api/packages/ returns array" "true" "false"
fi

# ===========================================================================
# [15] GET /api/packages/:id — get a specific package
# ===========================================================================
echo "[15] GET /api/packages/:id"
if [ -n "$PKG_ID" ]; then
    PKG_DETAIL_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$BASE_URL/api/packages/$PKG_ID" 2>/dev/null || echo "000")
    check "GET /api/packages/:id -> 200" "200" "$PKG_DETAIL_STATUS"
    PKG_DETAIL=$(curl -sf -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$BASE_URL/api/packages/$PKG_ID" 2>/dev/null || echo "{}")
    PKG_CODE=$(echo "$PKG_DETAIL" | python3 -c "import sys,json; print(json.load(sys.stdin).get('package',{}).get('code',''))" 2>/dev/null || echo "")
    check "GET /api/packages/:id body has package.code" "true" "$(echo "$PKG_CODE" | grep -q "PKG-GAP" && echo 'true' || echo 'false')"
else
    echo "  SKIP (no PKG_ID from setup)"
fi

# ===========================================================================
# [16] GET /api/packages/:id/rules — get package rules
# ===========================================================================
echo "[16] GET /api/packages/:id/rules"
if [ -n "$PKG_ID" ]; then
    RULES_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$BASE_URL/api/packages/$PKG_ID/rules" 2>/dev/null || echo "000")
    check "GET /api/packages/:id/rules -> 200" "200" "$RULES_STATUS"
    RULES=$(curl -sf -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$BASE_URL/api/packages/$PKG_ID/rules" 2>/dev/null || echo "[]")
    if echo "$RULES" | python3 -c "import sys,json; d=json.load(sys.stdin); assert isinstance(d,list) and len(d)>0" 2>/dev/null; then
        check "GET /api/packages/:id/rules returns non-empty array" "true" "true"
    else
        check "GET /api/packages/:id/rules returns non-empty array" "true" "false"
    fi
else
    echo "  SKIP (no PKG_ID from setup)"
fi

# ===========================================================================
# [17] PUT /api/packages/:id — update a package
# ===========================================================================
echo "[17] PUT /api/packages/:id"
if [ -n "$PKG_ID" ]; then
    PUT_PKG_RESP=$(curl -sf -X PUT "$BASE_URL/api/packages/$PKG_ID" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d '{"name":"Gap Test Package Updated","description":"Updated by test_gaps.sh"}' \
        2>/dev/null || echo "FAIL")
    PUT_PKG_STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X PUT "$BASE_URL/api/packages/$PKG_ID" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d '{"name":"Gap Test Package"}' \
        2>/dev/null || echo "000")
    check "PUT /api/packages/:id -> 200" "200" "$PUT_PKG_STATUS"
    PUT_PKG_NAME=$(echo "$PUT_PKG_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('name',''))" 2>/dev/null || echo "")
    check "PUT /api/packages/:id body has updated name" "true" "$(echo "$PUT_PKG_NAME" | grep -q "Updated" && echo 'true' || echo 'false')"
else
    echo "  SKIP (no PKG_ID from setup)"
fi

# ===========================================================================
# [18] GET /api/plans/ — list plans
# ===========================================================================
echo "[18] GET /api/plans/"
PLANS_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    "$BASE_URL/api/plans/" 2>/dev/null || echo "000")
check "GET /api/plans/ -> 200" "200" "$PLANS_STATUS"
PLANS=$(curl -sf -H "Authorization: Bearer $ADMIN_TOKEN" \
    "$BASE_URL/api/plans/" 2>/dev/null || echo "[]")
if echo "$PLANS" | python3 -c "import sys,json; d=json.load(sys.stdin); assert isinstance(d,list)" 2>/dev/null; then
    check "GET /api/plans/ returns array" "true" "true"
else
    check "GET /api/plans/ returns array" "true" "false"
fi

# ===========================================================================
# [19] GET /api/plans/:id — get a specific plan
# ===========================================================================
echo "[19] GET /api/plans/:id"
if [ -n "$PLAN_ID" ]; then
    PLAN_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$BASE_URL/api/plans/$PLAN_ID" 2>/dev/null || echo "000")
    check "GET /api/plans/:id -> 200" "200" "$PLAN_STATUS"
    PLAN_DETAIL=$(curl -sf -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$BASE_URL/api/plans/$PLAN_ID" 2>/dev/null || echo "{}")
    PLAN_CLIENT=$(echo "$PLAN_DETAIL" | python3 -c "import sys,json; print(json.load(sys.stdin).get('client_name',''))" 2>/dev/null || echo "")
    check "GET /api/plans/:id body has client_name" "true" "$([ -n "$PLAN_CLIENT" ] && echo 'true' || echo 'false')"
else
    echo "  SKIP (no PLAN_ID from setup)"
fi

# ===========================================================================
# [20] GET /api/plans/:id/packages — list packages for plan
# ===========================================================================
echo "[20] GET /api/plans/:id/packages"
if [ -n "$PLAN_ID" ]; then
    PP_LIST_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$BASE_URL/api/plans/$PLAN_ID/packages" 2>/dev/null || echo "000")
    check "GET /api/plans/:id/packages -> 200" "200" "$PP_LIST_STATUS"
    PP_LIST=$(curl -sf -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$BASE_URL/api/plans/$PLAN_ID/packages" 2>/dev/null || echo "[]")
    if echo "$PP_LIST" | python3 -c "import sys,json; d=json.load(sys.stdin); assert isinstance(d,list) and len(d)>0" 2>/dev/null; then
        check "GET /api/plans/:id/packages returns non-empty array" "true" "true"
    else
        check "GET /api/plans/:id/packages returns non-empty array" "true" "false"
    fi
else
    echo "  SKIP (no PLAN_ID from setup)"
fi

# ===========================================================================
# [21] GET /api/delivery/:id — get a specific delivery entry
# ===========================================================================
echo "[21] GET /api/delivery/:id"
if [ -n "$ENTRY_ID" ]; then
    ENTRY_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$BASE_URL/api/delivery/$ENTRY_ID" 2>/dev/null || echo "000")
    check "GET /api/delivery/:id -> 200" "200" "$ENTRY_STATUS"
    ENTRY_DETAIL=$(curl -sf -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$BASE_URL/api/delivery/$ENTRY_ID" 2>/dev/null || echo "{}")
    ENTRY_PLAN=$(echo "$ENTRY_DETAIL" | python3 -c "import sys,json; print(json.load(sys.stdin).get('plan_id',''))" 2>/dev/null || echo "")
    check "GET /api/delivery/:id body has plan_id" "true" "$([ "$ENTRY_PLAN" = "$PLAN_ID" ] && echo 'true' || echo 'false')"
else
    echo "  SKIP (no ENTRY_ID from setup)"
fi

# ===========================================================================
# [22] POST /api/delivery/:id/notes — create a note on a delivery entry
# ===========================================================================
echo "[22] POST /api/delivery/:id/notes"
NOTE_ID=""
if [ -n "$ENTRY_ID" ]; then
    NOTE_RESP=$(curl -sf -X POST "$BASE_URL/api/delivery/$ENTRY_ID/notes" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d "{\"delivery_entry_id\":\"$ENTRY_ID\",\"note\":\"Gap test eligibility note\",\"note_type\":\"eligibility\"}" \
        2>/dev/null || echo "FAIL")
    NOTE_STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE_URL/api/delivery/$ENTRY_ID/notes" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d "{\"delivery_entry_id\":\"$ENTRY_ID\",\"note\":\"Second note\"}" \
        2>/dev/null || echo "000")
    NOTE_ID=$(echo "$NOTE_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('id',''))" 2>/dev/null || echo "")
    check "POST /api/delivery/:id/notes returns id" "true" "$([ -n "$NOTE_ID" ] && echo 'true' || echo 'false')"
    check "POST /api/delivery/:id/notes -> 200" "200" "$NOTE_STATUS"
else
    echo "  SKIP (no ENTRY_ID from setup)"
fi

# ===========================================================================
# [23] GET /api/delivery/:id/notes — list notes for a delivery entry
# ===========================================================================
echo "[23] GET /api/delivery/:id/notes"
if [ -n "$ENTRY_ID" ]; then
    NOTES_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$BASE_URL/api/delivery/$ENTRY_ID/notes" 2>/dev/null || echo "000")
    check "GET /api/delivery/:id/notes -> 200" "200" "$NOTES_STATUS"
    NOTES=$(curl -sf -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$BASE_URL/api/delivery/$ENTRY_ID/notes" 2>/dev/null || echo "[]")
    if echo "$NOTES" | python3 -c "import sys,json; d=json.load(sys.stdin); assert isinstance(d,list)" 2>/dev/null; then
        check "GET /api/delivery/:id/notes returns array" "true" "true"
    else
        check "GET /api/delivery/:id/notes returns array" "true" "false"
    fi
else
    echo "  SKIP (no ENTRY_ID from setup)"
fi

# ===========================================================================
# [24] GET /api/billing/invoices/:invoice_id — get a specific invoice
# ===========================================================================
echo "[24] GET /api/billing/invoices/:invoice_id"
if [ -n "$INVOICE_ID" ]; then
    INV_DETAIL_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$BASE_URL/api/billing/invoices/$INVOICE_ID" 2>/dev/null || echo "000")
    check "GET /api/billing/invoices/:invoice_id -> 200" "200" "$INV_DETAIL_STATUS"
    INV_DETAIL=$(curl -sf -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$BASE_URL/api/billing/invoices/$INVOICE_ID" 2>/dev/null || echo "{}")
    INV_NUM=$(echo "$INV_DETAIL" | python3 -c "import sys,json; print(json.load(sys.stdin).get('invoice',{}).get('invoice_number','') or json.load(sys.stdin).get('invoice_number',''))" 2>/dev/null || echo "")
    # Try both "invoice.invoice_number" and flat "invoice_number"
    INV_NUM2=$(echo "$INV_DETAIL" | python3 -c "
import sys, json
d = json.load(sys.stdin)
v = d.get('invoice', d)
print(v.get('invoice_number', ''))
" 2>/dev/null || echo "")
    check "GET /api/billing/invoices/:invoice_id body has invoice_number" "true" "$([ -n "$INV_NUM2" ] && echo 'true' || echo 'false')"
else
    echo "  SKIP (no INVOICE_ID from setup)"
fi

# ===========================================================================
# [25] GET /api/payments/reconciliation/:run_id — get a specific recon run
# ===========================================================================
echo "[25] GET /api/payments/reconciliation/:run_id"
if [ -n "$RECON_ID" ]; then
    RECON_DETAIL_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$BASE_URL/api/payments/reconciliation/$RECON_ID" 2>/dev/null || echo "000")
    check "GET /api/payments/reconciliation/:run_id -> 200" "200" "$RECON_DETAIL_STATUS"
    RECON_DETAIL=$(curl -sf -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$BASE_URL/api/payments/reconciliation/$RECON_ID" 2>/dev/null || echo "{}")
    RECON_ORG=$(echo "$RECON_DETAIL" | python3 -c "import sys,json; print(json.load(sys.stdin).get('org_id',''))" 2>/dev/null || echo "")
    check "GET /api/payments/reconciliation/:run_id body has org_id" "true" "$([ "$RECON_ORG" = "$ADMIN_ORG_ID" ] && echo 'true' || echo 'false')"
else
    echo "  SKIP (no RECON_ID from setup)"
fi

# ===========================================================================
# [26] GET /api/payments/refunds/:refund_id — get a specific refund
# ===========================================================================
echo "[26] GET /api/payments/refunds/:refund_id"
if [ -n "$REFUND_ID" ]; then
    REFUND_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$BASE_URL/api/payments/refunds/$REFUND_ID" 2>/dev/null || echo "000")
    check "GET /api/payments/refunds/:refund_id -> 200" "200" "$REFUND_STATUS"
    REFUND_DETAIL=$(curl -sf -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$BASE_URL/api/payments/refunds/$REFUND_ID" 2>/dev/null || echo "{}")
    REFUND_AMOUNT=$(echo "$REFUND_DETAIL" | python3 -c "import sys,json; print(json.load(sys.stdin).get('amount',''))" 2>/dev/null || echo "")
    check "GET /api/payments/refunds/:refund_id body has amount" "true" "$([ -n "$REFUND_AMOUNT" ] && echo 'true' || echo 'false')"
else
    echo "  SKIP (no REFUND_ID from setup)"
fi

# ===========================================================================
# [27] PUT /api/users/:id — update a user
# ===========================================================================
echo "[27] PUT /api/users/:id"
if [ -n "$NEW_USER_ID" ]; then
    PUT_USER_RESP=$(curl -sf -X PUT "$BASE_URL/api/users/$NEW_USER_ID" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d "{\"email\":\"gap_updated_${RUN_SUFFIX}@example.com\",\"status\":\"active\"}" \
        2>/dev/null || echo "FAIL")
    PUT_USER_STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X PUT "$BASE_URL/api/users/$NEW_USER_ID" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d '{"status":"active"}' \
        2>/dev/null || echo "000")
    check "PUT /api/users/:id -> 200" "200" "$PUT_USER_STATUS"
    PUT_USER_EMAIL=$(echo "$PUT_USER_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('email',''))" 2>/dev/null || echo "")
    check "PUT /api/users/:id body has email field" "true" "$([ -n "$PUT_USER_EMAIL" ] && echo 'true' || echo 'false')"
else
    echo "  SKIP (no NEW_USER_ID from setup)"
fi

# ===========================================================================
# Summary
# ===========================================================================
echo ""
echo "==========================================="
echo "Results: $PASSED passed, $FAILED failed"
echo "==========================================="
[ "$FAILED" -eq 0 ] || exit 1
