#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
UPGRADE_SCRIPT="$SCRIPT_DIR/upgrade.sh"

FAKE_WASM=/tmp/fake_valid.wasm
echo -n -e "\x00\x61\x73\x6D\x01" > "$FAKE_WASM"  # Minimal WASM magic header

# --------------------- MOCKING HELPERS ----------------------
MOCK_BIN="$(pwd)/mock_bin"
mkdir -p "$MOCK_BIN"

enable_identity_mock() {
    echo '#!/usr/bin/env bash
if [[ "$1" = "keys" && "$2" = "address" ]]; then
    if [[ "${3:-}" == "ghost_id" ]]; then
        echo "mock keys address failure" >&2
        exit 1
    fi
    echo FAKE_ADDRESS
    exit 0
fi
echo "Mock stellar call"
exit 0' > "$MOCK_BIN/stellar"

    chmod +x "$MOCK_BIN/stellar"
}

export PATH="$MOCK_BIN:$PATH"
enable_identity_mock

fail() { echo "✘ FAIL: $1"; exit 1; }
pass() { echo "✔ PASS: $1"; }

run_expect_fail() {
    desc="$1"
    expected_msg="$2"
    shift 2

    set +e
    output=$("$UPGRADE_SCRIPT" "$@" 2>&1)
    exit_code=$?
    set -e

    if [[ $exit_code -eq 0 ]]; then
        echo "$output"
        fail "$desc (expected failure, got exit 0)"
    fi

    if ! echo "$output" | grep -q "$expected_msg"; then
        echo "$output"
        fail "$desc (expected message '$expected_msg')"
    fi

    pass "$desc"
}

echo "=== Upgrade Script Failure Tests ==="

#  1. Missing contract ID
run_expect_fail "Missing contract ID" "No contract ID specified"

#  2. Invalid contract ID format
run_expect_fail "Invalid format" "Contract ID format may be invalid" "BAD_ID" "/tmp/missing.wasm"

#  3. Missing WASM file argument
run_expect_fail "Missing WASM file" "No WASM file specified" "C1234567890123456789012345678901234567890123456789012345678"

# Nonexistent WASM
run_expect_fail "Invalid WASM file path" "WASM file not found" \
    "C1234567890123456789012345678901234567890123456789012345678" \
    "/tmp/not_real_contract.wasm"

#  5. Invalid identity
run_expect_fail "Missing identity" "Identity not found" \
    "C1234567890123456789012345678901234567890123456789012345678" \
    "$FAKE_WASM" \
    --source ghost_id


echo "All upgrade failure tests passed!"