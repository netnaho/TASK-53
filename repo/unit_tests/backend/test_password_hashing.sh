#!/usr/bin/env bash
# Unit test: verify password hashing module compiles and passes internal tests
# This runs `cargo test` in the backend which includes auth_service and encryption tests.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BACKEND_DIR="$SCRIPT_DIR/../../backend"

echo "Running backend unit tests (includes password hashing, encryption, permission cache)..."

if [ ! -d "$BACKEND_DIR" ]; then
    echo "ERROR: Backend directory not found at $BACKEND_DIR"
    exit 1
fi

cd "$BACKEND_DIR"

if command -v cargo &>/dev/null; then
    cargo test --lib 2>&1
    echo "Backend unit tests passed"
else
    echo "SKIP: Rust/Cargo not installed locally (tests run in Docker build)"
    exit 0
fi
