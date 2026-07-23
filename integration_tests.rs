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


// ============================================================================
// Cross-module version-gate integration tests (Issue #277)
//
// These tests wire a *real* GrainlifyContract instance as the governance
// backend for a ProgramEscrowContract instance and exercise the full
// `check_upgrade_approval` / `check_governance_version` path against genuine
// grainlify-core state rather than a hand-rolled mock client.
//
// Note on initial version: `GrainlifyContract::init_admin` sets the stored
// version to the crate constant `VERSION = 2`.  Tests that need a
// "below-threshold" scenario therefore use `min_governance_version = 3`
// (or lower the contract version to 1 via `set_version` first).
//
// Scenarios covered
// ─────────────────
// 1. `version_below_min_blocks_escrow_admin_ops`
//    grainlify-core's version is explicitly lowered to 1; program-escrow
//    requires version 2.  Admin operations gated on
//    `check_governance_requirements` must return `GovernanceVersionTooLow`.
//
// 2. `version_at_threshold_allows_escrow_admin_ops`
//    grainlify-core starts at VERSION=2 and program-escrow requires exactly
//    2.  All governance-gated admin operations must succeed immediately.
//
// 3. `no_approved_proposal_blocks_upgrade_gate`
//    Version gate is satisfied (VERSION=2, min=2) but `is_upg_ok` returns
//    `false` when no executed proposal for the candidate hash exists.
//
// 4. `mid_test_grainlify_core_upgrade_observed_by_program_escrow`
//    Complete governance lifecycle inside a single Env:
//    version lowered → blocked → proposal created/voted/finalized/executed →
//    version bumped → unblocked.  program-escrow observes every transition.
//
// 5. `upgrade_approval_exact_hash_match`
//    Only the exact hash of an executed proposal satisfies `is_upg_ok`.
//    A byte-adjacent hash and a never-proposed hash must both return `false`.
//
// 6. `governance_contract_replaced_mid_test`
//    program-escrow is re-pointed at a new governance contract mid-test.
//    All subsequent checks must use the new contract's state.
// ============================================================================

/// Deploy and initialise a GrainlifyContract with governance enabled.
///
/// Returns `(contract_address, client)`. Configured with 50 % quorum/threshold,
/// token-weighted voting, `voting_period` and `execution_delay` as supplied.
/// The stored version after `init_admin` is the crate constant `VERSION = 2`.
fn setup_grainlify_governance<'a>(
    env: &'a Env,
    admin: &Address,
    token: &Address,
    total_voting_power: i128,
    voting_period: u64,
    execution_delay: u64,
) -> (Address, GrainlifyContractClient<'a>) {
    use grainlify_core::governance::{GovernanceConfig, VotingScheme};
    let gov_id = env.register_contract(None, GrainlifyContract);
    let gov_client = GrainlifyContractClient::new(env, &gov_id);
    gov_client.init_admin(admin);
    gov_client.init_governance(
        admin,
        &GovernanceConfig {
            voting_period,
            execution_delay,
            quorum_percentage: 5000,
            approval_threshold: 5000,
            min_proposal_stake: 0,
            voting_scheme: VotingScheme::TokenWeighted,
            governance_token: token.clone(),
            token_total_voting_power: total_voting_power,
            one_person_total_voters: 0,
            snapshot_ledger: None,
        },
    );
    (gov_id, gov_client)
}

// ────────────────────────────────────────────────────────────────────────────
// Test 1: version below minimum blocks program-escrow admin operations
// ────────────────────────────────────────────────────────────────────────────

