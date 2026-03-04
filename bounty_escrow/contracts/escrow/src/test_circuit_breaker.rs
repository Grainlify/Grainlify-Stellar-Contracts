#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

fn create_token(
    env: &Env,
    admin: &Address,
) -> (token::Client<'static>, token::StellarAssetClient<'static>) {
    let addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    (
        token::Client::new(env, &addr),
        token::StellarAssetClient::new(env, &addr),
    )
}

fn setup(env: &Env, depositor_balance: i128) -> (BountyEscrowContractClient<'static>, Address, Address) {
    env.mock_all_auths();

    let admin = Address::generate(env);
    let token_admin = Address::generate(env);
    let depositor = Address::generate(env);

    let (token_client, token_sac) = create_token(env, &token_admin);

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(env, &contract_id);

    client.init(&admin, &token_client.address);
    token_sac.mint(&depositor, &depositor_balance);

    (client, admin, depositor)
}

fn lock_one(
    client: &BountyEscrowContractClient<'static>,
    env: &Env,
    depositor: &Address,
    bounty_id: u64,
    amount: i128,
) {
    let deadline = env.ledger().timestamp() + 10_000;
    client.lock_funds(depositor, &bounty_id, &amount, &deadline);
}

#[test]
fn test_circuit_default_state_closed() {
    let env = Env::default();
    let (client, _admin, _depositor) = setup(&env, 0);

    let status = client.get_circuit_status();
    assert_eq!(status.state, error_recovery::CircuitState::Closed);
    assert_eq!(status.failure_count, 0);
}

#[test]
fn test_emergency_open_blocks_lock_release_refund_claim_but_allows_views() {
    let env = Env::default();
    let (client, admin, depositor) = setup(&env, 10_000);

    lock_one(&client, &env, &depositor, 1, 1_000);

    client.emergency_open_circuit(&admin);

    let deadline = env.ledger().timestamp() + 10_000;
    assert!(client
        .try_lock_funds(&depositor, &2, &100, &deadline)
        .is_err());

    let contributor = Address::generate(&env);
    assert!(client.try_release_funds(&1, &contributor).is_err());

    env.ledger().set_timestamp(deadline + 1);
    assert!(client.try_refund(&1).is_err());

    // authorize_claim and claim should also be blocked
    assert!(client.try_authorize_claim(&1, &contributor).is_err());
    assert!(client.try_claim(&1).is_err());

    // read-only should still work
    assert_eq!(client.get_balance(), 1_000);
    let _info = client.get_escrow_info(&1);
}

#[test]
fn test_reset_transitions_open_to_half_open_then_half_open_to_closed() {
    let env = Env::default();
    let (client, admin, depositor) = setup(&env, 10_000);

    lock_one(&client, &env, &depositor, 1, 500);
    client.emergency_open_circuit(&admin);
    assert_eq!(client.get_circuit_status().state, error_recovery::CircuitState::Open);

    client.reset_circuit_breaker(&admin);
    assert_eq!(
        client.get_circuit_status().state,
        error_recovery::CircuitState::HalfOpen
    );

    client.reset_circuit_breaker(&admin);
    assert_eq!(
        client.get_circuit_status().state,
        error_recovery::CircuitState::Closed
    );
}

#[test]
fn test_half_open_closes_after_success_threshold() {
    let env = Env::default();
    let (client, admin, depositor) = setup(&env, 10_000);

    client.configure_circuit_breaker(&admin, &3u32, &1u32, &5u32);

    client.emergency_open_circuit(&admin);
    client.reset_circuit_breaker(&admin);
    assert_eq!(
        client.get_circuit_status().state,
        error_recovery::CircuitState::HalfOpen
    );

    // A successful protected op in half-open should close circuit when success_threshold=1
    let deadline = env.ledger().timestamp() + 10_000;
    client.lock_funds(&depositor, &1, &100, &deadline);

    assert_eq!(
        client.get_circuit_status().state,
        error_recovery::CircuitState::Closed
    );
}

#[test]
fn test_unauthorized_circuit_admin_cannot_open_or_reset_or_configure() {
    let env = Env::default();
    let (client, _admin, _depositor) = setup(&env, 0);

    let attacker = Address::generate(&env);

    assert!(client.try_emergency_open_circuit(&attacker).is_err());
    assert!(client.try_reset_circuit_breaker(&attacker).is_err());
    assert!(
        client
            .try_configure_circuit_breaker(&attacker, &3u32, &1u32, &5u32)
            .is_err()
    );
}
