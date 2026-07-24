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
fn test_admin_audit_view_defaults_after_init() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let (token_client, _) = create_token_contract(&env, &token_admin);
    let (escrow_client, _escrow_address) = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);

    let snapshot = escrow_client.get_admin_audit_view();

    assert_eq!(snapshot.version, 1);
    assert_eq!(snapshot.admin, admin);
    assert_eq!(snapshot.token, token_client.address);

    // By default fees are disabled and zeroed.
    assert_eq!(snapshot.fee_config.lock_fee_rate, 0);
    assert_eq!(snapshot.fee_config.release_fee_rate, 0);
    assert_eq!(snapshot.fee_config.fee_recipient, admin);
    assert_eq!(snapshot.fee_config.fee_enabled, false);

    // Pause flags are unpaused by default.
    assert_eq!(snapshot.pause_flags.lock_paused, false);
    assert_eq!(snapshot.pause_flags.release_paused, false);
    assert_eq!(snapshot.pause_flags.refund_paused, false);

    // Governance is not configured initially.
    assert!(snapshot.governance_contract.is_none());
    assert_eq!(snapshot.min_governance_version, 0);

    // No claim window or amount policy configured yet.
    assert_eq!(snapshot.claim_window, 0);
    assert_eq!(snapshot.has_amount_policy, false);
    assert_eq!(snapshot.min_lock_amount, 0);
    assert_eq!(snapshot.max_lock_amount, 0);
}

#[test]
fn test_admin_audit_view_tracks_config_changes() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let gov_contract = Address::generate(&env);

    let (token_client, _) = create_token_contract(&env, &token_admin);
    let (escrow_client, _escrow_address) = create_escrow_contract(&env);

    escrow_client.init(&admin, &token_client.address);

    // Configure fees
    escrow_client
        .update_fee_config(&Some(100), &Some(250), &Some(admin.clone()), &Some(true));

    // Configure pause flags
    escrow_client
        .set_paused(&Some(true), &Some(false), &Some(true));

    // Configure governance
    escrow_client
        .set_governance_contract(&gov_contract);
    escrow_client.set_min_governance_version(&2);

    // Configure claim window and amount policy
    escrow_client.set_claim_window(&3600);
    escrow_client
        .set_amount_policy(&admin, &10, &1_000_000);

    // Bump ledger time to make sure snapshots record realistic timestamps
    let now = env.ledger().timestamp();
    env.ledger().set_timestamp(now + 10);

    let snapshot = escrow_client.get_admin_audit_view();

    // Core identifiers stay consistent.
    assert_eq!(snapshot.admin, admin);
    assert_eq!(snapshot.token, token_client.address);

    // Fee configuration is reflected.
    assert_eq!(snapshot.fee_config.lock_fee_rate, 100);
    assert_eq!(snapshot.fee_config.release_fee_rate, 250);
    assert_eq!(snapshot.fee_config.fee_recipient, admin);
    assert_eq!(snapshot.fee_config.fee_enabled, true);

    // Pause flags reflect last update.
    assert_eq!(snapshot.pause_flags.lock_paused, true);
    assert_eq!(snapshot.pause_flags.release_paused, false);
    assert_eq!(snapshot.pause_flags.refund_paused, true);

    // Governance wiring is visible.
    assert_eq!(snapshot.governance_contract, Some(gov_contract));
    assert_eq!(snapshot.min_governance_version, 2);

    // Risk controls are surfaced for dashboards.
    assert_eq!(snapshot.claim_window, 3600);
    assert_eq!(snapshot.has_amount_policy, true);
    assert_eq!(snapshot.min_lock_amount, 10);
    assert_eq!(snapshot.max_lock_amount, 1_000_000);
}

