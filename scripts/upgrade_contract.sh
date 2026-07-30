#!/bin/bash
set -e

# Usage: ./upgrade_contract.sh <CONTRACT_ID> <WASM_FILE> <NETWORK> <SOURCE_IDENTITY>
# Example: ./upgrade_contract.sh C... contracts/grainlify-core/target/wasm32-unknown-unknown/release/grainlify_core.wasm testnet demo_user

CONTRACT_ID=$1
WASM_FILE=$2
NETWORK=${3:-testnet}
SOURCE=${4:-default}

if [ -z "$CONTRACT_ID" ] || [ -z "$WASM_FILE" ]; then
    echo "Usage: $0 <CONTRACT_ID> <WASM_FILE> [NETWORK] [SOURCE_IDENTITY]"
    exit 1
fi

echo "Uploading WASM..."
WASM_HASH=$(soroban contract upload --wasm "$WASM_FILE" --network "$NETWORK" --source "$SOURCE")
echo "WASM Hash: $WASM_HASH"

echo "Checking for an existing scheduled upgrade..."
EXISTING_SCHEDULE=$(soroban contract invoke \
    --id "$CONTRACT_ID" \
    --network "$NETWORK" \
    --source "$SOURCE" \
    -- \
    get_scheduled_upgrade 2>/dev/null || true)

EXISTING_HASH=""
if [ -n "$EXISTING_SCHEDULE" ]; then
    EXISTING_HASH=$(echo "$EXISTING_SCHEDULE" | jq -r '.wasm_hash // empty' 2>/dev/null || true)
fi

EXECUTABLE_AT=""

if [ -n "$EXISTING_HASH" ] && [ "$EXISTING_HASH" = "$WASM_HASH" ]; then
    echo "Matching upgrade already scheduled — skipping schedule_upgrade."
    EXECUTABLE_AT=$(echo "$EXISTING_SCHEDULE" | jq -r '.executable_at // empty')
else
    echo "Scheduling upgrade..."
    soroban contract invoke \
        --id "$CONTRACT_ID" \
        --network "$NETWORK" \
        --source "$SOURCE" \
        --send=yes \
        -- \
        schedule_upgrade \
        --wasm_hash "$WASM_HASH"

    SCHEDULED_JSON=$(soroban contract invoke \
        --id "$CONTRACT_ID" \
        --network "$NETWORK" \
        --source "$SOURCE" \
        -- \
        get_scheduled_upgrade)

    EXECUTABLE_AT=$(echo "$SCHEDULED_JSON" | jq -r '.executable_at // empty')
fi

if [ -z "$EXECUTABLE_AT" ]; then
    echo "Could not determine executable_at from get_scheduled_upgrade output."
    echo "Re-run this script once you've confirmed the timelock has elapsed."
    exit 0
fi

NOW_EPOCH=$(date +%s)
EXECUTABLE_HUMAN=$(date -u -d "@$EXECUTABLE_AT" '+%Y-%m-%d %H:%M:%S UTC' 2>/dev/null \
    || date -u -r "$EXECUTABLE_AT" '+%Y-%m-%d %H:%M:%S UTC')

if [ "$NOW_EPOCH" -lt "$EXECUTABLE_AT" ]; then
    WAIT_SECONDS=$((EXECUTABLE_AT - NOW_EPOCH))
    echo ""
    echo "=== Upgrade Scheduled — Timelock Pending ==="
    echo "  Contract ID:    $CONTRACT_ID"
    echo "  New WASM Hash:  $WASM_HASH"
    echo "  Executable at:  $EXECUTABLE_HUMAN ($EXECUTABLE_AT)"
    echo "  Time remaining: ${WAIT_SECONDS}s"
    echo ""
    echo "Re-run this exact command after the time above to complete the upgrade:"
    echo "  $0 $CONTRACT_ID $WASM_FILE $NETWORK $SOURCE"
    echo ""
    exit 0
fi

echo "Timelock has elapsed (executable at $EXECUTABLE_HUMAN) — proceeding to upgrade."

echo "Upgrading contract..."
if ! UPGRADE_RESULT=$(soroban contract invoke \
    --id "$CONTRACT_ID" \
    --network "$NETWORK" \
    --source "$SOURCE" \
    --send=yes \
    -- \
    upgrade \
    --new_wasm_hash "$WASM_HASH" 2>&1); then

    echo "Upgrade invocation failed."
    echo "Output: $UPGRADE_RESULT"
    echo ""
    echo "Possible causes:"
    echo "  - Source identity is not the contract admin"
    echo "  - Contract does not have an 'upgrade' function"
    echo "  - No scheduled upgrade exists yet, or its timelock hasn't elapsed"
    exit 1
fi

echo "Upgrade complete."