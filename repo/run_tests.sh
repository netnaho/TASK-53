#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "============================================"
echo " CareOps Test Runner"
echo "============================================"
echo ""

FAILED=0

# --- Wait for backend readiness ---
# When run right after `docker compose up -d`, the backend container may still
# be running migrations/seeds and Rocket may not yet be listening on :8000.
# Without this wait, every HTTP test below races the backend and fails with
# connection-refused (curl exit 7, status "000"). Poll /api/health/live until
# it returns 200, up to a generous timeout.
BACKEND_URL="${BACKEND_URL:-http://localhost:8000}"
READY_TIMEOUT="${BACKEND_READY_TIMEOUT:-120}"

echo -e "${YELLOW}Waiting for backend at ${BACKEND_URL}/api/health/live (timeout ${READY_TIMEOUT}s)...${NC}"
WAITED=0
while true; do
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" "${BACKEND_URL}/api/health/live" || echo "000")
    if [ "$STATUS" = "200" ]; then
        echo -e "${GREEN}Backend is ready (after ${WAITED}s).${NC}"
        break
    fi
    if [ "$WAITED" -ge "$READY_TIMEOUT" ]; then
        echo -e "${RED}Backend did not become ready within ${READY_TIMEOUT}s (last status: ${STATUS}).${NC}"
        echo -e "${RED}Aborting test run. Check 'docker compose logs backend' for details.${NC}"
        exit 1
    fi
    sleep 2
    WAITED=$((WAITED + 2))
done
echo ""

# --- Backend Unit Tests ---
echo -e "${YELLOW}[1/11] Backend Unit Tests (cargo test)${NC}"
if ! command -v cargo >/dev/null 2>&1; then
    echo -e "${YELLOW}  SKIP: cargo not installed locally (verified during Docker build)${NC}"
elif (cd "$SCRIPT_DIR/backend" && cargo test --lib 2>&1); then
    echo -e "${GREEN}  PASS${NC}"
else
    echo -e "${RED}  FAIL${NC}"
    FAILED=1
fi
echo ""

# --- Frontend Build Check ---
echo -e "${YELLOW}[2/11] Frontend Build Check${NC}"
if ! command -v cargo >/dev/null 2>&1; then
    echo -e "${YELLOW}  SKIP: cargo not installed locally (verified during Docker build)${NC}"
elif (cd "$SCRIPT_DIR/frontend" && cargo check 2>&1); then
    echo -e "${GREEN}  PASS${NC}"
else
    echo -e "${RED}  FAIL${NC}"
    FAILED=1
fi
echo ""

# --- Unit Tests: Password / Encryption ---
echo -e "${YELLOW}[3/11] Unit Tests: Security (password hashing, encryption)${NC}"
if [ -f "$SCRIPT_DIR/unit_tests/backend/test_password_hashing.sh" ]; then
    if bash "$SCRIPT_DIR/unit_tests/backend/test_password_hashing.sh"; then
        echo -e "${GREEN}  PASS${NC}"
    else
        echo -e "${RED}  FAIL${NC}"
        FAILED=1
    fi
else
    echo -e "${YELLOW}  SKIP${NC}"
fi
echo ""

# --- Unit Tests: Health ---
echo -e "${YELLOW}[4/11] Unit Tests: Health Endpoint${NC}"
if [ -f "$SCRIPT_DIR/unit_tests/backend/test_health.sh" ]; then
    if bash "$SCRIPT_DIR/unit_tests/backend/test_health.sh"; then
        echo -e "${GREEN}  PASS${NC}"
    else
        echo -e "${RED}  FAIL${NC}"
        FAILED=1
    fi
else
    echo -e "${YELLOW}  SKIP${NC}"
fi
echo ""

# --- API Smoke Tests ---
echo -e "${YELLOW}[5/11] API Smoke Tests${NC}"
if [ -f "$SCRIPT_DIR/API_tests/test_smoke.sh" ]; then
    if bash "$SCRIPT_DIR/API_tests/test_smoke.sh"; then
        echo -e "${GREEN}  PASS${NC}"
    else
        echo -e "${RED}  FAIL${NC}"
        FAILED=1
    fi
else
    echo -e "${YELLOW}  SKIP${NC}"
fi
echo ""

# --- API Auth Tests ---
echo -e "${YELLOW}[6/11] API Auth & Security Tests${NC}"
if [ -f "$SCRIPT_DIR/API_tests/test_auth.sh" ]; then
    if bash "$SCRIPT_DIR/API_tests/test_auth.sh"; then
        echo -e "${GREEN}  PASS${NC}"
    else
        echo -e "${RED}  FAIL${NC}"
        FAILED=1
    fi
else
    echo -e "${YELLOW}  SKIP${NC}"
fi
echo ""

# --- API Catalog & Delivery Tests ---
echo -e "${YELLOW}[7/11] API Catalog & Delivery Tests${NC}"
if [ -f "$SCRIPT_DIR/API_tests/test_catalog_delivery.sh" ]; then
    if bash "$SCRIPT_DIR/API_tests/test_catalog_delivery.sh"; then
        echo -e "${GREEN}  PASS${NC}"
    else
        echo -e "${RED}  FAIL${NC}"
        FAILED=1
    fi
else
    echo -e "${YELLOW}  SKIP${NC}"
fi
echo ""

# --- API Billing Tests ---
echo -e "${YELLOW}[8/11] API Billing Engine Tests${NC}"
if [ -f "$SCRIPT_DIR/API_tests/test_billing.sh" ]; then
    if bash "$SCRIPT_DIR/API_tests/test_billing.sh"; then
        echo -e "${GREEN}  PASS${NC}"
    else
        echo -e "${RED}  FAIL${NC}"
        FAILED=1
    fi
else
    echo -e "${YELLOW}  SKIP${NC}"
fi
echo ""

# --- API Scoring Tests ---
echo -e "${YELLOW}[9/11] API Scoring & Review Tests${NC}"
if [ -f "$SCRIPT_DIR/API_tests/test_scoring.sh" ]; then
    if bash "$SCRIPT_DIR/API_tests/test_scoring.sh"; then
        echo -e "${GREEN}  PASS${NC}"
    else
        echo -e "${RED}  FAIL${NC}"
        FAILED=1
    fi
else
    echo -e "${YELLOW}  SKIP${NC}"
fi
echo ""

# --- API Reports & Exports Tests ---
echo -e "${YELLOW}[10/11] API Reports & Exports Tests${NC}"
if [ -f "$SCRIPT_DIR/API_tests/test_reports.sh" ]; then
    if bash "$SCRIPT_DIR/API_tests/test_reports.sh"; then
        echo -e "${GREEN}  PASS${NC}"
    else
        echo -e "${RED}  FAIL${NC}"
        FAILED=1
    fi
else
    echo -e "${YELLOW}  SKIP${NC}"
fi
echo ""

# --- API Ops Controls & Observability Tests ---
echo -e "${YELLOW}[11/11] API Ops Controls & Observability Tests${NC}"
if [ -f "$SCRIPT_DIR/API_tests/test_ops.sh" ]; then
    if bash "$SCRIPT_DIR/API_tests/test_ops.sh"; then
        echo -e "${GREEN}  PASS${NC}"
    else
        echo -e "${RED}  FAIL${NC}"
        FAILED=1
    fi
else
    echo -e "${YELLOW}  SKIP${NC}"
fi
echo ""

echo "============================================"
if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All test suites passed or skipped.${NC}"
else
    echo -e "${RED}Some test suites failed.${NC}"
    exit 1
fi
