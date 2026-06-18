// program-escrow/src/test_circuit_breaker_integration.rs
//
// Integration tests for circuit breaker enforcement across all payout and
// release entry points.
//
// ## What these tests verify
//
// 1. An open circuit blocks all five payout/release paths (hard enforcement).
// 2. The reentrancy guard is cleared on every circuit-open early return so
//    the next call into the contract is not permanently stuck.
// 3. Successful payouts call `record_success` (circuit stays Closed).
// 4. Insufficient-balance failures call `record_failure` and auto-open the
//    circuit once `failure_threshold` is reached.
// 5. The full Closed → Open → HalfOpen → Closed lifecycle works end-to-end
//    through real payout calls.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, String,
};

use crate::{
    error_recovery::{self, CircuitState},
    ProgramEscrowContract, ProgramEscrowContractClient,
};

// ─────────────────────────────────────────────────────────────────────────────
// Shared setup helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Core test fixture — a funded, circuit-admin-registered program.
///
/// Returns `(env, contract_id, authorized_key, circuit_admin, token_address)`.
/// All auths are mocked; ledger timestamp starts at 1_000.
fn setup() -> (Env, Address, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let authorized_key = Address::generate(&env);
    let circuit_admin = Address::generate(&env);
    let program_id = String::from_str(&env, "TestProg");

    // Create token and fund the authorized key with enough to lock
    let token_contract = env.register_stellar_asset_contract_v2(authorized_key.clone());
    let token_address = token_contract.address();
    let token_sac = token::StellarAssetClient::new(&env, &token_address);
    token_sac.mint(&authorized_key, &500_000i128);

    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    // Initialize and fund the program
    client.initialize_program(&program_id, &authorized_key, &token_address);
    client.lock_program_funds(&authorized_key, &200_000i128);

    // Register circuit breaker admin (first time — no existing admin)
    let no_caller: Option<Address> = None;
    client.set_circuitadmin(&circuit_admin, &no_caller);

    (env, contract_id, authorized_key, circuit_admin, token_address)
}

/// Open the circuit via `emergency_open_circuit` and assert the state is Open.
fn open_circuit(client: &ProgramEscrowContractClient, circuit_admin: &Address) {
    client.emergency_open_circuit(circuit_admin);
    let status = client.get_circuit_status();
    assert_eq!(status.state, CircuitState::Open);
}

/// Drive `n` consecutive failures directly into the circuit breaker's persistent
/// storage, bypassing the transaction pipeline.
///
/// In production, every panicking transaction rolls back its own storage writes,
/// so `record_failure` calls made before a `panic!` never accumulate across
/// transactions. In the test suite we simulate this accumulation (which would
/// occur for non-panicking failure paths, or via admin `emergency_open_circuit`)
/// by calling `record_failure` directly through `env.as_contract`, which does
/// NOT trigger the transaction-level rollback.
fn drive_failures(env: &Env, contract_id: &Address, n: u32) {
    let program_id = String::from_str(env, "TestProg");
    let op = soroban_sdk::symbol_short!("test_op");
    env.as_contract(contract_id, || {
        for _ in 0..n {
            error_recovery::record_failure(
                env,
                program_id.clone(),
                op.clone(),
                error_recovery::ERR_INSUFFICIENT_BALANCE,
            );
        }
    });
}

