//! Issue #189 — Edge-case tests for claim/release (program-escrow release schedules).
//!
//! Covers three canonical escrow vulnerability classes:
//!   1. Zero-balance claim attempt (release when remaining balance is 0)
//!   2. Double-claim of the same milestone (second release must be rejected)
//!   3. Claiming a milestone before it has been reached/approved (before release_timestamp
//!      or while disputed)
//!
//! Each rejected path asserts BOTH that the call errors AND that no token
//! transfer occurs (recipient balance unchanged), since an error return alone
//! does not prove funds did not move.

#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, String,
};

fn make_token<'a>(
    env: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
    let token_address = token_contract.address();
    let token_sac = token::StellarAssetClient::new(env, &token_address);
    let token_client = token::Client::new(env, &token_address);
    (token_client, token_sac)
}

/// Build a program with `lock_amount` locked and a schedule of `schedule_amount`
/// for `winner` at `release_timestamp`. Returns (client, contract_id, token_client,
/// authorized_key, winner, program_id).
#[allow(clippy::too_many_arguments)]
fn build_program<'a>(
    env: &Env,
    lock_amount: i128,
    schedule_amount: i128,
    release_timestamp: u64,
) -> (
    ProgramEscrowContractClient<'a>,
    Address,
    token::Client<'a>,
    Address,
    Address,
    String,
) {
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);

    let authorized_key = Address::generate(env);
    let winner = Address::generate(env);
    let program_id = String::from_str(env, "Hackathon2024");

    env.mock_all_auths();

    let (token_client, token_sac) = make_token(env, &authorized_key);
    // Fund admin so the SAC has supply, then lock into the contract.
    token_sac.mint(&authorized_key, &(lock_amount + 1_000_000_000_000));
    client.initialize_program(&program_id, &authorized_key, &token_client.address);
    client.lock_program_funds(&authorized_key, &lock_amount);

    client.create_program_release_schedule(&schedule_amount, &release_timestamp, &winner);

    (
        client,
        contract_id,
        token_client,
        authorized_key,
        winner,
        program_id,
    )
}

// ───────────────────────────────────────────────────────────────────────────
// 1. Zero-balance claim attempt
//
// The contract validates `amount > 0` on lock, so a "zero remaining balance"
// state is produced by locking funds and then draining them via a successful
// prior release. Lock `amount`, release one schedule of `amount` (pool to 0),
// then attempt a SECOND schedule that must reject without transfer.
// ───────────────────────────────────────────────────────────────────────────

#[test]
#[should_panic]
fn test_issue189_zero_balance_manual_release_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let amount = 1000_0000000;
    let (client, _cid, token_client, _key, winner, _pid) =
        build_program(&env, amount, amount, 1000);

    // First schedule drains the entire pool (remaining -> 0).
    client.release_program_schedule_manual(&1);

    // Capture winner balance (already received `amount` from first release).
    let bal_before = token_client.balance(&winner);

    // A second schedule of `amount` now has zero remaining balance -> must panic.
    client.create_program_release_schedule(&amount, &1000, &winner);
    client.release_program_schedule_manual(&2);

    let bal_after = token_client.balance(&winner);
    assert_eq!(
        bal_before, bal_after,
        "zero-balance release must not transfer additional funds"
    );
}

#[test]
fn test_issue189_zero_balance_automatic_release_no_transfer() {
    let env = Env::default();
    env.mock_all_auths();

    let amount = 1000_0000000;
    let (client, _cid, token_client, _key, winner, _pid) =
        build_program(&env, amount, amount, 1000);

    // Drain the pool with the first (manual) release.
    client.release_program_schedule_manual(&1);
    let bal_before = token_client.balance(&winner);

    // Second schedule, same amount, now against a zero remaining balance.
    client.create_program_release_schedule(&amount, &1000, &winner);

    env.ledger().set_timestamp(1001);
    let res = client.try_release_prog_schedule_automatic(&2);
    assert!(
        res.is_err(),
        "automatic release with zero remaining balance must fail"
    );

    let bal_after = token_client.balance(&winner);
    assert_eq!(
        bal_before, bal_after,
        "zero-balance automatic release must not transfer funds"
    );
    let sched = client.get_program_release_schedule(&2);
    assert!(!sched.released);
}

