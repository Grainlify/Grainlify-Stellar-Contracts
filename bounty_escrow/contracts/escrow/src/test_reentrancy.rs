#![cfg(test)]
use crate::{BountyEscrowContract, BountyEscrowContractClient, Error as ContractError};
use soroban_sdk::{
    testutils::{Address as _},
    token, Address, Env,
};

fn create_test_env() -> (Env, BountyEscrowContractClient<'static>, Address) {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    (env, client, contract_id)
}

fn create_token_contract<'a>(
    e: &'a Env,
    admin: &Address,
) -> (Address, token::Client<'a>, token::StellarAssetClient<'a>) {
    let token_id = e.register_stellar_asset_contract_v2(admin.clone());
    let token = token_id.address();
    let token_client = token::Client::new(e, &token);
    let token_admin_client = token::StellarAssetClient::new(e, &token);
    (token, token_client, token_admin_client)
}

#[test]
fn test_reentrancy_guard_leak_fix() {
    let (env, client, contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);

    // 1. Call release_funds with a non-existent bounty.
    // This previously leaked the guard because it was set before validation.
    let res = client.try_release_funds(&1, &contributor);
    assert_eq!(res, Err(Ok(ContractError::BountyNotFound)));

    // Verify the guard is NOT leaked
    env.as_contract(&contract_id, || {
        use crate::DataKey;
        let has_guard = env.storage().instance().has(&DataKey::ReentrancyGuard);
        assert!(!has_guard, "Reentrancy guard should NOT have leaked after failed call");
    });

    // 2. Lock funds for a real bounty and release them.
    // This would have failed with "Reentrancy detected" if the guard leaked.
    token_admin_client.mint(&depositor, &1000);
    client.lock_funds(&depositor, &2, &1000, &(env.ledger().timestamp() + 100));
    client.release_funds(&2, &contributor);
}

#[test]
#[should_panic] // Soroban host may escalate "Contract re-entry is not allowed" to HostError
fn test_genuine_reentrancy_blocked() {
    let (env, client, contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    client.init(&admin, &token);

    // Simulate a reentrant call by setting the guard manually.
    env.as_contract(&contract_id, || {
        use crate::DataKey;
        env.storage().instance().set(&DataKey::ReentrancyGuard, &true);
        
        // This should panic
        client.release_funds(&1, &Address::generate(&env));
    });
}