#[test]
fn test_audit_views_across_upgrade_boundary() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let gov_contract = Address::generate(&env);

    let (token_client, _) = create_token_contract(&env, &token_admin);
    let (escrow_client, contract_id) = create_escrow_contract(&env);

    // Initialize contract
    escrow_client.init(&admin, &token_client.address);

    // 1. Seed realistic admin-audit state (several admin actions logged)
    escrow_client.update_fee_config(&Some(100), &Some(250), &Some(admin.clone()), &Some(true));
    escrow_client.set_paused(&Some(true), &Some(false), &Some(true));
    escrow_client.set_governance_contract(&gov_contract);
    escrow_client.set_min_governance_version(&2);
    escrow_client.set_claim_window(&3600);
    escrow_client.set_amount_policy(&admin, &10, &1_000_000);

    // Seed circuit breaker admin & log a failure
    escrow_client.set_circuit_breaker_admin(&admin);
    env.as_contract(&contract_id, || {
        crate::error_recovery::record_failure(&env, 10, soroban_sdk::symbol_short!("lock"), 100u32);
    });

    // Verify pre-upgrade state is correct
    let pre_snapshot = escrow_client.get_admin_audit_view();
    assert_eq!(pre_snapshot.version, 1);
    assert_eq!(pre_snapshot.admin, admin);
    assert_eq!(pre_snapshot.token, token_client.address);
    assert_eq!(pre_snapshot.claim_window, 3600);
    assert_eq!(pre_snapshot.fee_config.lock_fee_rate, 100);
    assert_eq!(pre_snapshot.fee_config.release_fee_rate, 250);
    assert_eq!(pre_snapshot.fee_config.fee_recipient, admin);
    assert_eq!(pre_snapshot.fee_config.fee_enabled, true);
    assert_eq!(pre_snapshot.pause_flags.lock_paused, true);
    assert_eq!(pre_snapshot.pause_flags.release_paused, false);
    assert_eq!(pre_snapshot.pause_flags.refund_paused, true);
    assert_eq!(pre_snapshot.governance_contract, Some(gov_contract.clone()));
    assert_eq!(pre_snapshot.min_governance_version, 2);
    assert_eq!(pre_snapshot.has_amount_policy, true);
    assert_eq!(pre_snapshot.min_lock_amount, 10);
    assert_eq!(pre_snapshot.max_lock_amount, 1_000_000);

    let pre_errors = escrow_client.get_circuit_error_log();
    assert_eq!(pre_errors.len(), 1);
    assert_eq!(pre_errors.get(0).unwrap().bounty_id, 10);

    // =========================================================================
    // 🚀 SIMULATE UPGRADE BOUNDARY
    // Re-register the contract code under the same contract ID.
    // In Soroban, this preserves the state and simulates an upgrade.
    // =========================================================================
    env.register_contract(&contract_id, BountyEscrowContract);
    let escrow_client = BountyEscrowContractClient::new(&env, &contract_id);

    // 2. Assert every audit view function still returns the pre-upgrade values correctly
    let post_snapshot = escrow_client.get_admin_audit_view();
    assert_eq!(post_snapshot.version, pre_snapshot.version);
    assert_eq!(post_snapshot.admin, pre_snapshot.admin);
    assert_eq!(post_snapshot.token, pre_snapshot.token);
    assert_eq!(post_snapshot.claim_window, pre_snapshot.claim_window);
    assert_eq!(post_snapshot.fee_config.lock_fee_rate, pre_snapshot.fee_config.lock_fee_rate);
    assert_eq!(post_snapshot.fee_config.release_fee_rate, pre_snapshot.fee_config.release_fee_rate);
    assert_eq!(post_snapshot.fee_config.fee_recipient, pre_snapshot.fee_config.fee_recipient);
    assert_eq!(post_snapshot.fee_config.fee_enabled, pre_snapshot.fee_config.fee_enabled);
    assert_eq!(post_snapshot.pause_flags.lock_paused, pre_snapshot.pause_flags.lock_paused);
    assert_eq!(post_snapshot.pause_flags.release_paused, pre_snapshot.pause_flags.release_paused);
    assert_eq!(post_snapshot.pause_flags.refund_paused, pre_snapshot.pause_flags.refund_paused);
    assert_eq!(post_snapshot.governance_contract, pre_snapshot.governance_contract);
    assert_eq!(post_snapshot.min_governance_version, pre_snapshot.min_governance_version);
    assert_eq!(post_snapshot.has_amount_policy, pre_snapshot.has_amount_policy);
    assert_eq!(post_snapshot.min_lock_amount, pre_snapshot.min_lock_amount);
    assert_eq!(post_snapshot.max_lock_amount, pre_snapshot.max_lock_amount);

    assert_eq!(escrow_client.get_circuit_breaker_admin(), Some(admin.clone()));

    let post_errors = escrow_client.get_circuit_error_log();
    assert_eq!(post_errors.len(), 1);
    let err0 = post_errors.get(0).unwrap();
    assert_eq!(err0.bounty_id, 10);
    assert_eq!(err0.operation, soroban_sdk::symbol_short!("lock"));
    assert_eq!(err0.error_code, 100);

    // 3. Add a test that an admin action performed after the simulated upgrade is correctly appended
    // (not overwriting or losing prior history).
    
    // Modify config post-upgrade
    escrow_client.set_claim_window(&7200);

    let updated_snapshot = escrow_client.get_admin_audit_view();
    assert_eq!(updated_snapshot.claim_window, 7200);
    // Other values are preserved
    assert_eq!(updated_snapshot.admin, admin);
    assert_eq!(updated_snapshot.fee_config.lock_fee_rate, 100);
    assert_eq!(updated_snapshot.pause_flags.lock_paused, true);

    // Record another failure post-upgrade
    env.as_contract(&contract_id, || {
        crate::error_recovery::record_failure(&env, 20, soroban_sdk::symbol_short!("rel"), 200u32);
    });

    let updated_errors = escrow_client.get_circuit_error_log();
    assert_eq!(updated_errors.len(), 2);
    
    // Pre-upgrade entry is preserved at index 0
    let saved_err0 = updated_errors.get(0).unwrap();
    assert_eq!(saved_err0.bounty_id, 10);
    assert_eq!(saved_err0.operation, soroban_sdk::symbol_short!("lock"));
    assert_eq!(saved_err0.error_code, 100);

    // Post-upgrade entry is appended at index 1
    let saved_err1 = updated_errors.get(1).unwrap();
    assert_eq!(saved_err1.bounty_id, 20);
    assert_eq!(saved_err1.operation, soroban_sdk::symbol_short!("rel"));
    assert_eq!(saved_err1.error_code, 200);
}


