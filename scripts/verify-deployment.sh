#!/bin/bash
# ==============================================================================
# Grainlify - Deployment Verification Script
# ==============================================================================
# Verifies that a deployed contract is healthy, responsive, and running the
# expected WASM when an expected artifact or hash is provided.
#
# This script performs basic health checks on a deployed contract:
#   - Attempts to read contract state (version, admin, or custom field)
#   - Runs read-only smoke checks against core view functions
#   - Optionally verifies the deployed WASM hash against an expected artifact
#   - Reports HEALTHY or UNRESPONSIVE status
#   - Optionally outputs detailed contract information
#
# USAGE:
#   ./scripts/verify-deployment.sh <contract_id> [options]
#
# ARGUMENTS:
#   <contract_id>       The deployed contract ID (C... format)
#
# OPTIONS:
#   -n, --network       Network (testnet|mainnet) [default: testnet]
#   -s, --source        Source identity for reading [default: from config]
#   -c, --config        Path to configuration file
#   -f, --function      Function to call for verification [default: get_version]
#   --check-admin       Verify admin address matches expected value
#   --expected-admin    Expected admin address for --check-admin
#   --expected-wasm     Local WASM artifact whose SHA-256 must match deployment
#   --expected-wasm-hash Expected deployed WASM hash (64 hex chars)
#   --smoke-functions   Comma-separated read-only functions [default: get_version,get_admin,get_pause_flags]
#   --skip-smoke        Skip read-only smoke checks
#   --json              Output result as JSON
#   -v, --verbose       Enable verbose output
#   -h, --help          Show this help message
#
# EXAMPLES:
#   # Basic health check
#   ./scripts/verify-deployment.sh CABC123...
#
#   # Check on mainnet with JSON output
#   ./scripts/verify-deployment.sh CABC123... -n mainnet --json
#
#   # Verify admin matches expected
#   ./scripts/verify-deployment.sh CABC123... --check-admin --expected-admin GABC...
#
#   # Verify deployed WASM hash and core read-only functions
#   ./scripts/verify-deployment.sh CABC123... --expected-wasm target/escrow.wasm
#
#   # Use custom verification function
#   ./scripts/verify-deployment.sh CABC123... -f get_balance
#
# EXIT CODES:
#   0 - Contract is healthy
#   1 - Contract is unresponsive or verification failed
#   2 - Invalid arguments or configuration error
#
# ==============================================================================

set -euo pipefail

# ------------------------------------------------------------------------------
# Script Setup
# ------------------------------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Source common utilities
source "$SCRIPT_DIR/utils/common.sh"

# ------------------------------------------------------------------------------
# Default Values
# ------------------------------------------------------------------------------

CONTRACT_ID=""
NETWORK="testnet"
SOURCE_IDENTITY=""
CONFIG_FILE=""
VERIFY_FUNCTION="get_version"
CHECK_ADMIN="false"
EXPECTED_ADMIN=""
EXPECTED_WASM_PATH=""
EXPECTED_WASM_HASH=""
RUN_SMOKE_TESTS="true"
SMOKE_FUNCTIONS="get_version,get_admin,get_pause_flags"
OUTPUT_JSON="false"
VERBOSE="false"

# ------------------------------------------------------------------------------
# Usage
# ------------------------------------------------------------------------------

show_usage() {
    head -50 "$0" | grep -E "^#" | sed 's/^# \?//'
    exit 0
}

# ------------------------------------------------------------------------------
# Argument Parsing
# ------------------------------------------------------------------------------

parse_args() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -n|--network)
                NETWORK="$2"
                shift 2
                ;;
            -s|--source)
                SOURCE_IDENTITY="$2"
                shift 2
                ;;
            -c|--config)
                CONFIG_FILE="$2"
                shift 2
                ;;
            -f|--function)
                VERIFY_FUNCTION="$2"
                shift 2
                ;;
            --check-admin)
                CHECK_ADMIN="true"
                shift
                ;;
            --expected-admin)
                EXPECTED_ADMIN="$2"
                shift 2
                ;;
            --expected-wasm)
                EXPECTED_WASM_PATH="$2"
                shift 2
                ;;
            --expected-wasm-hash)
                EXPECTED_WASM_HASH="$2"
                shift 2
                ;;
            --smoke-functions)
                SMOKE_FUNCTIONS="$2"
                shift 2
                ;;
            --skip-smoke)
                RUN_SMOKE_TESTS="false"
                shift
                ;;
            --json)
                OUTPUT_JSON="true"
                shift
                ;;
            -v|--verbose)
                VERBOSE="true"
                export VERBOSE
                shift
                ;;
            -h|--help)
                show_usage
                ;;
            -*)
                log_error "Unknown option: $1"
                exit 2
                ;;
            *)
                if [[ -z "$CONTRACT_ID" ]]; then
                    CONTRACT_ID="$1"
                else
                    log_error "Unexpected argument: $1"
                    exit 2
                fi
                shift
                ;;
        esac
    done
}

