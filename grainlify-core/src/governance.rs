use soroban_sdk::{contracttype, symbol_short, token, Address, BytesN, Env, Map, Symbol};

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum ProposalStatus {
    Pending,
    Active,
    Approved,
    Rejected,
    Executed,
    Expired,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum VoteType {
    For,
    Against,
    Abstain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum VotingScheme {
    OnePersonOneVote,
    TokenWeighted,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct Proposal {
    pub id: u32,
    pub proposer: Address,
    pub new_wasm_hash: BytesN<32>,
    pub description: Symbol,
    pub created_at: u64,
    pub voting_start: u64,
    pub voting_end: u64,
    pub execution_delay: u64,
    pub status: ProposalStatus,
    pub votes_for: i128,
    pub votes_against: i128,
    pub votes_abstain: i128,
    pub total_votes: u32,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct GovernanceConfig {
    pub voting_period: u64,
    pub execution_delay: u64,
    /// Quorum in basis points against the scheme-specific total voting power.
    pub quorum_percentage: u32,
    /// Approval threshold in basis points against non-abstaining votes.
    pub approval_threshold: u32,
    /// Minimum governance-token balance required to create a proposal.
    pub min_proposal_stake: i128,
    pub voting_scheme: VotingScheme,
    /// Soroban token used for token-weighted votes and proposal stake checks.
    pub governance_token: Address,
    /// Total eligible voters for one-person-one-vote quorum calculations.
    pub one_person_total_voters: u32,
    /// Total token voting power for token-weighted quorum calculations.
    pub token_total_voting_power: i128,
    /// Optional ledger recorded by governance policy for snapshot/stake-lock semantics.
    pub snapshot_ledger: Option<u32>,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct Vote {
    pub voter: Address,
    pub proposal_id: u32,
    pub vote_type: VoteType,
    pub voting_power: i128,
    pub timestamp: u64,
}

pub const PROPOSALS: Symbol = symbol_short!("PROPOSALS");
pub const PROPOSAL_COUNT: Symbol = symbol_short!("PROP_CNT");
pub const VOTES: Symbol = symbol_short!("VOTES");
pub const GOVERNANCE_CONFIG: Symbol = symbol_short!("GOV_CFG");

#[soroban_sdk::contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
    InvalidThreshold = 2,
    ThresholdTooLow = 3,
    InsufficientStake = 4,
    ProposalsNotFound = 5,
    ProposalNotFound = 6,
    ProposalNotActive = 7,
    VotingNotStarted = 8,
    VotingEnded = 9,
    VotingStillActive = 10,
    AlreadyVoted = 11,
    ProposalNotApproved = 12,
    ExecutionDelayNotMet = 13,
    ProposalExpired = 14,
    ZeroVotingPower = 15,
    InvalidTotalVotingPower = 16,
}

#[cfg_attr(not(target_arch = "wasm32"), soroban_sdk::contract)]
pub struct GovernanceContract;

#[cfg_attr(not(target_arch = "wasm32"), soroban_sdk::contractimpl)]
impl GovernanceContract {
    pub fn init_governance(
        env: Env,
        admin: Address,
        config: GovernanceConfig,
    ) -> Result<(), Error> {
        admin.require_auth();
        if config.quorum_percentage > 10000 || config.approval_threshold > 10000 {
            return Err(Error::InvalidThreshold);
        }
        if config.approval_threshold < 5000 {
            return Err(Error::ThresholdTooLow);
        }
        if config.min_proposal_stake < 0 {
            return Err(Error::InsufficientStake);
        }
        if total_voting_power(&config) <= 0 {
            return Err(Error::InvalidTotalVotingPower);
        }

        env.storage().instance().set(&GOVERNANCE_CONFIG, &config);
        env.storage().instance().set(&PROPOSAL_COUNT, &0u32);
        Ok(())
    }

    pub fn create_proposal(
        env: Env,
        proposer: Address,
        new_wasm_hash: BytesN<32>,
        description: Symbol,
    ) -> Result<u32, Error> {
        proposer.require_auth();
        let config: GovernanceConfig = env
            .storage()
            .instance()
            .get(&GOVERNANCE_CONFIG)
            .ok_or(Error::NotInitialized)?;
        enforce_min_proposal_stake(&env, &config, &proposer)?;

        let proposal_id: u32 = env.storage().instance().get(&PROPOSAL_COUNT).unwrap_or(0);
        let current_time = env.ledger().timestamp();

        let proposal = Proposal {
            id: proposal_id,
            proposer: proposer.clone(),
            new_wasm_hash,
            description,
            created_at: current_time,
            voting_start: current_time,
            voting_end: current_time + config.voting_period,
            execution_delay: config.execution_delay,
            status: ProposalStatus::Active,
            votes_for: 0,
            votes_against: 0,
            votes_abstain: 0,
            total_votes: 0,
        };

        let mut proposals: Map<u32, Proposal> = env
            .storage()
            .instance()
            .get(&PROPOSALS)
            .unwrap_or(Map::new(&env));
        proposals.set(proposal_id, proposal);
        env.storage().instance().set(&PROPOSALS, &proposals);
        env.storage()
            .instance()
            .set(&PROPOSAL_COUNT, &(proposal_id + 1));

        Ok(proposal_id)
    }

    pub fn cast_vote(
        env: Env,
        voter: Address,
        proposal_id: u32,
        vote_type: VoteType,
    ) -> Result<(), Error> {
        voter.require_auth();
        let mut proposals: Map<u32, Proposal> = env
            .storage()
            .instance()
            .get(&PROPOSALS)
            .ok_or(Error::ProposalsNotFound)?;
        let mut proposal = proposals.get(proposal_id).ok_or(Error::ProposalNotFound)?;

        if proposal.status != ProposalStatus::Active {
            return Err(Error::ProposalNotActive);
        }

        let current_time = env.ledger().timestamp();
        if current_time > proposal.voting_end {
            return Err(Error::VotingEnded);
        }

        let mut votes: Map<(u32, Address), Vote> = env
            .storage()
            .instance()
            .get(&VOTES)
            .unwrap_or(Map::new(&env));
        if votes.contains_key((proposal_id, voter.clone())) {
            return Err(Error::AlreadyVoted);
        }

        let config: GovernanceConfig = env
            .storage()
            .instance()
            .get(&GOVERNANCE_CONFIG)
            .ok_or(Error::NotInitialized)?;
        let voting_power = derive_voting_power(&env, &config, &voter);
        if voting_power <= 0 {
            return Err(Error::ZeroVotingPower);
        }

        match vote_type {
            VoteType::For => proposal.votes_for += voting_power,
            VoteType::Against => proposal.votes_against += voting_power,
            VoteType::Abstain => proposal.votes_abstain += voting_power,
        }
        proposal.total_votes += 1;

        votes.set(
            (proposal_id, voter.clone()),
            Vote {
                voter,
                proposal_id,
                vote_type,
                voting_power,
                timestamp: current_time,
            },
        );

        proposals.set(proposal_id, proposal);
        env.storage().instance().set(&PROPOSALS, &proposals);
        env.storage().instance().set(&VOTES, &votes);
        Ok(())
    }

    pub fn finalize_proposal(env: Env, proposal_id: u32) -> Result<ProposalStatus, Error> {
        let mut proposals: Map<u32, Proposal> = env
            .storage()
            .instance()
            .get(&PROPOSALS)
            .ok_or(Error::ProposalsNotFound)?;
        let mut proposal = proposals.get(proposal_id).ok_or(Error::ProposalNotFound)?;
        let config: GovernanceConfig = env
            .storage()
            .instance()
            .get(&GOVERNANCE_CONFIG)
            .ok_or(Error::NotInitialized)?;

        if env.ledger().timestamp() <= proposal.voting_end {
            return Err(Error::VotingStillActive);
        }

        let total_cast = proposal.votes_for + proposal.votes_against + proposal.votes_abstain;
        let total_power = total_voting_power(&config);
        if total_power <= 0 {
            return Err(Error::InvalidTotalVotingPower);
        }

        let quorum_bps = (total_cast * 10000) / total_power;
        if quorum_bps < config.quorum_percentage as i128 {
            proposal.status = ProposalStatus::Rejected;
            proposals.set(proposal_id, proposal.clone());
            env.storage().instance().set(&PROPOSALS, &proposals);
            return Ok(proposal.status);
        }

        let approval_votes = proposal.votes_for + proposal.votes_against;
        if approval_votes == 0 {
            proposal.status = ProposalStatus::Rejected;
        } else {
            let approval_bps = (proposal.votes_for * 10000) / approval_votes;
            if approval_bps >= config.approval_threshold as i128 {
                proposal.status = ProposalStatus::Approved;
            } else {
                proposal.status = ProposalStatus::Rejected;
            }
        }

        proposals.set(proposal_id, proposal.clone());
        env.storage().instance().set(&PROPOSALS, &proposals);
        Ok(proposal.status)
    }

    /// Marks an approved proposal as executed after its execution delay has elapsed.
    pub fn execute_proposal(env: Env, proposal_id: u32) -> Result<(), Error> {
        let mut proposals: Map<u32, Proposal> = env
            .storage()
            .instance()
            .get(&PROPOSALS)
            .ok_or(Error::ProposalsNotFound)?;
        let mut proposal = proposals.get(proposal_id).ok_or(Error::ProposalNotFound)?;

        if proposal.status != ProposalStatus::Approved {
            return Err(Error::ProposalNotApproved);
        }

        let executable_at = proposal.voting_end.saturating_add(proposal.execution_delay);
        if env.ledger().timestamp() < executable_at {
            return Err(Error::ExecutionDelayNotMet);
        }

        proposal.status = ProposalStatus::Executed;
        proposals.set(proposal_id, proposal);
        env.storage().instance().set(&PROPOSALS, &proposals);
        Ok(())
    }

    /// Returns true when an executed governance proposal approved `wasm_hash`.
    pub fn is_upgrade_approved(env: Env, wasm_hash: BytesN<32>) -> bool {
        let proposals: Map<u32, Proposal> = match env.storage().instance().get(&PROPOSALS) {
            Some(proposals) => proposals,
            None => return false,
        };
        let proposal_count: u32 = env.storage().instance().get(&PROPOSAL_COUNT).unwrap_or(0);
        let now = env.ledger().timestamp();

        let mut proposal_id = 0;
        while proposal_id < proposal_count {
            if let Some(proposal) = proposals.get(proposal_id) {
                let executable_at = proposal.voting_end.saturating_add(proposal.execution_delay);
                if proposal.new_wasm_hash == wasm_hash
                    && proposal.status == ProposalStatus::Executed
                    && now >= executable_at
                {
                    return true;
                }
            }
            proposal_id += 1;
        }

        false
    }
}

/// Derives voting power for the configured scheme.
///
/// `OnePersonOneVote` always returns `1` for an authenticated address.
/// `TokenWeighted` reads the voter's current balance from the configured
/// governance token. `snapshot_ledger` is recorded policy metadata only; the
/// standard Soroban token interface does not expose historical balances.
fn derive_voting_power(env: &Env, config: &GovernanceConfig, voter: &Address) -> i128 {
    match config.voting_scheme {
        VotingScheme::OnePersonOneVote => 1,
        VotingScheme::TokenWeighted => {
            token::Client::new(env, &config.governance_token).balance(voter)
        }
    }
}

fn total_voting_power(config: &GovernanceConfig) -> i128 {
    match config.voting_scheme {
        VotingScheme::OnePersonOneVote => config.one_person_total_voters as i128,
        VotingScheme::TokenWeighted => config.token_total_voting_power,
    }
}

fn enforce_min_proposal_stake(
    env: &Env,
    config: &GovernanceConfig,
    proposer: &Address,
) -> Result<(), Error> {
    if config.min_proposal_stake == 0 {
        return Ok(());
    }

    let balance = token::Client::new(env, &config.governance_token).balance(proposer);
    if balance < config.min_proposal_stake {
        return Err(Error::InsufficientStake);
    }
    Ok(())
}

// Add this function to the GovernanceContract impl block

/// Marks an expired proposal as Expired if its voting end timestamp has passed.
/// Permissionless - anyone can call this to help maintain storage hygiene.
/// 
/// Storage Retention Decision:
/// We choose to flag proposals as Expired rather than deleting them entirely.
/// Rationale:
/// 1. Soroban storage TTL: Deleting would free rent, but we'd lose historical
///    vote records needed for audits and transparency
/// 2. Off-chain consumers need to verify proposal history - deletion breaks
///    this and would require additional off-chain indexing
/// 3. Expired flag with storage TTL extension is sufficient to signal
///    terminal state while maintaining data integrity
/// 4. Future optimization: Implement a garbage collection mechanism that
///    deletes only vote records (not proposal metadata) after a configurable
///    retention period to balance storage costs
pub fn sweep_expired_proposal(env: Env, proposal_id: u32) -> Result<(), Error> {
    let mut proposals: Map<u32, Proposal> = env
        .storage()
        .instance()
        .get(&PROPOSALS)
        .ok_or(Error::ProposalsNotFound)?;
    let mut proposal = proposals.get(proposal_id).ok_or(Error::ProposalNotFound)?;
    
    // Only mark as expired if the voting period has ended
    let current_time = env.ledger().timestamp();
    if current_time <= proposal.voting_end {
        return Err(Error::VotingStillActive);
    }
    
    // If already in terminal state, no-op (don't override Approved/Rejected/Executed)
    if proposal.status == ProposalStatus::Expired {
        return Ok(());
    }
    
    // If proposal is in a terminal state (Approved/Rejected/Executed), don't mark as expired
    // This ensures we don't override legitimate final states
    match proposal.status {
        ProposalStatus::Approved | ProposalStatus::Rejected | ProposalStatus::Executed => {
            return Ok(());
        }
        _ => {
            proposal.status = ProposalStatus::Expired;
            proposals.set(proposal_id, proposal);
            env.storage().instance().set(&PROPOSALS, &proposals);
            Ok(())
        }
    }
}

// Add this helper inside the test module
fn get_proposal_status(env: &Env, proposal_id: u32) -> Option<ProposalStatus> {
    let proposals: Map<u32, Proposal> = env.storage().instance().get(&PROPOSALS)?;
    proposals.get(proposal_id).map(|p| p.status)
}



#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};

    fn setup_test(
        env: &Env,
        voting_scheme: VotingScheme,
        quorum_percentage: u32,
        min_proposal_stake: i128,
        one_person_total_voters: u32,
    ) -> (
        GovernanceContractClient<'_>,
        Option<token::StellarAssetClient<'_>>,
        Address,
    ) {
        let contract_id = env.register_contract(None, GovernanceContract);
        let client = GovernanceContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        let user = Address::generate(env);
        let needs_token = voting_scheme == VotingScheme::TokenWeighted || min_proposal_stake > 0;
        let (token_address, token_admin_client) = if needs_token {
            let token_admin = Address::generate(env);
            let token_address = env
                .register_stellar_asset_contract_v2(token_admin)
                .address();
            (
                token_address.clone(),
                Some(token::StellarAssetClient::new(env, &token_address)),
            )
        } else {
            (Address::generate(env), None)
        };

        let config = GovernanceConfig {
            voting_period: 100,
            execution_delay: 0,
            quorum_percentage,
            approval_threshold: 5000,
            min_proposal_stake,
            voting_scheme,
            governance_token: token_address,
            one_person_total_voters,
            token_total_voting_power: 100,
            snapshot_ledger: None,
        };

        env.mock_all_auths();
        client.init_governance(&admin, &config);
        (client, token_admin_client, user)
    }

    fn create_test_proposal(
        env: &Env,
        client: &GovernanceContractClient,
        proposer: &Address,
    ) -> u32 {
        client.create_proposal(
            proposer,
            &BytesN::from_array(env, &[0u8; 32]),
            &symbol_short!("test"),
        )
    }

    #[test]
    fn test_edge_case_double_voting() {
        let env = Env::default();
        let (client, _, user) = setup_test(&env, VotingScheme::OnePersonOneVote, 1000, 0, 10);
        let prop_id = create_test_proposal(&env, &client, &user);

        client.cast_vote(&user, &prop_id, &VoteType::For);

        let result = client.try_cast_vote(&user, &prop_id, &VoteType::For);
        assert_eq!(result, Err(Ok(Error::AlreadyVoted)));
    }

    #[test]
    fn test_edge_case_voting_after_expiration() {
        let env = Env::default();
        let (client, _, user) = setup_test(&env, VotingScheme::OnePersonOneVote, 1000, 0, 10);
        let prop_id = create_test_proposal(&env, &client, &user);

        env.ledger().with_mut(|li| li.timestamp = 200);

        let result = client.try_cast_vote(&user, &prop_id, &VoteType::For);
        assert_eq!(result, Err(Ok(Error::VotingEnded)));
    }

    #[test]
    fn test_edge_case_exact_threshold() {
        let env = Env::default();
        let (client, _, user1) = setup_test(&env, VotingScheme::OnePersonOneVote, 1000, 0, 2);
        let user2 = Address::generate(&env);
        let prop_id = create_test_proposal(&env, &client, &user1);

        client.cast_vote(&user1, &prop_id, &VoteType::For);
        client.cast_vote(&user2, &prop_id, &VoteType::Against);

        env.ledger().with_mut(|li| li.timestamp = 200);
        let status = client.finalize_proposal(&prop_id);

        assert_eq!(status, ProposalStatus::Approved);
    }

    #[test]
    fn test_edge_case_below_threshold() {
        let env = Env::default();
        let (client, _, user1) = setup_test(&env, VotingScheme::OnePersonOneVote, 1000, 0, 3);
        let user2 = Address::generate(&env);
        let user3 = Address::generate(&env);
        let prop_id = create_test_proposal(&env, &client, &user1);

        client.cast_vote(&user1, &prop_id, &VoteType::For);
        client.cast_vote(&user2, &prop_id, &VoteType::Against);
        client.cast_vote(&user3, &prop_id, &VoteType::Against);

        env.ledger().with_mut(|li| li.timestamp = 200);
        let status = client.finalize_proposal(&prop_id);

        assert_eq!(status, ProposalStatus::Rejected);
    }

    #[test]
    fn test_token_weighted_derives_power_from_balance() {
        let env = Env::default();
        let (client, token_admin_client, proposer) =
            setup_test(&env, VotingScheme::TokenWeighted, 5000, 0, 0);
        let token_admin_client = token_admin_client.unwrap();
        let voter_for = Address::generate(&env);
        let voter_against = Address::generate(&env);

        token_admin_client.mint(&voter_for, &60);
        token_admin_client.mint(&voter_against, &40);

        let prop_id = create_test_proposal(&env, &client, &proposer);
        client.cast_vote(&voter_for, &prop_id, &VoteType::For);
        client.cast_vote(&voter_against, &prop_id, &VoteType::Against);

        env.ledger().with_mut(|li| li.timestamp = 200);
        let status = client.finalize_proposal(&prop_id);

        assert_eq!(status, ProposalStatus::Approved);
    }

    #[test]
    fn test_token_weighted_rejects_zero_balance_voter() {
        let env = Env::default();
        let (client, _, proposer) = setup_test(&env, VotingScheme::TokenWeighted, 1000, 0, 0);
        let zero_balance_voter = Address::generate(&env);
        let prop_id = create_test_proposal(&env, &client, &proposer);

        let result = client.try_cast_vote(&zero_balance_voter, &prop_id, &VoteType::For);

        assert_eq!(result, Err(Ok(Error::ZeroVotingPower)));
    }

    #[test]
    fn test_token_weighted_quorum_just_met() {
        let env = Env::default();
        let (client, token_admin_client, proposer) =
            setup_test(&env, VotingScheme::TokenWeighted, 5000, 0, 0);
        let token_admin_client = token_admin_client.unwrap();
        let voter = Address::generate(&env);
        let non_voter = Address::generate(&env);

        token_admin_client.mint(&voter, &50);
        token_admin_client.mint(&non_voter, &50);

        let prop_id = create_test_proposal(&env, &client, &proposer);
        client.cast_vote(&voter, &prop_id, &VoteType::For);

        env.ledger().with_mut(|li| li.timestamp = 200);
        assert_eq!(client.finalize_proposal(&prop_id), ProposalStatus::Approved);
    }

    #[test]
    fn test_token_weighted_quorum_just_missed_rejects() {
        let env = Env::default();
        let (client, token_admin_client, proposer) =
            setup_test(&env, VotingScheme::TokenWeighted, 5000, 0, 0);
        let token_admin_client = token_admin_client.unwrap();
        let voter = Address::generate(&env);
        let non_voter = Address::generate(&env);

        token_admin_client.mint(&voter, &49);
        token_admin_client.mint(&non_voter, &51);

        let prop_id = create_test_proposal(&env, &client, &proposer);
        client.cast_vote(&voter, &prop_id, &VoteType::For);

        env.ledger().with_mut(|li| li.timestamp = 200);
        let status = client.finalize_proposal(&prop_id);

        assert_eq!(status, ProposalStatus::Rejected);
    }

    #[test]
    fn test_one_person_and_token_weighted_can_diverge() {
        let env = Env::default();
        let (one_person_client, _, one_person_proposer) =
            setup_test(&env, VotingScheme::OnePersonOneVote, 1000, 0, 2);
        let heavy_against = Address::generate(&env);

        let one_person_prop = create_test_proposal(&env, &one_person_client, &one_person_proposer);
        one_person_client.cast_vote(&one_person_proposer, &one_person_prop, &VoteType::For);
        one_person_client.cast_vote(&heavy_against, &one_person_prop, &VoteType::Against);

        env.ledger().with_mut(|li| li.timestamp = 200);
        assert_eq!(
            one_person_client.finalize_proposal(&one_person_prop),
            ProposalStatus::Approved
        );

        let env = Env::default();
        let (token_client, token_admin_client, token_proposer) =
            setup_test(&env, VotingScheme::TokenWeighted, 1000, 0, 0);
        let token_admin_client = token_admin_client.unwrap();
        let heavy_against = Address::generate(&env);

        token_admin_client.mint(&token_proposer, &1);
        token_admin_client.mint(&heavy_against, &99);

        let token_prop = create_test_proposal(&env, &token_client, &token_proposer);
        token_client.cast_vote(&token_proposer, &token_prop, &VoteType::For);
        token_client.cast_vote(&heavy_against, &token_prop, &VoteType::Against);

        env.ledger().with_mut(|li| li.timestamp = 200);
        assert_eq!(
            token_client.finalize_proposal(&token_prop),
            ProposalStatus::Rejected
        );
    }

    #[test]
    fn test_create_proposal_enforces_minimum_stake() {
        let env = Env::default();
        let (client, token_admin_client, proposer) =
            setup_test(&env, VotingScheme::TokenWeighted, 1000, 10, 0);
        let token_admin_client = token_admin_client.unwrap();

        let result = client.try_create_proposal(
            &proposer,
            &BytesN::from_array(&env, &[0u8; 32]),
            &symbol_short!("test"),
        );
        assert_eq!(result, Err(Ok(Error::InsufficientStake)));

        token_admin_client.mint(&proposer, &10);
        let prop_id = create_test_proposal(&env, &client, &proposer);
        assert_eq!(prop_id, 0);
    }

    #[test]
    fn test_upgrade_approval_requires_executed_matching_hash_after_delay() {
        let env = Env::default();
        let contract_id = env.register_contract(None, GovernanceContract);
        let client = GovernanceContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let proposer = Address::generate(&env);
        let approved_hash = BytesN::from_array(&env, &[7u8; 32]);
        let other_hash = BytesN::from_array(&env, &[9u8; 32]);

        let config = GovernanceConfig {
            voting_period: 100,
            execution_delay: 50,
            quorum_percentage: 1000,
            approval_threshold: 5000,
            min_proposal_stake: 0,
            voting_scheme: VotingScheme::OnePersonOneVote,
            governance_token: Address::generate(&env),
            one_person_total_voters: 1,
            token_total_voting_power: 100,
            snapshot_ledger: None,
        };

        env.mock_all_auths();
        client.init_governance(&admin, &config);
        let proposal_id =
            client.create_proposal(&proposer, &approved_hash, &symbol_short!("upgrade"));
        client.cast_vote(&proposer, &proposal_id, &VoteType::For);

        env.ledger().with_mut(|li| li.timestamp = 101);
        assert_eq!(
            client.finalize_proposal(&proposal_id),
            ProposalStatus::Approved
        );
        assert!(!client.is_upgrade_approved(&approved_hash));
        assert_eq!(
            client.try_execute_proposal(&proposal_id),
            Err(Ok(Error::ExecutionDelayNotMet)),
        );

        env.ledger().with_mut(|li| li.timestamp = 150);
        client.execute_proposal(&proposal_id);

        assert!(client.is_upgrade_approved(&approved_hash));
        assert!(!client.is_upgrade_approved(&other_hash));
    }
}

#[test]
fn test_sweep_expired_before_expiry_rejected() {
    let env = Env::default();
    let (client, _, proposer) = setup_test(&env, VotingScheme::OnePersonOneVote, 1000, 0, 10);
    let prop_id = create_test_proposal(&env, &client, &proposer);
    
    // Try to sweep before voting ends - should fail
    let result = client.try_sweep_expired_proposal(&prop_id);
    assert_eq!(result, Err(Ok(Error::VotingStillActive)));
}

#[test]
fn test_sweep_expired_after_expiry_succeeds() {
    let env = Env::default();
    let (client, _, proposer) = setup_test(&env, VotingScheme::OnePersonOneVote, 1000, 0, 10);
    let prop_id = create_test_proposal(&env, &client, &proposer);
    
    // Advance past voting end
    env.ledger().with_mut(|li| li.timestamp = 200);
    
    // Sweep should succeed and mark as Expired
    client.sweep_expired_proposal(&prop_id);
    
    // Verify status is now Expired
    let status = get_proposal_status(&env, prop_id);
    assert_eq!(status, Some(ProposalStatus::Expired));
}

#[test]
fn test_sweep_expired_no_op_on_terminal_states() {
    let env = Env::default();
    let (client, _, proposer) = setup_test(&env, VotingScheme::OnePersonOneVote, 1000, 0, 10);
    let prop_id = create_test_proposal(&env, &client, &proposer);
    
    // Cast vote to make it finalizable
    client.cast_vote(&proposer, &prop_id, &VoteType::For);
    
    // Advance time and finalize to Approved state
    env.ledger().with_mut(|li| li.timestamp = 200);
    let status = client.finalize_proposal(&prop_id);
    assert_eq!(status, ProposalStatus::Approved);
    
    // Try to sweep - should be no-op
    client.sweep_expired_proposal(&prop_id);
    
    // Verify status remains Approved
    // We can check by trying to sweep again (should still be Approved)
    // but we need to verify the status didn't change to Expired
    // Let's check via execution
    assert_eq!(client.try_execute_proposal(&prop_id), Ok(()));
}

#[test]
fn test_sweep_expired_after_rejected_proposal() {
    let env = Env::default();
    let (client, _, proposer) = setup_test(&env, VotingScheme::OnePersonOneVote, 1000, 0, 10);
    let prop_id = create_test_proposal(&env, &client, &proposer);
    
    // Cast insufficient votes (not enough quorum)
    // With quorum 1000 (10%) and total voters 10, need at least 1 vote for quorum
    // But no votes = rejected
    env.ledger().with_mut(|li| li.timestamp = 200);
    let status = client.finalize_proposal(&prop_id);
    assert_eq!(status, ProposalStatus::Rejected);
    
    // Try to sweep - should be no-op since already terminal
    client.sweep_expired_proposal(&prop_id);
    
    // Verify still Rejected
    // Cannot execute rejected proposal
    assert_eq!(
        client.try_execute_proposal(&prop_id),
        Err(Ok(Error::ProposalNotApproved))
    );
}

#[test]
fn test_sweep_expired_called_multiple_times() {
    let env = Env::default();
    let (client, _, proposer) = setup_test(&env, VotingScheme::OnePersonOneVote, 1000, 0, 10);
    let prop_id = create_test_proposal(&env, &client, &proposer);
    
    // Advance past voting end
    env.ledger().with_mut(|li| li.timestamp = 200);
    
    // First sweep should succeed
    client.sweep_expired_proposal(&prop_id);
    
    // Second sweep should be no-op and succeed
    client.sweep_expired_proposal(&prop_id);
    
    // Verify still expired
    let vote_result = client.try_cast_vote(&proposer, &prop_id, &VoteType::For);
    assert_eq!(vote_result, Err(Ok(Error::ProposalNotActive)));
}

#[test]
fn test_sweep_expired_with_active_proposal_voting_ongoing() {
    let env = Env::default();
    let (client, _, proposer) = setup_test(&env, VotingScheme::OnePersonOneVote, 1000, 0, 10);
    let prop_id = create_test_proposal(&env, &client, &proposer);
    
    // During voting period (timestamp < voting_end)
    env.ledger().with_mut(|li| li.timestamp = 50);
    
    let result = client.try_sweep_expired_proposal(&prop_id);
    assert_eq!(result, Err(Ok(Error::VotingStillActive)));
    
    // Voting should still work
    client.cast_vote(&proposer, &prop_id, &VoteType::For);
}

#[test]
fn test_sweep_expired_with_executed_proposal() {
    let env = Env::default();
    let (client, _, proposer) = setup_test(&env, VotingScheme::OnePersonOneVote, 1000, 0, 10);
    let prop_id = create_test_proposal(&env, &client, &proposer);
    
    // Create config with execution delay
    let admin = Address::generate(&env);
    let config = GovernanceConfig {
        voting_period: 100,
        execution_delay: 50,
        quorum_percentage: 1000,
        approval_threshold: 5000,
        min_proposal_stake: 0,
        voting_scheme: VotingScheme::OnePersonOneVote,
        governance_token: Address::generate(&env),
        one_person_total_voters: 10,
        token_total_voting_power: 100,
        snapshot_ledger: None,
    };
    
    env.mock_all_auths();
    client.init_governance(&admin, &config);
    let prop_id = client.create_proposal(&proposer, &BytesN::from_array(&env, &[0u8; 32]), &symbol_short!("test"));
    
    client.cast_vote(&proposer, &prop_id, &VoteType::For);
    
    env.ledger().with_mut(|li| li.timestamp = 200);
    let status = client.finalize_proposal(&prop_id);
    assert_eq!(status, ProposalStatus::Approved);
    
    // Execute
    client.execute_proposal(&prop_id);
    
    // Try to sweep - should be no-op
    client.sweep_expired_proposal(&prop_id);
    
    // Verify still Executed
    assert!(client.is_upgrade_approved(&BytesN::from_array(&env, &[0u8; 32])));
}
