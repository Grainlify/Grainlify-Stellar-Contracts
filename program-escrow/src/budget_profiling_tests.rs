#![cfg(test)]

extern crate std;

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token, vec, Address, Env, Map, String, Symbol, TryFromVal, Val, Vec,
};
use std::println;

const PAYOUT_AMOUNT: i128 = 1_000;
const INITIAL_FUNDS: i128 = 10_000_000;
const SINGLE_PAYOUT_CPU_CEILING: u64 = 1_000_000;
const SINGLE_PAYOUT_MEM_CEILING: u64 = 200_000;
const TRIGGER_RELEASES_CPU_CEILING: u64 = 3_500_000;
const TRIGGER_RELEASES_MEM_CEILING: u64 = 700_000;
const BATCH_BASE_CPU_CEILING: u64 = 1_000_000;
const BATCH_PER_RECIPIENT_CPU_CEILING: u64 = 250_000;
const BATCH_BASE_MEM_CEILING: u64 = 500_000;
const BATCH_PER_RECIPIENT_MEM_CEILING: u64 = 45_000;

#[derive(Clone, Copy, Debug)]
struct BudgetSample {
    cpu: u64,
    mem: u64,
}

fn setup_program(env: &Env, initial_amount: i128) -> ProgramEscrowContractClient<'static> {
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);

    let admin = Address::generate(env);
    let token_admin = Address::generate(env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = token_contract.address();
    let token_admin_client = token::StellarAssetClient::new(env, &token_id);
    let program_id = String::from_str(env, "budget-profile");

    client.init_program(&program_id, &admin, &token_id);
    token_admin_client.mint(&admin, &initial_amount);
    client.lock_program_funds(&admin, &initial_amount);

    client
}

fn reset_budget(env: &Env) {
    let mut budget = env.budget();
    budget.reset_default();
}

fn budget_sample(env: &Env) -> BudgetSample {
    let budget = env.budget();
    BudgetSample {
        cpu: budget.cpu_instruction_cost(),
        mem: budget.memory_bytes_cost(),
    }
}

fn batch_vectors(env: &Env, batch_size: u32) -> (Vec<Address>, Vec<i128>) {
    let mut recipients = vec![env];
    let mut amounts = vec![env];

    for _ in 0..batch_size {
        recipients.push_back(Address::generate(env));
        amounts.push_back(PAYOUT_AMOUNT);
    }

    (recipients, amounts)
}

fn measure_batch_payout(batch_size: u32) -> BudgetSample {
    let env = Env::default();
    let client = setup_program(&env, INITIAL_FUNDS);
    let (recipients, amounts) = batch_vectors(&env, batch_size);

    reset_budget(&env);
    client.batch_payout(&recipients, &amounts);
    budget_sample(&env)
}

fn get_u32_event_field(env: &Env, data: &Val, field: &str) -> Option<u32> {
    let data_map: Map<Symbol, Val> = Map::try_from_val(env, data).ok()?;
    let field_val = data_map.get(Symbol::new(env, field))?;
    u32::try_from_val(env, &field_val).ok()
}

fn latest_batch_gas_proxy_metrics(
    env: &Env,
    client: &ProgramEscrowContractClient<'_>,
) -> Option<(u32, u32, u32, u32, u32)> {
    let events = env.events().all();

    for (contract, _topics, data) in events.iter().rev() {
        if contract != client.address {
            continue;
        }

        let transfer_ops = get_u32_event_field(env, &data, "gas_proxy_transfer_ops");
        let history_appends = get_u32_event_field(env, &data, "gas_proxy_history_appends");
        let storage_reads = get_u32_event_field(env, &data, "gas_proxy_storage_reads");
        let storage_writes = get_u32_event_field(env, &data, "gas_proxy_storage_writes");
        let events_emitted = get_u32_event_field(env, &data, "gas_proxy_events_emitted");

        if let (
            Some(transfer_ops),
            Some(history_appends),
            Some(storage_reads),
            Some(storage_writes),
            Some(events_emitted),
        ) = (
            transfer_ops,
            history_appends,
            storage_reads,
            storage_writes,
            events_emitted,
        ) {
            return Some((
                transfer_ops,
                history_appends,
                storage_reads,
                storage_writes,
                events_emitted,
            ));
        }
    }

    None
}

