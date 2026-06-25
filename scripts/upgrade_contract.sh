#!/bin/bash
# ==============================================================================
# Grainlify - Smart Contract Upgrade Helper
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Source common utilities
source "$SCRIPT_DIR/utils/common.sh"

CONTRACT_ID=$1
WASM_FILE=$2
NETWORK=${3:-testnet}
SOURCE=${4:-default}

if [ -z "$CONTRACT_ID" ] || [ -z "$WASM_FILE" ]; then
    echo "Usage: $0 <CONTRACT_ID> <WASM_FILE> [NETWORK] [SOURCE_IDENTITY]"
    exit 1
fi

# Set configuration environment variables for common utilities
export SOROBAN_NETWORK="$NETWORK"
export DEPLOYER_IDENTITY="$SOURCE"
DEPLOYMENT_LOG="${PROJECT_ROOT}/deployments/${NETWORK}.json"

# Run check_dependencies to ensure CLI & jq are installed
check_dependencies

CLI_CMD=$(get_cli_command)

# Derive contract name from WASM file
CONTRACT_NAME=$(basename "$WASM_FILE" .wasm)

log_info "Uploading WASM ($WASM_FILE)..."
WASM_HASH=$($CLI_CMD contract install --wasm "$WASM_FILE" --network "$NETWORK" --source "$SOURCE")
log_info "WASM Hash: $WASM_HASH"

log_info "Upgrading contract $CONTRACT_ID to new WASM..."
$CLI_CMD contract invoke \
    --id "$CONTRACT_ID" \
    --network "$NETWORK" \
    --source "$SOURCE" \
    --send=yes \
    -- \
    upgrade \
    --new_wasm_hash "$WASM_HASH"

log_success "Upgrade complete."

# Record deployment to registry
log_info "Recording upgrade in registry..."
append_to_registry "$DEPLOYMENT_LOG" "$CONTRACT_ID" "$WASM_HASH" "$CONTRACT_NAME" "$WASM_FILE"
