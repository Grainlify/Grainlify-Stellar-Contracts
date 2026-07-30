#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, String, Vec,
};

fn setup_program(
    env: &Env,
    initial_funds: i128,
) -> (
    ProgramEscrowContractClient<'static>,
    Address,
    Address,
    token::Client<'static>,
) {
    env.mock_all_auths();
    env.budget().reset_unlimited();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(env, &contract_id);

    let admin = Address::generate(env);
    let token_admin = Address::generate(env);
    let token_id = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::Client::new(env, &token_id);
    let token_admin_client = token::StellarAssetClient::new(env, &token_id);

    client.init_program(
        &String::from_str(env, "schedule-pagination"),
        &admin,
        &token_id,
    );

    if initial_funds > 0 {
        token_admin_client.mint(&admin, &initial_funds);
        client.lock_program_funds(&admin, &initial_funds);
    }

    (client, contract_id, admin, token_client)
}

fn schedule(
    recipient: Address,
    schedule_id: u64,
    release_timestamp: u64,
    released: bool,
) -> ProgramReleaseSchedule {
    ProgramReleaseSchedule {
        schedule_id,
        recipient,
        amount: 1,
        release_timestamp,
        released,
        released_at: if released {
            Some(release_timestamp)
        } else {
            None
        },
        released_by: None,
    }
}

fn store_schedules(env: &Env, contract_id: &Address, schedules: &Vec<ProgramReleaseSchedule>) {
    env.as_contract(contract_id, || {
        env.storage().persistent().set(&SCHEDULES, schedules);
    });
}

#[test]
fn test_schedule_pagination_raw_pages_limits_and_wrapper() {
    let env = Env::default();
    let (client, contract_id, _admin, _token) = setup_program(&env, 0);

    let mut schedules = Vec::new(&env);
    for id in 1..=105u64 {
        schedules.push_back(schedule(Address::generate(&env), id, 10_000, false));
    }
    store_schedules(&env, &contract_id, &schedules);

    let first = client.get_program_release_schedules(&0, &2);
    assert_eq!(first.len(), 2);
    assert_eq!(first.get(0).unwrap().schedule_id, 1);
    assert_eq!(first.get(1).unwrap().schedule_id, 2);

    let second = client.get_program_release_schedules(&2, &2);
    assert_eq!(second.len(), 2);
    assert_eq!(second.get(0).unwrap().schedule_id, 3);
    assert_eq!(second.get(1).unwrap().schedule_id, 4);

    assert_eq!(client.get_program_release_schedules(&500, &10).len(), 0);
    assert_eq!(client.get_program_release_schedules(&0, &0).len(), 0);

    let capped = client.get_program_release_schedules(&0, &u32::MAX);
    assert_eq!(capped.len(), MAX_QUERY_LIMIT);

    let wrapped = client.get_all_prog_release_schedules(&0, &u32::MAX);
    assert_eq!(wrapped, capped);
}

#[test]
fn test_schedule_pagination_filtered_offsets_count_matches() {
    let env = Env::default();
    env.ledger().set_timestamp(1_000);

    let (client, contract_id, _admin, _token) = setup_program(&env, 0);

    let mut schedules = Vec::new(&env);
    schedules.push_back(schedule(Address::generate(&env), 1, 2_000, false));
    schedules.push_back(schedule(Address::generate(&env), 2, 900, true));
    schedules.push_back(schedule(Address::generate(&env), 3, 900, false));
    schedules.push_back(schedule(Address::generate(&env), 4, 2_000, true));
    schedules.push_back(schedule(Address::generate(&env), 5, 2_000, false));
    schedules.push_back(schedule(Address::generate(&env), 6, 950, false));
    schedules.push_back(schedule(Address::generate(&env), 7, 1_000, false));
    schedules.push_back(schedule(Address::generate(&env), 8, 800, true));
    store_schedules(&env, &contract_id, &schedules);

    // Matching pending IDs are 1, 3, 5, 6, 7.
    let pending = client.get_pending_schedules(&1, &2);
    assert_eq!(pending.len(), 2);
    assert_eq!(pending.get(0).unwrap().schedule_id, 3);
    assert_eq!(pending.get(1).unwrap().schedule_id, 5);

    // Matching due IDs are 3, 6, 7.
    let due = client.get_due_schedules(&1, &1);
    assert_eq!(due.len(), 1);
    assert_eq!(due.get(0).unwrap().schedule_id, 6);

    assert_eq!(client.get_pending_program_schedules(&1, &2), pending);
    assert_eq!(client.get_due_program_schedules(&1, &1), due);

    assert_eq!(client.get_pending_schedules(&0, &0).len(), 0);
    assert_eq!(client.get_due_schedules(&100, &10).len(), 0);
}

#[test]
fn test_schedule_pagination_internal_release_reaches_beyond_query_cap() {
    let env = Env::default();
    let (client, contract_id, _admin, token_client) =
        setup_program(&env, (MAX_QUERY_LIMIT + 1) as i128);

    let mut schedules = Vec::new(&env);
    let mut final_recipient = Address::generate(&env);

    for id in 1..=(MAX_QUERY_LIMIT + 1) {
        let recipient = Address::generate(&env);
        if id == MAX_QUERY_LIMIT + 1 {
            final_recipient = recipient.clone();
        }

        schedules.push_back(schedule(recipient, id as u64, 10_000, false));
    }

    store_schedules(&env, &contract_id, &schedules);

    let public_page = client.get_program_release_schedules(&0, &u32::MAX);
    assert_eq!(public_page.len(), MAX_QUERY_LIMIT);
    assert!(public_page
        .iter()
        .all(|item| item.schedule_id <= MAX_QUERY_LIMIT as u64));

    // Internal lookup and mutation must still operate on the complete vector.
    let beyond_cap = client.get_program_release_schedule(&((MAX_QUERY_LIMIT + 1) as u64));
    assert!(!beyond_cap.released);

    client.release_program_schedule_manual(&((MAX_QUERY_LIMIT + 1) as u64));

    let released = client.get_program_release_schedule(&((MAX_QUERY_LIMIT + 1) as u64));
    assert!(released.released);
    assert_eq!(token_client.balance(&final_recipient), 1);
}
