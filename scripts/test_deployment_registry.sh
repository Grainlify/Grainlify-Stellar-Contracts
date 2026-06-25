#!/usr/bin/env bash
# ==============================================================================
# Grainlify - Deployment Registry Integration Tests
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Target test config and registry filenames
NETWORK="testnet"
CONFIG_FILE="$SCRIPT_DIR/config/${NETWORK}.env"
CONFIG_BACKUP=""
REGISTRY_FILE="$PROJECT_ROOT/deployments/${NETWORK}.json"

fail() {
    echo -e "\033[0;31m✘ FAIL: $1\033[0m"
    cleanup
    exit 1
}

pass() {
    echo -e "\033[0;32m✔ PASS: $1\033[0m"
}

cleanup() {
    echo "Cleaning up temporary files..."
    rm -f "$REGISTRY_FILE"
    rm -rf "$MOCK_BIN"
    rm -f "$UPGRADES_LOG"
    rm -f "$ROLLBACKS_LOG"
    if [[ -n "$CONFIG_BACKUP" && -f "$CONFIG_BACKUP" ]]; then
        mv "$CONFIG_BACKUP" "$CONFIG_FILE"
    else
        rm -f "$CONFIG_FILE"
    fi
}

# Back up config if exists
if [[ -f "$CONFIG_FILE" ]]; then
    CONFIG_BACKUP="$(mktemp)"
    cp "$CONFIG_FILE" "$CONFIG_BACKUP"
fi

# 1. Create a mock config file for testnet
echo "Creating mock configuration..."
cat << EOF > "$CONFIG_FILE"
SOROBAN_RPC_URL="https://soroban-testnet.stellar.org"
SOROBAN_NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
SOROBAN_NETWORK="testnet"
DEPLOYER_IDENTITY="mock-deployer"
VERBOSE="false"
DEPLOYMENT_LOG="deployments/${NETWORK}.json"
REQUIRE_CONFIRMATION="false"
CLI_TIMEOUT="5"
RETRY_ATTEMPTS="1"
RETRY_DELAY="1"
EOF

# 2. Setup mock stellar CLI
MOCK_BIN="$(mktemp -d)"
MOCK_ID="CBUTTERFLY1234567890123456789012345678901234567890123456" # Exactly 56 chars
MOCK_ESCROW_ID="CESCROW1234567890123456789012345678901234567890123456789" # Exactly 56 chars

mkdir -p "$MOCK_BIN"

# Global variable to hold what the mock CLI should return for WASM hash
export MOCK_CURRENT_WASM_HASH="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"

cat << EOF > "$MOCK_BIN/stellar"
#!/usr/bin/env bash
# Mock stellar CLI

cmd="\$1"
shift

if [[ "\$cmd" == "keys" && "\$1" == "address" ]]; then
    echo "G_MOCK_ADDRESS_1234567890"
    exit 0
fi

if [[ "\$cmd" == "contract" ]]; then
    subcmd="\$1"
    shift
    
    if [[ "\$subcmd" == "install" ]]; then
        # Return new hash
        echo "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
        exit 0
    fi
    
    if [[ "\$subcmd" == "deploy" ]]; then
        echo "$MOCK_ID"
        exit 0
    fi
    
    if [[ "\$subcmd" == "invoke" ]]; then
        # Invoke mock, e.g. for get_version or upgrade
        if [[ "\${*:-}" == *"get_version"* ]]; then
            echo "1"
        else
            echo "Mock invoke success"
        fi
        exit 0
    fi
    
    if [[ "\$subcmd" == "info" ]]; then
        # Read from environment
        echo "{\"wasm_hash\": \"\$MOCK_CURRENT_WASM_HASH\"}"
        exit 0
    fi
fi

echo "Unknown mock command: \$cmd \$*" >&2
exit 1
EOF

chmod +x "$MOCK_BIN/stellar"
export PATH="$MOCK_BIN:$PATH"

# Set up test upgrades and rollbacks log paths
UPGRADES_LOG="$PROJECT_ROOT/deployments/upgrades.json"
ROLLBACKS_LOG="$PROJECT_ROOT/deployments/rollbacks.json"

# Make sure we clean up at exit
trap cleanup EXIT

# 3. Create a mock registry deployments file
echo "Initializing mock registry deployments file..."
# Contract 1 (escrow) has two versions deployed previously:
# Version 1: aaaaaaaa... (deployed first)
# Version 2: bbbbbbbb... (deployed second)
mkdir -p "$(dirname "$REGISTRY_FILE")"
cat << EOF > "$REGISTRY_FILE"
{
  "deployments": [
    {
      "contract_name": "escrow",
      "contract_id": "$MOCK_ESCROW_ID",
      "wasm_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "soroban_sdk": "21.0.0",
      "deployer": "mock-deployer",
      "timestamp": "2026-06-24T12:00:00Z",
      "network": "testnet",
      "deployed_at": "2026-06-24T12:00:00Z",
      "status": "deployed"
    },
    {
      "contract_name": "escrow",
      "contract_id": "$MOCK_ESCROW_ID",
      "wasm_hash": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "soroban_sdk": "21.0.0",
      "deployer": "mock-deployer",
      "timestamp": "2026-06-25T12:00:00Z",
      "network": "testnet",
      "deployed_at": "2026-06-25T12:00:00Z",
      "status": "deployed"
    }
  ],
  "metadata": {
    "created": "2026-06-24T12:00:00Z",
    "version": "1.0"
  }
}
EOF

