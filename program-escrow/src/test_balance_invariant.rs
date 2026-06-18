#![cfg(test)]

/// # Token-Balance Invariant Test Suite — Program Escrow
///
/// Proves that `remaining_balance` (stored in `ProgramData`) never diverges
/// from the real SAC token balance held by the contract across every payout
/// and release lifecycle path:
///
/// ```text
/// token_client.balance(&contract_id)  ==  program_data.remaining_balance
/// ```
///
/// ## Covered paths
///
/// 1. `single_payout` — sequential payouts drain `remaining_balance` step-by-step
/// 2. `batch_payout` — batch payouts atomically decrement `remaining_balance`
/// 3. `trigger_program_releases` — automatic schedule bulk-trigger keeps invariant
/// 4. `release_program_schedule_manual` — manual single schedule release
/// 5. `release_prog_schedule_automatic` — automatic single schedule release
/// 6. Insufficient-balance rejection — invariant is untouched on panic
/// 7. Top-up + mixed payouts — `lock_program_funds` top-up followed by mixed paths

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, vec, Address, Env, String,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_client(env: &Env) -> (ProgramEscrowContractClient<'static>, Address) {
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);
    (client, contract_id)
}

fn make_token(
    env: &Env,
    admin: &Address,
) -> (token::Client<'static>, token::StellarAssetClient<'static>, Address) {
    let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
    let token_id = token_contract.address();
    (
        token::Client::new(env, &token_id),
        token::StellarAssetClient::new(env, &token_id),
        token_id,
    )
}

/// Core invariant check: SAC balance held by the contract must equal
/// `remaining_balance` tracked inside `ProgramData`.
fn assert_balance_invariant(
    client: &ProgramEscrowContractClient,
    token_client: &token::Client,
    contract_id: &Address,
    label: &str,
) {
    let sac = token_client.balance(contract_id);
    let remaining = client.get_remaining_balance();
    assert_eq!(
        sac,
        remaining,
        "[{}] INVARIANT VIOLATED — SAC balance ({}) ≠ remaining_balance ({})",
        label,
        sac,
        remaining,
    );
}

/// Set up a fully initialized and funded program, returning
/// (client, contract_id, admin, token_client, token_sac_client).
fn setup(
    env: &Env,
    amount: i128,
) -> (
    ProgramEscrowContractClient<'static>,
    Address,
    Address,
    token::Client<'static>,
    token::StellarAssetClient<'static>,
    Address, // token_id
) {
    env.mock_all_auths();
    let (client, contract_id) = make_client(env);
    let admin = Address::generate(env);
    let (token_client, token_sac, token_id) = make_token(env, &admin);
    let program_id = String::from_str(env, "test-program");

    token_sac.mint(&admin, &amount);
    client.init_program(&program_id, &admin, &token_id);
    if amount > 0 {
        client.lock_program_funds(&admin, &amount);
    }

    (client, contract_id, admin, token_client, token_sac, token_id)
}

// ---------------------------------------------------------------------------
// Test 1 — sequential single_payout drains remaining_balance step-by-step
// ---------------------------------------------------------------------------

#[test]
fn test_invariant_sequential_single_payouts() {
    let env = Env::default();
    let total: i128 = 9_000;
    let (client, contract_id, _admin, token_client, _token_sac, _token_id) =
        setup(&env, total);

    assert_balance_invariant(&client, &token_client, &contract_id, "initial");

    let w1 = Address::generate(&env);
    let w2 = Address::generate(&env);
    let w3 = Address::generate(&env);

    client.single_payout(&w1, &3_000);
    assert_balance_invariant(&client, &token_client, &contract_id, "after payout 1");
    assert_eq!(client.get_remaining_balance(), 6_000);

    client.single_payout(&w2, &2_500);
    assert_balance_invariant(&client, &token_client, &contract_id, "after payout 2");
    assert_eq!(client.get_remaining_balance(), 3_500);

    client.single_payout(&w3, &3_500);
    assert_balance_invariant(&client, &token_client, &contract_id, "after payout 3 (drain)");
    assert_eq!(client.get_remaining_balance(), 0);
    assert_eq!(token_client.balance(&contract_id), 0);
}

// ---------------------------------------------------------------------------
// Test 2 — batch_payout atomically decrements remaining_balance
// ---------------------------------------------------------------------------

#[test]
fn test_invariant_batch_payout() {
    let env = Env::default();
    let total: i128 = 12_000;
    let (client, contract_id, _admin, token_client, _token_sac, _token_id) =
        setup(&env, total);

    assert_balance_invariant(&client, &token_client, &contract_id, "initial");

    let w1 = Address::generate(&env);
    let w2 = Address::generate(&env);
    let w3 = Address::generate(&env);

    let recipients = vec![&env, w1.clone(), w2.clone(), w3.clone()];
    let amounts = vec![&env, 4_000_i128, 3_000_i128, 5_000_i128];

    client.batch_payout(&recipients, &amounts);
    assert_balance_invariant(&client, &token_client, &contract_id, "after batch payout");
    assert_eq!(client.get_remaining_balance(), 0);
    assert_eq!(token_client.balance(&contract_id), 0);
}

// ---------------------------------------------------------------------------
// Test 3 — trigger_program_releases bulk-triggers due schedules
// ---------------------------------------------------------------------------

