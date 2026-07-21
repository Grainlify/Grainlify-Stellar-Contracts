#![cfg(test)]

use crate::{
    governance_integration, BountyEscrowContract, BountyEscrowContractClient, Error, EscrowStatus,
    ReleaseFundsItem,
};
use grainlify_core::{GrainlifyContract, GrainlifyContractClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, vec, Address, BytesN, Env,
};

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

mod mock_governance_with_proposal_state {
    use crate::governance_integration::GovernanceError;
    use soroban_sdk::{contract, contractimpl, symbol_short, BytesN, Env, Map, Symbol};

    const PROPOSAL_STATES: Symbol = symbol_short!("PR_STATE");
    const STATUS_PENDING: u32 = 1;
    const STATUS_APPROVED: u32 = 2;
    const STATUS_REJECTED: u32 = 3;
    const STATUS_EXECUTED: u32 = 4;

    #[contract]
    pub struct MockGovernanceWithProposalState;

    #[contractimpl]
    impl MockGovernanceWithProposalState {
        pub fn get_ver(_env: Env) -> u32 {
            2
        }

        pub fn get_version_numeric_encoded(_env: Env) -> u32 {
            20_000
        }

        pub fn is_upg_ok(_env: Env, _wasm_hash: BytesN<32>) -> bool {
            false
        }

        pub fn set_proposal_status(env: Env, proposal_id: u32, status: u32) {
            let mut statuses: Map<u32, u32> = env
                .storage()
                .instance()
                .get(&PROPOSAL_STATES)
                .unwrap_or(Map::new(&env));
            statuses.set(proposal_id, status);
            env.storage().instance().set(&PROPOSAL_STATES, &statuses);
        }

        pub fn get_proposal_status(env: Env, proposal_id: u32) -> u32 {
            let statuses: Map<u32, u32> = env
                .storage()
                .instance()
                .get(&PROPOSAL_STATES)
                .unwrap_or(Map::new(&env));
            statuses.get(proposal_id).unwrap_or(0)
        }

        pub fn execute_proposal(env: Env, proposal_id: u32) -> Result<(), GovernanceError> {
            let mut statuses: Map<u32, u32> = env
                .storage()
                .instance()
                .get(&PROPOSAL_STATES)
                .ok_or(GovernanceError::ProposalsNotFound)?;
            let status = statuses
                .get(proposal_id)
                .ok_or(GovernanceError::ProposalNotFound)?;

            if status != STATUS_APPROVED {
                return Err(GovernanceError::ProposalNotApproved);
            }

            statuses.set(proposal_id, STATUS_EXECUTED);
            env.storage().instance().set(&PROPOSAL_STATES, &statuses);
            Ok(())
        }

        pub fn pending_status(_env: Env) -> u32 {
            STATUS_PENDING
        }

        pub fn approved_status(_env: Env) -> u32 {
            STATUS_APPROVED
        }

        pub fn rejected_status(_env: Env) -> u32 {
            STATUS_REJECTED
        }

        pub fn executed_status(_env: Env) -> u32 {
            STATUS_EXECUTED
        }
    }
}

fn create_token_contract<'a>(
    env: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract_address = env.register_stellar_asset_contract(admin.clone());
    (
        token::Client::new(env, &contract_address),
        token::StellarAssetClient::new(env, &contract_address),
    )
}

struct ValueTransferSetup<'a> {
    env: Env,
    depositor: Address,
    contributor: Address,
    escrow: BountyEscrowContractClient<'a>,
}

impl<'a> ValueTransferSetup<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);
        let (_token, token_admin) = create_token_contract(&env, &admin);

        let contract_id = env.register_contract(None, BountyEscrowContract);
        let escrow = BountyEscrowContractClient::new(&env, &contract_id);
        escrow.init(&admin, &_token.address);
        token_admin.mint(&depositor, &1_000_000);

        Self {
            env,
            depositor,
            contributor,
            escrow,
        }
    }

    fn configure_mock_governance(&self, min_version: u32) {
        let gov_contract_id = self
            .env
            .register_contract(None, mock_governance::MockGovernanceContract);
        self.escrow.set_governance_contract(&gov_contract_id);
        self.escrow.set_min_governance_version(&min_version);
    }

    fn lock_bounty(&self, bounty_id: u64, amount: i128, deadline: u64) {
        self.escrow
            .lock_funds(&self.depositor, &bounty_id, &amount, &deadline);
    }
}

fn configure_stateful_governance<'a>(
    env: &'a Env,
    escrow: &BountyEscrowContractClient<'a>,
) -> mock_governance_with_proposal_state::MockGovernanceWithProposalStateClient<'a> {
    let gov_contract_id = env.register_contract(
        None,
        mock_governance_with_proposal_state::MockGovernanceWithProposalState,
    );
    escrow.set_governance_contract(&gov_contract_id);
    escrow.set_min_governance_version(&2);
    mock_governance_with_proposal_state::MockGovernanceWithProposalStateClient::new(
        env,
        &gov_contract_id,
    )
}

#[test]
fn test_governance_proposal_execution_rejects_pending_proposal() {
    let setup = ValueTransferSetup::new();
    let governance = configure_stateful_governance(&setup.env, &setup.escrow);

    governance.set_proposal_status(&1, &governance.pending_status());

    assert_eq!(
        setup.escrow.try_execute_governance_proposal(&1),
        Err(Ok(Error::GovernanceProposalNotExecutable))
    );
    assert_eq!(
        governance.get_proposal_status(&1),
        governance.pending_status()
    );
}

