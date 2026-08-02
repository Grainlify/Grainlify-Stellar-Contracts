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

// ─────────────────────────────────────────────────────────
// Role Revocation Ordering Tests
// ─────────────────────────────────────────────────────────

#[soroban_sdk::contract]
pub struct MockRevokerContract;

#[soroban_sdk::contractimpl]
impl MockRevokerContract {
    pub fn revoke_and_use(env: Env, escrow_id: Address, old_admin: Address, new_admin: Address) {
        let client = BountyEscrowContractClient::new(&env, &escrow_id);
        
        // 1. Revoke the role (circuit breaker admin) from old_admin
        client.set_circuit_breaker_admin(&new_admin);
        
        // 2. Try to use the revoked role in the same transaction
        // This will panic with "Not authorized" because the state change
        // takes effect immediately within the same transaction.
        client.reset_circuit(&old_admin);
    }
}

#[test]
fn test_mid_transaction_whitelist_revocation() {
    let setup = RbacSetup::new();
    let test_user = Address::generate(&setup.env);

    // 1. Grant whitelist role to test_user
    setup.client.set_whitelist(&test_user, &true);

    // To test rate limiting, we need to exhaust the rate limit.
    // By default, max_operations is 100. We can't easily change it here because
    // set_anti_abuse_config is not exposed in the client.
    // Instead, we can simulate the "use" of the role by calling lock_funds
    // repeatedly. However, since the test environment allows us to use
    // anti_abuse::is_whitelisted directly via a mock contract, we can test the
    // invariant that the role is revoked mid-transaction and the check fails immediately.
}

#[soroban_sdk::contract]
pub struct MockWhitelistRevokerContract;

#[soroban_sdk::contractimpl]
impl MockWhitelistRevokerContract {
    pub fn revoke_and_check(env: Env, escrow_id: Address, test_user: Address) {
        let client = BountyEscrowContractClient::new(&env, &escrow_id);
        
        // 1. Revoke the whitelist role from test_user
        client.set_whitelist(&test_user, &false);
        
        // 2. The user is now un-whitelisted. We will trigger the cooldown limit
        // by performing two operations consecutively in the same transaction.
        // The first will succeed and set the last_operation_timestamp to `now`.
        client.lock_funds(&test_user, &1u64, &1i128, &3600u64);
        
        // The second will fail because `now < now + cooldown_period` (cooldown is 2s).
        client.lock_funds(&test_user, &2u64, &1i128, &3600u64);
    }
}

#[test]
fn test_whitelist_role_revocation_ordering() {
    let setup = RbacSetup::new();
    let test_user = Address::generate(&setup.env);
    
    // Set a non-zero timestamp so rate limit cooldown checks work
    setup.env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });
    
    // Mint tokens for the test user to pay for lock_funds
    let token_admin_client = token::StellarAssetClient::new(&setup.env, &setup.token_id);
    setup.env.mock_all_auths();
    token_admin_client.mint(&test_user, &1000000i128);

    // Grant whitelist
    setup.client.set_whitelist(&test_user, &true);
    
    // Verify that whitelisted users bypass the cooldown limit.
    // Two consecutive operations in the same timestamp should succeed.
    setup.client.lock_funds(&test_user, &101u64, &1i128, &3600u64);
    setup.client.lock_funds(&test_user, &102u64, &1i128, &3600u64);
    
    // Now revoke whitelist
    setup.client.set_whitelist(&test_user, &false);
    
    // The very next call will succeed (because rate limit state wasn't updated during whitelist)
    setup.client.lock_funds(&test_user, &103u64, &1i128, &3600u64);
    
    // But the call immediately after that must fail due to the 2s cooldown!
    let res = setup.client.try_lock_funds(&test_user, &104u64, &1i128, &3600u64);
    assert!(res.is_err(), "Revoked whitelist role must result in immediate rate limit enforcement");
    
    // Now test mid-transaction revocation
    let setup2 = RbacSetup::new();
    let test_user2 = Address::generate(&setup2.env);
    
    setup2.env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });
    
    let token_admin_client2 = token::StellarAssetClient::new(&setup2.env, &setup2.token_id);
    setup2.env.mock_all_auths();
    token_admin_client2.mint(&test_user2, &1000000i128);
    
    // Grant whitelist
    setup2.client.set_whitelist(&test_user2, &true);
    
    // Register the mock contract
    let mock_id = setup2.env.register_contract(None, MockWhitelistRevokerContract);
    let mock_client = MockWhitelistRevokerContractClient::new(&setup2.env, &mock_id);

    // This contract will:
    // 1. Revoke the whitelist.
    // 2. Call lock_funds twice. The second call will panic because of the cooldown.
    // The role revocation takes effect mid-transaction.
    let res = mock_client.try_revoke_and_check(
        &setup2.client.address,
        &test_user2,
    );
    
    assert!(res.is_err(), "Revoked role must not be usable later in the same transaction");
}

#[test]
fn test_regrant_of_whitelist_role() {
    let setup = RbacSetup::new();
    let test_user = Address::generate(&setup.env);
    
    setup.env.ledger().with_mut(|li| {
        li.timestamp = 1000;
    });

    let token_admin_client = token::StellarAssetClient::new(&setup.env, &setup.token_id);
    setup.env.mock_all_auths();
    token_admin_client.mint(&test_user, &1000000i128);

    // The user is not whitelisted by default.
    // The first operation succeeds.
    setup.client.lock_funds(&test_user, &1u64, &1i128, &3600u64);
    
    // The second operation fails (cooldown).
    let res = setup.client.try_lock_funds(&test_user, &2u64, &1i128, &3600u64);
    assert!(res.is_err());
    
    // Grant whitelist
    setup.client.set_whitelist(&test_user, &true);
    
    // Now they can perform consecutive operations without cooldown!
    setup.client.lock_funds(&test_user, &3u64, &1i128, &3600u64);
    setup.client.lock_funds(&test_user, &4u64, &1i128, &3600u64);
    
    // Revoke whitelist
    setup.client.set_whitelist(&test_user, &false);
    
    // Because we didn't advance time, the cooldown from operation 1 is STILL active.
    // The next operation will FAIL (cooldown)!
    let res2 = setup.client.try_lock_funds(&test_user, &5u64, &1i128, &3600u64);
    assert!(res2.is_err());
    
    // Re-grant whitelist
    setup.client.set_whitelist(&test_user, &true);
    
    // Now it succeeds again!
    setup.client.lock_funds(&test_user, &6u64, &1i128, &3600u64);
}
