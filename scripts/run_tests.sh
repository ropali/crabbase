#!/usr/bin/env bash
# scripts/run_tests.sh - Human-readable test runner for Crabbase
#
# Usage:
#   ./scripts/run_tests.sh              # Run all tests (Unit + DB + HTTP Integration)
#   ./scripts/run_tests.sh unit         # Run instant unit tests (crabbase_core)
#   ./scripts/run_tests.sh db           # Run database repository tests
#   ./scripts/run_tests.sh api          # Run HTTP API integration tests
#   ./scripts/run_tests.sh matrix       # Run live Crabbase Feature Matrix (or --no-run)
#   ./scripts/run_tests.sh <suite>      # Run specific suite (e.g. records, rules, auth, validation, types, settings)

set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
python3 "$DIR/runner.py" "$@"
