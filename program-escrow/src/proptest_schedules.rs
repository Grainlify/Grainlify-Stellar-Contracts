#![cfg(test)]

extern crate std;

use crate::{ProgramEscrowContract, ProgramEscrowContractClient};
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestCaseError, TestRng, TestRunner};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, String,
};
use std::format;

const CASES: u32 = 16;
const MAX_SHRINK_ITERS: u32 = 64;

#[derive(Clone, Debug)]
struct ScheduleOp {
    amount: i128,
    release_offset: i64,
}

fn schedule_op_strategy() -> impl Strategy<Value = ScheduleOp> {
    (1_i128..=10_000_i128, -500_i64..=1000_i64).prop_map(|(amount, release_offset)| ScheduleOp {
        amount,
        release_offset,
    })
}

fn proptest_config() -> ProptestConfig {
    ProptestConfig {
        cases: CASES,
        max_shrink_iters: MAX_SHRINK_ITERS,
        failure_persistence: None,
        ..ProptestConfig::default()
    }
}

fn deterministic_runner() -> TestRunner {
    let config = proptest_config();
    let algorithm = config.rng_algorithm;
    TestRunner::new_with_rng(config, TestRng::deterministic_rng(algorithm))
}

fn make_client(env: &Env) -> (ProgramEscrowContractClient<'static>, Address) {
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);
    (client, contract_id)
}

fn fund_contract(
    env: &Env,
    funder: &Address,
    amount: i128,
) -> (token::Client<'static>, Address) {
    let tokenadmin = Address::generate(env);
    let token_contract = env.register_stellar_asset_contract_v2(tokenadmin.clone());
    let token_id = token_contract.address();
    let token_client = token::Client::new(env, &token_id);
    let token_sac = token::StellarAssetClient::new(env, &token_id);
    if amount > 0 {
        token_sac.mint(funder, &amount);
    }
    (token_client, token_id)
}

fn setup_program(
    env: &Env,
    amount: i128,
) -> (
    ProgramEscrowContractClient<'static>,
    Address,
    Address,
    token::Client<'static>,
) {
    env.mock_all_auths();
    let (client, contract_id) = make_client(env);
    let admin = Address::generate(env);
    let (token_client, token_id) = fund_contract(env, &admin, amount);
    let program_id = String::from_str(env, "test-program");
    client.init_program(&program_id, &admin, &token_id);
    if amount > 0 {
        client.lock_program_funds(&admin, &amount);
    }
    (client, admin, contract_id, token_client)
}

// 1. Boundary cases: schedule with zero or negative amount panics at creation time.
#[test]
fn test_zero_and_negative_amount_schedule_reverted() {
    let env = Env::default();
    let (client, _admin, _cid, _token) = setup_program(&env, 10_000);
    
    let recipient = Address::generate(&env);
    
    // Zero amount schedule must panic
    let result_zero = env.as_contract(&client.address, || {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            client.create_program_release_schedule(&0, &1000, &recipient);
        }))
    });
    assert!(result_zero.is_err());

    // Negative amount schedule must panic
    let result_neg = env.as_contract(&client.address, || {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            client.create_program_release_schedule(&-100, &1000, &recipient);
        }))
    });
    assert!(result_neg.is_err());
}

// 2. Invariant: get_due_schedules exactly equals the set of schedules with release_timestamp <= now and !released
fn run_due_schedules_test(ops: std::vec::Vec<ScheduleOp>) -> Result<(), TestCaseError> {
    let env = Env::default();
    let (client, _admin, _cid, _token) = setup_program(&env, 1_000_000);
    let base_time: u64 = 1000;
    
    let mut expected_due_ids = std::vec::Vec::new();
    let mut created_schedules = std::vec::Vec::new();

    for op in ops.iter() {
        let release_timestamp = if op.release_offset >= 0 {
            base_time + (op.release_offset as u64)
        } else {
            base_time - (op.release_offset.unsigned_abs())
        };

        let recipient = Address::generate(&env);
        let schedule = client.create_program_release_schedule(&op.amount, &release_timestamp, &recipient);
        created_schedules.push(schedule.clone());

        if release_timestamp <= base_time {
            expected_due_ids.push(schedule.schedule_id);
        }
    }

    env.ledger().with_mut(|li| li.timestamp = base_time);

    let due_schedules = client.get_due_schedules();
    let mut actual_due_ids = std::vec::Vec::new();
    for ds in due_schedules.iter() {
        assert!(!ds.released);
        assert!(ds.release_timestamp <= base_time);
        actual_due_ids.push(ds.schedule_id);
    }

    expected_due_ids.sort();
    actual_due_ids.sort();
    prop_assert_eq!(expected_due_ids, actual_due_ids);

    Ok(())
}