/// grainlify-core's version is explicitly lowered to 1 (below the required
/// min of 2).  Every governance-gated admin operation on program-escrow must
/// fail with `GovernanceVersionTooLow` — not panic.
#[test]
fn test_cross_contract_version_below_min_blocks_escrow_admin_ops() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let token_admin = Address::generate(&env);
    let (token, _tc, token_admin_client) = create_token_contract(&env, &token_admin);
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let balance: i128 = 10_000_000_000_000;
    token_admin_client.mint(&voter, &balance);

    // Deploy real grainlify-core; init_admin stores VERSION=2 by default.
    let (gov_id, gov_client) = setup_grainlify_governance(
        &env, &admin, &token, balance, /*voting_period=*/ 100, /*execution_delay=*/ 50,
    );
    assert_eq!(gov_client.get_version(), 2,
        "grainlify-core should start at VERSION=2 after init_admin");

    // Explicitly lower to version 1 to create a below-threshold scenario.
    gov_client.set_version(&1);
    assert_eq!(gov_client.get_version(), 1,
        "version should be 1 after set_version(1)");

    // Wire to program-escrow and require min_governance_version = 2.
    let escrow_id = env.register_contract(None, ProgramEscrowContract);
    let escrow = ProgramEscrowContractClient::new(&env, &escrow_id);
    escrow.setadmin(&admin);
    escrow.set_governance_contract(&gov_id);
    escrow.set_min_governance_version(&2);

    // ── Assert: governance-gated ops fail (version 1 < min 2) ─────────────
    assert!(
        escrow.try_update_rate_limit_config(&3600, &10, &60).is_err(),
        "update_rate_limit_config must fail when gov version (1) < min_version (2)"
    );
    assert!(
        escrow.try_set_paused(&Some(true), &None, &None).is_err(),
        "set_paused must fail when gov version (1) < min_version (2)"
    );

    // ── Sanity: no voter variable used → suppress unused warning ──────────
    let _ = voter;
}

// ────────────────────────────────────────────────────────────────────────────
// Test 2: version at threshold allows program-escrow admin operations
// ────────────────────────────────────────────────────────────────────────────

/// grainlify-core starts at VERSION=2.  program-escrow requires exactly 2.
/// All governance-gated admin operations must succeed immediately — no bump
/// required.
#[test]
fn test_cross_contract_version_at_threshold_allows_escrow_admin_ops() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let token_admin = Address::generate(&env);
    let (token, _tc, token_admin_client) = create_token_contract(&env, &token_admin);
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let balance: i128 = 10_000_000_000_000;
    token_admin_client.mint(&voter, &balance);

    // init_admin stores VERSION=2; wire escrow with min_governance_version=2.
    let (gov_id, gov_client) = setup_grainlify_governance(
        &env, &admin, &token, balance, 100, 50,
    );
    assert_eq!(gov_client.get_version(), 2,
        "grainlify-core must start at VERSION=2");

    let escrow_id = env.register_contract(None, ProgramEscrowContract);
    let escrow = ProgramEscrowContractClient::new(&env, &escrow_id);
    escrow.setadmin(&admin);
    escrow.set_governance_contract(&gov_id);
    escrow.set_min_governance_version(&2); // exactly at threshold

    // ── Assert: governance-gated ops succeed immediately ───────────────────
    escrow.update_rate_limit_config(&7200, &5, &120);
    let cfg = escrow.get_rate_limit_config();
    assert_eq!(cfg.window_size, 7200,
        "rate-limit window_size should be updated — version is at threshold");
    assert_eq!(cfg.max_operations, 5);

    escrow.set_paused(&Some(false), &Some(false), &Some(false));
    let flags = escrow.get_pause_flags();
    assert!(!flags.lock_paused && !flags.release_paused && !flags.refund_paused,
        "pause flags should be cleared — version is at threshold");

    // Suppress unused warning
    let _ = voter;
}

// ────────────────────────────────────────────────────────────────────────────
// Test 3: no approved proposal blocks the upgrade gate
// ────────────────────────────────────────────────────────────────────────────

