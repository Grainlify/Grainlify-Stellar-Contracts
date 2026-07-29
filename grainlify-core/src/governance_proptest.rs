#![cfg(test)]

extern crate std;

use crate::governance::{
    Error as GovError, GovernanceConfig, GovernanceContract, GovernanceContractClient, Proposal,
    ProposalStatus, VoteType, VotingScheme, GOVERNANCE_CONFIG, PROPOSALS,
};
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestCaseError, TestRng, TestRunner};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, BytesN, Env, Map, symbol_short,
};
use std::format;

const CASES: u32 = 32;
const MAX_SHRINK_ITERS: u32 = 64;

fn proptest_config() -> ProptestConfig {
    ProptestConfig {
        cases: CASES,
        max_shrink_iters: MAX_SHRINK_ITERS,
        failure_persistence: None,
        ..ProptestConfig::default()
    }
}

fn deterministic_runner() -> TestRunner {
    let config = proptest_config();
    let algorithm = config.rng_algorithm;
    TestRunner::new_with_rng(config, TestRng::deterministic_rng(algorithm))
}

#[derive(Clone, Debug)]
struct VoterOp {
    vote_type: VoteType,
    power: i128,
}

fn vote_type_strategy() -> impl Strategy<Value = VoteType> {
    prop_oneof![
        Just(VoteType::For),
        Just(VoteType::Against),
        Just(VoteType::Abstain),
    ]
}

fn voter_op_strategy() -> impl Strategy<Value = VoterOp> {
    (
        vote_type_strategy(),
        prop_oneof![
            1_i128..=1_000_i128,
            1_000_000_i128..=1_000_000_000_i128,
            (i128::MAX / 4)..=i128::MAX,
        ],
    )
        .prop_map(|(vote_type, power)| VoterOp { vote_type, power })
}

fn setup_proptest_env(
    voting_scheme: VotingScheme,
    token_total_voting_power: i128,
    quorum_percentage: u32,
    approval_threshold: u32,
) -> (
    Env,
    Address,
    GovernanceContractClient<'static>,
    Option<token::StellarAssetClient<'static>>,
    Address,
    u32,
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, GovernanceContract);
    let client = GovernanceContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    let (token_address, token_admin) = if voting_scheme == VotingScheme::TokenWeighted {
        let token_admin_addr = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin_addr);
        let token_addr = token_contract.address();
        (
            token_addr.clone(),
            Some(token::StellarAssetClient::new(&env, &token_addr)),
        )
    } else {
        (Address::generate(&env), None)
    };

    let config = GovernanceConfig {
        voting_period: 100,
        execution_delay: 0,
        quorum_percentage,
        approval_threshold,
        min_proposal_stake: 0,
        voting_scheme,
        governance_token: token_address,
        one_person_total_voters: 1000,
        token_total_voting_power,
        snapshot_ledger: None,
    };

    client.init_governance(&admin, &config);

    let proposer = Address::generate(&env);
    let wasm_hash = BytesN::from_array(&env, &[1u8; 32]);
    let prop_id = client.create_proposal(&proposer, &wasm_hash, &symbol_short!("proptest"));

    (env, contract_id, client, token_admin, proposer, prop_id)
}

fn get_proposal_from_env(env: &Env, contract_id: &Address, prop_id: u32) -> Proposal {
    env.as_contract(contract_id, || {
        let proposals: Map<u32, Proposal> = env
            .storage()
            .instance()
            .get(&PROPOSALS)
            .expect("Proposals map not found");
        proposals.get(prop_id).expect("Proposal not found")
    })
}

