#!/usr/bin/env bash
# ==============================================================================
# Grainlify - rollback.sh timelock integration test (issue #488)
# ==============================================================================
# Exercises perform_rollback's schedule_upgrade + timelock handling against a
# fake `stellar` CLI that simulates grainlify-core's ScheduledUpgrade state
# machine, instead of a real deployed contract. This lets the test control
# timing precisely (a real 24h default delay is not something CI should ever
# wait on) while still driving rollback.sh exactly as it would invoke a real
# CLI — same subcommands, same flags, same stdout/exit-code contract.
#
# Does NOT replace testing against a real local sandbox deployment (see the
# issue's own suggested execution step 4); it covers the bash orchestration
# logic that a real-network test would be slow and non-deterministic for.
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROLLBACK_SCRIPT="$SCRIPT_DIR/rollback.sh"

TEST_TMP="$(mktemp -d)"

# rollback.sh writes real rollback history to deployments/rollbacks.json
# relative to the repo root (by design — that's what a real operator run is
# supposed to do). Back it up and restore it so running this test doesn't
# pollute the real registry with fake test entries.
ROLLBACK_LOG_PATH="$SCRIPT_DIR/../deployments/rollbacks.json"
ROLLBACK_LOG_BACKUP=""
if [[ -f "$ROLLBACK_LOG_PATH" ]]; then
    ROLLBACK_LOG_BACKUP="$TEST_TMP/rollbacks.json.orig"
    cp "$ROLLBACK_LOG_PATH" "$ROLLBACK_LOG_BACKUP"
fi

cleanup() {
    if [[ -n "$ROLLBACK_LOG_BACKUP" ]]; then
        cp "$ROLLBACK_LOG_BACKUP" "$ROLLBACK_LOG_PATH"
    else
        rm -f "$ROLLBACK_LOG_PATH"
    fi
    rm -rf "$TEST_TMP"
}
trap cleanup EXIT

FAKE_CLI_DIR="$TEST_TMP/bin"
STATE_DIR="$TEST_TMP/state"
mkdir -p "$FAKE_CLI_DIR" "$STATE_DIR"

CONTRACT_ID="CTESTCONTRACTID000000000000000000000000000000000000000001"
TARGET_HASH="1111111111111111111111111111111111111111111111111111111111111111"
TARGET_HASH="${TARGET_HASH:0:64}"

fail() { echo "✘ FAIL: $1"; exit 1; }
pass() { echo "✔ PASS: $1"; }

# ------------------------------------------------------------------------------
# Fake `stellar` CLI
# ------------------------------------------------------------------------------
# Understands exactly the subset of `stellar keys address` / `stellar contract
# invoke -- <fn>` that rollback.sh calls, and simulates ScheduledUpgrade state
# (schedule_upgrade / get_scheduled_upgrade / is_upgrade_ready / upgrade /
# get_upgrade_delay) in $STATE_DIR/schedule.json, driven by a real wall clock
# so rollback.sh's own `date +%s` reads agree with it.
write_fake_cli() {
    local delay_seconds="$1"

    cat > "$FAKE_CLI_DIR/stellar" <<EOF
#!/usr/bin/env bash
set -euo pipefail
STATE_DIR="$STATE_DIR"
DELAY=$delay_seconds
SCHEDULE_FILE="\$STATE_DIR/schedule.json"

if [[ "\$1" == "keys" && "\$2" == "address" ]]; then
    echo "GFAKEADMINADDRESS0000000000000000000000000000000000000"
    exit 0
fi

if [[ "\$1" == "contract" && "\$2" == "invoke" ]]; then
    # Find the function name: first arg after the literal "--"
    fn=""
    args=("\$@")
    for i in "\${!args[@]}"; do
        if [[ "\${args[\$i]}" == "--" ]]; then
            fn="\${args[\$((i+1))]}"
            break
        fi
    done

    case "\$fn" in
        get_upgrade_delay)
            echo "\$DELAY"
            ;;
        is_upgrade_ready)
            if [[ -f "\$SCHEDULE_FILE" ]]; then
                executable_at=\$(cat "\$SCHEDULE_FILE")
                now=\$(date +%s)
                if [[ "\$now" -ge "\$executable_at" ]]; then
                    echo "true"
                else
                    echo "false"
                fi
            else
                echo "false"
            fi
            ;;
        schedule_upgrade)
            now=\$(date +%s)
            executable_at=\$((now + DELAY))
            echo "\$executable_at" > "\$SCHEDULE_FILE"
            echo "{\"wasm_hash\":\"$TARGET_HASH\",\"scheduled_at\":\$now,\"executable_at\":\$executable_at}"
            ;;
        get_scheduled_upgrade)
            if [[ -f "\$SCHEDULE_FILE" ]]; then
                executable_at=\$(cat "\$SCHEDULE_FILE")
                echo "{\"wasm_hash\":\"$TARGET_HASH\",\"executable_at\":\$executable_at}"
            else
                echo "null"
            fi
            ;;
        upgrade)
            if [[ ! -f "\$SCHEDULE_FILE" ]]; then
                echo "HostError: panic: No scheduled upgrade" >&2
                exit 1
            fi
            executable_at=\$(cat "\$SCHEDULE_FILE")
            now=\$(date +%s)
            if [[ "\$now" -lt "\$executable_at" ]]; then
                echo "HostError: panic: Upgrade timelock not elapsed" >&2
                exit 1
            fi
            rm -f "\$SCHEDULE_FILE"
            echo "success"
            ;;
        *)
            echo "fake stellar CLI: unhandled function '\$fn'" >&2
            exit 1
            ;;
    esac
    exit 0
fi

