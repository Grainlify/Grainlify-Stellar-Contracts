#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

fn create_token_contract<'a>(
    e: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract_address = e
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    (
        token::Client::new(e, &contract_address),
        token::StellarAssetClient::new(e, &contract_address),
    )
}

fn create_escrow_contract<'a>(e: &Env) -> (BountyEscrowContractClient<'a>, Address) {
    let contract_id = e.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(e, &contract_id);
    (client, contract_id)
}

#[test]
fn test_granular_pause_lock() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let (token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    let (escrow_client, _escrow_address) = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);

    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.lock_paused, false);
    assert_eq!(flags.release_paused, false);
    assert_eq!(flags.refund_paused, false);

    token_admin_client.mint(&depositor, &1000);

    let bounty_id_1: u64 = 1;
    let deadline = env.ledger().timestamp() + 1000;
    escrow_client.lock_funds(&depositor, &bounty_id_1, &100, &deadline);

    escrow_client.set_paused(&Some(true), &None, &None);
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.lock_paused, true);

    let bounty_id_2: u64 = 2;
    let res = escrow_client.try_lock_funds(&depositor, &bounty_id_2, &100, &deadline);
    assert!(res.is_err());

    escrow_client.set_paused(&Some(false), &None, &None);
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.lock_paused, false);

    escrow_client.lock_funds(&depositor, &bounty_id_2, &100, &deadline);
}

#[test]
fn test_granular_pause_release() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let (token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    let (escrow_client, _) = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);
    token_admin_client.mint(&depositor, &1000);

    let bounty_id: u64 = 1;
    let deadline = env.ledger().timestamp() + 1000;
    escrow_client.lock_funds(&depositor, &bounty_id, &100, &deadline);

    escrow_client.set_paused(&None, &Some(true), &None);
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.release_paused, true);

    let res = escrow_client.try_release_funds(&bounty_id, &contributor);
    assert!(res.is_err());

    escrow_client.set_paused(&None, &Some(false), &None);
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.release_paused, false);

    escrow_client.release_funds(&bounty_id, &contributor);
}

#[test]
fn test_granular_pause_refund() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let (token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    let (escrow_client, _) = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);
    token_admin_client.mint(&depositor, &1000);

    let bounty_id: u64 = 1;
    let deadline = env.ledger().timestamp() + 1000;

    escrow_client.lock_funds(&depositor, &bounty_id, &100, &deadline);

    env.ledger().set_timestamp(deadline + 1);

    escrow_client.set_paused(&None, &None, &Some(true));
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.refund_paused, true);

    let res = escrow_client.try_refund(&bounty_id);
    assert!(res.is_err());

    escrow_client.set_paused(&None, &None, &Some(false));
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.refund_paused, false);

    escrow_client.refund(&bounty_id);
}

#[test]
fn test_mixed_pause_states() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let (token_client, _) = create_token_contract(&env, &admin);
    let (escrow_client, _) = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);

    escrow_client.set_paused(&Some(true), &Some(true), &Some(false));
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.lock_paused, true);
    assert_eq!(flags.release_paused, true);
    assert_eq!(flags.refund_paused, false);

    escrow_client.set_paused(&None, &Some(false), &None);
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.lock_paused, true);
    assert_eq!(flags.release_paused, false);
    assert_eq!(flags.refund_paused, false);
}

#[test]
fn test_pause_timing_single_call_immediacy() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let (token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    let (escrow_client, _) = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);
    token_admin_client.mint(&depositor, &1000);

    let deadline = env.ledger().timestamp() + 1000;

    // Successful state-changing call
    escrow_client.lock_funds(&depositor, &1, &100, &deadline);

    // Pause the contract immediately
    escrow_client.set_paused(&Some(true), &None, &None);

    // Confirm every subsequent call in the same test run is correctly rejected while paused
    let res = escrow_client.try_lock_funds(&depositor, &2, &100, &deadline);
    assert!(res.is_err());
}

#[test]
fn test_unpause_restores_exact_state() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let (token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    let (escrow_client, _) = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);
    token_admin_client.mint(&depositor, &1000);

    let deadline = env.ledger().timestamp() + 1000;

    // Pause contract
    escrow_client.set_paused(&Some(true), &None, &None);
    let res = escrow_client.try_lock_funds(&depositor, &1, &100, &deadline);
    assert!(res.is_err());

    // Unpause contract
    escrow_client.set_paused(&Some(false), &None, &None);

    // Verify unpausing correctly restores exactly the pre-pause behavior with no residual state
    escrow_client.lock_funds(&depositor, &1, &100, &deadline);
    
    // Check flags are completely clean
    let flags = escrow_client.get_pause_flags();
    assert_eq!(flags.lock_paused, false);
    assert_eq!(flags.release_paused, false);
    assert_eq!(flags.refund_paused, false);
    assert_eq!(flags.global_paused, false);
}

#[test]
fn test_batch_pause_behavior() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let (token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    let (escrow_client, _) = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);
    token_admin_client.mint(&depositor, &1000);

    let deadline = env.ledger().timestamp() + 1000;

    // 1. Test batch lock funds pause
    escrow_client.set_paused(&Some(true), &None, &None);
    let items_lock = vec![
        &env,
        LockFundsItem {
            bounty_id: 1,
            depositor: depositor.clone(),
            amount: 100,
            deadline,
        },
        LockFundsItem {
            bounty_id: 2,
            depositor: depositor.clone(),
            amount: 100,
            deadline,
        },
    ];
    let res_lock = escrow_client.try_batch_lock_funds(&items_lock);
    assert!(res_lock.is_err());

    // Restore lock functionality
    escrow_client.set_paused(&Some(false), &None, &None);
    escrow_client.batch_lock_funds(&items_lock);

    // 2. Test batch release funds pause
    escrow_client.set_paused(&None, &Some(true), &None);
    let items_release = vec![
        &env,
        ReleaseFundsItem {
            bounty_id: 1,
            contributor: contributor.clone(),
        },
        ReleaseFundsItem {
            bounty_id: 2,
            contributor: contributor.clone(),
        },
    ];
    let res_release = escrow_client.try_batch_release_funds(&items_release);
    assert!(res_release.is_err());

    // Restore release functionality
    escrow_client.set_paused(&None, &Some(false), &None);
    escrow_client.batch_release_funds(&items_release);

    // 3. Test sweep expired refunds pause
    let items_lock_expire = vec![
        &env,
        LockFundsItem {
            bounty_id: 3,
            depositor: depositor.clone(),
            amount: 100,
            deadline,
        },
    ];
    escrow_client.batch_lock_funds(&items_lock_expire);

    env.ledger().set_timestamp(deadline + 1);

    escrow_client.set_paused(&None, &None, &Some(true));
    let items_sweep = vec![&env, 3];
    let res_sweep = escrow_client.try_sweep_expired_refunds(&items_sweep);
    assert!(res_sweep.is_err());

    // Restore refund functionality
    escrow_client.set_paused(&None, &None, &Some(false));
    escrow_client.sweep_expired_refunds(&items_sweep);
}