#[test]
fn test_invariant_trigger_program_releases() {
    let env = Env::default();
    let total: i128 = 6_000;
    let (client, contract_id, admin, token_client, _token_sac, _token_id) =
        setup(&env, total);

    assert_balance_invariant(&client, &token_client, &contract_id, "initial");

    let w1 = Address::generate(&env);
    let w2 = Address::generate(&env);

    // Create two due schedules (release_timestamp = 0, now is also 0)
    client.create_program_release_schedule(&2_000, &0, &w1);
    client.create_program_release_schedule(&4_000, &0, &w2);

    // Invariant still holds — no funds have been transferred yet
    assert_balance_invariant(
        &client,
        &token_client,
        &contract_id,
        "after creating schedules",
    );

    // Trigger all due schedules
    let released = client.trigger_program_releases();
    assert_eq!(released, 2);

    assert_balance_invariant(
        &client,
        &token_client,
        &contract_id,
        "after trigger_program_releases",
    );
    assert_eq!(client.get_remaining_balance(), 0);
    assert_eq!(token_client.balance(&contract_id), 0);
}

// ---------------------------------------------------------------------------
// Test 4 — release_program_schedule_manual keeps invariant
// ---------------------------------------------------------------------------

#[test]
fn test_invariant_release_program_schedule_manual() {
    let env = Env::default();
    let total: i128 = 8_000;
    let (client, contract_id, _admin, token_client, _token_sac, _token_id) =
        setup(&env, total);

    assert_balance_invariant(&client, &token_client, &contract_id, "initial");

    let w1 = Address::generate(&env);
    let w2 = Address::generate(&env);

    // Create two schedules (release_timestamp = 0 so they are immediately eligible)
    let sched1 = client.create_program_release_schedule(&3_000, &0, &w1);
    let sched2 = client.create_program_release_schedule(&5_000, &0, &w2);

    assert_balance_invariant(&client, &token_client, &contract_id, "after creating schedules");

    // Release first schedule manually
    client.release_program_schedule_manual(&sched1.schedule_id);
    assert_balance_invariant(
        &client,
        &token_client,
        &contract_id,
        "after manual release 1",
    );
    assert_eq!(client.get_remaining_balance(), 5_000);

    // Release second schedule manually
    client.release_program_schedule_manual(&sched2.schedule_id);
    assert_balance_invariant(
        &client,
        &token_client,
        &contract_id,
        "after manual release 2",
    );
    assert_eq!(client.get_remaining_balance(), 0);
    assert_eq!(token_client.balance(&contract_id), 0);
}

// ---------------------------------------------------------------------------
// Test 5 — release_prog_schedule_automatic keeps invariant
// ---------------------------------------------------------------------------

#[test]
fn test_invariant_release_prog_schedule_automatic() {
    let env = Env::default();
    let total: i128 = 5_000;
    let (client, contract_id, _admin, token_client, _token_sac, _token_id) =
        setup(&env, total);

    assert_balance_invariant(&client, &token_client, &contract_id, "initial");

    let winner = Address::generate(&env);

    // Create a schedule with release_timestamp = 100
    let sched = client.create_program_release_schedule(&5_000, &100, &winner);

    assert_balance_invariant(&client, &token_client, &contract_id, "after creating schedule");

    // Advance ledger timestamp past release point
    env.ledger().set_timestamp(200);

    client.release_prog_schedule_automatic(&sched.schedule_id);
    assert_balance_invariant(
        &client,
        &token_client,
        &contract_id,
        "after automatic release",
    );
    assert_eq!(client.get_remaining_balance(), 0);
    assert_eq!(token_client.balance(&contract_id), 0);
}

// ---------------------------------------------------------------------------
// Test 6 — over-payout attempt panics; invariant is preserved
// ---------------------------------------------------------------------------

#[test]
fn test_invariant_overpayout_rejected() {
    let env = Env::default();
    let total: i128 = 1_000;
    let (client, contract_id, _admin, token_client, _token_sac, _token_id) =
        setup(&env, total);

    assert_balance_invariant(&client, &token_client, &contract_id, "initial");

    let winner = Address::generate(&env);

    // Attempt a payout exceeding available funds — should panic
    let result = client.try_single_payout(&winner, &2_000);
    assert!(result.is_err(), "expected over-payout to fail");

    // Invariant must still hold after the rejected call
    assert_balance_invariant(
        &client,
        &token_client,
        &contract_id,
        "after rejected over-payout",
    );
    assert_eq!(client.get_remaining_balance(), total);
}

// ---------------------------------------------------------------------------
// Test 7 — top-up via lock_program_funds then mixed payout paths
// ---------------------------------------------------------------------------

#[test]
fn test_invariant_topup_then_mixed_payouts() {
    let env = Env::default();
    let initial: i128 = 5_000;
    let topup: i128 = 3_000;
    let (client, contract_id, admin, token_client, token_sac, _token_id) =
        setup(&env, initial);

    assert_balance_invariant(&client, &token_client, &contract_id, "after initial lock");

    // Top up with additional funds
    token_sac.mint(&admin, &topup);
    client.lock_program_funds(&admin, &topup);
    assert_balance_invariant(&client, &token_client, &contract_id, "after top-up");
    assert_eq!(client.get_remaining_balance(), initial + topup);

    let w1 = Address::generate(&env);
    let w2 = Address::generate(&env);
    let w3 = Address::generate(&env);

    // Mix 1: single payout
    client.single_payout(&w1, &2_000);
    assert_balance_invariant(&client, &token_client, &contract_id, "after single_payout");

    // Mix 2: batch payout
    let recipients = vec![&env, w2.clone()];
    let amounts = vec![&env, 3_000_i128];
    client.batch_payout(&recipients, &amounts);
    assert_balance_invariant(&client, &token_client, &contract_id, "after batch_payout");

    // Mix 3: manual schedule release
    let sched = client.create_program_release_schedule(&3_000, &0, &w3);
    client.release_program_schedule_manual(&sched.schedule_id);
    assert_balance_invariant(
        &client,
        &token_client,
        &contract_id,
        "after manual schedule release",
    );
    assert_eq!(client.get_remaining_balance(), 0);
    assert_eq!(token_client.balance(&contract_id), 0);
}
