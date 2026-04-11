#!/usr/bin/env bash
# API tests: Authentication, authorization boundaries, and security endpoints
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

echo "==========================="
echo " Auth & Security API Tests"
echo "==========================="
echo "Target: $BASE_URL"
echo ""

# --- Test 1: Login with valid credentials ---
echo "[1] Login with valid credentials"
LOGIN_RESP=$(curl -sf -X POST "$BASE_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"admin","password":"Admin123!"}' 2>/dev/null || echo "FAIL")

if echo "$LOGIN_RESP" | grep -q '"token"'; then
    TOKEN=$(echo "$LOGIN_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['token'])" 2>/dev/null || echo "")
    check "Login returns token" "true" "true"
else
    check "Login returns token" "true" "false"
    TOKEN=""
fi

# --- Test 2: Login with invalid password ---
echo "[2] Login with invalid password"
STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"admin","password":"wrong"}' 2>/dev/null || echo "000")
check "Invalid password returns 401" "401" "$STATUS"

# --- Test 3: Login with nonexistent user ---
echo "[3] Login with nonexistent user"
STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"nouser","password":"test"}' 2>/dev/null || echo "000")
check "Nonexistent user returns 401" "401" "$STATUS"

# --- Test 4: Access protected endpoint without token ---
echo "[4] Access /api/users without token"
STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/api/users/")
check "No token returns 401" "401" "$STATUS"

# --- Test 5: Access protected endpoint with valid token ---
echo "[5] Access /api/users with admin token"
if [ -n "$TOKEN" ]; then
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $TOKEN" \
        "$BASE_URL/api/users/" 2>/dev/null || echo "000")
    check "Admin can access users" "200" "$STATUS"
else
    echo "  SKIP (no token from login)"
fi

# --- Test 6: Get current user profile ---
echo "[6] GET /api/auth/me with admin token"
if [ -n "$TOKEN" ]; then
    ME_RESP=$(curl -sf -H "Authorization: Bearer $TOKEN" "$BASE_URL/api/auth/me" 2>/dev/null || echo "FAIL")
    if echo "$ME_RESP" | grep -q '"permissions"'; then
        check "Current user has permissions array" "true" "true"
    else
        check "Current user has permissions array" "true" "false"
    fi
else
    echo "  SKIP (no token)"
fi

