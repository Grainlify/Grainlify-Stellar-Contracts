#![cfg(test)]

use crate::{BountyEscrowContract, BountyEscrowContractClient};
use soroban_sdk::{
    testutils::Address as _,
    Address, Env,
};

// Mock governance contract for testing
mod mock_governance {
    use soroban_sdk::{contract, contractimpl, Env};

    #[contract]
    pub struct MockGovernanceContract;

    #[contractimpl]
    impl MockGovernanceContract {
        pub fn get_ver(_env: Env) -> u32 {
            2
        }
    }
}

#[test]
fn test_set_governance_contract() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let governance_addr = Address::generate(&env);

    let _ = client.init(&admin, &token);

    // Set governance contract
    let _ = client.set_governance_contract(&governance_addr);

    // Verify it was set
    let stored = client.get_governance_contract();
    assert_eq!(stored, Some(governance_addr));
}

#[test]
fn test_set_min_governance_version() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    let _ = client.init(&admin, &token);

    // Set minimum version
    let _ = client.set_min_governance_version(&2);

    // Verify it was set
    assert_eq!(client.get_min_governance_version(), 2);
}

#[test]
fn test_governance_version_check_with_mock() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    let _ = client.init(&admin, &token);

    // Register mock governance contract
    let gov_contract_id = env.register_contract(None, mock_governance::MockGovernanceContract);

    // Set governance contract and minimum version
    let _ = client.set_governance_contract(&gov_contract_id);
    let _ = client.set_min_governance_version(&2);

    // Admin operations should work when governance version is met
    let _ = client.set_paused(&Some(true), &None, &None);
}

#[test]
#[should_panic(expected = "Governance version requirement not met")]
fn test_governance_version_check_fails_when_version_too_low() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    let _ = client.init(&admin, &token);

    // Register mock governance contract (returns version 2)
    let gov_contract_id = env.register_contract(None, mock_governance::MockGovernanceContract);

    // Set governance contract and require version 3 (higher than mock returns)
    let _ = client.set_governance_contract(&gov_contract_id);
    let _ = client.set_min_governance_version(&3);

    // This should panic because governance version (2) < required version (3)
    let _ = client.set_paused(&Some(true), &None, &None);
}

#[test]
fn test_admin_operations_work_without_governance() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    let _ = client.init(&admin, &token);

    // Admin operations should work without governance configured
    let _ = client.set_paused(&Some(true), &None, &None);
    let _ = client.update_fee_config(&Some(100), &None, &None, &None);
}

#[test]
fn test_governance_integration_with_bounty_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    let _ = client.init(&admin, &token);

    // Register mock governance contract
    let gov_contract_id = env.register_contract(None, mock_governance::MockGovernanceContract);
    let _ = client.set_governance_contract(&gov_contract_id);
    let _ = client.set_min_governance_version(&2);

    // Admin operations should respect governance
    let _ = client.set_paused(&Some(false), &Some(false), &Some(false));

    // Fee config changes should respect governance
    let _ = client.update_fee_config(&Some(50), &Some(25), &None, &Some(true));

    let fee_config = client.get_fee_config();
    assert_eq!(fee_config.lock_fee_rate, 50);
    assert_eq!(fee_config.release_fee_rate, 25);
    assert!(fee_config.fee_enabled);
}

#[test]
fn test_governance_prevents_unauthorized_config_changes() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    let _ = client.init(&admin, &token);

    // Register mock governance contract
    let gov_contract_id = env.register_contract(None, mock_governance::MockGovernanceContract);
    let _ = client.set_governance_contract(&gov_contract_id);
    let _ = client.set_min_governance_version(&2);

    // Multisig config changes should respect governance
    let signers = soroban_sdk::vec![&env, Address::generate(&env), Address::generate(&env)];
    let _ = client.update_multisig_config(&1000_0000000, &signers, &2);

    let config = client.get_multisig_config();
    assert_eq!(config.threshold_amount, 1000_0000000);
    assert_eq!(config.required_signatures, 2);
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_governance_not_initialized_error() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let governance_addr = Address::generate(&env);

    // Should fail because contract is not initialized
    let _ = client.set_governance_contract(&governance_addr);
}
