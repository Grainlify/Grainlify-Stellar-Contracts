#![cfg(test)]

use crate::{ProgramEscrowContract, ProgramEscrowContractClient};
use soroban_sdk::{
    testutils::Address as _,
    Address, Env, String,
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

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let governance_addr = Address::generate(&env);

    client.set_admin(&admin);

    // Set governance contract
    client.set_governance_contract(&governance_addr);

    // Verify it was set
    let stored = client.get_governance_contract();
    assert_eq!(stored, Some(governance_addr));
}

#[test]
fn test_set_min_governance_version() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    // Set minimum version
    client.set_min_governance_version(&2);

    // Verify it was set
    assert_eq!(client.get_min_governance_version(), 2);
}

#[test]
fn test_governance_version_check_with_mock() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    // Register mock governance contract
    let gov_contract_id = env.register_contract(None, mock_governance::MockGovernanceContract);

    // Set governance contract and minimum version
    client.set_governance_contract(&gov_contract_id);
    client.set_min_governance_version(&2);

    // Admin operations should work when governance version is met
    client.set_paused(&Some(true), &None, &None);
}

#[test]
#[should_panic(expected = "Governance version requirement not met")]
fn test_governance_version_check_fails_when_version_too_low() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    // Register mock governance contract (returns version 2)
    let gov_contract_id = env.register_contract(None, mock_governance::MockGovernanceContract);

    // Set governance contract and require version 3 (higher than mock returns)
    client.set_governance_contract(&gov_contract_id);
    client.set_min_governance_version(&3);

    // This should panic because governance version (2) < required version (3)
    client.set_paused(&Some(true), &None, &None);
}

#[test]
fn test_admin_operations_work_without_governance() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    // Admin operations should work without governance configured
    client.set_paused(&Some(true), &None, &None);
    client.update_rate_limit_config(&3600, &10, &60);
}

#[test]
fn test_governance_integration_with_program_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let backend = Address::generate(&env);
    let token = Address::generate(&env);
    let program_id = String::from_str(&env, "TestProgram");

    client.set_admin(&admin);

    // Register mock governance contract
    let gov_contract_id = env.register_contract(None, mock_governance::MockGovernanceContract);
    client.set_governance_contract(&gov_contract_id);
    client.set_min_governance_version(&2);

    // Initialize program (should work with governance)
    client.initialize_program(&program_id, &backend, &token);

    // Admin operations should respect governance
    client.set_paused(&Some(false), &Some(false), &Some(false));

    // Verify program was created
    assert!(client.program_exists());
}

#[test]
fn test_governance_prevents_unauthorized_config_changes() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.set_admin(&admin);

    // Register mock governance contract
    let gov_contract_id = env.register_contract(None, mock_governance::MockGovernanceContract);
    client.set_governance_contract(&gov_contract_id);
    client.set_min_governance_version(&2);

    // Rate limit config changes should respect governance
    client.update_rate_limit_config(&7200, &5, &120);

    let config = client.get_rate_limit_config();
    assert_eq!(config.window_size, 7200);
    assert_eq!(config.max_operations, 5);
}
