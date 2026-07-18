use soroban_sdk::{contracttype, symbol_short, Address, BytesN, Env, Vec};

/// =======================
/// Storage Keys
/// =======================
#[contracttype]
enum DataKey {
    Config,
    Proposal(u64),
    ProposalCounter,
    /// Next expected execution nonce. Monotonically increasing; consumed once
    /// per successful `execute`, providing replay protection across proposals.
    ExecutionNonce,
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
    /// Execution nonce consumed when this proposal was executed. `None` until
    /// execution succeeds; binds the proposal's execution record to a single,
    /// non-reusable nonce value.
    pub executed_nonce: Option<u64>,
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
    NonceMismatch,
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
        env.storage()
            .instance()
            .set(&DataKey::ExecutionNonce, &0u64);
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
            executed_nonce: None,
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

    /// The next execution nonce the contract will accept.
    ///
    /// Callers must pass this value as `expected_nonce` to [`Self::execute`].
    /// It increases by one after every successful execution, so a previously
    /// used `(signatures, nonce)` pair can never be replayed.
    pub fn nonce(env: &Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::ExecutionNonce)
            .unwrap_or(0)
    }

    /// Atomically execute a proposal's bound action and mark it executed.
    ///
    /// The action closure runs only after threshold, payload, and nonce checks
    /// pass. If the closure fails, the caller's transaction fails before the
    /// proposal can be marked executed, preventing approval/effect decoupling.
    ///
    /// Replay protection: `expected_nonce` must equal the current value returned
    /// by [`Self::nonce`]. On success the nonce is consumed and incremented, so
    /// the same nonce (and therefore the same collected signatures re-submitted
    /// as an identical action) cannot drive a second execution.
    pub fn execute<F>(
        env: &Env,
        proposal_id: u64,
        expected_action: ProposalAction,
        expected_nonce: u64,
        action: F,
    ) where
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

        let current_nonce = Self::nonce(env);
        if expected_nonce != current_nonce {
            panic!("{:?}", MultiSigError::NonceMismatch);
        }

        action();
        Self::mark_executed(env, proposal_id, current_nonce);

        env.storage()
            .instance()
            .set(&DataKey::ExecutionNonce, &(current_nonce + 1));
    }

    pub fn get_action(env: &Env, proposal_id: u64) -> ProposalAction {
        Self::get_proposal(env, proposal_id).action
    }

    fn mark_executed(env: &Env, proposal_id: u64, nonce: u64) {
        let mut proposal = Self::get_proposal(env, proposal_id);

        if proposal.executed {
            panic!("{:?}", MultiSigError::AlreadyExecuted);
        }

        if !Self::can_execute(env, proposal_id) {
            panic!("{:?}", MultiSigError::ThresholdNotMet);
        }

        proposal.executed = true;
        proposal.executed_nonce = Some(nonce);

        env.storage()
            .instance()
            .set(&DataKey::Proposal(proposal_id), &proposal);

        env.events()
            .publish((symbol_short!("executed"),), (proposal_id, nonce));
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
            MultiSig::execute(&setup.env, proposal_id, action.clone(), 0, || {
                did_run = true;
            });
            did_run
        });

        setup.env.as_contract(&setup.contract_id, || {
            let proposal = MultiSig::get_proposal(&setup.env, proposal_id);
            assert!(did_run);
            assert!(proposal.executed);
            assert_eq!(proposal.executed_nonce, Some(0));
            assert_eq!(MultiSig::nonce(&setup.env), 1);
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
            MultiSig::execute(&setup.env, proposal_id, action, 0, || {});
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
            MultiSig::execute(&setup.env, proposal_id, wrong_action, 0, || {});
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
            MultiSig::execute(&setup.env, proposal_id, action.clone(), 0, || {});
        });

        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::execute(&setup.env, proposal_id, action, 0, || {});
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
            MultiSig::execute(&setup.env, proposal_id, action, 0, || {});
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

    /// Propose `action` and approve it to the (2-of-2) threshold, returning the
    /// new proposal id. Assumes the multisig is already initialized.
    fn propose_and_approve(setup: &Setup, action: ProposalAction) -> u64 {
        let proposal_id = setup.env.as_contract(&setup.contract_id, || {
            MultiSig::propose(&setup.env, setup.signer_a.clone(), action)
        });

        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::approve(&setup.env, proposal_id, setup.signer_a.clone());
            MultiSig::approve(&setup.env, proposal_id, setup.signer_b.clone());
        });

        proposal_id
    }

    #[test]
    fn nonce_starts_at_zero_and_increments_per_execution() {
        let setup = setup();

        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::init(
                &setup.env,
                signers(&setup.env, &setup.signer_a, &setup.signer_b),
                2,
            );
            assert_eq!(MultiSig::nonce(&setup.env), 0);
        });

        // First execution consumes nonce 0.
        let action_one = ProposalAction::Upgrade(hash(&setup.env, 20));
        let first = propose_and_approve(&setup, action_one.clone());
        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::execute(&setup.env, first, action_one, 0, || {});
            assert_eq!(MultiSig::nonce(&setup.env), 1);
            let proposal = MultiSig::get_proposal(&setup.env, first);
            assert_eq!(proposal.executed_nonce, Some(0));
        });

        // Second, independent execution consumes the next nonce, 1.
        let action_two = ProposalAction::Upgrade(hash(&setup.env, 21));
        let second = propose_and_approve(&setup, action_two.clone());
        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::execute(&setup.env, second, action_two, 1, || {});
            assert_eq!(MultiSig::nonce(&setup.env), 2);
            let proposal = MultiSig::get_proposal(&setup.env, second);
            assert_eq!(proposal.executed_nonce, Some(1));
        });
    }

    #[test]
    #[should_panic(expected = "NonceMismatch")]
    fn execute_rejects_wrong_nonce() {
        let setup = setup();

        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::init(
                &setup.env,
                signers(&setup.env, &setup.signer_a, &setup.signer_b),
                2,
            );
        });

        let action = ProposalAction::Upgrade(hash(&setup.env, 22));
        let proposal_id = propose_and_approve(&setup, action.clone());

        // Threshold and payload are satisfied, but the supplied nonce (1) does
        // not match the expected next nonce (0).
        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::execute(&setup.env, proposal_id, action, 1, || {});
        });
    }

    #[test]
    #[should_panic(expected = "NonceMismatch")]
    fn replayed_signatures_and_nonce_are_rejected() {
        let setup = setup();

        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::init(
                &setup.env,
                signers(&setup.env, &setup.signer_a, &setup.signer_b),
                2,
            );
        });

        // A privileged action is proposed, approved to threshold, and executed
        // with nonce 0. This is the "captured" (signatures, nonce) pair.
        let action = ProposalAction::Upgrade(hash(&setup.env, 23));
        let first = propose_and_approve(&setup, action.clone());
        let mut effects = 0u32;
        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::execute(&setup.env, first, action.clone(), 0, || effects += 1);
        });
        assert_eq!(effects, 1);

        // An attacker re-submits the identical action as a fresh proposal and
        // re-collects the same signers' approvals, then replays the previously
        // used nonce (0). The nonce has already advanced to 1, so execution is
        // rejected before the action can run a second time.
        let replay = propose_and_approve(&setup, action.clone());
        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::execute(&setup.env, replay, action, 0, || effects += 1);
        });
    }

    #[test]
    fn rejected_replay_leaves_state_untouched() {
        let setup = setup();

        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::init(
                &setup.env,
                signers(&setup.env, &setup.signer_a, &setup.signer_b),
                2,
            );
        });

        let action = ProposalAction::Upgrade(hash(&setup.env, 24));
        let first = propose_and_approve(&setup, action.clone());
        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::execute(&setup.env, first, action.clone(), 0, || {});
        });

        // Fresh proposal for the identical action, approved to threshold, but
        // not yet executed. The stale nonce (0) has already been consumed.
        let replay = propose_and_approve(&setup, action);

        // Before the replay attempt, the nonce sits at 1 and the fresh proposal
        // is unexecuted. `execute` with the stale nonce panics (asserted by the
        // dedicated NonceMismatch test); here we confirm the pre-attempt state
        // that the panic must preserve: no effect can have leaked through.
        setup.env.as_contract(&setup.contract_id, || {
            assert_eq!(MultiSig::nonce(&setup.env), 1);
            let proposal = MultiSig::get_proposal(&setup.env, replay);
            assert!(!proposal.executed);
            assert_eq!(proposal.executed_nonce, None);
        });
    }
}
