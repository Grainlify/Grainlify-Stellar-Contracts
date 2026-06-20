#![cfg(test)]

use crate::{governance_integration, BountyEscrowContract, BountyEscrowContractClient, Error};
use grainlify_core::{GrainlifyContract, GrainlifyContractClient};
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env};

// Mock governance contract for testing
mod mock_governance {
    use soroban_sdk::{contract, contractimpl, BytesN, Env};

    #[contract]
    pub struct MockGovernanceContract;

    #[contractimpl]
    impl MockGovernanceContract {
        pub fn get_ver(_env: Env) -> u32 {
            2
        }

        pub fn get_version_numeric_encoded(_env: Env) -> u32 {
            20_000
        }

        pub fn is_upg_ok(env: Env, wasm_hash: BytesN<32>) -> bool {
            wasm_hash == BytesN::from_array(&env, &[7u8; 32])
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
fn test_governance_version_gate_with_real_grainlify_core_contract() {
    let env = Env::default();
    env.mock_all_auths();

    let escrow_id = env.register_contract(None, BountyEscrowContract);
    let escrow = BountyEscrowContractClient::new(&env, &escrow_id);

    let grainlify_id = env.register_contract(None, GrainlifyContract);
    let grainlify = GrainlifyContractClient::new(&env, &grainlify_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    escrow.init(&admin, &token);
    grainlify.init_admin(&admin);

    escrow.set_governance_contract(&grainlify_id);
    escrow.set_min_governance_version(&3);

    // The real grainlify-core contract starts below this required version, so guarded
    // bounty-escrow admin operations must reject before the version bump.
    assert_eq!(
        escrow.try_set_paused(&Some(true), &None, &None),
        Err(Ok(Error::GovernanceVersionTooLow))
    );
    assert_eq!(
        escrow.try_update_fee_config(&Some(100), &None, &None, &None),
        Err(Ok(Error::GovernanceVersionTooLow))
    );

    // After the real governance contract version reaches the minimum, the same
    // cross-contract gated operations should succeed end-to-end.
    grainlify.set_version(&3);

    escrow.set_paused(&Some(true), &None, &None);
    escrow.update_fee_config(&Some(100), &None, &None, &Some(true));

    let fee_config = escrow.get_fee_config();
    assert_eq!(fee_config.lock_fee_rate, 100);
    assert!(fee_config.fee_enabled);
}

#[test]
fn test_governance_version_gate_uses_real_numeric_encoded_semver() {
    let env = Env::default();
    env.mock_all_auths();

    let escrow_id = env.register_contract(None, BountyEscrowContract);
    let escrow = BountyEscrowContractClient::new(&env, &escrow_id);

    let grainlify_id = env.register_contract(None, GrainlifyContract);
    let grainlify = GrainlifyContractClient::new(&env, &grainlify_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    escrow.init(&admin, &token);
    grainlify.init_admin(&admin);
    grainlify.set_version(&2);

    assert_eq!(grainlify.get_version_numeric_encoded(), 20_000);

    escrow.set_governance_contract(&grainlify_id);
    escrow.set_min_governance_version(&20_000);

    // A numeric-encoded v2.0.0 minimum must pass through the same real
    // cross-contract boundary instead of comparing the simple raw version `2`.
    escrow.set_paused(&None, &Some(true), &None);
}

#[test]
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

    // This should return a typed error because governance version (2) < required version (3)
    let result = client.try_set_paused(&Some(true), &None, &None);
    assert_eq!(result, Err(Ok(Error::GovernanceVersionTooLow)));
}

#[test]
fn test_governance_version_too_low_blocks_fee_config_with_typed_error() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    let _ = client.init(&admin, &token);

    let gov_contract_id = env.register_contract(None, mock_governance::MockGovernanceContract);
    let _ = client.set_governance_contract(&gov_contract_id);
    let _ = client.set_min_governance_version(&3);

    let result = client.try_update_fee_config(&Some(100), &None, &None, &None);
    assert_eq!(result, Err(Ok(Error::GovernanceVersionTooLow)));
}

#[test]
fn test_upgrade_approval_requires_matching_executed_governance_hash() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    let _ = client.init(&admin, &token);

    let gov_contract_id = env.register_contract(None, mock_governance::MockGovernanceContract);
    let _ = client.set_governance_contract(&gov_contract_id);
    let _ = client.set_min_governance_version(&2);

    let approved_hash = BytesN::from_array(&env, &[7u8; 32]);
    let wrong_hash = BytesN::from_array(&env, &[9u8; 32]);

    env.as_contract(&contract_id, || {
        assert!(governance_integration::check_upgrade_approval(
            &env,
            &approved_hash,
        ));
        assert!(!governance_integration::check_upgrade_approval(
            &env,
            &wrong_hash,
        ));
    });
}

#[test]
fn test_upgrade_approval_denies_when_governance_is_not_configured() {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let wasm_hash = BytesN::from_array(&env, &[7u8; 32]);

    env.as_contract(&contract_id, || {
        assert!(!governance_integration::check_upgrade_approval(
            &env, &wasm_hash,
        ));
    });
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