echo "fake stellar CLI: unhandled invocation: \$*" >&2
exit 1
EOF
    chmod +x "$FAKE_CLI_DIR/stellar"
}

run_rollback() {
    # -n testnet: config/testnet.env points SOROBAN_RPC_URL at the real,
    # publicly reachable testnet endpoint, so preflight's network-connectivity
    # check (a plain curl reachability probe, not an actual RPC call) passes
    # without needing a local Soroban node running for this test.
    PATH="$FAKE_CLI_DIR:$PATH" \
    "$ROLLBACK_SCRIPT" "$CONTRACT_ID" "$TARGET_HASH" \
        -n testnet --force "$@" 2>&1
}

echo "=== rollback.sh timelock integration tests (fake CLI) ==="

# 1. Short delay: script schedules, auto-waits, then upgrades — succeeds
#    end-to-end in one invocation. This is the issue's core acceptance
#    criterion: rollback succeeds against a deployment with an active
#    timelock.
rm -f "$STATE_DIR/schedule.json"
write_fake_cli 2
output=$(MAX_AUTO_WAIT_SECONDS=30 run_rollback) || fail "short-delay rollback exited non-zero: $output"
echo "$output" | grep -q "Rollback executed successfully" \
    || fail "short-delay rollback did not report success: $output"
echo "$output" | grep -qi "will wait it out automatically" \
    || fail "short-delay rollback did not disclose the timelock upfront: $output"
pass "short delay (2s, within MAX_AUTO_WAIT_SECONDS): schedules, waits, upgrades — succeeds in one run"

# 2. Long delay: script schedules and exits cleanly with re-run
#    instructions, instead of blocking for the full delay.
rm -f "$STATE_DIR/schedule.json"
write_fake_cli 86400
start=$(date +%s)
output=$(MAX_AUTO_WAIT_SECONDS=5 run_rollback) || fail "long-delay rollback exited non-zero: $output"
elapsed=$(( $(date +%s) - start ))
[[ "$elapsed" -lt 10 ]] || fail "long-delay rollback blocked for ${elapsed}s instead of exiting promptly"
echo "$output" | grep -qi "Re-run this exact command" \
    || fail "long-delay rollback did not print re-run instructions: $output"
echo "$output" | grep -qi "exceeds MAX_AUTO_WAIT_SECONDS" \
    || fail "long-delay rollback did not explain why it stopped: $output"
[[ -f "$STATE_DIR/schedule.json" ]] || fail "long-delay rollback did not actually call schedule_upgrade"
pass "long delay (86400s, exceeds MAX_AUTO_WAIT_SECONDS): schedules, exits promptly (${elapsed}s) with re-run instructions"

# 3. Re-running after the (simulated) timelock has elapsed picks up the
#    existing schedule and completes without scheduling again.
sed -i "s/.*/$(( $(date +%s) - 1 ))/" "$STATE_DIR/schedule.json"
output=$(MAX_AUTO_WAIT_SECONDS=5 run_rollback) || fail "re-run after elapsed timelock exited non-zero: $output"
echo "$output" | grep -q "Rollback executed successfully" \
    || fail "re-run after elapsed timelock did not succeed: $output"
echo "$output" | grep -qi "already scheduled and its timelock has elapsed" \
    || fail "re-run after elapsed timelock did not recognise the existing schedule: $output"
pass "re-run after timelock elapses: recognises the existing schedule, skips re-scheduling, upgrades"

# 4. schedule_upgrade failure (e.g. wrong admin) surfaces a clear error and
#    does not attempt the upgrade call.
rm -f "$STATE_DIR/schedule.json"
write_fake_cli 60
cat > "$FAKE_CLI_DIR/stellar" <<'EOF'
#!/usr/bin/env bash
if [[ "$1" == "keys" ]]; then echo "GFAKE"; exit 0; fi
if [[ "$*" == *"get_upgrade_delay"* ]]; then echo "60"; exit 0; fi
if [[ "$*" == *"is_upgrade_ready"* ]]; then echo "false"; exit 0; fi
if [[ "$*" == *"schedule_upgrade"* ]]; then
    echo "HostError: panic: Unauthorized" >&2
    exit 1
fi
echo "unexpected call: $*" >&2
exit 1
EOF
chmod +x "$FAKE_CLI_DIR/stellar"
set +e
output=$(run_rollback)
exit_code=$?
set -e
[[ "$exit_code" -ne 0 ]] || fail "schedule_upgrade failure should have exited non-zero"
echo "$output" | grep -q "schedule_upgrade failed" \
    || fail "schedule_upgrade failure did not surface a clear error: $output"
pass "schedule_upgrade failure (e.g. wrong admin) surfaces a clear error, never reaches the upgrade call"

# 5. --dry-run: describes the schedule_upgrade + upgrade two-step plan and
#    the configured delay, without ever calling schedule_upgrade for real.
rm -f "$STATE_DIR/schedule.json"
write_fake_cli 3600
output=$(run_rollback --dry-run) || fail "dry-run exited non-zero: $output"
echo "$output" | grep -q "DRY RUN" \
    || fail "dry-run did not report simulation mode: $output"
echo "$output" | grep -q "schedule_upgrade" \
    || fail "dry-run did not mention the schedule_upgrade step: $output"
echo "$output" | grep -q "3600" \
    || fail "dry-run did not surface the configured delay: $output"
[[ ! -f "$STATE_DIR/schedule.json" ]] \
    || fail "dry-run must not actually call schedule_upgrade"
pass "dry-run describes the schedule_upgrade + upgrade plan and the delay, without executing anything"

echo ""
echo "All rollback.sh timelock tests passed!"