/// Even when the version gate passes, `is_upg_ok` returns `false` when no
/// executed governance proposal exists for the candidate wasm_hash.
/// A rejected or pending proposal must not satisfy the gate.
#[test]
fn test_cross_contract_no_approved_proposal_blocks_upgrade_gate() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let token_admin = Address::generate(&env);
    let (token, _tc, token_admin_client) = create_token_contract(&env, &token_admin);
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let balance: i128 = 10_000_000_000_000;
    token_admin_client.mint(&voter, &balance);

    let voting_period = 100u64;
    let execution_delay = 50u64;

    let (gov_id, gov_client) = setup_grainlify_governance(
        &env, &admin, &token, balance, voting_period, execution_delay,
    );

    // init_admin stores VERSION=2; min_governance_version=2 → version gate
    // is satisfied.  We only need to verify the upgrade-hash gate separately.
    assert_eq!(gov_client.get_version(), 2,
        "grainlify-core starts at VERSION=2");

    let escrow_id = env.register_contract(None, ProgramEscrowContract);
    let escrow = ProgramEscrowContractClient::new(&env, &escrow_id);
    escrow.setadmin(&admin);
    escrow.set_governance_contract(&gov_id);
    escrow.set_min_governance_version(&2);

    let candidate_hash = BytesN::from_array(&env, &[0xabu8; 32]);

    // ── Case A: no proposals at all ─────────────────────────────────────────
    // is_upg_ok should return false — no executed proposal for this hash.
    assert!(
        !gov_client.is_upg_ok(&candidate_hash),
        "is_upg_ok must be false when no proposal exists for the hash"
    );

    // ── Case B: proposal created but not voted/finalized ────────────────────
    let prop_id = gov_client.create_proposal(
        &voter,
        &candidate_hash,
        &symbol_short!("test_upg"),
    );
    assert!(
        !gov_client.is_upg_ok(&candidate_hash),
        "is_upg_ok must be false when proposal is only Pending/Active"
    );

    // ── Case C: proposal finalized but rejected (no votes cast) ─────────────
    env.ledger().with_mut(|li| li.timestamp += voting_period + 1);
    let status = gov_client.finalize_proposal(&prop_id);
    assert_eq!(status, governance::ProposalStatus::Rejected,
        "proposal with zero votes should be Rejected");
    assert!(
        !gov_client.is_upg_ok(&candidate_hash),
        "is_upg_ok must be false for a Rejected proposal"
    );
}

// ────────────────────────────────────────────────────────────────────────────
// Test 4: mid-test grainlify-core upgrade is correctly observed by program-escrow
// ────────────────────────────────────────────────────────────────────────────

