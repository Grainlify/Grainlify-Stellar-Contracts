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
        .update_fee_config(&Some(100), &Some(250), &Some(admin.clone()), &Some(true))
        .unwrap();

    // Configure pause flags
    escrow_client
        .set_paused(&Some(true), &Some(false), &Some(true))
        .unwrap();

    // Configure governance
    escrow_client
        .set_governance_contract(&gov_contract)
        .unwrap();
    escrow_client.set_min_governance_version(&2).unwrap();

    // Configure claim window and amount policy
    escrow_client.set_claim_window(&3600).unwrap();
    escrow_client
        .set_amount_policy(&admin, &10, &1_000_000)
        .unwrap();

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