#[test]
fn proptest_vote_tally_summation_invariant() {
    let mut runner = deterministic_runner();
    let ops_strategy = proptest::collection::vec(voter_op_strategy(), 1..=20);

    runner
        .run(&ops_strategy, |ops| {
            let (env, contract_id, client, token_admin, _proposer, prop_id) = setup_proptest_env(
                VotingScheme::TokenWeighted,
                i128::MAX,
                5000,
                5000,
            );
            let token_admin = token_admin.expect("token admin expected for TokenWeighted");

            let mut expected_for: i128 = 0;
            let mut expected_against: i128 = 0;
            let mut expected_abstain: i128 = 0;

            for op in ops {
                let voter = Address::generate(&env);
                if op.power > 0 {
                    token_admin.mint(&voter, &op.power);
                }

                let target_expected = match op.vote_type {
                    VoteType::For => expected_for,
                    VoteType::Against => expected_against,
                    VoteType::Abstain => expected_abstain,
                };

                let will_overflow = target_expected.checked_add(op.power).is_none();
                let res = client.try_cast_vote(&voter, &prop_id, &op.vote_type);

                if op.power <= 0 {
                    if res != Err(Ok(GovError::ZeroVotingPower)) {
                        return Err(TestCaseError::fail(format!(
                            "Expected ZeroVotingPower error for power {}, got {:?}",
                            op.power, res
                        )));
                    }
                } else if will_overflow {
                    if res != Err(Ok(GovError::VoteWeightOverflow)) {
                        return Err(TestCaseError::fail(format!(
                            "Expected VoteWeightOverflow error when adding {} to {}, got {:?}",
                            op.power, target_expected, res
                        )));
                    }
                } else {
                    if res.is_err() {
                        return Err(TestCaseError::fail(format!(
                            "Expected cast_vote success for power {}, got {:?}",
                            op.power, res
                        )));
                    }
                    match op.vote_type {
                        VoteType::For => expected_for += op.power,
                        VoteType::Against => expected_against += op.power,
                        VoteType::Abstain => expected_abstain += op.power,
                    }
                }
            }

            // Verify proposal tallies match expected exactly and never wrapped
            let proposal = get_proposal_from_env(&env, &contract_id, prop_id);

            if proposal.votes_for != expected_for {
                return Err(TestCaseError::fail(format!(
                    "votes_for mismatch: expected {}, got {}",
                    expected_for, proposal.votes_for
                )));
            }
            if proposal.votes_against != expected_against {
                return Err(TestCaseError::fail(format!(
                    "votes_against mismatch: expected {}, got {}",
                    expected_against, proposal.votes_against
                )));
            }
            if proposal.votes_abstain != expected_abstain {
                return Err(TestCaseError::fail(format!(
                    "votes_abstain mismatch: expected {}, got {}",
                    expected_abstain, proposal.votes_abstain
                )));
            }

            // Invariant: sum of votes_for + votes_against + votes_abstain equals sum of individual voting powers when within i128
            let sum_of_parts = proposal
                .votes_for
                .checked_add(proposal.votes_against)
                .and_then(|s| s.checked_add(proposal.votes_abstain));

            let expected_total_power_opt = expected_for
                .checked_add(expected_against)
                .and_then(|s| s.checked_add(expected_abstain));

            if let (Some(sum_parts), Some(expected_total)) = (sum_of_parts, expected_total_power_opt) {
                if sum_parts != expected_total {
                    return Err(TestCaseError::fail(format!(
                        "Sum invariant violated: sum_parts={}, expected_total={}",
                        sum_parts, expected_total
                    )));
                }
            }

            Ok(())
        })
        .unwrap();
}

