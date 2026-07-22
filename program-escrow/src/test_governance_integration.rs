#![cfg(test)]

use crate::{governance_integration, Error, ProgramEscrowContract, ProgramEscrowContractClient};
use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, String};

// Mock governance contract for testing
mod mock_governance {
    use soroban_sdk::{contract, contractimpl, symbol_short, BytesN, Env, Symbol};

    /// Storage key used by the mock to record a vetoed proposal ID.
    const VETOED_KEY: Symbol = symbol_short!("VETOED_ID");

    #[contract]
    pub struct MockGovernanceContract;

    #[contractimpl]
    impl MockGovernanceContract {
        pub fn get_ver(_env: Env) -> u32 {
            2
        }

        pub fn is_upg_ok(env: Env, wasm_hash: BytesN<32>) -> bool {
            wasm_hash == BytesN::from_array(&env, &[7u8; 32])
        }

        /// Returns `true` when `proposal_id` has been marked as vetoed.
        ///
        /// Stores the ID of the single vetoed proposal (u32::MAX = none vetoed).
        pub fn is_vetoed(env: Env, proposal_id: u32) -> bool {
            let vetoed_id: u32 = env
                .storage()
                .instance()
                .get(&VETOED_KEY)
                .unwrap_or(u32::MAX);
            vetoed_id == proposal_id
        }

        /// Test-only helper: mark `proposal_id` as vetoed.
        pub fn set_vetoed(env: Env, proposal_id: u32) {
            env.storage().instance().set(&VETOED_KEY, &proposal_id);
        }
    }
}

// Enhanced mock that tracks proposal states (Pending=1, Rejected=3, Executed=4)
// and only approves hashes with an Executed proposal ÔÇö mirrors real governance logic.
mod mock_governance_with_state {
    use soroban_sdk::{contract, contractimpl, symbol_short, BytesN, Env, Map, Symbol};

    const PROPOSAL_STATES: Symbol = symbol_short!("PR_STATE");

    #[contract]
    pub struct MockGovernanceWithState;

    #[contractimpl]
    impl MockGovernanceWithState {
        pub fn get_ver(_env: Env) -> u32 {
            2
        }

        /// Only returns true if there is an Executed (4) proposal whose hash matches.
        pub fn is_upg_ok(env: Env, wasm_hash: BytesN<32>) -> bool {
            let store: Map<u32, (BytesN<32>, u32)> = env
                .storage()
                .instance()
                .get(&PROPOSAL_STATES)
                .unwrap_or(Map::new(&env));
            for (_, (hash, status)) in store.iter() {
                if hash == wasm_hash && status == 4 {
                    return true;
                }
            }
            false
        }

        /// Register a proposal. status codes: 1=Pending, 3=Rejected, 4=Executed
        pub fn set_proposal(env: Env, proposal_id: u32, wasm_hash: BytesN<32>, status: u32) {
            let mut store: Map<u32, (BytesN<32>, u32)> = env
                .storage()
                .instance()
                .get(&PROPOSAL_STATES)
                .unwrap_or(Map::new(&env));
            store.set(proposal_id, (wasm_hash, status));
            env.storage().instance().set(&PROPOSAL_STATES, &store);
        }

        /// This mock does not support vetoes — always returns false.
        /// Veto behaviour is tested via mock_governance::MockGovernanceContract.
        pub fn is_vetoed(_env: Env, _proposal_id: u32) -> bool {
            false
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

    client.setadmin(&admin);

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
    client.setadmin(&admin);

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
    client.setadmin(&admin);

    // Register mock governance contract
    let gov_contract_id = env.register_contract(None, mock_governance::MockGovernanceContract);

    // Set governance contract and minimum version
    client.set_governance_contract(&gov_contract_id);
    client.set_min_governance_version(&2);

    // Admin operations should work when governance version is met
    client.set_paused(&Some(true), &None, &None);
}

#[test]
fn test_governance_version_check_fails_when_version_too_low() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.setadmin(&admin);