/// Runs a complete governance lifecycle inside a single Env:
///
/// Phase 1 – blocked: grainlify-core at VERSION=2, escrow requires min=3.
/// Phase 2 – governance lifecycle: proposal created → voted For → finalized
///           Approved → executed after delay → is_upg_ok returns true.
/// Phase 3 – admin bumps grainlify-core to version 3.
/// Phase 4 – unblocked: version gate passes; all escrow admin ops succeed.
#[test]
fn test_cross_contract_mid_test_upgrade_observed_by_program_escrow() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let token_admin = Address::generate(&env);
    let (token, _tc, token_admin_client) = create_token_contract(&env, &token_admin);
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let balance: i128 = 10_000_000_000_000;
    token_admin_client.mint(&voter, &balance);

    let voting_period = 100u64;
    let execution_delay = 50u64;

    let (gov_id, gov_client) = setup_grainlify_governance(
        &env, &admin, &token, balance, voting_period, execution_delay,
    );

    // Set up program-escrow requiring min_governance_version = 3.
    // grainlify-core starts at VERSION=2, so the version gate is initially blocked.
    let escrow_id = env.register_contract(None, ProgramEscrowContract);
    let escrow = ProgramEscrowContractClient::new(&env, &escrow_id);
    let backend = Address::generate(&env);
    escrow.setadmin(&admin);
    escrow.set_governance_contract(&gov_id);
    escrow.set_min_governance_version(&3);

    // ── Phase 1: blocked — gov at VERSION=2, min=3 ─────────────────────────
    assert_eq!(gov_client.get_version(), 2,
        "grainlify-core starts at VERSION=2");
    assert!(
        escrow.try_update_rate_limit_config(&3600, &10, &60).is_err(),
        "phase 1: escrow must be blocked (version 2 < min 3)"
    );

    // ── Phase 2: full governance proposal lifecycle ─────────────────────────
    let upgrade_hash = BytesN::from_array(&env, &[0x77u8; 32]);
    let prop_id = gov_client.create_proposal(
        &voter,
        &upgrade_hash,
        &symbol_short!("v3_upg"),
    );

    // Cast a decisive For-vote (100 % of total voting power → quorum met).
    gov_client.cast_vote(&voter, &prop_id, &governance::VoteType::For);

    // Advance past voting period and finalize.
    env.ledger().with_mut(|li| li.timestamp += voting_period + 1);
    let status = gov_client.finalize_proposal(&prop_id);
    assert_eq!(status, governance::ProposalStatus::Approved,
        "proposal must be Approved after full quorum For-vote");

    // is_upg_ok must still be false — execution delay not yet elapsed.
    assert!(
        !gov_client.is_upg_ok(&upgrade_hash),
        "is_upg_ok must be false before execution delay elapses"
    );

    // Advance past execution delay and execute.
    env.ledger().with_mut(|li| li.timestamp += execution_delay + 1);
    gov_client.execute_proposal(&prop_id);

    // Now is_upg_ok must be true.
    assert!(
        gov_client.is_upg_ok(&upgrade_hash),
        "is_upg_ok must be true after proposal is executed"
    );

    // escrow is still blocked — version (2) < min (3).
    assert!(
        escrow.try_update_rate_limit_config(&3600, &10, &60).is_err(),
        "phase 2 end: version gate still blocks escrow (version 2 < min 3)"
    );

    // ── Phase 3: admin bumps grainlify-core to version 3 ───────────────────
    gov_client.set_version(&3);
    assert_eq!(gov_client.get_version(), 3,
        "grainlify-core must report version 3 after set_version(3)");

    // ── Phase 4: program-escrow gate now passes ─────────────────────────────
    escrow.update_rate_limit_config(&1800, &20, &30);
    let cfg = escrow.get_rate_limit_config();
    assert_eq!(cfg.window_size, 1800,
        "rate-limit config should update after the version gate passes");

    let program_id = String::from_str(&env, "prog-277");
    let token_addr = Address::generate(&env);
    escrow.initialize_program(&program_id, &backend, &token_addr);
    assert!(escrow.program_exists(),
        "program must be initialized after the upgrade gate passes");

    // Upgrade hash approval persists after the version bump.
    assert!(
        gov_client.is_upg_ok(&upgrade_hash),
        "is_upg_ok must remain true after version bump"
    );
}

// ────────────────────────────────────────────────────────────────────────────
// Test 5: upgrade approval passes only for the exact executed-proposal hash
// ────────────────────────────────────────────────────────────────────────────

/// Only the specific hash bound to an executed governance proposal satisfies
/// `is_upg_ok`.  A byte-adjacent hash and a never-proposed hash must return
/// `false`, even when the version gate is fully satisfied.
#[test]
fn test_cross_contract_upgrade_approval_exact_hash_match() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let token_admin = Address::generate(&env);
    let (token, _tc, token_admin_client) = create_token_contract(&env, &token_admin);
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let balance: i128 = 10_000_000_000_000;
    token_admin_client.mint(&voter, &balance);

    let voting_period = 100u64;
    let execution_delay = 50u64;

    let (gov_id, gov_client) = setup_grainlify_governance(
        &env, &admin, &token, balance, voting_period, execution_delay,
    );

    // VERSION starts at 2; min_governance_version=2 → version gate satisfied.
    assert_eq!(gov_client.get_version(), 2);

    let approved_hash = BytesN::from_array(&env, &[0x11u8; 32]);
    let different_hash = BytesN::from_array(&env, &[0x12u8; 32]); // differs by one byte
    let never_proposed_hash = BytesN::from_array(&env, &[0xffu8; 32]);

    // Run a proposal for `approved_hash` all the way to Executed.
    let prop_id = gov_client.create_proposal(&voter, &approved_hash, &symbol_short!("exact"));
    gov_client.cast_vote(&voter, &prop_id, &governance::VoteType::For);
    env.ledger().with_mut(|li| li.timestamp += voting_period + 1);
    gov_client.finalize_proposal(&prop_id);
    env.ledger().with_mut(|li| li.timestamp += execution_delay + 1);
    gov_client.execute_proposal(&prop_id);

    let escrow_id = env.register_contract(None, ProgramEscrowContract);
    let escrow = ProgramEscrowContractClient::new(&env, &escrow_id);
    escrow.setadmin(&admin);
    escrow.set_governance_contract(&gov_id);
    escrow.set_min_governance_version(&2);

    // ── Assertions ──────────────────────────────────────────────────────────
    assert!(
        gov_client.is_upg_ok(&approved_hash),
        "is_upg_ok must be true for the exact executed hash"
    );
    assert!(
        !gov_client.is_upg_ok(&different_hash),
        "is_upg_ok must be false for a hash differing by one byte"
    );
    assert!(
        !gov_client.is_upg_ok(&never_proposed_hash),
        "is_upg_ok must be false for a hash that was never proposed"
    );

    // Version-gated escrow operation succeeds (version 2 == min 2).
    escrow.update_rate_limit_config(&3600, &10, &60);
    let cfg = escrow.get_rate_limit_config();
    assert_eq!(cfg.window_size, 3600,
        "version-gated op should succeed regardless of which hash is approved");
}