# ------------------------------------------------------------------------------
# Validation
# ------------------------------------------------------------------------------

validate_inputs() {
    if [[ "$OUTPUT_JSON" != "true" ]]; then
        log_section "Validating Inputs"
    fi

    # Check contract ID
    if [[ -z "$CONTRACT_ID" ]]; then
        log_error "No contract ID specified"
        echo "Usage: $0 <contract_id> [options]"
        exit 2
    fi

    # Basic format validation
    if [[ ! "$CONTRACT_ID" =~ ^C[A-Z0-9]{55}$ ]]; then
        log_warn "Contract ID format may be invalid: $CONTRACT_ID"
    fi

    if [[ -n "$EXPECTED_WASM_PATH" && ! -f "$EXPECTED_WASM_PATH" ]]; then
        log_error "Expected WASM file not found: $EXPECTED_WASM_PATH"
        exit 2
    fi

    if [[ -n "$EXPECTED_WASM_HASH" ]]; then
        EXPECTED_WASM_HASH=$(normalize_hash "$EXPECTED_WASM_HASH" || true)
        if [[ ! "$EXPECTED_WASM_HASH" =~ ^[a-f0-9]{64}$ ]]; then
            log_error "Expected WASM hash must be 64 hex characters"
            exit 2
        fi
    fi

    if [[ "$RUN_SMOKE_TESTS" == "true" && -z "$SMOKE_FUNCTIONS" ]]; then
        log_error "Smoke functions cannot be empty unless --skip-smoke is used"
        exit 2
    fi

    if [[ "$OUTPUT_JSON" != "true" ]]; then
        log_info "Contract ID: $CONTRACT_ID"
        log_success "Inputs validated"
    fi
}

# ------------------------------------------------------------------------------
# Configuration
# ------------------------------------------------------------------------------

load_verify_config() {
    if [[ "$OUTPUT_JSON" != "true" ]]; then
        log_section "Loading Configuration"
    fi

    # Set default config
    if [[ -z "$CONFIG_FILE" ]]; then
        CONFIG_FILE="$SCRIPT_DIR/config/${NETWORK}.env"
    fi

    # Load config if exists
    if [[ -f "$CONFIG_FILE" ]]; then
        load_config "$CONFIG_FILE"
    fi

    # Override with command line
    if [[ -n "$SOURCE_IDENTITY" ]]; then
        export DEPLOYER_IDENTITY="$SOURCE_IDENTITY"
    fi

    # Set defaults
    : "${SOROBAN_RPC_URL:=https://soroban-testnet.stellar.org}"
    : "${SOROBAN_NETWORK:=$NETWORK}"
    : "${DEPLOYER_IDENTITY:=default}"
    : "${CLI_TIMEOUT:=30}"

    export SOROBAN_RPC_URL
    export SOROBAN_NETWORK

    if [[ "$OUTPUT_JSON" != "true" ]]; then
        log_info "Network: $SOROBAN_NETWORK"
        log_success "Configuration loaded"
    fi
}

# ------------------------------------------------------------------------------
# Verification Functions
# ------------------------------------------------------------------------------

sanitize_field() {
    tr '\n' ' ' | sed 's/|/\//g; s/[[:space:]]\+/ /g; s/^ //; s/ $//'
}

normalize_hash() {
    local raw="$1"
    echo "$raw" \
        | tr '[:upper:]' '[:lower:]' \
        | sed -E 's/^0x//; s/[^a-f0-9].*$//'
}

invoke_view_function() {
    local function_name="$1"
    local cli_cmd
    cli_cmd=$(get_cli_command 2>/dev/null)

    run_with_timeout "$CLI_TIMEOUT" \
        "$cli_cmd" contract invoke \
        --id "$CONTRACT_ID" \
        --network "$SOROBAN_NETWORK" \
        --source "$DEPLOYER_IDENTITY" \
        -- \
        "$function_name"
}