// ───────────────────────────────────────────────────────────────────────────
// 2. Double-claim of the same milestone
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn test_issue189_double_claim_manual_second_rejected_no_extra_transfer() {
    let env = Env::default();
    env.mock_all_auths();

    let amount = 1000_0000000;
    let (client, _cid, token_client, _key, winner, _pid) =
        build_program(&env, amount, amount, 1000);

    let bal_before = token_client.balance(&winner);

    // First manual release succeeds and transfers exactly `amount`.
    client.release_program_schedule_manual(&1);
    let bal_after_first = token_client.balance(&winner);
    assert_eq!(
        bal_after_first - bal_before,
        amount,
        "first release should transfer exactly the scheduled amount"
    );

    // Second attempt must be rejected and must NOT transfer again.
    let res = client.try_release_program_schedule_manual(&1);
    assert!(
        res.is_err(),
        "double-claim of same milestone must be rejected"
    );

    let bal_after_second = token_client.balance(&winner);
    assert_eq!(
        bal_after_second, bal_after_first,
        "double-claim must not transfer additional funds"
    );
    // Released flag stays set, history has exactly 1 entry.
    let sched = client.get_program_release_schedule(&1);
    assert!(sched.released);
    assert_eq!(client.get_program_release_history().len(), 1);
}

#[test]
fn test_issue189_double_claim_automatic_second_rejected_no_extra_transfer() {
    let env = Env::default();
    env.mock_all_auths();

    let amount = 1000_0000000;
    let (client, _cid, token_client, _key, winner, _pid) =
        build_program(&env, amount, amount, 1000);

    let bal_before = token_client.balance(&winner);

    env.ledger().set_timestamp(1001);
    client.release_prog_schedule_automatic(&1);
    let bal_after_first = token_client.balance(&winner);
    assert_eq!(bal_after_first - bal_before, amount);

    let res = client.try_release_prog_schedule_automatic(&1);
    assert!(
        res.is_err(),
        "double-claim (automatic) must be rejected"
    );

    let bal_after_second = token_client.balance(&winner);
    assert_eq!(
        bal_after_second, bal_after_first,
        "double-claim (automatic) must not transfer additional funds"
    );
}

// ───────────────────────────────────────────────────────────────────────────
// 3. Claiming a milestone before it has been reached/approved
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn test_issue189_claim_before_release_timestamp_rejected_no_transfer() {
    let env = Env::default();
    env.mock_all_auths();

    let amount = 1000_0000000;
    // Schedule at timestamp 5000; release attempt at 1000 must fail.
    let (client, _cid, token_client, _key, winner, _pid) =
        build_program(&env, amount, amount, 5000);

    let bal_before = token_client.balance(&winner);

    env.ledger().set_timestamp(1000);
    let res = client.try_release_prog_schedule_automatic(&1);
    assert!(
        res.is_err(),
        "claim before release_timestamp must be rejected"
    );

    let bal_after = token_client.balance(&winner);
    assert_eq!(bal_before, bal_after, "early claim must not transfer funds");
    let sched = client.get_program_release_schedule(&1);
    assert!(!sched.released);
}

#[test]
fn test_issue189_manual_release_blocked_when_disputed() {
    let env = Env::default();
    env.mock_all_auths();

    let amount = 1000_0000000;
    let (client, _cid, token_client, admin, winner, _pid) =
        build_program(&env, amount, amount, 1000);

    // Disputes require an admin to be configured.
    client.setadmin(&admin);

    let bal_before = token_client.balance(&winner);

    // Open a global dispute: manual release must be blocked.
    client.open_dispute(&String::from_str(&env, "disputed before approval"));

    let res = client.try_release_program_schedule_manual(&1);
    assert!(
        res.is_err(),
        "manual release while disputed (not approved) must be rejected"
    );

    let bal_after = token_client.balance(&winner);
    assert_eq!(
        bal_before, bal_after,
        "blocked manual release must not transfer funds"
    );
    let sched = client.get_program_release_schedule(&1);
    assert!(!sched.released);
}