// ────────────────────────────────────────────────────────────────────────────
// Test 6: governance contract replaced mid-test
// ────────────────────────────────────────────────────────────────────────────

/// When program-escrow is re-pointed at a new governance contract via
/// `set_governance_contract`, all subsequent checks must use the new
/// contract's state.
///
/// gov1 — version explicitly lowered to 1 (below-threshold for min=2).
/// gov2 — stays at default VERSION=2 (at-threshold for min=2).
#[test]
fn test_cross_contract_governance_contract_replaced_mid_test() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let token_admin = Address::generate(&env);
    let (token, _tc, token_admin_client) = create_token_contract(&env, &token_admin);
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let balance: i128 = 10_000_000_000_000;
    token_admin_client.mint(&voter, &balance);

    // ── gov1: lower to version 1 (below threshold) ─────────────────────────
    let (gov1_id, gov1_client) = setup_grainlify_governance(
        &env, &admin, &token, balance, 100, 50,
    );
    gov1_client.set_version(&1); // explicitly drop below threshold
    assert_eq!(gov1_client.get_version(), 1,
        "gov1 must be at version 1 after set_version(1)");

    // ── gov2: stays at default VERSION=2 (at threshold) ────────────────────
    let (gov2_id, gov2_client) = setup_grainlify_governance(
        &env, &admin, &token, balance, 100, 50,
    );
    assert_eq!(gov2_client.get_version(), 2,
        "gov2 must be at VERSION=2 by default");

    // ── program-escrow initially points to gov1 ─────────────────────────────
    let escrow_id = env.register_contract(None, ProgramEscrowContract);
    let escrow = ProgramEscrowContractClient::new(&env, &escrow_id);
    escrow.setadmin(&admin);
    escrow.set_governance_contract(&gov1_id);
    escrow.set_min_governance_version(&2);

    // Blocked: gov1 reports version 1 < min 2.
    assert!(
        escrow.try_update_rate_limit_config(&3600, &10, &60).is_err(),
        "should be blocked while pointing at gov1 (version 1 < min 2)"
    );

    // ── Switch escrow to gov2 ────────────────────────────────────────────────
    escrow.set_governance_contract(&gov2_id);
    assert_eq!(
        escrow.get_governance_contract(),
        Some(gov2_id.clone()),
        "escrow must now reference gov2"
    );

    // Unblocked: gov2 reports version 2 == min 2.
    escrow.update_rate_limit_config(&1800, &5, &90);
    let cfg = escrow.get_rate_limit_config();
    assert_eq!(cfg.window_size, 1800,
        "config should update after governance contract is replaced");

    // ── Verify gov1 state is untouched ───────────────────────────────────────
    assert_eq!(gov1_client.get_version(), 1,
        "gov1 version must remain unchanged after escrow was re-pointed at gov2");

    // Suppress unused warning
    let _ = voter;
}
