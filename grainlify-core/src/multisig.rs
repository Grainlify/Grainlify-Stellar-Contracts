use soroban_sdk::{contracttype, symbol_short, Address, BytesN, Env, Vec};

/// =======================
/// Storage Keys
/// =======================
#[contracttype]
enum DataKey {
    Config,
    Proposal(u64),
    ProposalCounter,
}

/// =======================
/// Multisig Configuration
/// =======================
#[contracttype]
#[derive(Clone)]
pub struct MultiSigConfig {
    pub signers: Vec<Address>,
    pub threshold: u32,
}

/// =======================
/// Proposal Structure
/// =======================
#[contracttype]
#[derive(Clone, Eq, PartialEq)]
pub enum ProposalAction {
    Upgrade(BytesN<32>),
}

#[contracttype]
#[derive(Clone)]
pub struct Proposal {
    pub action: ProposalAction,
    pub approvals: Vec<Address>,
    pub executed: bool,
    pub signers: Vec<Address>,
    pub threshold: u32,
}

/// =======================
/// Errors
/// =======================
#[derive(Debug)]
pub enum MultiSigError {
    NotSigner,
    AlreadyApproved,
    ProposalNotFound,
    AlreadyExecuted,
    ThresholdNotMet,
    InvalidThreshold,
    ActionMismatch,
}

/// =======================
/// Public API
/// =======================
pub struct MultiSig;

impl MultiSig {
    /// Initialize multisig configuration
    pub fn init(env: &Env, signers: Vec<Address>, threshold: u32) {
        if env.storage().instance().has(&DataKey::Config) {
            panic!("multisig already initialized");
        }

        if threshold == 0 || threshold > signers.len() as u32 {
            panic!("{:?}", MultiSigError::InvalidThreshold);
        }

        let config = MultiSigConfig { signers, threshold };
        env.storage().instance().set(&DataKey::Config, &config);
        env.storage()
            .instance()
            .set(&DataKey::ProposalCounter, &0u64);
    }

    /// Create a new proposal bound to a concrete action payload.
    pub fn propose(env: &Env, proposer: Address, action: ProposalAction) -> u64 {
        proposer.require_auth();

        let config = Self::get_config(env);
        Self::assert_signer(&config, &proposer);

        let mut counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::ProposalCounter)
            .unwrap_or(0);

        counter += 1;

        let proposal = Proposal {
            action,
            approvals: Vec::new(env),
            executed: false,
            signers: config.signers,
            threshold: config.threshold,
        };

        env.storage()
            .instance()
            .set(&DataKey::Proposal(counter), &proposal);
        env.storage()
            .instance()
            .set(&DataKey::ProposalCounter, &counter);

        env.events().publish((symbol_short!("proposal"),), counter);

        #[allow(irrefutable_let_patterns)]
        if let ProposalAction::Upgrade(ref wasm_hash) = proposal.action {
            env.events().publish(
                (symbol_short!("upg_prop"),),
                crate::UpgradeProposed {
                    version: crate::EVENT_VERSION,
                    proposal_id: counter,
                    proposer: proposer.clone(),
                    wasm_hash: wasm_hash.clone(),
                },
            );
        }