# Make a fake valid WASM file for verify script validation
FAKE_WASM="/tmp/fake_valid_test.wasm"
echo -n -e "\x00\x61\x73\x6D\x01" > "$FAKE_WASM"

echo "=== Running Registry Integration Tests ==="

# ------------------------------------------------------------------------------
# Test 1: verify-deployment.sh with contract name (resolving ID and hash)
# ------------------------------------------------------------------------------
echo "Test 1: Verify using contract name..."
# On-chain hash is currently set to bbbbbbbb... in mock
MOCK_CURRENT_WASM_HASH="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
export MOCK_CURRENT_WASM_HASH

# This should automatically resolve "escrow" to CESCROW... and expect hash bbbbbbbb...
# Since they match, it should succeed
if ! "$SCRIPT_DIR/verify-deployment.sh" "escrow" -n "$NETWORK" --skip-smoke; then
    fail "verify-deployment.sh failed using contract name 'escrow'"
fi
pass "verify-deployment.sh correctly resolved contract name and passed"

# ------------------------------------------------------------------------------
# Test 2: verify-deployment.sh with contract ID (auto-resolving expected hash)
# ------------------------------------------------------------------------------
echo "Test 2: Verify using contract ID with auto-resolving expected hash..."
if ! "$SCRIPT_DIR/verify-deployment.sh" "$MOCK_ESCROW_ID" -n "$NETWORK" --skip-smoke; then
    fail "verify-deployment.sh failed using contract ID with auto-resolved hash"
fi
pass "verify-deployment.sh correctly auto-resolved expected hash and passed"

# ------------------------------------------------------------------------------
# Test 3: Drift detection (hash mismatch)
# ------------------------------------------------------------------------------
echo "Test 3: Verify drift detection (hash mismatch)..."
# Set on-chain hash to some mutated value, indicating drift/unauthorized upgrade
MOCK_CURRENT_WASM_HASH="mutatedhash12345678901234567890123456789012345678901234567890123"
export MOCK_CURRENT_WASM_HASH

# Since on-chain is mutated, but registry expects bbbbbbbb..., this must fail (exit 1)
set +e
"$SCRIPT_DIR/verify-deployment.sh" "escrow" -n "$NETWORK" --skip-smoke > /dev/null 2>&1
exit_code=$?
set -e

if [[ $exit_code -ne 1 ]]; then
    fail "verify-deployment.sh did not fail on hash mismatch (exit code was $exit_code, expected 1)"
fi
pass "verify-deployment.sh correctly detected hash drift and failed with exit code 1"

# ------------------------------------------------------------------------------
# Test 4: rollback.sh automatic hash resolution (second-to-last)
# ------------------------------------------------------------------------------
echo "Test 4: Rollback using contract name and automatic previous hash lookup..."
# We run rollback.sh with ONLY the contract name.
# It should:
#   1. Resolve "escrow" to CESCROW...
#   2. Find that the previous version (second-to-last) hash is aaaaaaaa...
#   3. Invoke upgrade to aaaaaaaa...
#   4. Append a new entry to deployments/testnet.json with hash aaaaaaaa...
if ! "$SCRIPT_DIR/rollback.sh" "escrow" -n "$NETWORK" --force; then
    fail "rollback.sh failed with contract name"
fi

# Verify it appended a new record with hash aaaaaaaa...
latest_recorded_hash=$(jq -r '.deployments | last | .wasm_hash' "$REGISTRY_FILE")
if [[ "$latest_recorded_hash" != "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" ]]; then
    fail "rollback.sh did not write the correct rolled back hash to registry. Got: $latest_recorded_hash, expected aaaaaaaa..."
fi

latest_recorded_sdk=$(jq -r '.deployments | last | .soroban_sdk' "$REGISTRY_FILE")
if [[ -z "$latest_recorded_sdk" || "$latest_recorded_sdk" == "null" ]]; then
    fail "Registry entry is missing 'soroban_sdk' key"
fi

latest_recorded_timestamp=$(jq -r '.deployments | last | .timestamp' "$REGISTRY_FILE")
if [[ -z "$latest_recorded_timestamp" || "$latest_recorded_timestamp" == "null" ]]; then
    fail "Registry entry is missing 'timestamp' key"
fi

pass "rollback.sh correctly auto-resolved previous hash, executed, and updated the registry with SDK and timestamp"

# ------------------------------------------------------------------------------
# Test 5: upgrade_contract.sh updates the registry
# ------------------------------------------------------------------------------
echo "Test 5: upgrade_contract.sh registers upgrades..."
# Upgrade contract CESCROW... to new WASM
if ! "$SCRIPT_DIR/upgrade_contract.sh" "$MOCK_ESCROW_ID" "$FAKE_WASM" "$NETWORK" "mock-deployer"; then
    fail "upgrade_contract.sh failed"
fi

# Check that the upgrade was recorded to deployments/testnet.json
# upgrade_contract.sh installs new WASM which mock CLI returns as cccccccc...
latest_hash=$(jq -r '.deployments | last | .wasm_hash' "$REGISTRY_FILE")
if [[ "$latest_hash" != "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc" ]]; then
    fail "upgrade_contract.sh did not record the upgrade in the registry. Got: $latest_hash, expected cccccccc..."
fi

pass "upgrade_contract.sh successfully upgraded contract and updated the registry"

# Clean up fake WASM
rm -f "$FAKE_WASM"

echo "=== All Registry Integration Tests Passed! ==="