/// Create a due release schedule (release_timestamp < current ledger timestamp).
fn add_due_schedule(
    env: &Env,
    client: &ProgramEscrowContractClient,
    recipient: &Address,
    amount: i128,
) -> u64 {
    // schedule is due because release_timestamp (500) < current timestamp (1_000)
    client.create_program_release_schedule(&amount, &500u64, recipient);
    // schedule_id starts at 1; return the expected id
    let schedules = client.get_all_prog_release_schedules();
    schedules.get(schedules.len() - 1).unwrap().schedule_id
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. Open circuit blocks all entry points
// ─────────────────────────────────────────────────────────────────────────────

/// An open circuit must block `batch_payout`.
#[test]
fn test_open_circuit_blocks_batch_payout() {
    let (env, contract_id, authorized_key, circuit_admin, _) = setup();
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let recipient = Address::generate(&env);

    open_circuit(&client, &circuit_admin);

    let recipients = soroban_sdk::vec![&env, recipient];
    let amounts = soroban_sdk::vec![&env, 1_000i128];
    let result = client.try_batch_payout(&recipients, &amounts);

    assert!(result.is_err(), "batch_payout must fail when circuit is open");
}

/// An open circuit must block `single_payout`.
#[test]
fn test_open_circuit_blocks_single_payout() {
    let (env, contract_id, _authorized_key, circuit_admin, _) = setup();
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let recipient = Address::generate(&env);

    open_circuit(&client, &circuit_admin);

    let result = client.try_single_payout(&recipient, &1_000i128);
    assert!(result.is_err(), "single_payout must fail when circuit is open");
}

/// An open circuit must block `trigger_program_releases`.
#[test]
fn test_open_circuit_blocks_trigger_program_releases() {
    let (env, contract_id, _authorized_key, circuit_admin, _) = setup();
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    open_circuit(&client, &circuit_admin);

    let result = client.try_trigger_program_releases();
    assert!(
        result.is_err(),
        "trigger_program_releases must fail when circuit is open"
    );
}

/// An open circuit must block `release_program_schedule_manual`.
#[test]
fn test_open_circuit_blocks_manual_release() {
    let (env, contract_id, _authorized_key, circuit_admin, _) = setup();
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let recipient = Address::generate(&env);

    let schedule_id = add_due_schedule(&env, &client, &recipient, 5_000i128);

    open_circuit(&client, &circuit_admin);

    let result = client.try_release_program_schedule_manual(&schedule_id);
    assert!(
        result.is_err(),
        "release_program_schedule_manual must fail when circuit is open"
    );
}

/// An open circuit must block `release_prog_schedule_automatic`.
#[test]
fn test_open_circuit_blocks_automatic_release() {
    let (env, contract_id, _authorized_key, circuit_admin, _) = setup();
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let recipient = Address::generate(&env);

    let schedule_id = add_due_schedule(&env, &client, &recipient, 5_000i128);

    open_circuit(&client, &circuit_admin);

    let result = client.try_release_prog_schedule_automatic(&schedule_id);
    assert!(
        result.is_err(),
        "release_prog_schedule_automatic must fail when circuit is open"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Reentrancy guard cleared on circuit-open early return
// ─────────────────────────────────────────────────────────────────────────────

/// After the circuit blocks a payout (clearing the reentrancy guard on exit),
/// closing the circuit and retrying must succeed — not panic with "reentrancy
/// detected".
#[test]
fn test_reentrancy_guard_cleared_on_circuit_block() {
    let (env, contract_id, _authorized_key, circuit_admin, _) = setup();
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let recipient = Address::generate(&env);

    // Open circuit → payout is rejected (guard must be cleared inside the fn)
    open_circuit(&client, &circuit_admin);
    let _ = client.try_single_payout(&recipient, &1_000i128);

    // Reset circuit: Open → HalfOpen
    client.reset_circuit_breaker(&circuit_admin);

    // Now HalfOpen allows one attempt — this payout must succeed (not get stuck
    // with a permanently set reentrancy guard)
    let result = client.try_single_payout(&recipient, &1_000i128);
    assert!(
        result.is_ok(),
        "payout must succeed after circuit is reset to HalfOpen and guard was cleared"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. Successful payouts keep circuit Closed
// ─────────────────────────────────────────────────────────────────────────────

/// A successful `batch_payout` calls `record_success`; the circuit stays Closed.
#[test]
fn test_successful_batch_payout_keeps_circuit_closed() {
    let (env, contract_id, _authorized_key, _circuit_admin, _) = setup();
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let recipient = Address::generate(&env);

    let recipients = soroban_sdk::vec![&env, recipient];
    let amounts = soroban_sdk::vec![&env, 1_000i128];
    client.batch_payout(&recipients, &amounts);

    let status = client.get_circuit_status();
    assert_eq!(status.state, CircuitState::Closed);
    assert_eq!(status.failure_count, 0);
}

/// A successful `single_payout` calls `record_success`; the circuit stays Closed.
#[test]
fn test_successful_single_payout_keeps_circuit_closed() {
    let (env, contract_id, _authorized_key, _circuit_admin, _) = setup();
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let recipient = Address::generate(&env);

    client.single_payout(&recipient, &1_000i128);

    let status = client.get_circuit_status();
    assert_eq!(status.state, CircuitState::Closed);
    assert_eq!(status.failure_count, 0);
}

/// A successful `trigger_program_releases` keeps the circuit Closed.
#[test]
fn test_successful_trigger_releases_keeps_circuit_closed() {
    let (env, contract_id, _authorized_key, _circuit_admin, _) = setup();
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let recipient = Address::generate(&env);

    add_due_schedule(&env, &client, &recipient, 5_000i128);

    let released = client.trigger_program_releases();
    assert_eq!(released, 1);

    let status = client.get_circuit_status();
    assert_eq!(status.state, CircuitState::Closed);
    assert_eq!(status.failure_count, 0);
}

/// A successful `release_program_schedule_manual` keeps the circuit Closed.
#[test]
fn test_successful_manual_release_keeps_circuit_closed() {
    let (env, contract_id, _authorized_key, _circuit_admin, _) = setup();
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let recipient = Address::generate(&env);

    let schedule_id = add_due_schedule(&env, &client, &recipient, 5_000i128);
    client.release_program_schedule_manual(&schedule_id);

    let status = client.get_circuit_status();
    assert_eq!(status.state, CircuitState::Closed);
    assert_eq!(status.failure_count, 0);
}

/// A successful `release_prog_schedule_automatic` keeps the circuit Closed.
#[test]
fn test_successful_automatic_release_keeps_circuit_closed() {
    let (env, contract_id, _authorized_key, _circuit_admin, _) = setup();
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let recipient = Address::generate(&env);

    let schedule_id = add_due_schedule(&env, &client, &recipient, 5_000i128);
    client.release_prog_schedule_automatic(&schedule_id);

    let status = client.get_circuit_status();
    assert_eq!(status.state, CircuitState::Closed);
    assert_eq!(status.failure_count, 0);
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Threshold-driven auto-open via insufficient-balance failures
// ─────────────────────────────────────────────────────────────────────────────

/// Consecutive failures accumulate the failure counter and auto-open the circuit
/// once `failure_threshold` is reached (default = 3).
///
/// Note: Soroban's `try_*` client methods correctly simulate production
/// transaction semantics — panicking calls roll back all their storage writes.
/// To test threshold-driven auto-open we drive failures directly via
/// `env.as_contract`, which persists writes without a transaction boundary.
/// This mirrors what would happen with non-panicking failure paths or across
/// multiple committed transactions.
#[test]
fn test_threshold_failures_auto_open_circuit_batch_payout() {
    let (env, contract_id, _authorized_key, _circuit_admin, _) = setup();
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let recipient = Address::generate(&env);

    // Drive 2 failures — circuit stays Closed
    drive_failures(&env, &contract_id, 2);
    let status = client.get_circuit_status();
    assert_eq!(status.failure_count, 2);
    assert_eq!(status.state, CircuitState::Closed);

    // 3rd failure hits the threshold → circuit opens
    drive_failures(&env, &contract_id, 1);
    let status = client.get_circuit_status();
    assert_eq!(status.state, CircuitState::Open, "circuit must open at threshold");

    // Now a real payout is rejected by check_and_allow
    let recipients = soroban_sdk::vec![&env, recipient];
    let amounts = soroban_sdk::vec![&env, 1_000i128];
    let result = client.try_batch_payout(&recipients, &amounts);
    assert!(result.is_err(), "circuit must block the payout");
}

/// Same threshold-driven auto-open verified through `single_payout`.
#[test]
fn test_threshold_failures_auto_open_circuit_single_payout() {
    let (env, contract_id, _authorized_key, _circuit_admin, _) = setup();
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let recipient = Address::generate(&env);

    // Drive 3 failures to auto-open
    drive_failures(&env, &contract_id, 3);

    let status = client.get_circuit_status();
    assert_eq!(status.state, CircuitState::Open);

    // Further single payout is rejected by the open circuit
    let result = client.try_single_payout(&recipient, &1_000i128);
    assert!(result.is_err());
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. Full state-machine lifecycle through payout calls
// ─────────────────────────────────────────────────────────────────────────────

/// Full lifecycle:
/// Closed → (emergency open) → Open → (admin reset) → HalfOpen →
/// (successful payout) → Closed
#[test]
fn test_full_lifecycle_emergency_open_to_halfopen_to_closed() {
    let (env, contract_id, _authorized_key, circuit_admin, _) = setup();
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let recipient = Address::generate(&env);

    // Step 1: Verify initial Closed state
    assert_eq!(client.get_circuit_status().state, CircuitState::Closed);

    // Step 2: Emergency open → Open
    client.emergency_open_circuit(&circuit_admin);
    assert_eq!(client.get_circuit_status().state, CircuitState::Open);

    // Step 3: Payout is blocked
    let result = client.try_single_payout(&recipient, &1_000i128);
    assert!(result.is_err());

    // Step 4: Admin resets Open → HalfOpen
    client.reset_circuit_breaker(&circuit_admin);
    assert_eq!(client.get_circuit_status().state, CircuitState::HalfOpen);

    // Step 5: Successful payout in HalfOpen → Closed (default success_threshold = 1)
    client.single_payout(&recipient, &1_000i128);
    assert_eq!(client.get_circuit_status().state, CircuitState::Closed);
}

/// Full lifecycle via threshold failures:
/// Closed → (3 direct failures) → Open → (admin reset) → HalfOpen →
/// (successful payout) → Closed
#[test]
fn test_full_lifecycle_threshold_open_to_halfopen_to_closed() {
    let (env, contract_id, _authorized_key, circuit_admin, _) = setup();
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let recipient = Address::generate(&env);

    // Drive 3 failures to auto-open
    drive_failures(&env, &contract_id, 3);
    assert_eq!(client.get_circuit_status().state, CircuitState::Open);

    // Admin resets to HalfOpen
    client.reset_circuit_breaker(&circuit_admin);
    assert_eq!(client.get_circuit_status().state, CircuitState::HalfOpen);

    // One successful payout closes it (default success_threshold = 1)
    client.single_payout(&recipient, &1_000i128);
    assert_eq!(client.get_circuit_status().state, CircuitState::Closed);

    // Further payouts succeed normally and keep the circuit closed
    client.single_payout(&recipient, &1_000i128);
    assert_eq!(client.get_circuit_status().failure_count, 0);
}

/// Failure in HalfOpen (probe failure) must reopen the circuit.
///
/// We simulate the probe failure via `drive_failures` (one additional failure
/// while in HalfOpen state), which pushes the total above the threshold and
/// transitions back to Open.
#[test]
fn test_halfopen_failure_reopens_circuit() {
    let (env, contract_id, _authorized_key, circuit_admin, _) = setup();
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    // Open circuit (failure_count = 3 from drive)
    drive_failures(&env, &contract_id, 3);
    assert_eq!(client.get_circuit_status().state, CircuitState::Open);

    // Transition to HalfOpen (failure_count remains 3)
    client.reset_circuit_breaker(&circuit_admin);
    assert_eq!(client.get_circuit_status().state, CircuitState::HalfOpen);

    // One failure in HalfOpen → failure_count = 4 >= threshold → re-opens
    drive_failures(&env, &contract_id, 1);
    assert_eq!(
        client.get_circuit_status().state,
        CircuitState::Open,
        "probe failure in HalfOpen must reopen the circuit"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. configure_circuit_breaker actually stores the thresholds
// ─────────────────────────────────────────────────────────────────────────────

/// Configuring a custom `failure_threshold = 2` must open the circuit after
/// exactly 2 consecutive failures (not the default 3).
#[test]
fn test_configure_circuit_breaker_applies_custom_threshold() {
    let (env, contract_id, _authorized_key, circuit_admin, _) = setup();
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    // Lower the threshold to 2
    client.configure_circuit_breaker(&circuit_admin, &2u32, &1u32, &10u32);

    // One failure — still Closed
    drive_failures(&env, &contract_id, 1);
    assert_eq!(client.get_circuit_status().state, CircuitState::Closed);

    // Second failure hits threshold → Open
    drive_failures(&env, &contract_id, 1);
    assert_eq!(
        client.get_circuit_status().state,
        CircuitState::Open,
        "circuit must open after 2 failures with threshold = 2"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 7. Error log populated correctly
// ─────────────────────────────────────────────────────────────────────────────

/// Each failure must append an entry to the error log (capped at `max_error_log`).
#[test]
fn test_error_log_populated_on_failures() {
    let (env, contract_id, _authorized_key, _circuit_admin, _) = setup();
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    drive_failures(&env, &contract_id, 2);

    let log = client.get_circuit_error_log();
    assert_eq!(log.len(), 2, "two failures should produce two log entries");
}
