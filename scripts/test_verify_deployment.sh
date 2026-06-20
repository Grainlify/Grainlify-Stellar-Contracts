#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
VERIFY_SCRIPT="$SCRIPT_DIR/verify-deployment.sh"
CONTRACT_ID="CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"

fail() {
    echo "FAIL: $1"
    exit 1
}

pass() {
    echo "PASS: $1"
}

run_expect_success() {
    local desc="$1"
    shift

    local output
    if ! output=$("$VERIFY_SCRIPT" "$CONTRACT_ID" "$@" 2>&1); then
        echo "$output"
        fail "$desc"
    fi

    echo "$output"
    pass "$desc"
}

run_expect_fail() {
    local desc="$1"
    local expected="$2"
    shift 2

    local output
    local exit_code
    set +e
    output=$("$VERIFY_SCRIPT" "$CONTRACT_ID" "$@" 2>&1)
    exit_code=$?
    set -e

    if [[ $exit_code -eq 0 ]]; then
        echo "$output"
        fail "$desc (expected non-zero exit)"
    fi

    if ! echo "$output" | grep -q "$expected"; then
        echo "$output"
        fail "$desc (missing expected output: $expected)"
    fi

    pass "$desc"
}

MOCK_ROOT="$(mktemp -d)"
trap 'rm -rf "$MOCK_ROOT"' EXIT

MOCK_BIN="$MOCK_ROOT/bin"
mkdir -p "$MOCK_BIN"
cp "$SCRIPT_DIR/testdata/stellar-mock.sh" "$MOCK_BIN/stellar"
chmod +x "$MOCK_BIN/stellar"

export PATH="$MOCK_BIN:$PATH"
export CLI_TIMEOUT=5

FAKE_WASM="$MOCK_ROOT/contract.wasm"
printf '\000asm\001\000\000\000' > "$FAKE_WASM"
export MOCK_WASM_HASH
MOCK_WASM_HASH="$(sha256sum "$FAKE_WASM" | awk '{print $1}')"

echo "=== verify-deployment.sh tests ==="

success_output=$(run_expect_success \
    "WASM hash and smoke checks pass" \
    --expected-wasm "$FAKE_WASM" \
    --smoke-functions get_version,get_admin,get_pause_flags)

echo "$success_output" | grep -q "WASM Hash:       MATCH" || fail "success output missing hash match"
echo "$success_output" | grep -q "Smoke Tests:     PASS" || fail "success output missing smoke pass"

run_expect_fail \
    "WASM hash mismatch fails verification" \
    "WASM hash mismatch" \
    --expected-wasm-hash bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb \
    --smoke-functions get_version,get_admin,get_pause_flags

export MOCK_FAIL_SMOKE="get_pause_flags"
run_expect_fail \
    "Smoke function failure fails verification" \
    "Smoke test failed: get_pause_flags" \
    --expected-wasm "$FAKE_WASM" \
    --smoke-functions get_version,get_admin,get_pause_flags
unset MOCK_FAIL_SMOKE

echo "All verify-deployment tests passed!"