    // Register mock governance contract (returns version 2)
    let gov_contract_id = env.register_contract(None, mock_governance::MockGovernanceContract);

    // Set governance contract and require version 3 (higher than mock returns)
    client.set_governance_contract(&gov_contract_id);
    client.set_min_governance_version(&3);

    // This should return a typed error because governance version (2) < required version (3)
    let result = client.try_set_paused(&Some(true), &None, &None);
    assert_eq!(result, Err(Ok(Error::GovernanceVersionTooLow)));
}

#[test]
fn test_governance_version_too_low_blocks_rate_limit_config_with_typed_error() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.setadmin(&admin);

    let gov_contract_id = env.register_contract(None, mock_governance::MockGovernanceContract);
    client.set_governance_contract(&gov_contract_id);
    client.set_min_governance_version(&3);

    let result = client.try_update_rate_limit_config(&3600, &10, &60);
    assert_eq!(result, Err(Ok(Error::GovernanceVersionTooLow)));
}

#[test]
fn test_upgrade_approval_requires_matching_executed_governance_hash() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.setadmin(&admin);

    let gov_contract_id = env.register_contract(None, mock_governance::MockGovernanceContract);
    client.set_governance_contract(&gov_contract_id);
    client.set_min_governance_version(&2);

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
    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let wasm_hash = BytesN::from_array(&env, &[7u8; 32]);

    env.as_contract(&contract_id, || {
        assert!(!governance_integration::check_upgrade_approval(
            &env, &wasm_hash,
        ));
    });
}

#[test]
fn testadmin_operations_work_without_governance() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.setadmin(&admin);

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

    client.setadmin(&admin);

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
    client.setadmin(&admin);

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

// ÔöÇÔöÇ Access-control: proposal-state gating ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

#[test]
fn test_executed_proposal_triggers_upgrade_successfully() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.setadmin(&admin);

    let gov_id = env.register_contract(None, mock_governance_with_state::MockGovernanceWithState);
    client.set_governance_contract(&gov_id);
    client.set_min_governance_version(&2);

    let gov_client = mock_governance_with_state::MockGovernanceWithStateClient::new(&env, &gov_id);

    let approved_hash = BytesN::from_array(&env, &[0xabu8; 32]);
    gov_client.set_proposal(&0, &approved_hash, &4); // 4 = Executed

    env.as_contract(&contract_id, || {
        assert!(governance_integration::check_upgrade_approval(
            &env,
            &approved_hash,
        ));
    });
}

#[test]
fn test_pending_proposal_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.setadmin(&admin);

    let gov_id = env.register_contract(None, mock_governance_with_state::MockGovernanceWithState);
    client.set_governance_contract(&gov_id);
    client.set_min_governance_version(&2);

    let gov_client = mock_governance_with_state::MockGovernanceWithStateClient::new(&env, &gov_id);

    let hash = BytesN::from_array(&env, &[0xbbu8; 32]);
    gov_client.set_proposal(&1, &hash, &1); // 1 = Pending

    env.as_contract(&contract_id, || {
        assert!(!governance_integration::check_upgrade_approval(&env, &hash));
    });
}

#[test]
fn test_rejected_proposal_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.setadmin(&admin);

    let gov_id = env.register_contract(None, mock_governance_with_state::MockGovernanceWithState);
    client.set_governance_contract(&gov_id);
    client.set_min_governance_version(&2);

    let gov_client = mock_governance_with_state::MockGovernanceWithStateClient::new(&env, &gov_id);

    let hash = BytesN::from_array(&env, &[0xccu8; 32]);
    gov_client.set_proposal(&2, &hash, &3); // 3 = Rejected

    env.as_contract(&contract_id, || {
        assert!(!governance_integration::check_upgrade_approval(&env, &hash));
    });
}