# --- Test 7: Unauthorized role cannot access admin endpoints ---
echo "[7] Auditor cannot write users"
AUDITOR_RESP=$(curl -sf -X POST "$BASE_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"auditor","password":"Auditor123!"}' 2>/dev/null || echo "FAIL")
AUDITOR_TOKEN=$(echo "$AUDITOR_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['token'])" 2>/dev/null || echo "")

if [ -n "$AUDITOR_TOKEN" ]; then
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $AUDITOR_TOKEN" \
        -H "Content-Type: application/json" \
        -X POST "$BASE_URL/api/users/" \
        -d '{"username":"test","email":"test@test.com","password":"Test123!","org_id":"fake"}' \
        2>/dev/null || echo "000")
    check "Auditor cannot create users (403)" "403" "$STATUS"
else
    echo "  SKIP (auditor login failed)"
fi

# --- Test 8: Access audit endpoint ---
echo "[8] Auditor can access audit logs"
if [ -n "$AUDITOR_TOKEN" ]; then
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $AUDITOR_TOKEN" \
        "$BASE_URL/api/audit/" 2>/dev/null || echo "000")
    check "Auditor can read audit logs" "200" "$STATUS"
else
    echo "  SKIP (no auditor token)"
fi

# --- Test 9: Coach cannot access admin ---
echo "[9] Coach cannot access admin org"
COACH_RESP=$(curl -sf -X POST "$BASE_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"coach","password":"Coach123!"}' 2>/dev/null || echo "FAIL")
COACH_TOKEN=$(echo "$COACH_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['token'])" 2>/dev/null || echo "")

if [ -n "$COACH_TOKEN" ]; then
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $COACH_TOKEN" \
        "$BASE_URL/api/admin/org/" 2>/dev/null || echo "000")
    check "Coach cannot access admin org (403)" "403" "$STATUS"
else
    echo "  SKIP (coach login failed)"
fi

# --- Test 10: Logout invalidates session ---
echo "[10] Logout invalidates token"
if [ -n "$TOKEN" ]; then
    # Logout
    curl -sf -X POST -H "Authorization: Bearer $TOKEN" "$BASE_URL/api/auth/logout" 2>/dev/null || true
    # Try to use the same token
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $TOKEN" \
        "$BASE_URL/api/auth/me" 2>/dev/null || echo "000")
    check "Revoked token returns 401" "401" "$STATUS"
else
    echo "  SKIP (no token)"
fi

# --- Test 11: Health endpoints remain public ---
echo "[11] Health endpoints don't require auth"
STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/api/health/live")
check "Health endpoint is public" "200" "$STATUS"

# =============================================================================
# Cross-org tenant boundary enforcement tests (TASK-53)
# These tests verify that role/scope management endpoints enforce data-scope
# checks, blocking cross-org privilege manipulation.
# =============================================================================

# Re-login admin (token may have been invalidated by logout test above)
echo ""
echo "--- Cross-Org Boundary Tests (TASK-53) ---"
ADMIN_RESP=$(curl -sf -X POST "$BASE_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d '{"username":"admin","password":"Admin123!"}' 2>/dev/null || echo "FAIL")
ADMIN_TOKEN=$(echo "$ADMIN_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['token'])" 2>/dev/null || echo "")

ADMIN_ID=""
if [ -n "$ADMIN_TOKEN" ]; then
    ADMIN_ID=$(curl -sf -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$BASE_URL/api/auth/me" 2>/dev/null \
        | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('id') or d.get('user_id') or '')" 2>/dev/null || echo "")
fi

# --- Test 12: Admin can read own-org user roles (positive baseline) ---
echo "[12] Admin can read same-org user roles"
if [ -n "$ADMIN_TOKEN" ] && [ -n "$ADMIN_ID" ]; then
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$BASE_URL/api/users/$ADMIN_ID/roles" 2>/dev/null || echo "000")
    check "Admin reads own-org user roles (200)" "200" "$STATUS"
else
    echo "  SKIP (admin login/id unavailable)"
fi

# --- Test 13: Nonexistent target user returns 404 on roles endpoint ---
echo "[13] GET roles for nonexistent user returns 404"
if [ -n "$ADMIN_TOKEN" ]; then
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$BASE_URL/api/users/00000000-0000-0000-0000-000000000000/roles" 2>/dev/null || echo "000")
    check "Nonexistent user roles returns 404" "404" "$STATUS"
else
    echo "  SKIP (no admin token)"
fi

# --- Test 14: Nonexistent target user returns 404 on scopes endpoint ---
echo "[14] GET scopes for nonexistent user returns 404"
if [ -n "$ADMIN_TOKEN" ]; then
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$BASE_URL/api/users/00000000-0000-0000-0000-000000000000/scopes" 2>/dev/null || echo "000")
    check "Nonexistent user scopes returns 404" "404" "$STATUS"
else
    echo "  SKIP (no admin token)"
fi

# --- Test 15: Coach cannot assign roles (lacks permission — first auth layer) ---
echo "[15] Coach cannot assign roles to users"
if [ -n "$COACH_TOKEN" ] && [ -n "$ADMIN_ID" ]; then
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $COACH_TOKEN" \
        -H "Content-Type: application/json" \
        -X POST "$BASE_URL/api/users/$ADMIN_ID/roles" \
        -d '{"role_id":"fake-role-id"}' \
        2>/dev/null || echo "000")
    check "Coach cannot assign roles (403)" "403" "$STATUS"
else
    echo "  SKIP (coach login or admin id unavailable)"
fi

# --- Test 16: Coach cannot revoke roles (lacks permission) ---
echo "[16] Coach cannot revoke roles from users"
if [ -n "$COACH_TOKEN" ] && [ -n "$ADMIN_ID" ]; then
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $COACH_TOKEN" \
        -X DELETE "$BASE_URL/api/users/$ADMIN_ID/roles/fake-role-id" \
        2>/dev/null || echo "000")
    check "Coach cannot revoke roles (403)" "403" "$STATUS"
else
    echo "  SKIP (coach login or admin id unavailable)"
fi

# --- Test 17: Coach cannot manage scopes (lacks permission) ---
echo "[17] Coach cannot read user scopes"
if [ -n "$COACH_TOKEN" ] && [ -n "$ADMIN_ID" ]; then
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $COACH_TOKEN" \
        "$BASE_URL/api/users/$ADMIN_ID/scopes" \
        2>/dev/null || echo "000")
    check "Coach cannot read user scopes (403)" "403" "$STATUS"
else
    echo "  SKIP (coach login or admin id unavailable)"
fi

# --- Test 18: Coach cannot assign scopes (lacks permission) ---
echo "[18] Coach cannot assign scopes to users"
if [ -n "$COACH_TOKEN" ] && [ -n "$ADMIN_ID" ]; then
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $COACH_TOKEN" \
        -H "Content-Type: application/json" \
        -X POST "$BASE_URL/api/users/$ADMIN_ID/scopes" \
        -d '{"org_id":"fake-org","access_level":"read"}' \
        2>/dev/null || echo "000")
    check "Coach cannot assign scopes (403)" "403" "$STATUS"
else
    echo "  SKIP (coach login or admin id unavailable)"
fi

# --- Test 19: Auditor cannot assign roles (lacks permission — read-only role) ---
echo "[19] Auditor cannot assign roles"
if [ -n "$AUDITOR_TOKEN" ] && [ -n "$ADMIN_ID" ]; then
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $AUDITOR_TOKEN" \
        -H "Content-Type: application/json" \
        -X POST "$BASE_URL/api/users/$ADMIN_ID/roles" \
        -d '{"role_id":"fake-role-id"}' \
        2>/dev/null || echo "000")
    check "Auditor cannot assign roles (403)" "403" "$STATUS"
else
    echo "  SKIP (auditor login or admin id unavailable)"
fi

# --- Test 20: Revoke scope on nonexistent user returns 404 ---
echo "[20] DELETE scope for nonexistent user returns 404"
if [ -n "$ADMIN_TOKEN" ]; then
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -X DELETE "$BASE_URL/api/users/00000000-0000-0000-0000-000000000000/scopes/fake-scope" \
        2>/dev/null || echo "000")
    check "Revoke scope nonexistent user returns 404" "404" "$STATUS"
else
    echo "  SKIP (no admin token)"
fi

# --- Test 21: Assign role to nonexistent user returns 404 ---
echo "[21] POST role for nonexistent user returns 404"
if [ -n "$ADMIN_TOKEN" ]; then
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -H "Content-Type: application/json" \
        -X POST "$BASE_URL/api/users/00000000-0000-0000-0000-000000000000/roles" \
        -d '{"role_id":"fake-role-id"}' \
        2>/dev/null || echo "000")
    check "Assign role nonexistent user returns 404" "404" "$STATUS"
else
    echo "  SKIP (no admin token)"
fi

# =============================================================================
# Deterministic cross-org tenant isolation (tests 22-26)
#
# Strategy:
#   1. Admin (System Administrator — bypasses all scopes) creates org-B.
#   2. Admin creates user-B inside org-B.
#   3. Admin discovers org-A (the seeded demo org) and the Operations Manager
#      role (which carries api.users.read but NOT System Administrator bypass).
#   4. Admin creates test-actor inside org-A, assigns Ops Manager role + scope
#      limited to org-A.
#   5. Test-actor tries to read user-B (in org-B) → 403 from data-scope check.
#   6. Test-actor reads own-org user (admin, in org-A) → 200 (positive control).
#
# This proves the data-scope enforcement layer blocks cross-org access for
# a privileged-but-scoped actor, independent of the permission layer.
# =============================================================================

echo ""
echo "--- Deterministic Cross-Org Isolation (tests 22-26) ---"

# Discover org-A (seeded demo org)
ORG_A_ID=""
if [ -n "$ADMIN_TOKEN" ]; then
    ORG_A_ID=$(curl -sf -H "Authorization: Bearer $ADMIN_TOKEN" \
        "$BASE_URL/api/admin/org/" 2>/dev/null \
        | python3 -c "import sys,json; orgs=json.load(sys.stdin); print(orgs[0]['id'] if orgs else '')" \
        2>/dev/null || echo "")
fi

# Create org-B
ORG_B_ID=""
if [ -n "$ADMIN_TOKEN" ]; then
    ORG_B_RESP=$(curl -sf -X POST "$BASE_URL/api/admin/org/" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d "{\"name\":\"CrossOrgTest Org B $(date +%s)\"}" \
        2>/dev/null || echo "FAIL")
    ORG_B_ID=$(echo "$ORG_B_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('id',''))" 2>/dev/null || echo "")
fi

# Create user-B in org-B
USER_B_ID=""
if [ -n "$ADMIN_TOKEN" ] && [ -n "$ORG_B_ID" ]; then
    USER_B_NAME="crossorg_userb_$(date +%s)"
    USER_B_RESP=$(curl -sf -X POST "$BASE_URL/api/users/" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d "{\"username\":\"$USER_B_NAME\",\"email\":\"${USER_B_NAME}@test.local\",\"password\":\"UserB123!\",\"org_id\":\"$ORG_B_ID\"}" \
        2>/dev/null || echo "FAIL")
    USER_B_ID=$(echo "$USER_B_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('id',''))" 2>/dev/null || echo "")
fi

# Find the Operations Manager role ID (has api.users.read, NOT System Admin)
OPS_MGR_ROLE_ID=""
if [ -n "$ADMIN_TOKEN" ]; then
    OPS_MGR_ROLE_ID=$(curl -sf -H "Authorization: Bearer $ADMIN_TOKEN" "$BASE_URL/api/roles/" 2>/dev/null \
        | python3 -c "
import sys,json
roles=json.load(sys.stdin)
for r in roles:
    if r['name']=='Operations Manager':
        print(r['id']); break
" 2>/dev/null || echo "")
fi

# Create test-actor in org-A
ACTOR_TOKEN=""
ACTOR_ID=""
if [ -n "$ADMIN_TOKEN" ] && [ -n "$ORG_A_ID" ] && [ -n "$OPS_MGR_ROLE_ID" ]; then
    ACTOR_NAME="crossorg_actor_$(date +%s)"
    ACTOR_RESP=$(curl -sf -X POST "$BASE_URL/api/users/" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $ADMIN_TOKEN" \
        -d "{\"username\":\"$ACTOR_NAME\",\"email\":\"${ACTOR_NAME}@test.local\",\"password\":\"Actor123!\",\"org_id\":\"$ORG_A_ID\"}" \
        2>/dev/null || echo "FAIL")
    ACTOR_ID=$(echo "$ACTOR_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('id',''))" 2>/dev/null || echo "")

    if [ -n "$ACTOR_ID" ]; then
        # Assign Operations Manager role
        curl -sf -X POST "$BASE_URL/api/users/$ACTOR_ID/roles" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $ADMIN_TOKEN" \
            -d "{\"role_id\":\"$OPS_MGR_ROLE_ID\"}" 2>/dev/null > /dev/null

        # Grant data scope for org-A ONLY (no scope for org-B)
        curl -sf -X POST "$BASE_URL/api/users/$ACTOR_ID/scopes" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $ADMIN_TOKEN" \
            -d "{\"org_id\":\"$ORG_A_ID\",\"access_level\":\"write\"}" \
            2>/dev/null > /dev/null

        # Login as the scoped test-actor
        ACTOR_LOGIN=$(curl -sf -X POST "$BASE_URL/api/auth/login" \
            -H "Content-Type: application/json" \
            -d "{\"username\":\"$ACTOR_NAME\",\"password\":\"Actor123!\"}" \
            2>/dev/null || echo "FAIL")
        ACTOR_TOKEN=$(echo "$ACTOR_LOGIN" | python3 -c "import sys,json; print(json.load(sys.stdin)['token'])" 2>/dev/null || echo "")
    fi
fi

# Run the cross-org assertions
if [ -n "$ACTOR_TOKEN" ] && [ -n "$USER_B_ID" ] && [ -n "$ADMIN_ID" ]; then
    echo "[22] Scoped actor CANNOT read user in org-B (cross-org -> 403)"
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $ACTOR_TOKEN" \
        "$BASE_URL/api/users/$USER_B_ID" 2>/dev/null || echo "000")
    check "Cross-org GET user -> 403" "403" "$STATUS"

    echo "[23] Scoped actor CAN read user in own org-A (positive control -> 200)"
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $ACTOR_TOKEN" \
        "$BASE_URL/api/users/$ADMIN_ID" 2>/dev/null || echo "000")
    check "Same-org GET user -> 200" "200" "$STATUS"

    echo "[24] Scoped actor CANNOT list roles for user in org-B (cross-org -> 403)"
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $ACTOR_TOKEN" \
        "$BASE_URL/api/users/$USER_B_ID/roles" 2>/dev/null || echo "000")
    # Ops Manager lacks api.roles.read, so this is 403 from permission check.
    # Either way the actor is blocked — scope or permission, the boundary holds.
    check "Cross-org GET user roles -> 403" "403" "$STATUS"

    echo "[25] Scoped actor CANNOT read scopes for user in org-B"
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $ACTOR_TOKEN" \
        "$BASE_URL/api/users/$USER_B_ID/scopes" 2>/dev/null || echo "000")
    check "Cross-org GET user scopes -> 403" "403" "$STATUS"

    echo "[26] Scoped actor CANNOT assign scope to user in org-B"
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer $ACTOR_TOKEN" \
        -H "Content-Type: application/json" \
        -X POST "$BASE_URL/api/users/$USER_B_ID/scopes" \
        -d "{\"org_id\":\"$ORG_B_ID\",\"access_level\":\"read\"}" \
        2>/dev/null || echo "000")
    check "Cross-org POST scope -> 403" "403" "$STATUS"
else
    # Fixture setup failed — report the reason, do not silently skip
    echo "[22-26] Cross-org fixture setup:"
    [ -z "$ADMIN_TOKEN" ] && echo "  FAIL  admin login failed" && FAILED=$((FAILED + 1))
    [ -z "$ORG_A_ID" ]    && echo "  FAIL  could not discover org-A" && FAILED=$((FAILED + 1))
    [ -z "$ORG_B_ID" ]    && echo "  FAIL  could not create org-B" && FAILED=$((FAILED + 1))
    [ -z "$USER_B_ID" ]   && echo "  FAIL  could not create user-B in org-B" && FAILED=$((FAILED + 1))
    [ -z "$OPS_MGR_ROLE_ID" ] && echo "  FAIL  could not find Operations Manager role" && FAILED=$((FAILED + 1))
    [ -z "$ACTOR_ID" ]    && echo "  FAIL  could not create test-actor in org-A" && FAILED=$((FAILED + 1))
    [ -z "$ACTOR_TOKEN" ] && echo "  FAIL  could not login as test-actor" && FAILED=$((FAILED + 1))
    # If we get here without ADMIN_TOKEN, earlier tests already failed.
    # Bound the skip: these tests REQUIRE a running backend with seed data.
    if [ -n "$ADMIN_TOKEN" ]; then
        echo "  FAIL  cross-org fixture creation failed — check admin API access"
        FAILED=$((FAILED + 1))
    fi
fi

echo ""
echo "==========================="
echo "Results: $PASSED passed, $FAILED failed"
echo "==========================="

if [ "$FAILED" -gt 0 ]; then
    echo "(Failures may indicate backend is not running or seed data is missing)"
    exit 1
fi
