#!/bin/bash
set -e

# Usage: ./upgrade_contract.sh <CONTRACT_ID> <WASM_FILE> <NETWORK> <SOURCE_IDENTITY> [WAIT_SECONDS]
# Example: ./upgrade_contract.sh C... target/wasm32-unknown-unknown/release/grainlify_core.wasm testnet demo_user 310
# If WAIT_SECONDS is omitted, the script uploads and schedules the upgrade, then exits
# before execution so callers can wait for the configured timelock themselves.

CONTRACT_ID=$1
WASM_FILE=$2
NETWORK=${3:-testnet}
SOURCE=${4:-default}
WAIT_SECONDS=${5:-}

if [ -z "$CONTRACT_ID" ] || [ -z "$WASM_FILE" ]; then
    echo "Usage: $0 <CONTRACT_ID> <WASM_FILE> [NETWORK] [SOURCE_IDENTITY] [WAIT_SECONDS]"
    exit 1
fi

echo "Uploading WASM..."
# Use 'upload' instead of 'install' as per deprecation warning
WASM_HASH=$(soroban contract upload --wasm "$WASM_FILE" --network "$NETWORK" --source "$SOURCE")
echo "WASM Hash: $WASM_HASH"

echo "Scheduling contract upgrade..."
soroban contract invoke \
    --id "$CONTRACT_ID" \
    --network "$NETWORK" \
    --source "$SOURCE" \
    --send=yes \
    -- \
    schedule_upgrade \
    --wasm_hash "$WASM_HASH"

if [ -z "$WAIT_SECONDS" ]; then
    echo "Upgrade scheduled. Wait until the scheduled executable_at timestamp, then run:"
    echo "soroban contract invoke --id $CONTRACT_ID --network $NETWORK --source $SOURCE --send=yes -- upgrade --new_wasm_hash $WASM_HASH"
    exit 0
fi

echo "Waiting $WAIT_SECONDS seconds for the upgrade timelock..."
sleep "$WAIT_SECONDS"

echo "Executing contract upgrade..."
soroban contract invoke \
    --id "$CONTRACT_ID" \
    --network "$NETWORK" \
    --source "$SOURCE" \
    --send=yes \
    -- \
    upgrade \
    --new_wasm_hash "$WASM_HASH"

echo "Upgrade complete."