#[test]
fn budget_profiling_batch_payout_scales_linearly_to_max_batch_size() {
    let samples = [
        (1_u32, measure_batch_payout(1)),
        (10_u32, measure_batch_payout(10)),
        (50_u32, measure_batch_payout(50)),
        (MAX_BATCH_SIZE, measure_batch_payout(MAX_BATCH_SIZE)),
    ];

    for (batch_size, sample) in samples {
        let cpu_ceiling =
            BATCH_BASE_CPU_CEILING + (batch_size as u64 * BATCH_PER_RECIPIENT_CPU_CEILING);
        let mem_ceiling =
            BATCH_BASE_MEM_CEILING + (batch_size as u64 * BATCH_PER_RECIPIENT_MEM_CEILING);

        println!(
            "batch_payout size={batch_size} cpu={} mem={}",
            sample.cpu, sample.mem
        );
        assert!(sample.cpu > 0, "budget CPU should be measured");
        assert!(sample.mem > 0, "budget memory should be measured");
        assert!(
            sample.cpu <= cpu_ceiling,
            "batch size {batch_size} CPU {} exceeded ceiling {cpu_ceiling}",
            sample.cpu
        );
        assert!(
            sample.mem <= mem_ceiling,
            "batch size {batch_size} memory {} exceeded ceiling {mem_ceiling}",
            sample.mem
        );
    }
}

#[test]
fn budget_profiling_single_payout_and_trigger_releases_stay_under_regression_ceiling() {
    let env_single = Env::default();
    let single_client = setup_program(&env_single, INITIAL_FUNDS);
    let recipient = Address::generate(&env_single);

    reset_budget(&env_single);
    single_client.single_payout(&recipient, &PAYOUT_AMOUNT);
    let single = budget_sample(&env_single);
    println!("single_payout cpu={} mem={}", single.cpu, single.mem);

    assert!(single.cpu > 0);
    assert!(single.mem > 0);
    assert!(single.cpu <= SINGLE_PAYOUT_CPU_CEILING);
    assert!(single.mem <= SINGLE_PAYOUT_MEM_CEILING);

    let env_release = Env::default();
    let release_client = setup_program(&env_release, INITIAL_FUNDS);
    let release_at = env_release.ledger().timestamp().saturating_add(10);

    for _ in 0..10 {
        release_client.create_program_release_schedule(
            &PAYOUT_AMOUNT,
            &release_at,
            &Address::generate(&env_release),
        );
    }
    env_release.ledger().set_timestamp(release_at);

    reset_budget(&env_release);
    let released = release_client.trigger_program_releases();
    let trigger = budget_sample(&env_release);
    println!(
        "trigger_program_releases count={released} cpu={} mem={}",
        trigger.cpu, trigger.mem
    );

    assert_eq!(released, 10);
    assert!(trigger.cpu > 0);
    assert!(trigger.mem > 0);
    assert!(trigger.cpu <= TRIGGER_RELEASES_CPU_CEILING);
    assert!(trigger.mem <= TRIGGER_RELEASES_MEM_CEILING);
}

#[test]
fn budget_profiling_gas_proxy_fields_match_operation_counts() {
    let env = Env::default();
    let client = setup_program(&env, INITIAL_FUNDS);
    let batch_size = 7_u32;
    let (recipients, amounts) = batch_vectors(&env, batch_size);

    let data = client.batch_payout(&recipients, &amounts);
    let (transfer_ops, history_appends, storage_reads, storage_writes, events_emitted) =
        latest_batch_gas_proxy_metrics(&env, &client).expect("batch gas proxy metrics missing");

    assert_eq!(data.payout_history.len(), batch_size);
    assert_eq!(transfer_ops, batch_size);
    assert_eq!(history_appends, batch_size);
    assert_eq!(storage_reads, 1);
    assert_eq!(storage_writes, 1);
    assert_eq!(events_emitted, 1);
}

#[test]
#[should_panic(expected = "All amounts must be greater than zero")]
fn budget_profiling_zero_amount_batch_is_still_rejected() {
    let env = Env::default();
    let client = setup_program(&env, INITIAL_FUNDS);
    let recipients = vec![&env, Address::generate(&env)];
    let amounts = vec![&env, 0_i128];

    reset_budget(&env);
    client.batch_payout(&recipients, &amounts);
}