# Check if contract responds to a function call
check_contract_responsive() {
    local result=""
    local status="UNRESPONSIVE"
    local error_msg=""

    if [[ "$OUTPUT_JSON" != "true" ]]; then
        log_info "Checking contract responsiveness..." >&2
        log_info "Calling function: $VERIFY_FUNCTION" >&2
    fi

    # Try to invoke the verification function
    if result=$(invoke_view_function "$VERIFY_FUNCTION" 2>&1); then
        status="HEALTHY"
        result=$(echo "$result" | sanitize_field)
    else
        error_msg="$result"
        # Check if it's a "function not found" vs "contract unreachable"
        if echo "$result" | grep -qi "not found\|does not exist\|no such"; then
            error_msg="Function '$VERIFY_FUNCTION' not found in contract"
        fi
        error_msg=$(echo "$error_msg" | sanitize_field)
    fi

    echo "$status|$result|$error_msg"
}

# Check admin address if requested
check_admin_address() {
    local cli_cmd
    cli_cmd=$(get_cli_command 2>/dev/null)

    local admin_result=""

    # Try common admin getter functions
    for func in "get_admin" "admin" "owner" "get_owner"; do
        if admin_result=$(invoke_view_function "$func" 2>&1); then
            echo "$admin_result"
            return 0
        fi
    done

    echo ""
    return 1
}