#[test]
fn proptest_finalize_proposal_never_panics_and_bps_in_range() {
    let mut runner = deterministic_runner();

    let tallies_and_config_strategy = (
        0_i128..=i128::MAX, // votes_for
        0_i128..=i128::MAX, // votes_against
        0_i128..=i128::MAX, // votes_abstain
        i128::MIN..=i128::MAX, // total_voting_power
        0_u32..=10000_u32,  // quorum_percentage
        5000_u32..=10000_u32, // approval_threshold
    );

    runner
        .run(
            &tallies_and_config_strategy,
            |(votes_for, votes_against, votes_abstain, total_voting_power, quorum_percentage, approval_threshold)| {
                let env = Env::default();
                env.mock_all_auths();

                let contract_id = env.register_contract(None, GovernanceContract);
                let client = GovernanceContractClient::new(&env, &contract_id);
                let token_addr = Address::generate(&env);

                let config = GovernanceConfig {
                    voting_period: 100,
                    execution_delay: 0,
                    quorum_percentage,
                    approval_threshold,
                    min_proposal_stake: 0,
                    voting_scheme: VotingScheme::TokenWeighted,
                    governance_token: token_addr,
                    one_person_total_voters: 1000,
                    token_total_voting_power: total_voting_power,
                    snapshot_ledger: None,
                };

                let prop_id = 0u32;
                let proposal = Proposal {
                    id: prop_id,
                    proposer: Address::generate(&env),
                    new_wasm_hash: BytesN::from_array(&env, &[2u8; 32]),
                    description: symbol_short!("test"),
                    created_at: 0,
                    voting_start: 0,
                    voting_end: 100,
                    execution_delay: 0,
                    status: ProposalStatus::Active,
                    votes_for,
                    votes_against,
                    votes_abstain,
                    total_votes: 10,
                };

                // Store config and proposal under contract context
                env.as_contract(&contract_id, || {
                    env.storage().instance().set(&GOVERNANCE_CONFIG, &config);
                    let mut proposals: Map<u32, Proposal> = Map::new(&env);
                    proposals.set(prop_id, proposal);
                    env.storage().instance().set(&PROPOSALS, &proposals);
                });

                // Advance ledger timestamp beyond voting_end
                env.ledger().with_mut(|li| li.timestamp = 101);

                // Entrypoint call must NEVER panic for any valid i128 inputs
                let res = client.try_finalize_proposal(&prop_id);

                // Trace evaluation logic matching finalize_proposal in governance.rs
                let total_cast_opt = votes_for
                    .checked_add(votes_against)
                    .and_then(|s| s.checked_add(votes_abstain));

                if total_cast_opt.is_none() {
                    if res != Err(Ok(GovError::VoteWeightOverflow)) {
                        return Err(TestCaseError::fail(format!(
                            "Expected VoteWeightOverflow for overflowing total_cast, got {:?}",
                            res
                        )));
                    }
                    return Ok(());
                }

                if total_voting_power <= 0 {
                    if res != Err(Ok(GovError::InvalidTotalVotingPower)) {
                        return Err(TestCaseError::fail(format!(
                            "Expected InvalidTotalVotingPower for total_voting_power {}, got {:?}",
                            total_voting_power, res
                        )));
                    }
                    return Ok(());
                }

                let total_cast = total_cast_opt.unwrap();
                let quorum_mul_opt = total_cast.checked_mul(10000);
                if quorum_mul_opt.is_none() {
                    if res != Err(Ok(GovError::VoteWeightOverflow)) {
                        return Err(TestCaseError::fail(format!(
                            "Expected VoteWeightOverflow for quorum multiplication overflow, got {:?}",
                            res
                        )));
                    }
                    return Ok(());
                }

                let quorum_bps = quorum_mul_opt.unwrap() / total_voting_power;

                if quorum_bps < quorum_percentage as i128 {
                    if res != Ok(Ok(ProposalStatus::Rejected)) {
                        return Err(TestCaseError::fail(format!(
                            "Expected Rejected status for quorum_bps {} < {}, got {:?}",
                            quorum_bps, quorum_percentage, res
                        )));
                    }
                    return Ok(());
                }

                let approval_votes_opt = votes_for.checked_add(votes_against);
                if approval_votes_opt.is_none() {
                    if res != Err(Ok(GovError::VoteWeightOverflow)) {
                        return Err(TestCaseError::fail(format!(
                            "Expected VoteWeightOverflow for approval_votes overflow, got {:?}",
                            res
                        )));
                    }
                    return Ok(());
                }

                let approval_votes = approval_votes_opt.unwrap();
                if approval_votes == 0 {
                    if res != Ok(Ok(ProposalStatus::Rejected)) {
                        return Err(TestCaseError::fail(format!(
                            "Expected Rejected status for zero approval_votes, got {:?}",
                            res
                        )));
                    }
                    return Ok(());
                }

                let approval_mul_opt = votes_for.checked_mul(10000);
                if approval_mul_opt.is_none() {
                    if res != Err(Ok(GovError::VoteWeightOverflow)) {
                        return Err(TestCaseError::fail(format!(
                            "Expected VoteWeightOverflow for approval multiplication overflow, got {:?}",
                            res
                        )));
                    }
                    return Ok(());
                }

                let approval_bps = approval_mul_opt.unwrap() / approval_votes;

                // When total_cast <= total_voting_power, quorum_bps must be in [0, 10000]
                if total_cast <= total_voting_power {
                    if quorum_bps < 0 || quorum_bps > 10000 {
                        return Err(TestCaseError::fail(format!(
                            "quorum_bps {} out of bounds [0, 10000]",
                            quorum_bps
                        )));
                    }
                }

                // approval_bps must always be in [0, 10000] since votes_for <= votes_for + votes_against
                if approval_bps < 0 || approval_bps > 10000 {
                    return Err(TestCaseError::fail(format!(
                        "approval_bps {} out of bounds [0, 10000]",
                        approval_bps
                    )));
                }

                let expected_status = if approval_bps >= approval_threshold as i128 {
                    ProposalStatus::Approved
                } else {
                    ProposalStatus::Rejected
                };

                if res != Ok(Ok(expected_status.clone())) {
                    return Err(TestCaseError::fail(format!(
                        "Expected status {:?}, got {:?}",
                        expected_status, res
                    )));
                }

                Ok(())
            },
        )
        .unwrap();
}