#[test]
fn test_governance_proposal_execution_rejects_rejected_proposal() {
    let setup = ValueTransferSetup::new();
    let governance = configure_stateful_governance(&setup.env, &setup.escrow);

    governance.set_proposal_status(&2, &governance.rejected_status());

    assert_eq!(
        setup.escrow.try_execute_governance_proposal(&2),
        Err(Ok(Error::GovernanceProposalNotExecutable))
    );
    assert_eq!(
        governance.get_proposal_status(&2),
        governance.rejected_status()
    );
}

#[test]
fn test_governance_proposal_execution_rejects_already_executed_proposal() {
    let setup = ValueTransferSetup::new();
    let governance = configure_stateful_governance(&setup.env, &setup.escrow);

    governance.set_proposal_status(&3, &governance.executed_status());

    assert_eq!(
        setup.escrow.try_execute_governance_proposal(&3),
        Err(Ok(Error::GovernanceProposalNotExecutable))
    );
    assert_eq!(
        governance.get_proposal_status(&3),
        governance.executed_status()
    );
}

#[test]
fn test_governance_proposal_execution_consumes_approved_proposal_once() {
    let setup = ValueTransferSetup::new();
    let governance = configure_stateful_governance(&setup.env, &setup.escrow);

    governance.set_proposal_status(&4, &governance.approved_status());

    setup.escrow.execute_governance_proposal(&4);
    assert_eq!(
        governance.get_proposal_status(&4),
        governance.executed_status()
    );

    assert_eq!(
        setup.escrow.try_execute_governance_proposal(&4),
        Err(Ok(Error::GovernanceProposalNotExecutable))
    );
}

#[test]
fn test_governance_version_too_low_blocks_value_transfers() {
    let setup = ValueTransferSetup::new();
    setup.configure_mock_governance(3);

    let now = setup.env.ledger().timestamp();
    let active_deadline = now + 100;
    let expired_deadline = now + 10;

    setup.lock_bounty(1, 100, active_deadline);
    setup.lock_bounty(2, 100, active_deadline);
    setup.lock_bounty(3, 100, expired_deadline);
    setup.lock_bounty(4, 100, expired_deadline);
    setup.lock_bounty(5, 100, active_deadline);
    setup.lock_bounty(6, 100, active_deadline);

    assert_eq!(
        setup.escrow.try_release_funds(&1, &setup.contributor),
        Err(Ok(Error::GovernanceVersionTooLow))
    );
    assert_eq!(
        setup
            .escrow
            .try_partial_release(&2, &setup.contributor, &25),
        Err(Ok(Error::GovernanceVersionTooLow))
    );

    setup.env.ledger().set_timestamp(expired_deadline + 1);
    assert_eq!(
        setup.escrow.try_refund(&3),
        Err(Ok(Error::GovernanceVersionTooLow))
    );

    let expired_ids = vec![&setup.env, 4_u64];
    assert_eq!(
        setup.escrow.try_sweep_expired_refunds(&expired_ids),
        Err(Ok(Error::GovernanceVersionTooLow))
    );

    let release_items = vec![
        &setup.env,
        ReleaseFundsItem {
            bounty_id: 5,
            contributor: setup.contributor.clone(),
        },
        ReleaseFundsItem {
            bounty_id: 6,
            contributor: setup.contributor.clone(),
        },
    ];
    assert_eq!(
        setup.escrow.try_batch_release_funds(&release_items),
        Err(Ok(Error::GovernanceVersionTooLow))
    );

    for bounty_id in 1_u64..=6 {
        let escrow = setup.escrow.get_escrow_info(&bounty_id);
        assert_eq!(escrow.status, EscrowStatus::Locked);
        assert_eq!(escrow.remaining_amount, 100);
    }
}

#[test]
fn test_governance_version_met_allows_value_transfers() {
    let setup = ValueTransferSetup::new();
    setup.configure_mock_governance(2);

    let now = setup.env.ledger().timestamp();
    let active_deadline = now + 100;
    let expired_deadline = now + 10;

    setup.lock_bounty(11, 100, active_deadline);
    setup.lock_bounty(12, 100, active_deadline);
    setup.lock_bounty(13, 100, expired_deadline);
    setup.lock_bounty(14, 100, expired_deadline);
    setup.lock_bounty(15, 100, active_deadline);
    setup.lock_bounty(16, 100, active_deadline);

    setup.escrow.release_funds(&11, &setup.contributor);
    setup.escrow.partial_release(&12, &setup.contributor, &25);

    setup.env.ledger().set_timestamp(expired_deadline + 1);
    setup.escrow.refund(&13);
    let expired_ids = vec![&setup.env, 14_u64];
    assert_eq!(setup.escrow.sweep_expired_refunds(&expired_ids), 1);

    let release_items = vec![
        &setup.env,
        ReleaseFundsItem {
            bounty_id: 15,
            contributor: setup.contributor.clone(),
        },
        ReleaseFundsItem {
            bounty_id: 16,
            contributor: setup.contributor.clone(),
        },
    ];
    assert_eq!(setup.escrow.batch_release_funds(&release_items), 2);

    assert_eq!(
        setup.escrow.get_escrow_info(&11).status,
        EscrowStatus::Released
    );
    let partial = setup.escrow.get_escrow_info(&12);
    assert_eq!(partial.status, EscrowStatus::Locked);
    assert_eq!(partial.remaining_amount, 75);
    assert_eq!(
        setup.escrow.get_escrow_info(&13).status,
        EscrowStatus::Refunded
    );
    assert_eq!(
        setup.escrow.get_escrow_info(&14).status,
        EscrowStatus::Refunded
    );
    assert_eq!(
        setup.escrow.get_escrow_info(&15).status,
        EscrowStatus::Released
    );
    assert_eq!(
        setup.escrow.get_escrow_info(&16).status,
        EscrowStatus::Released
    );
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
