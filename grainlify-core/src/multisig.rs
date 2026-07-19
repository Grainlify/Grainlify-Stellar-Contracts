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
    /// Removing the signer would leave fewer signers than the configured
    /// threshold, making the multisig permanently un-executable.
    RemovalWouldBreakThreshold,
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

    /// Remove a signer from the multisig configuration.
    ///
    /// # Pre-condition: threshold viability guard
    /// The removal is rejected with [`MultiSigError::RemovalWouldBreakThreshold`]
    /// if `(current_signer_count - 1) < threshold`.  This prevents an admin
    /// from accidentally (or maliciously) leaving the multisig in a state where
    /// the required number of approvals can never be gathered, which would
    /// permanently lock any funds or actions protected by the multisig.
    ///
    /// # Errors
    /// - Panics with `NotSigner` if `signer_to_remove` is not in the current
    ///   signer set.
    /// - Panics with `RemovalWouldBreakThreshold` if the removal would leave
    ///   fewer signers than the configured threshold.
    pub fn remove_signer(env: &Env, caller: Address, signer_to_remove: Address) {
        caller.require_auth();

        let mut config = Self::get_config(env);

        // Verify the address to be removed is actually a current signer.
        if !config.signers.contains(&signer_to_remove) {
            panic!("{:?}", MultiSigError::NotSigner);
        }

        // Guard: removing this signer must not make the threshold unreachable.
        // After removal there will be (len - 1) signers; that count must still
        // be >= threshold.
        let remaining = config.signers.len() as u32 - 1;
        if remaining < config.threshold {
            panic!("{:?}", MultiSigError::RemovalWouldBreakThreshold);
        }

        // Rebuild the signer list without the removed address.
        let mut new_signers = Vec::new(env);
        for i in 0..config.signers.len() {
            let s = config.signers.get(i).unwrap();
            if s != signer_to_remove {
                new_signers.push_back(s);
            }
        }

        config.signers = new_signers;
        env.storage().instance().set(&DataKey::Config, &config);

        env.events().publish(
            (symbol_short!("rm_sgnr"),),
            signer_to_remove,
        );
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

    // =====================================================================
    // Threshold edge-case tests (issue #184)
    // =====================================================================

    /// Helper: build a Vec<Address> from a slice of &Address.
    fn addr_vec(env: &Env, addrs: &[&Address]) -> Vec<Address> {
        let mut v = Vec::new(env);
        for a in addrs {
            v.push_back((*a).clone());
        }
        v
    }

    // --- 0-threshold is explicitly invalid -----------------------------------

    /// Initialising with threshold=0 must be rejected with InvalidThreshold,
    /// regardless of the signer count.
    #[test]
    #[should_panic(expected = "InvalidThreshold")]
    fn zero_threshold_is_rejected() {
        let setup = setup();
        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::init(
                &setup.env,
                signers(&setup.env, &setup.signer_a, &setup.signer_b),
                0, // threshold == 0 → must panic
            );
        });
    }

    /// threshold > signers.len() is also an invalid configuration.
    #[test]
    #[should_panic(expected = "InvalidThreshold")]
    fn threshold_exceeding_signer_count_is_rejected() {
        let setup = setup();
        setup.env.as_contract(&setup.contract_id, || {
            // 2 signers, threshold = 3 → impossible to ever reach
            MultiSig::init(
                &setup.env,
                signers(&setup.env, &setup.signer_a, &setup.signer_b),
                3,
            );
        });
    }

    // --- 1-of-N: any single signer suffices ----------------------------------

    /// A 1-of-3 configuration must become executable as soon as ONE signer
    /// approves — the other two approvals are unnecessary.
    #[test]
    fn one_of_n_single_approval_suffices() {
        let setup = setup();
        let action = ProposalAction::Upgrade(hash(&setup.env, 20));

        let proposal_id = setup.env.as_contract(&setup.contract_id, || {
            let three_signers = addr_vec(
                &setup.env,
                &[&setup.signer_a, &setup.signer_b, &setup.signer_c],
            );
            MultiSig::init(&setup.env, three_signers, 1); // 1-of-3
            MultiSig::propose(&setup.env, setup.signer_a.clone(), action.clone())
        });

        // Before any approval: not executable.
        setup.env.as_contract(&setup.contract_id, || {
            assert!(!MultiSig::can_execute(&setup.env, proposal_id));
        });

        // After exactly ONE approval: must be executable.
        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::approve(&setup.env, proposal_id, setup.signer_b.clone());
            assert!(
                MultiSig::can_execute(&setup.env, proposal_id),
                "1-of-3: should be executable after a single approval"
            );
        });

        // Execute succeeds and the action closure runs.
        let did_run = setup.env.as_contract(&setup.contract_id, || {
            let mut ran = false;
            MultiSig::execute(&setup.env, proposal_id, action.clone(), || {
                ran = true;
            });
            ran
        });

        assert!(did_run, "action closure must have been called");
    }

    // --- N-of-N: all signers must approve (unanimous) -----------------------

    /// A 3-of-3 configuration must NOT be executable until every signer has
    /// approved.  After the third approval it becomes executable.
    #[test]
    fn n_of_n_requires_all_signers() {
        let setup = setup();
        let action = ProposalAction::Upgrade(hash(&setup.env, 21));

        let proposal_id = setup.env.as_contract(&setup.contract_id, || {
            let three_signers = addr_vec(
                &setup.env,
                &[&setup.signer_a, &setup.signer_b, &setup.signer_c],
            );
            MultiSig::init(&setup.env, three_signers, 3); // 3-of-3
            MultiSig::propose(&setup.env, setup.signer_a.clone(), action.clone())
        });

        // After 1st approval: not executable.
        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::approve(&setup.env, proposal_id, setup.signer_a.clone());
            assert!(!MultiSig::can_execute(&setup.env, proposal_id));
        });

        // After 2nd approval: still not executable.
        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::approve(&setup.env, proposal_id, setup.signer_b.clone());
            assert!(!MultiSig::can_execute(&setup.env, proposal_id));
        });

        // After 3rd (final) approval: now executable.
        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::approve(&setup.env, proposal_id, setup.signer_c.clone());
            assert!(
                MultiSig::can_execute(&setup.env, proposal_id),
                "3-of-3: should be executable only after all signers approve"
            );
        });

        // Execute succeeds.
        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::execute(&setup.env, proposal_id, action.clone(), || {});
            let proposal = MultiSig::get_proposal(&setup.env, proposal_id);
            assert!(proposal.executed);
        });
    }

    // --- Exactly-at-threshold: must pass ------------------------------------

    /// With a 2-of-3 config, reaching exactly 2 approvals (= threshold) must
    /// make the proposal executable.  This guards against an off-by-one where
    /// the implementation uses `>` instead of `>=`.
    #[test]
    fn exactly_at_threshold_is_executable() {
        let setup = setup();
        let action = ProposalAction::Upgrade(hash(&setup.env, 22));

        let proposal_id = setup.env.as_contract(&setup.contract_id, || {
            let three_signers = addr_vec(
                &setup.env,
                &[&setup.signer_a, &setup.signer_b, &setup.signer_c],
            );
            MultiSig::init(&setup.env, three_signers, 2); // 2-of-3
            MultiSig::propose(&setup.env, setup.signer_a.clone(), action.clone())
        });

        // 1 approval → below threshold.
        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::approve(&setup.env, proposal_id, setup.signer_a.clone());
            assert!(!MultiSig::can_execute(&setup.env, proposal_id));
        });

        // 2 approvals → exactly at threshold → must be executable.
        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::approve(&setup.env, proposal_id, setup.signer_b.clone());
            assert!(
                MultiSig::can_execute(&setup.env, proposal_id),
                "exactly-at-threshold (2-of-3): must be executable"
            );
        });

        // Execute succeeds without needing the third signer.
        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::execute(&setup.env, proposal_id, action.clone(), || {});
        });
    }

    // --- One-short-of-threshold: must fail ----------------------------------

    /// With a 3-of-3 config, having only 2 approvals (one short of the
    /// threshold) must be rejected when execute is called.  This guards against
    /// an off-by-one where `>=` is accidentally replaced with `>`.
    #[test]
    #[should_panic(expected = "ThresholdNotMet")]
    fn one_short_of_threshold_is_not_executable() {
        let setup = setup();
        let action = ProposalAction::Upgrade(hash(&setup.env, 23));

        let proposal_id = setup.env.as_contract(&setup.contract_id, || {
            let three_signers = addr_vec(
                &setup.env,
                &[&setup.signer_a, &setup.signer_b, &setup.signer_c],
            );
            MultiSig::init(&setup.env, three_signers, 3); // 3-of-3
            MultiSig::propose(&setup.env, setup.signer_a.clone(), action.clone())
        });

        // Only 2 out of 3 required approvals.
        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::approve(&setup.env, proposal_id, setup.signer_a.clone());
        });

        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::approve(&setup.env, proposal_id, setup.signer_b.clone());
        });

        // Attempting to execute with one approval short must panic.
        setup.env.as_contract(&setup.contract_id, || {
            MultiSig::execute(&setup.env, proposal_id, action, || {});
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

    // =====================================================================
    // remove_signer: threshold-viability guard tests  (issue #183)
    // =====================================================================

    /// Normal removal: 3 signers, threshold=2, remove one → 2 signers remain
    /// which is exactly the threshold → still viable, must succeed.
    ///
    /// This test also doubles as the *boundary* case (remaining == threshold).
    #[test]
    fn remove_signer_normal_above_threshold_succeeds() {
        let setup = setup();
        setup.env.as_contract(&setup.contract_id, || {
            // 3 signers, threshold = 2 (well above minimum)
            let three_signers = addr_vec(
                &setup.env,
                &[&setup.signer_a, &setup.signer_b, &setup.signer_c],
            );
            MultiSig::init(&setup.env, three_signers, 2);

            // Remove signer_c → 2 remain, threshold still = 2: viable.
            MultiSig::remove_signer(
                &setup.env,
                setup.signer_a.clone(), // caller
                setup.signer_c.clone(), // target
            );

            let config = MultiSig::get_config(&setup.env);
            assert_eq!(
                config.signers.len(),
                2,
                "signer count should drop to 2 after removal"
            );
            assert!(
                !config.signers.contains(&setup.signer_c),
                "removed signer must not appear in the config"
            );
            assert!(
                config.signers.contains(&setup.signer_a),
                "remaining signers must still be present"
            );
            assert!(
                config.signers.contains(&setup.signer_b),
                "remaining signers must still be present"
            );
        });
    }

    /// Boundary case: removing exactly down to the threshold (remaining == threshold).
    /// With 3 signers and threshold=2, removing one signer leaves exactly 2 —
    /// matching the threshold.  This must be *allowed*.
    #[test]
    fn remove_signer_boundary_exactly_at_threshold_is_allowed() {
        let setup = setup();
        setup.env.as_contract(&setup.contract_id, || {
            // 3 signers, threshold = 2
            let three_signers = addr_vec(
                &setup.env,
                &[&setup.signer_a, &setup.signer_b, &setup.signer_c],
            );
            MultiSig::init(&setup.env, three_signers, 2);

            // Removing signer_c leaves exactly 2 = threshold → must succeed.
            MultiSig::remove_signer(
                &setup.env,
                setup.signer_a.clone(),
                setup.signer_c.clone(),
            );

            let config = MultiSig::get_config(&setup.env);
            assert_eq!(
                config.signers.len() as u32,
                config.threshold,
                "after boundary removal, signer count must equal threshold exactly"
            );
        });
    }

    /// Rejected removal: 2 signers, threshold=2 → removing either signer
    /// would leave only 1 signer < threshold=2.
    /// The call must panic with `RemovalWouldBreakThreshold`.
    #[test]
    #[should_panic(expected = "RemovalWouldBreakThreshold")]
    fn remove_signer_below_threshold_is_rejected() {
        let setup = setup();
        setup.env.as_contract(&setup.contract_id, || {
            // 2 signers, threshold = 2 (unanimous 2-of-2)
            MultiSig::init(
                &setup.env,
                signers(&setup.env, &setup.signer_a, &setup.signer_b),
                2,
            );

            // Removing signer_b would leave 1 signer < threshold=2 → must panic.
            MultiSig::remove_signer(
                &setup.env,
                setup.signer_a.clone(),
                setup.signer_b.clone(),
            );
        });
    }
}