#[test]
fn proptest_sequential_voters_lifecycle_invariants() {
    let mut runner = deterministic_runner();
    let ops_strategy = proptest::collection::vec(voter_op_strategy(), 5..=25);

    runner
        .run(&ops_strategy, |ops| {
            let (env, _contract_id, client, token_admin, _proposer, prop_id) = setup_proptest_env(
                VotingScheme::TokenWeighted,
                100_000_000,
                2000, // 20% quorum
                6000, // 60% approval threshold
            );
            let token_admin = token_admin.expect("token admin expected");

            let mut expected_for: i128 = 0;
            let mut expected_against: i128 = 0;
            let mut expected_abstain: i128 = 0;

            for op in ops {
                let voter = Address::generate(&env);
                token_admin.mint(&voter, &op.power);

                let res = client.try_cast_vote(&voter, &prop_id, &op.vote_type);
                let target_expected = match op.vote_type {
                    VoteType::For => expected_for,
                    VoteType::Against => expected_against,
                    VoteType::Abstain => expected_abstain,
                };

                if target_expected.checked_add(op.power).is_some() {
                    assert!(res.is_ok());
                    match op.vote_type {
                        VoteType::For => expected_for += op.power,
                        VoteType::Against => expected_against += op.power,
                        VoteType::Abstain => expected_abstain += op.power,
                    }
                } else {
                    assert_eq!(res, Err(Ok(GovError::VoteWeightOverflow)));
                }
            }

            // Advance ledger time past voting period
            env.ledger().with_mut(|li| li.timestamp = 200);

            let res = client.try_finalize_proposal(&prop_id);

            let total_cast_opt = expected_for
                .checked_add(expected_against)
                .and_then(|s| s.checked_add(expected_abstain));

            if total_cast_opt.is_none() {
                assert_eq!(res, Err(Ok(GovError::VoteWeightOverflow)));
                return Ok(());
            }

            let total_cast = total_cast_opt.unwrap();
            let total_power: i128 = 100_000_000;
            let quorum_mul_opt = total_cast.checked_mul(10000);

            if quorum_mul_opt.is_none() {
                assert_eq!(res, Err(Ok(GovError::VoteWeightOverflow)));
                return Ok(());
            }

            assert!(res.is_ok(), "finalize_proposal failed unexpectedly: {:?}", res);

            let status = res.as_ref().unwrap().as_ref().unwrap().clone();
            let quorum_bps = quorum_mul_opt.unwrap() / total_power;

            if quorum_bps < 2000 {
                assert_eq!(status, ProposalStatus::Rejected);
            } else {
                let approval_votes_opt = expected_for.checked_add(expected_against);
                if approval_votes_opt.is_none() {
                    assert_eq!(res, Ok(Ok(ProposalStatus::Rejected)));
                    return Ok(());
                }

                let approval_votes = approval_votes_opt.unwrap();
                if approval_votes == 0 {
                    assert_eq!(status, ProposalStatus::Rejected);
                } else {
                    let approval_mul_opt = expected_for.checked_mul(10000);
                    if approval_mul_opt.is_none() {
                        assert_eq!(res, Ok(Ok(ProposalStatus::Rejected)));
                        return Ok(());
                    }
                    let approval_bps = approval_mul_opt.unwrap() / approval_votes;
                    if approval_bps >= 6000 {
                        assert_eq!(status, ProposalStatus::Approved);
                    } else {
                        assert_eq!(status, ProposalStatus::Rejected);
                    }
                }
            }

            Ok(())
        })
        .unwrap();
}