#[test]
fn proptest_due_schedules_matching_timestamp_selection() {
    let mut runner = deterministic_runner();
    let strategy = proptest::collection::vec(schedule_op_strategy(), 1..=40);
    runner
        .run(&strategy, |ops| run_due_schedules_test(ops))
        .expect("due schedules invariants must hold");
}

// 3. Invariant: trigger_program_releases is idempotent (re-triggering releases nothing new)
fn run_trigger_idempotency_test(ops: std::vec::Vec<ScheduleOp>) -> Result<(), TestCaseError> {
    let env = Env::default();
    let mut total_amount = 0i128;
    for op in ops.iter() {
        total_amount += op.amount;
    }

    let (client, _admin, cid, token_client) = setup_program(&env, total_amount);
    let base_time: u64 = 1000;

    for op in ops.iter() {
        // Schedule all to be due at base_time (offset <= 0)
        let release_timestamp = base_time - (op.release_offset.abs() as u64 % 500);
        let recipient = Address::generate(&env);
        client.create_program_release_schedule(&op.amount, &release_timestamp, &recipient);
    }

    env.ledger().with_mut(|li| li.timestamp = base_time);

    // First trigger
    let released_count_first = client.trigger_program_releases();
    let balance_after_first = client.get_remaining_balance();

    // Second trigger (idempotency check)
    let released_count_second = client.trigger_program_releases();
    let balance_after_second = client.get_remaining_balance();

    prop_assert_eq!(released_count_second, 0);
    prop_assert_eq!(balance_after_first, balance_after_second);
    prop_assert_eq!(token_client.balance(&cid), balance_after_second);

    Ok(())
}

#[test]
fn proptest_trigger_program_releases_idempotency() {
    let mut runner = deterministic_runner();
    let strategy = proptest::collection::vec(schedule_op_strategy(), 1..=40);
    runner
        .run(&strategy, |ops| run_trigger_idempotency_test(ops))
        .expect("trigger releases idempotency invariant must hold");
}

// 4. Invariant: total scheduled never exceeds remaining program balance + token balance conservation
fn run_conservation_of_funds_test(ops: std::vec::Vec<ScheduleOp>) -> Result<(), TestCaseError> {
    let env = Env::default();
    let mut total_amount = 0i128;
    for op in ops.iter() {
        total_amount += op.amount;
    }

    // Allocate slightly more than total scheduled amount
    let initial_balance = total_amount + 5000i128;
    let (client, _admin, cid, token_client) = setup_program(&env, initial_balance);
    let base_time: u64 = 1000;

    for op in ops.iter() {
        let release_timestamp = base_time - (op.release_offset.abs() as u64 % 500);
        let recipient = Address::generate(&env);
        client.create_program_release_schedule(&op.amount, &release_timestamp, &recipient);
    }

    env.ledger().with_mut(|li| li.timestamp = base_time);

    // Trigger releases
    let released_count = client.trigger_program_releases();
    let remaining_balance = client.get_remaining_balance();

    prop_assert!(released_count > 0);
    // Conservation: remaining balance + total scheduled amount must equal initial balance
    prop_assert_eq!(remaining_balance + total_amount, initial_balance);
    // Token balance matches remaining balance
    prop_assert_eq!(token_client.balance(&cid), remaining_balance);

    Ok(())
}

#[test]
fn proptest_schedules_conservation_of_funds() {
    let mut runner = deterministic_runner();
    let strategy = proptest::collection::vec(schedule_op_strategy(), 1..=40);
    runner
        .run(&strategy, |ops| run_conservation_of_funds_test(ops))
        .expect("schedules conservation properties must hold");
}
