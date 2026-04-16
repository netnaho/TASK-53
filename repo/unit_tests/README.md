# Unit Tests

This directory contains unit tests organized by component:

- `backend/` - Backend Rust unit tests and shell-based smoke tests
- (future) `frontend/` - Frontend component tests

## Running

All unit tests run inside Docker containers via `run_tests.sh` — no local `cargo` installation
is required on the host.

```bash
# Run all tests (unit + API) via the project test runner (fully Docker-contained)
./run_tests.sh

# Run backend unit tests only (inside the backend-unit-tests container)
docker compose --profile test run --rm backend-unit-tests

# Run frontend unit tests only (inside the frontend-unit-tests container)
docker compose --profile test run --rm frontend-unit-tests
```
