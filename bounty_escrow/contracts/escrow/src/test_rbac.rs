#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, vec, Address, Env, String,
};

struct RbacSetup<'a> {
    env: Env,
    admin: Address,
    anti_abuse_admin: Address,
    depositor: Address,
    recipient: Address,
    random: Address,
    client: BountyEscrowContractClient<'a>,
    token_id: Address,
}

impl<'a> RbacSetup<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths(); // Enable for setup only
        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let anti_abuse_admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let recipient = Address::generate(&env);
        let random = Address::generate(&env);

        let token_admin = Address::generate(&env);
        let token_id = env
            .register_stellar_asset_contract_v2(token_admin.clone())
            .address();

        // Initialize contract
        client.init(&admin, &token_id);

        // Initialize anti-abuse admin via contract admin
        client.set_anti_abuse_admin(&anti_abuse_admin);

        Self {
            env,
            admin,
            anti_abuse_admin,
            depositor,
            recipient,
            random,
            client,
            token_id,
        }
    }
}

// ─────────────────────────────────────────────────────────
// Contract Admin Tests
// ─────────────────────────────────────────────────────────

#[test]
fn test_admin_contract_permissions() {
    let setup = RbacSetup::new();
    // mock_all_auths is already active from setup

    // Admin should be able to pause
    setup.client.set_paused(&Some(true), &None, &None);
    assert!(setup.client.get_pause_flags().lock_paused);

    // Admin should be able to update fee config
    setup.client.update_fee_config(
        &Some(100),
        &Some(100),
        &Some(setup.admin.clone()),
        &Some(true),
    );
}

#[test]
fn test_random_cannot_pause() {
    // init with all auth mocked (setup only)
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();

    env.mock_all_auths();
    client.init(&admin, &token_id);

    // Clear all mocked auth. `mock_auths(&[])` DOES override
    // `mock_all_auths`, so a subsequent admin call with no authorized caller
    // is rejected (require_auth aborts -> try_ returns Err).
    env.mock_auths(&[]);
    let res = client.try_set_paused(&Some(true), &None, &None);
    assert!(
        res.is_err(),
        "random caller must be rejected by set_paused authorization"
    );
}

// ─────────────────────────────────────────────────────────
// Anti-Abuse Admin Tests
// ─────────────────────────────────────────────────────────

#[test]
fn test_anti_abuse_admin_can_be_set_by_admin() {
    let setup = RbacSetup::new();

    let new_anti_abuse_admin = Address::generate(&setup.env);
    setup.client.set_anti_abuse_admin(&new_anti_abuse_admin);
    assert_eq!(
        setup.client.get_anti_abuse_admin(),
        Some(new_anti_abuse_admin)
    );
}

#[test]
fn test_admin_can_set_whitelist() {
    let setup = RbacSetup::new();

    // Contract Admin can set whitelist in our implementation
    setup.client.set_whitelist(&setup.random, &true);
}

// ─────────────────────────────────────────────────────────
// Operative Permissions (Depositor/Recipient)
// ─────────────────────────────────────────────────────────

#[test]
fn test_depositor_permissions() {
    let setup = RbacSetup::new();

    // Depositor should be able to lock funds
    let bounty_id = 1u64;
    let amount = 1000i128;
    let deadline = setup.env.ledger().timestamp() + 3600;

    // Setup token balance
    let sac_client = token::StellarAssetClient::new(&setup.env, &setup.token_id);
    sac_client.mint(&setup.depositor, &amount);

    // Signatures: lock_funds(depositor, bounty_id, amount, deadline)
    setup
        .client
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
}

#[test]
fn test_random_cannot_lock_funds_for_depositor() {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();

    // init + mint with all auth mocked (setup only)
    env.mock_all_auths();
    client.init(&admin, &token_id);
    let sac_client = token::StellarAssetClient::new(&env, &token_id);
    sac_client.mint(&depositor, &1000i128);

    // Clear mocked auth so the depositor auth is no longer satisfied.
    env.mock_auths(&[]);
    let res = client.try_lock_funds(&depositor, &1u64, &1000i128, &3600u64);
    assert!(
        res.is_err(),
        "an unauthenticated caller must be rejected by lock_funds"
    );
}
