#![cfg(test)]

//! Comprehensive Integration Tests for Grainlify Contracts
//!
//! This module tests:
//! - Cross-contract interactions (escrow + program-escrow)
//! - Upgrade scenarios with state migration
//! - Multi-contract workflows (lock → release → payout)
//! - Error propagation across contracts
//! - Event emission and indexing
//! - Performance tests for batch operations

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, BytesN, Env, String, symbol_short,
};

use grainlify_core::{GrainlifyContract, GrainlifyContractClient, governance};
use bounty_escrow::{BountyEscrowContract, BountyEscrowContractClient};
use program_escrow::{ProgramEscrowContract, ProgramEscrowContractClient};

// Helper to create token contract
fn create_token_contract<'a>(
    env: &'a Env,
    admin: &Address,
) -> (Address, token::Client<'a>, token::StellarAssetClient<'a>) {
    let token_id = env.register_stellar_asset_contract_v2(admin.clone());
    let token = token_id.address();
    let token_client = token::Client::new(env, &token);
    let token_admin_client = token::StellarAssetClient::new(env, &token);
    (token, token_client, token_admin_client)
}

#[test]
fn test_end_to_end_cross_contract_governance_trigger() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    // 1. Set up Token
    let token_admin = Address::generate(&env);
    let (token, token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    let admin = Address::generate(&env);
    let backend = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let voter = Address::generate(&env);
    let program_id = String::from_str(&env, "program-1");

    let amount = 10_000_000_000_000i128;
    token_admin_client.mint(&depositor, &amount);
    token_admin_client.mint(&voter, &amount);

    // 2. Deploy & Init Grainlify Core (Governance)
    let gov_id = env.register_contract(None, GrainlifyContract);
    let gov_client = GrainlifyContractClient::new(&env, &gov_id);
    gov_client.init_admin(&admin);

    let voting_period = 100;
    let execution_delay = 50;
    let gov_config = governance::GovernanceConfig {
        voting_period,
        execution_delay,
        quorum_percentage: 5000, // 50%
        approval_threshold: 5000, // 50%
        min_proposal_stake: 10,
        voting_scheme: governance::VotingScheme::TokenWeighted,
        governance_token: token.clone(),
        token_total_voting_power: amount,
        one_person_total_voters: 0,
        snapshot_ledger: None,
    };
    gov_client.init_governance(&admin, &gov_config);

    // 3. Deploy & Init Bounty Escrow
    let bounty_escrow_id = env.register_contract(None, BountyEscrowContract);
    let bounty_client = BountyEscrowContractClient::new(&env, &bounty_escrow_id);
    bounty_client.init(&admin, &token);
    bounty_client.set_governance_contract(&gov_id);
    
    // We require governance version 2 to unlock certain escrow admin actions
    bounty_client.set_min_governance_version(&2);

    // 4. Deploy & Init Program Escrow
    let program_escrow_id = env.register_contract(None, ProgramEscrowContract);
    let program_client = ProgramEscrowContractClient::new(&env, &program_escrow_id);
    program_client.setadmin(&admin);
    program_client.initialize_program(&program_id, &backend, &token);
    program_client.set_governance_contract(&gov_id);
    
    // Program escrow also requires version 2
    program_client.set_min_governance_version(&2);

    // =========================================================================
    // SCENARIO 1: Dependent actions fail before governance upgrade
    // =========================================================================

    // Action A: Bounty Escrow Interaction
    let bounty_id = 1u64;
    let deadline = 5000u64;
    bounty_client.lock_funds(&depositor, &bounty_id, &amount, &deadline);
    
    // Release should fail (version too low)
    let res_bounty = bounty_client.try_release_funds(&bounty_id, &contributor);
    assert!(res_bounty.is_err());

    // Action B: Program Escrow Interaction
    // Update rate limit config should fail (version too low)
    let res_program = program_client.try_update_rate_limit_config(&3600, &10, &60);
    assert!(res_program.is_err());


    // =========================================================================
    // SCENARIO 2: A governance proposal fails to pass (dependent action should never fire)
    // =========================================================================

    let bad_wasm_hash = BytesN::from_array(&env, &[1; 32]);
    let proposal_id_bad = gov_client.create_proposal(
        &voter,
        &bad_wasm_hash,
        &symbol_short!("bad_upg"),
    );

    // Fast-forward past the voting period without casting enough votes
    env.ledger().with_mut(|li| li.timestamp += voting_period + 1);
    
    // Finalize the failed proposal
    let status_bad = gov_client.finalize_proposal(&proposal_id_bad);
    assert_eq!(status_bad, governance::ProposalStatus::Rejected);

    // Dependent actions still fail since governance didn't pass / upgrade didn't happen
    let res_bounty_bad = bounty_client.try_release_funds(&bounty_id, &contributor);
    assert!(res_bounty_bad.is_err());


    // =========================================================================
    // SCENARIO 3: Governance proposal passes and successfully triggers the dependent escrow action
    // =========================================================================

    // In a real flow, a successful governance proposal approves an upgrade hash.
    // The admin then applies that upgrade (or updates config). Here we update the 
    // governance contract's version to 2 to unlock the escrows.
    
    // Create a good proposal
    let good_wasm_hash = BytesN::from_array(&env, &[2; 32]);
    let proposal_id_good = gov_client.create_proposal(
        &voter,
        &good_wasm_hash,
        &symbol_short!("good_upg"),
    );

    // Voter casts vote for the proposal
    gov_client.cast_vote(&voter, &proposal_id_good, &governance::VoteType::For);

    // Fast-forward past voting period
    env.ledger().with_mut(|li| li.timestamp += voting_period + 1);

    // Finalize the successful proposal
    let status_good = gov_client.finalize_proposal(&proposal_id_good);
    assert_eq!(status_good, governance::ProposalStatus::Approved);

    // Fast-forward past execution delay
    env.ledger().with_mut(|li| li.timestamp += execution_delay + 1);

    // Execute the proposal
    gov_client.execute_proposal(&proposal_id_good);

    // Verify upgrade approval
    assert!(gov_client.is_upg_ok(&good_wasm_hash));
    
    // The governance proposal passed! The admin now applies the upgrade 
    // by bumping the governance version, triggering dependent actions to succeed.
    gov_client.set_version(&2);
    assert_eq!(gov_client.get_version(), 2);

    // Now dependent actions across both escrow contracts should succeed
    
    // Bounty escrow release
    bounty_client.release_funds(&bounty_id, &contributor);
    assert_eq!(token_client.balance(&contributor), amount);
    
    // Program escrow config update
    program_client.update_rate_limit_config(&3600, &10, &60);
    let config = program_client.get_rate_limit_config();
    assert_eq!(config.window_size, 3600);
}
