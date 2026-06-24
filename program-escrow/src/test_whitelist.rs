#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env, String, vec};

struct WhitelistSetup<'a> {
    env: Env,
    admin: Address,
    operator: Address,
    recipient: Address,
    token_id: Address,
    program_id: String,
    client: ProgramEscrowContractClient<'a>,
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
}

impl<'a> WhitelistSetup<'a> {
    fn new() -> Self {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let operator = Address::generate(&env);
        let recipient = Address::generate(&env);

        let token_admin_addr = Address::generate(&env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin_addr.clone()).address();
        let token = token::Client::new(&env, &token_id);
        let token_admin = token::StellarAssetClient::new(&env, &token_id);

        let program_id = String::from_str(&env, "WL-Test");

        // Initialize contract with admin
        client.initialize_contract(&admin);

        // Initialize program
        client.init_program(&program_id, &operator, &token_id);

        Self {
            env,
            admin,
            operator,
            recipient,
            token_id,
            program_id,
            client,
            token,
            token_admin,
        }
    }
}

#[test]
fn test_set_and_get_whitelist() {
    let setup = WhitelistSetup::new();
    
    // Default: not whitelisted
    assert!(!setup.client.is_whitelisted(&setup.recipient));

    setup.env.mock_all_auths();

    // Set to whitelisted
    setup.client.set_whitelist(&setup.recipient, &true);
    assert!(setup.client.is_whitelisted(&setup.recipient));

    // Remove from whitelist
    setup.client.set_whitelist(&setup.recipient, &false);
    assert!(!setup.client.is_whitelisted(&setup.recipient));
}

#[test]
#[should_panic]
fn test_set_whitelist_requires_admin() {
    let setup = WhitelistSetup::new();
    // No mock_all_auths or using non-admin caller
    // This will panic due to require_auth on admin
    let random = Address::generate(&setup.env);
    setup.client.set_whitelist(&random, &true);
}

#[test]
fn test_set_and_get_enforcement() {
    let setup = WhitelistSetup::new();

    // Default: not enforced
    assert!(!setup.client.is_whitelist_enforced());

    setup.env.mock_all_auths();

    // Enable enforcement
    setup.client.set_whitelist_enforced(&true);
    assert!(setup.client.is_whitelist_enforced());

    // Disable enforcement
    setup.client.set_whitelist_enforced(&false);
    assert!(!setup.client.is_whitelist_enforced());
}

#[test]
#[should_panic]
fn test_set_enforcement_requires_admin() {
    let setup = WhitelistSetup::new();
    setup.client.set_whitelist_enforced(&true);
}

#[test]
fn test_payout_with_enforcement_disabled() {
    let setup = WhitelistSetup::new();
    
    // Fund program
    setup.env.mock_all_auths();
    setup.token_admin.mint(&setup.operator, &10000);
    setup.client.lock_program_funds(&setup.operator, &5000);

    // Whitelist is not enforced by default, so payout should succeed
    setup.client.single_payout(&setup.recipient, &1000);
    assert_eq!(setup.token.balance(&setup.recipient), 1000);
}

#[test]
#[should_panic(expected = "Recipient not whitelisted")]
fn test_single_payout_with_enforcement_enabled_panics() {
    let setup = WhitelistSetup::new();

    // Fund program
    setup.env.mock_all_auths();
    setup.token_admin.mint(&setup.operator, &10000);
    setup.client.lock_program_funds(&setup.operator, &5000);

    // Enable enforcement
    setup.client.set_whitelist_enforced(&true);

    // Recipient not whitelisted, single payout should panic
    setup.client.single_payout(&setup.recipient, &1000);
}

#[test]
#[should_panic(expected = "Recipient not whitelisted")]
fn test_batch_payout_with_enforcement_enabled_panics() {
    let setup = WhitelistSetup::new();

    // Fund program
    setup.env.mock_all_auths();
    setup.token_admin.mint(&setup.operator, &10000);
    setup.client.lock_program_funds(&setup.operator, &5000);

    // Enable enforcement
    setup.client.set_whitelist_enforced(&true);

    let recipients = vec![&setup.env, setup.recipient.clone()];
    let amounts = vec![&setup.env, 1000];

    // Recipient not whitelisted, batch payout should panic
    setup.client.batch_payout(&recipients, &amounts);
}

#[test]
fn test_payout_with_enforcement_enabled_succeeds_for_whitelisted() {
    let setup = WhitelistSetup::new();

    // Fund program
    setup.env.mock_all_auths();
    setup.token_admin.mint(&setup.operator, &10000);
    setup.client.lock_program_funds(&setup.operator, &5000);

    // Enable enforcement and whitelist recipient
    setup.client.set_whitelist_enforced(&true);
    setup.client.set_whitelist(&setup.recipient, &true);

    // Recipient whitelisted, single payout should succeed
    setup.client.single_payout(&setup.recipient, &1000);
    assert_eq!(setup.token.balance(&setup.recipient), 1000);

    // Batch payout should also succeed
    let recipients = vec![&setup.env, setup.recipient.clone()];
    let amounts = vec![&setup.env, 1000];
    setup.client.batch_payout(&recipients, &amounts);
    assert_eq!(setup.token.balance(&setup.recipient), 2000);
}