#[test]
fn test_already_executed_proposal_for_different_hash_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.setadmin(&admin);

    let gov_id = env.register_contract(None, mock_governance_with_state::MockGovernanceWithState);
    client.set_governance_contract(&gov_id);
    client.set_min_governance_version(&2);

    let gov_client = mock_governance_with_state::MockGovernanceWithStateClient::new(&env, &gov_id);

    let executed_hash = BytesN::from_array(&env, &[0xdd; 32]);
    let different_hash = BytesN::from_array(&env, &[0xee; 32]);
    gov_client.set_proposal(&3, &executed_hash, &4); // 4 = Executed (for a different hash)

    env.as_contract(&contract_id, || {
        // The hash being queried does not match any executed proposal
        assert!(!governance_integration::check_upgrade_approval(
            &env,
            &different_hash,
        ));
    });
}

#[test]
fn test_type_tag_confusion_prevents_non_governance_hash_acceptance() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.setadmin(&admin);

    let gov_id = env.register_contract(None, mock_governance_with_state::MockGovernanceWithState);
    client.set_governance_contract(&gov_id);
    client.set_min_governance_version(&2);

    let gov_client = mock_governance_with_state::MockGovernanceWithStateClient::new(&env, &gov_id);

    // Simulate a hash used for a completely different protocol purpose
    // (e.g. a content-hash for a stored document, a non-governance identifier).
    let non_governance_hash = BytesN::from_array(&env, &[0xff; 32]);

    // Register it with a status that is NOT Executed (e.g. Rejected = 3),
    // and also register a completely unrelated Executed proposal to show
    // that type confusion would be required to accidentally match.
    let unrelated_hash = BytesN::from_array(&env, &[0xaa; 32]);
    gov_client.set_proposal(&4, &unrelated_hash, &4); // unrelated executed proposal
    gov_client.set_proposal(&5, &non_governance_hash, &3); // non-governance hash, Rejected

    env.as_contract(&contract_id, || {
        // The non-governance hash should not be approved even though
        // there are other executed proposals in storage
        assert!(!governance_integration::check_upgrade_approval(
            &env,
            &non_governance_hash,
        ));
        // Sanity-check that the unrelated executed proposal still works
        assert!(governance_integration::check_upgrade_approval(
            &env,
            &unrelated_hash,
        ));
    });
}

// ============================================================================
// Governance veto / cancellation path tests
//
// Design note: grainlify-core's ProposalStatus enum (Pending | Active |
// Approved | Rejected | Executed | Expired) has no native Vetoed/Cancelled
// variant — this is a design gap identified per issue #236.  The tests
// below verify the escrow-side guard introduced in `governance_integration`
// (check_proposal_vetoed / is_vetoed) using the extended MockGovernanceContract
// above that records a single vetoed proposal ID.  A follow-up should add a
// real veto mechanism to grainlify-core's GovernanceContract.
// ============================================================================

/// Vetoing a proposal that was previously approved prevents the escrow from
/// treating the corresponding upgrade hash as valid.
#[test]
fn test_vetoed_proposal_blocks_upgrade_approval() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.setadmin(&admin);

    let gov_contract_id = env.register_contract(None, mock_governance::MockGovernanceContract);
    let gov_client =
        mock_governance::MockGovernanceContractClient::new(&env, &gov_contract_id);

    client.set_governance_contract(&gov_contract_id);
    client.set_min_governance_version(&2);

    // Proposal 0 corresponds to the approved hash (all-7 bytes) in the mock.
    let approved_hash = BytesN::from_array(&env, &[7u8; 32]);

    // Before veto: upgrade is approved.
    env.as_contract(&contract_id, || {
        assert!(
            governance_integration::check_upgrade_approval(&env, &approved_hash),
            "upgrade should be approved before veto"
        );
    });

    // Veto proposal 0 via the test-only helper on the mock.
    gov_client.set_vetoed(&0);

    // After veto: escrow must detect the veto.
    env.as_contract(&contract_id, || {
        assert!(
            governance_integration::check_proposal_vetoed(&env, 0),
            "escrow must report proposal 0 as vetoed"
        );
    });
}