        counter
    }

    /// Approve an existing proposal
    pub fn approve(env: &Env, proposal_id: u64, signer: Address) {
        signer.require_auth();

        let mut proposal = Self::get_proposal(env, proposal_id);
        Self::assert_proposal_signer(&proposal, &signer);

        if proposal.executed {
            panic!("{:?}", MultiSigError::AlreadyExecuted);
        }

        if proposal.approvals.contains(&signer) {
            panic!("{:?}", MultiSigError::AlreadyApproved);
        }

        proposal.approvals.push_back(signer.clone());

        env.storage()
            .instance()
            .set(&DataKey::Proposal(proposal_id), &proposal);

        env.events()
            .publish((symbol_short!("approved"),), (proposal_id, signer.clone()));

        #[allow(irrefutable_let_patterns)]
        if let ProposalAction::Upgrade(_) = proposal.action {
            env.events().publish(
                (symbol_short!("upg_appr"),),
                crate::UpgradeApproved {
                    version: crate::EVENT_VERSION,
                    proposal_id,
                    signer: signer.clone(),
                    approval_count: proposal.approvals.len() as u32,
                },
            );
        }
    }

    /// Check if proposal is executable
    pub fn can_execute(env: &Env, proposal_id: u64) -> bool {
        let proposal = Self::get_proposal(env, proposal_id);

        !proposal.executed && proposal.approvals.len() >= proposal.threshold
    }

    /// Atomically execute a proposal's bound action and mark it executed.
    ///
    /// The action closure runs only after threshold and payload checks pass.
    /// If the closure fails, the caller's transaction fails before the proposal
    /// can be marked executed, preventing approval/effect decoupling.
    pub fn execute<F>(env: &Env, proposal_id: u64, expected_action: ProposalAction, action: F)
    where
        F: FnOnce(),
    {
        let proposal = Self::get_proposal(env, proposal_id);

        if proposal.executed {
            panic!("{:?}", MultiSigError::AlreadyExecuted);
        }

        if proposal.action != expected_action {
            panic!("{:?}", MultiSigError::ActionMismatch);
        }

        if !Self::can_execute(env, proposal_id) {
            panic!("{:?}", MultiSigError::ThresholdNotMet);
        }

        action();
        Self::mark_executed(env, proposal_id);
    }

    pub fn get_action(env: &Env, proposal_id: u64) -> ProposalAction {
        Self::get_proposal(env, proposal_id).action
    }

    fn mark_executed(env: &Env, proposal_id: u64) {
        let mut proposal = Self::get_proposal(env, proposal_id);

        if proposal.executed {
            panic!("{:?}", MultiSigError::AlreadyExecuted);
        }

        if !Self::can_execute(env, proposal_id) {
            panic!("{:?}", MultiSigError::ThresholdNotMet);
        }

        proposal.executed = true;

        env.storage()
            .instance()
            .set(&DataKey::Proposal(proposal_id), &proposal);

        env.events()
            .publish((symbol_short!("executed"),), proposal_id);
    }

    /// =======================
    /// Internal Helpers
    /// =======================

    fn get_config(env: &Env) -> MultiSigConfig {
        env.storage()
            .instance()
            .get(&DataKey::Config)
            .expect("multisig not initialized")
    }

    fn get_proposal(env: &Env, proposal_id: u64) -> Proposal {
        env.storage()
            .instance()
            .get(&DataKey::Proposal(proposal_id))
            .unwrap_or_else(|| panic!("{:?}", MultiSigError::ProposalNotFound))
    }

    fn assert_signer(config: &MultiSigConfig, signer: &Address) {
        if !config.signers.contains(signer) {
            panic!("{:?}", MultiSigError::NotSigner);
        }
    }

    fn assert_proposal_signer(proposal: &Proposal, signer: &Address) {
        if !proposal.signers.contains(signer) {
            panic!("{:?}", MultiSigError::NotSigner);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::GrainlifyContract;
    use soroban_sdk::{testutils::Address as _, Env};

    struct Setup {
        env: Env,
        contract_id: Address,
        signer_a: Address,
        signer_b: Address,
        signer_c: Address,
    }

    fn setup() -> Setup {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let signer_a = Address::generate(&env);
        let signer_b = Address::generate(&env);
        let signer_c = Address::generate(&env);

        Setup {
            env,
            contract_id,
            signer_a,
            signer_b,
            signer_c,
        }
    }

    fn signers(env: &Env, signer_a: &Address, signer_b: &Address) -> Vec<Address> {
        let mut signers = Vec::new(env);
        signers.push_back(signer_a.clone());
        signers.push_back(signer_b.clone());
        signers
    }

    fn hash(env: &Env, byte: u8) -> BytesN<32> {
        BytesN::from_array(env, &[byte; 32])
    }

    #[test]
    fn execute_runs_bound_action_and_marks_proposal_once() {
        let setup = setup();
        let action = ProposalAction::Upgrade(hash(&setup.env, 7));

        let proposal_id = setup.env.as_contract(&setup.contract_id, || {
            MultiSig::init(
                &setup.env,
                signers(&setup.env, &setup.signer_a, &setup.signer_b),
                2,
            );

            MultiSig::propose(&setup.env, setup.signer_a.clone(), action.clone())
        });

        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::approve(&setup.env, proposal_id, setup.signer_a.clone());
            assert!(!MultiSig::can_execute(&setup.env, proposal_id));
        });

        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::approve(&setup.env, proposal_id, setup.signer_b.clone());
            assert!(MultiSig::can_execute(&setup.env, proposal_id));
        });

        let did_run = setup.env.as_contract(&setup.contract_id, || {
            let mut did_run = false;
            MultiSig::execute(&setup.env, proposal_id, action.clone(), || {
                did_run = true;
            });
            did_run
        });

        setup.env.as_contract(&setup.contract_id, || {
            let proposal = MultiSig::get_proposal(&setup.env, proposal_id);
            assert!(did_run);
            assert!(proposal.executed);
            assert!(!MultiSig::can_execute(&setup.env, proposal_id));
        });
    }

    #[test]
    #[should_panic(expected = "ThresholdNotMet")]
    fn execute_rejects_below_threshold() {
        let setup = setup();
        let action = ProposalAction::Upgrade(hash(&setup.env, 8));

        let proposal_id = setup.env.as_contract(&setup.contract_id, || {
            MultiSig::init(
                &setup.env,
                signers(&setup.env, &setup.signer_a, &setup.signer_b),
                2,
            );

            MultiSig::propose(&setup.env, setup.signer_a.clone(), action.clone())
        });

        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::approve(&setup.env, proposal_id, setup.signer_a.clone());
        });

        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::execute(&setup.env, proposal_id, action, || {});
        });
    }

    #[test]
    #[should_panic(expected = "ActionMismatch")]
    fn execute_rejects_mismatched_payload() {
        let setup = setup();
        let stored_action = ProposalAction::Upgrade(hash(&setup.env, 9));
        let wrong_action = ProposalAction::Upgrade(hash(&setup.env, 10));

        let proposal_id = setup.env.as_contract(&setup.contract_id, || {
            MultiSig::init(
                &setup.env,
                signers(&setup.env, &setup.signer_a, &setup.signer_b),
                2,
            );

            MultiSig::propose(&setup.env, setup.signer_a.clone(), stored_action)
        });

        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::approve(&setup.env, proposal_id, setup.signer_a.clone());
        });

        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::approve(&setup.env, proposal_id, setup.signer_b.clone());
        });

        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::execute(&setup.env, proposal_id, wrong_action, || {});
        });
    }

    #[test]
    #[should_panic(expected = "AlreadyExecuted")]
    fn second_execute_is_rejected() {
        let setup = setup();
        let action = ProposalAction::Upgrade(hash(&setup.env, 13));

        let proposal_id = setup.env.as_contract(&setup.contract_id, || {
            MultiSig::init(
                &setup.env,
                signers(&setup.env, &setup.signer_a, &setup.signer_b),
                2,
            );

            MultiSig::propose(&setup.env, setup.signer_a.clone(), action.clone())
        });

        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::approve(&setup.env, proposal_id, setup.signer_a.clone());
        });

        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::approve(&setup.env, proposal_id, setup.signer_b.clone());
        });

        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::execute(&setup.env, proposal_id, action.clone(), || {});
        });

        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::execute(&setup.env, proposal_id, action, || {});
        });
    }

    #[test]
    #[should_panic(expected = "AlreadyExecuted")]
    fn approve_after_execute_is_rejected() {
        let setup = setup();
        let action = ProposalAction::Upgrade(hash(&setup.env, 11));

        let proposal_id = setup.env.as_contract(&setup.contract_id, || {
            MultiSig::init(
                &setup.env,
                signers(&setup.env, &setup.signer_a, &setup.signer_b),
                2,
            );

            MultiSig::propose(&setup.env, setup.signer_a.clone(), action.clone())
        });

        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::approve(&setup.env, proposal_id, setup.signer_a.clone());
        });

        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::approve(&setup.env, proposal_id, setup.signer_b.clone());
        });

        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::execute(&setup.env, proposal_id, action, || {});
        });

        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::approve(&setup.env, proposal_id, setup.signer_a.clone());
        });
    }

    #[test]
    fn signer_and_threshold_snapshot_prevents_retroactive_validation() {
        let setup = setup();
        let action = ProposalAction::Upgrade(hash(&setup.env, 12));

        let proposal_id = setup.env.as_contract(&setup.contract_id, || {
            MultiSig::init(
                &setup.env,
                signers(&setup.env, &setup.signer_a, &setup.signer_b),
                2,
            );

            MultiSig::propose(&setup.env, setup.signer_a.clone(), action)
        });

        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::approve(&setup.env, proposal_id, setup.signer_a.clone());
        });

        setup.env.as_contract(&setup.contract_id, || {
            let mut changed_signers = Vec::new(&setup.env);
            changed_signers.push_back(setup.signer_a.clone());
            changed_signers.push_back(setup.signer_c.clone());
            setup.env.storage().instance().set(
                &DataKey::Config,
                &MultiSigConfig {
                    signers: changed_signers,
                    threshold: 1,
                },
            );

            assert!(!MultiSig::can_execute(&setup.env, proposal_id));

            let proposal = MultiSig::get_proposal(&setup.env, proposal_id);
            assert_eq!(proposal.threshold, 2);
            assert!(proposal.signers.contains(&setup.signer_b));
            assert!(!proposal.signers.contains(&setup.signer_c));
        });
    }
}