extract_wasm_hash() {
    local output="$1"
    local hash=""

    if command -v jq &> /dev/null && echo "$output" | jq -e . > /dev/null 2>&1; then
        hash=$(echo "$output" | jq -r '
            .wasm_hash // .wasmHash // .code_hash // .codeHash // .hash // empty
        ' 2>/dev/null | head -n 1)
    fi

    if [[ -z "$hash" ]]; then
        hash=$(echo "$output" \
            | grep -Eoi '(wasm[_ -]?hash|code[_ -]?hash|hash)[" :]+(0x)?[a-f0-9]{64}' \
            | grep -Eoi '[a-f0-9]{64}' \
            | head -n 1 || true)
    fi

    if [[ -z "$hash" ]]; then
        hash=$(echo "$output" | grep -Eoi '\b[0-9a-fA-F]{64}\b' | head -n 1 || true)
    fi

    if [[ -n "$hash" ]]; then
        normalize_hash "$hash"
    fi
}

fetch_deployed_wasm_hash() {
    local cli_cmd
    cli_cmd=$(get_cli_command 2>/dev/null)

    local output=""
    local hash=""

    if output=$(run_with_timeout "$CLI_TIMEOUT" \
        "$cli_cmd" contract info hash \
        --contract-id "$CONTRACT_ID" \
        --network "$SOROBAN_NETWORK" 2>&1); then
        hash=$(extract_wasm_hash "$output")
        if [[ -n "$hash" ]]; then
            echo "$hash"
            return 0
        fi
    fi

    if output=$(run_with_timeout "$CLI_TIMEOUT" \
        "$cli_cmd" contract info \
        --id "$CONTRACT_ID" \
        --network "$SOROBAN_NETWORK" 2>&1); then
        hash=$(extract_wasm_hash "$output")
        if [[ -n "$hash" ]]; then
            echo "$hash"
            return 0
        fi
    fi

    echo "Unable to read deployed WASM hash from contract info output" >&2
    return 1
}

verify_wasm_hash() {
    local expected_hash="$EXPECTED_WASM_HASH"
    local actual_hash=""
    local error_msg=""

    if [[ -n "$EXPECTED_WASM_PATH" ]]; then
        expected_hash=$(get_file_hash "$EXPECTED_WASM_PATH")
        if [[ "$expected_hash" == "unknown" ]]; then
            echo "FAILED|||No SHA-256 tool available for expected WASM"
            return 0
        fi
        expected_hash=$(normalize_hash "$expected_hash")
    fi

    if [[ -z "$expected_hash" ]]; then
        echo "SKIPPED|||"
        return 0
    fi

    if [[ "$OUTPUT_JSON" != "true" ]]; then
        log_info "Verifying deployed WASM hash..." >&2
    fi

    if actual_hash=$(fetch_deployed_wasm_hash 2>&1); then
        actual_hash=$(normalize_hash "$actual_hash")
        if [[ "$actual_hash" == "$expected_hash" ]]; then
            echo "MATCH|$expected_hash|$actual_hash|"
        else
            error_msg="WASM hash mismatch (expected: $expected_hash, got: $actual_hash)"
            echo "MISMATCH|$expected_hash|$actual_hash|$error_msg"
        fi
    else
        error_msg=$(echo "$actual_hash" | sanitize_field)
        echo "FAILED|$expected_hash||$error_msg"
    fi
}

run_smoke_checks() {
    if [[ "$RUN_SMOKE_TESTS" != "true" ]]; then
        echo "SKIPPED||"
        return 0
    fi

    if [[ "$OUTPUT_JSON" != "true" ]]; then
        log_info "Running read-only smoke checks: $SMOKE_FUNCTIONS" >&2
    fi

    local status="PASS"
    local results=""
    local errors=""
    local old_ifs="$IFS"
    IFS=','
    read -ra functions <<< "$SMOKE_FUNCTIONS"
    IFS="$old_ifs"

    local func
    for func in "${functions[@]}"; do
        func=$(echo "$func" | xargs)
        [[ -z "$func" ]] && continue

        local result=""
        if result=$(invoke_view_function "$func" 2>&1); then
            result=$(echo "$result" | sanitize_field)
            results="${results}${results:+; }${func}:PASS:${result}"
        else
            result=$(echo "$result" | sanitize_field)
            status="FAIL"
            results="${results}${results:+; }${func}:FAIL:${result}"
            errors="${errors}${errors:+; }Smoke test failed: ${func}"
        fi
    done

    if [[ -z "$results" ]]; then
        echo "SKIPPED||No smoke functions provided"
    else
        echo "$status|$results|$errors"
    fi
}

# ------------------------------------------------------------------------------
# Output Functions
# ------------------------------------------------------------------------------

output_result() {
    local status="$1"
    local function_result="$2"
    local error_msg="$3"
    local admin_match="$4"
    local wasm_status="$5"
    local wasm_expected="$6"
    local wasm_actual="$7"
    local wasm_error="$8"
    local smoke_status="$9"
    local smoke_results="${10}"
    local smoke_error="${11}"

    if [[ "$OUTPUT_JSON" == "true" ]]; then
        # JSON output
        jq -n \
            --arg contract_id "$CONTRACT_ID" \
            --arg network "$SOROBAN_NETWORK" \
            --arg status "$status" \
            --arg function "$VERIFY_FUNCTION" \
            --arg result "$function_result" \
            --arg error "$error_msg" \
            --arg admin_check "$admin_match" \
            --arg wasm_status "$wasm_status" \
            --arg wasm_expected "$wasm_expected" \
            --arg wasm_actual "$wasm_actual" \
            --arg wasm_error "$wasm_error" \
            --arg smoke_status "$smoke_status" \
            --arg smoke_results "$smoke_results" \
            --arg smoke_error "$smoke_error" \
            --arg timestamp "$(get_timestamp)" \
            '{
                contract_id: $contract_id,
                network: $network,
                status: $status,
                verification: {
                    function: $function,
                    result: $result,
                    error: (if $error == "" then null else $error end)
                },
                admin_check: (if $admin_check == "" then null else $admin_check end),
                wasm_hash: {
                    status: $wasm_status,
                    expected: (if $wasm_expected == "" then null else $wasm_expected end),
                    actual: (if $wasm_actual == "" then null else $wasm_actual end),
                    error: (if $wasm_error == "" then null else $wasm_error end)
                },
                smoke_tests: {
                    status: $smoke_status,
                    results: (if $smoke_results == "" then null else $smoke_results end),
                    error: (if $smoke_error == "" then null else $smoke_error end)
                },
                verified_at: $timestamp
            }'
    else
        # Human-readable output
        log_section "Verification Result"
        echo ""

        if [[ "$status" == "HEALTHY" ]]; then
            echo -e "  Status:          ${GREEN}$status${NC}"
        else
            echo -e "  Status:          ${RED}$status${NC}"
        fi

        echo "  Contract ID:     $CONTRACT_ID"
        echo "  Network:         $SOROBAN_NETWORK"
        echo "  Function:        $VERIFY_FUNCTION"

        if [[ -n "$function_result" && "$status" == "HEALTHY" ]]; then
            echo "  Result:          $function_result"
        fi

        if [[ -n "$error_msg" ]]; then
            echo "  Error:           $error_msg"
        fi

        if [[ -n "$admin_match" ]]; then
            echo "  Admin Check:     $admin_match"
        fi

        if [[ -n "$wasm_status" && "$wasm_status" != "SKIPPED" ]]; then
            echo "  WASM Hash:       $wasm_status"
            echo "  Expected Hash:   $wasm_expected"
            echo "  Deployed Hash:   ${wasm_actual:-unknown}"
            if [[ -n "$wasm_error" ]]; then
                echo "  WASM Error:      $wasm_error"
            fi
        fi

        if [[ -n "$smoke_status" ]]; then
            echo "  Smoke Tests:     $smoke_status"
            if [[ -n "$smoke_results" ]]; then
                echo "  Smoke Results:   $smoke_results"
            fi
            if [[ -n "$smoke_error" ]]; then
                echo "  Smoke Error:     $smoke_error"
            fi
        fi

        echo ""
    fi
}

