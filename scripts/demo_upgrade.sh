#!/bin/bash
set -e

# Configuration
NETWORK="testnet"
CONTRACT_DIR="grainlify-core"
SRC_FILE="$CONTRACT_DIR/src/lib.rs"
SOURCE="demo_user"
WASM_FILE="target/wasm32-unknown-unknown/release/grainlify_core.wasm"
MIN_UPGRADE_WAIT_SECONDS=${MIN_UPGRADE_WAIT_SECONDS:-310}

echo "=== Grainlify Contract Upgrade Demo ==="

# 1. Build V1
echo "[1/10] Building V1..."
cargo build --target wasm32-unknown-unknown --release --manifest-path "$CONTRACT_DIR/Cargo.toml"
WASM_V1="$WASM_FILE"

# 2. Setup Identity
echo "[2/10] Setting up Identity..."
soroban keys generate "$SOURCE" --network "$NETWORK" --overwrite || true
soroban keys fund "$SOURCE" --network "$NETWORK"

# 3. Deploy V1
echo "[3/10] Deploying V1..."
ID=$(soroban contract deploy --wasm "$WASM_V1" --source "$SOURCE" --network "$NETWORK")
echo "Contract Deployed: $ID"

# 4. Initialize V1
echo "[4/10] Initializing V1..."
ADMIN_ADDR=$(soroban keys address "$SOURCE")
soroban contract invoke --id "$ID" --source "$SOURCE" --network "$NETWORK" --send=yes -- init_admin --admin "$ADMIN_ADDR"

# Use the minimum supported delay so the demo is runnable without waiting 24 hours.
echo "[5/10] Setting minimum upgrade delay..."
soroban contract invoke --id "$ID" --source "$SOURCE" --network "$NETWORK" --send=yes -- set_upgrade_delay --delay_seconds 300

# 6. Check Version
echo "[6/10] Checking Version (Expect: 2)..."
VER=$(soroban contract invoke --id "$ID" --source "$SOURCE" --network "$NETWORK" -- get_version)
echo "Current Version: $VER"

if [[ "$VER" != *"2"* ]]; then
    echo "Error: Expected version 2, got $VER"
    exit 1
fi

# 7. Modify Code to V3
echo "[7/10] Modifying code to Version 3..."
cp "$SRC_FILE" "$SRC_FILE.bak"
restore_source() {
    if [ -f "$SRC_FILE.bak" ]; then
        mv "$SRC_FILE.bak" "$SRC_FILE"
    fi
}
trap restore_source EXIT
# Change get_version to return hardcoded 3.
sed -i 's/env.storage().instance().get(&DataKey::Version).unwrap_or(0)/3/' "$SRC_FILE"

# 8. Build V3
echo "[8/10] Building V3..."
cargo build --target wasm32-unknown-unknown --release --manifest-path "$CONTRACT_DIR/Cargo.toml"
WASM_V2="$WASM_FILE"

# 9. Schedule and execute upgrade
echo "[9/10] Scheduling and executing Contract Upgrade..."
./scripts/upgrade_contract.sh "$ID" "$WASM_V2" "$NETWORK" "$SOURCE" "$MIN_UPGRADE_WAIT_SECONDS"

# 10. Check Version
echo "[10/10] Checking Version (Expect: 3)..."
VER=$(soroban contract invoke --id "$ID" --source "$SOURCE" --network "$NETWORK" -- get_version)
echo "Current Version: $VER"

if [[ "$VER" != *"3"* ]]; then
    echo "Error: Expected version 3, got $VER"
    exit 1
fi

echo "=== Demo Successful ==="