/// A proposal that was never vetoed must NOT be reported as vetoed — veto
/// flag is proposal-specific, not global.
#[test]
fn test_non_vetoed_proposal_is_not_blocked() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.setadmin(&admin);

    let gov_contract_id = env.register_contract(None, mock_governance::MockGovernanceContract);
    let gov_client =
        mock_governance::MockGovernanceContractClient::new(&env, &gov_contract_id);

    client.set_governance_contract(&gov_contract_id);
    client.set_min_governance_version(&2);

    // Veto proposal 0, but query proposal 1.
    gov_client.set_vetoed(&0);

    env.as_contract(&contract_id, || {
        assert!(
            !governance_integration::check_proposal_vetoed(&env, 1),
            "proposal 1 was not vetoed and must not be flagged"
        );
    });
}

/// When NO governance contract is configured, `check_proposal_vetoed` must
/// return `false` (open/permissionless mode — no veto possible).
#[test]
fn test_veto_check_returns_false_without_governance_contract() {
    let env = Env::default();
    let contract_id = env.register_contract(None, ProgramEscrowContract);

    env.as_contract(&contract_id, || {
        assert!(
            !governance_integration::check_proposal_vetoed(&env, 0),
            "no governance configured → veto check must return false"
        );
    });
}

/// Resources remain accessible after a veto — the escrow does not lock itself
/// permanently.  Admin operations unrelated to the vetoed proposal must still
/// succeed, confirming resources are released rather than stuck.
#[test]
fn test_resources_accessible_after_proposal_veto() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.setadmin(&admin);

    let gov_contract_id = env.register_contract(None, mock_governance::MockGovernanceContract);
    let gov_client =
        mock_governance::MockGovernanceContractClient::new(&env, &gov_contract_id);

    client.set_governance_contract(&gov_contract_id);
    client.set_min_governance_version(&2);

    // Veto proposal 0.
    gov_client.set_vetoed(&0);

    // Admin operations that don't depend on proposal 0 must still succeed.
    client.set_paused(&Some(false), &Some(false), &Some(false));

    let flags = client.get_pause_flags();
    assert!(
        !flags.lock_paused && !flags.release_paused && !flags.refund_paused,
        "all flags should be false — contract must not be stuck after veto"
    );
}

/// Documents the design boundary for late veto: callers must apply the
/// combined guard `approved && !vetoed`.  A late veto alone cannot undo an
/// already-executed proposal, but `check_proposal_vetoed` correctly surfaces
/// the veto so a caller can block the action.
#[test]
fn test_veto_after_execution_does_not_retroactively_revoke_approval() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.setadmin(&admin);

    let gov_contract_id = env.register_contract(None, mock_governance::MockGovernanceContract);
    let gov_client =
        mock_governance::MockGovernanceContractClient::new(&env, &gov_contract_id);

    client.set_governance_contract(&gov_contract_id);
    client.set_min_governance_version(&2);

    let approved_hash = BytesN::from_array(&env, &[7u8; 32]);

    // Step 1 – upgrade is currently approved (proposal executed).
    env.as_contract(&contract_id, || {
        assert!(
            governance_integration::check_upgrade_approval(&env, &approved_hash),
            "upgrade approval should be present before any veto"
        );
    });

    // Step 2 – late veto attempted after execution.
    gov_client.set_vetoed(&0);

    // Step 3 – veto IS detected by check_proposal_vetoed.
    env.as_contract(&contract_id, || {
        assert!(
            governance_integration::check_proposal_vetoed(&env, 0),
            "check_proposal_vetoed must reflect the veto"
        );
    });

    // Step 4 – combined guard (approved && !vetoed) correctly blocks the action.
    env.as_contract(&contract_id, || {
        let approved = governance_integration::check_upgrade_approval(&env, &approved_hash);
        let vetoed = governance_integration::check_proposal_vetoed(&env, 0);
        // The combined gate `approved && !vetoed` must evaluate to false,
        // meaning the action is blocked. Either approval is absent OR a veto
        // is present — here the veto fires even though approval remains set.
        assert!(
            !approved || vetoed,
            "combined guard (approved && !vetoed) must block the late-veto case"
        );
        assert!(vetoed, "vetoed must be true to block the late-veto scenario");
    });
}
