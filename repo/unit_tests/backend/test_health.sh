#!/usr/bin/env bash
# Unit test: verify backend health endpoint returns expected structure
# Requires the backend to be running on localhost:8000

set -euo pipefail

BASE_URL="${BACKEND_URL:-http://localhost:8000}"

echo "Testing health/liveness endpoint..."
RESPONSE=$(curl -sf "$BASE_URL/api/health/live" 2>/dev/null || echo "FAIL")

if echo "$RESPONSE" | grep -q '"status"'; then
    echo "  Health endpoint returned valid JSON with status field"
    exit 0
else
    echo "  Health endpoint did not return expected response: $RESPONSE"
    echo "  (Backend may not be running)"
    exit 1
fi
