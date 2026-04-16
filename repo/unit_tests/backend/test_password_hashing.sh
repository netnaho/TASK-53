#!/usr/bin/env bash
# Security unit test: password hashing, encryption, and permission cache.
#
# When run from run_tests.sh, this executes inside the backend-unit-tests
# Docker container (rust:1.88-bookworm) via:
#   docker compose --profile test run --rm backend-unit-tests \
#       cargo test --lib auth_service encryption permission_cache
#
# Running directly on a host with cargo:
#   cd repo/backend && cargo test --lib

set -euo pipefail

BACKEND_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../backend" && pwd)"

echo "Security unit tests (password hashing, encryption, permission cache)..."

if command -v cargo &>/dev/null; then
    (cd "$BACKEND_DIR" && cargo test --lib 2>&1)
    echo "Security unit tests passed."
else
    echo "INFO: cargo not available locally — tests run inside Docker container."
    echo "      Use: docker compose --profile test run --rm backend-unit-tests"
    exit 0
fi