# ------------------------------------------------------------------------------
# Main Verification
# ------------------------------------------------------------------------------

perform_verification() {
    if [[ "$OUTPUT_JSON" != "true" ]]; then
        log_section "Performing Verification"
    fi

    # Check dependencies (quietly for JSON mode)
    local cli_cmd
    if [[ "$OUTPUT_JSON" != "true" ]]; then
        log_info "Checking dependencies..."
        cli_cmd=$(get_cli_command)
        log_info "Using CLI: $cli_cmd"
        if [[ "$OUTPUT_JSON" == "true" ]]; then
            check_jq_installed
        fi
        log_success "All dependencies satisfied"
    else
        get_cli_command > /dev/null
        check_jq_installed > /dev/null 2>&1
    fi

    # Main responsiveness check
    local check_result
    check_result=$(check_contract_responsive)

    local status
    local function_result
    local error_msg
    IFS='|' read -r status function_result error_msg <<< "$check_result"

    # Admin check if requested
    local admin_match=""
    if [[ "$CHECK_ADMIN" == "true" ]]; then
        if [[ "$OUTPUT_JSON" != "true" ]]; then
            log_info "Checking admin address..."
        fi

        local current_admin
        current_admin=$(check_admin_address)

        if [[ -n "$current_admin" ]]; then
            if [[ -n "$EXPECTED_ADMIN" ]]; then
                if [[ "$current_admin" == *"$EXPECTED_ADMIN"* ]]; then
                    admin_match="MATCH ($current_admin)"
                else
                    admin_match="MISMATCH (expected: $EXPECTED_ADMIN, got: $current_admin)"
                    status="UNHEALTHY"
                fi
            else
                admin_match="$current_admin"
            fi
        else
            admin_match="Unable to retrieve admin"
        fi
    fi

    local wasm_check
    wasm_check=$(verify_wasm_hash)

    local wasm_status
    local wasm_expected
    local wasm_actual
    local wasm_error
    IFS='|' read -r wasm_status wasm_expected wasm_actual wasm_error <<< "$wasm_check"

    if [[ "$wasm_status" == "MISMATCH" || "$wasm_status" == "FAILED" ]]; then
        status="UNHEALTHY"
    fi

    local smoke_check
    smoke_check=$(run_smoke_checks)

    local smoke_status
    local smoke_results
    local smoke_error
    IFS='|' read -r smoke_status smoke_results smoke_error <<< "$smoke_check"

    if [[ "$smoke_status" == "FAIL" ]]; then
        status="UNHEALTHY"
    fi

    # Output result
    output_result \
        "$status" \
        "$function_result" \
        "$error_msg" \
        "$admin_match" \
        "$wasm_status" \
        "$wasm_expected" \
        "$wasm_actual" \
        "$wasm_error" \
        "$smoke_status" \
        "$smoke_results" \
        "$smoke_error"

    # Exit with appropriate code
    if [[ "$status" == "HEALTHY" ]]; then
        if [[ "$OUTPUT_JSON" != "true" ]]; then
            log_success "Contract verification passed"
        fi
        return 0
    else
        if [[ "$OUTPUT_JSON" != "true" ]]; then
            log_error "Contract verification failed"
        fi
        return 1
    fi
}

# ------------------------------------------------------------------------------
# Main
# ------------------------------------------------------------------------------

main() {
    parse_args "$@"
    validate_inputs
    load_verify_config
    perform_verification
}

# Run main if executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
