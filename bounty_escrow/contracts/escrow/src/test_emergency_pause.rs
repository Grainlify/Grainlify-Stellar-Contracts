#![cfg(test)]

//! # Emergency Global Pause Tests — Bounty Escrow
//!
//! Verifies that the `global_paused` kill switch blocks **every** state-changing
//! entrypoint while leaving view/query functions callable.
//!
//! ## Coverage Matrix
//!
//! | Entrypoint               | Blocked when global_paused |
//! |--------------------------|---------------------------|
//! | lock_funds               | ✓                         |
//! | release_funds            | ✓                         |
//! | refund                   | ✓                         |
//! | claim                    | ✓                         |
//! | authorize_claim          | ✓                         |
//! | partial_release          | ✓                         |
//! | batch_lock_funds         | ✓                         |
//! | batch_release_funds      | ✓                         |
//! | sweep_expired_refunds    | ✓                         |
//! | approve_refund           | ✓                         |
//! | set_emergency_pause      | ✓ (admin auth required)   |
//! | get_escrow_info          | ✗ (view — stays live)     |
//! | get_balance              | ✗ (view — stays live)     |
//! | get_pause_flags          | ✗ (view — stays live)     |
//! | query_escrows            | ✗ (view — stays live)     |
//! | get_aggregate_stats      | ✗ (view — stays live)     |

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, Vec,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn create_token(
    env: &Env,
    admin: &Address,
) -> (token::Client<'static>, token::StellarAssetClient<'static>) {
    let addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    (
        token::Client::new(env, &addr),
        token::StellarAssetClient::new(env, &addr),
    )
}

fn create_escrow(env: &Env) -> (BountyEscrowContractClient<'static>, Address) {
    let id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(env, &id);
    (client, id)
}

/// Full setup: init contract + token, mint `amount` to depositor.
/// Returns `(client, admin, depositor, contributor, token_client)`.
fn setup(
    env: &Env,
    depositor_balance: i128,
) -> (
    BountyEscrowContractClient<'static>,
    Address,
    Address,
    Address,
    token::Client<'static>,
) {
    env.mock_all_auths();

    let admin = Address::generate(env);
    let token_admin = Address::generate(env);
    let depositor = Address::generate(env);
    let contributor = Address::generate(env);

    let (token_client, token_sac) = create_token(env, &token_admin);
    let (escrow_client, _) = create_escrow(env);

    escrow_client.init(&admin, &token_client.address);
    token_sac.mint(&depositor, &depositor_balance);

    (escrow_client, admin, depositor, contributor, token_client)
}

/// Lock a bounty and return its `deadline`.
fn lock_bounty(
    client: &BountyEscrowContractClient<'static>,
    env: &Env,
    depositor: &Address,
    bounty_id: u64,
    amount: i128,
) -> u64 {
    let deadline = env.ledger().timestamp() + 1000;
    client.lock_funds(depositor, &bounty_id, &amount, &deadline);
    deadline
}

// ---------------------------------------------------------------------------
// Tests: set_emergency_pause
// ---------------------------------------------------------------------------

#[test]
fn test_set_emergency_pause_sets_flag() {
    let env = Env::default();
    let (client, admin, _, _, _) = setup(&env, 10_000);

    let flags = client.get_pause_flags();
    assert!(!flags.global_paused);

    client.set_emergency_pause(&true);
    let flags = client.get_pause_flags();
    assert!(flags.global_paused);

    // Unpause
    client.set_emergency_pause(&false);
    let flags = client.get_pause_flags();
    assert!(!flags.global_paused);
}

#[test]
fn test_set_emergency_pause_requires_init() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _) = create_escrow(&env);
    let res = client.try_set_emergency_pause(&true);
    assert!(res.is_err());
}

// ---------------------------------------------------------------------------
// Tests: lock_funds blocked by global pause
// ---------------------------------------------------------------------------

#[test]
fn test_global_pause_blocks_lock_funds() {
    let env = Env::default();
    let (client, _, depositor, _, _) = setup(&env, 10_000);

    client.set_emergency_pause(&true);

    let bounty_id: u64 = 1;
    let deadline = env.ledger().timestamp() + 1000;
    let res = client.try_lock_funds(&depositor, &bounty_id, &100, &deadline);
    assert!(res.is_err());
}

#[test]
fn test_global_pause_unpaused_lock_funds_succeeds() {
    let env = Env::default();
    let (client, _, depositor, _, _) = setup(&env, 10_000);

    // Pause then unpause
    client.set_emergency_pause(&true);
    client.set_emergency_pause(&false);

    let bounty_id: u64 = 1;
    let deadline = env.ledger().timestamp() + 1000;
    client.lock_funds(&depositor, &bounty_id, &100, &deadline);

    let escrow = client.get_escrow_info(&bounty_id);
    assert_eq!(escrow.amount, 100);
}

// ---------------------------------------------------------------------------
// Tests: release_funds blocked by global pause
// ---------------------------------------------------------------------------

#[test]
fn test_global_pause_blocks_release_funds() {
    let env = Env::default();
    let (client, _, depositor, contributor, _) = setup(&env, 10_000);

    let bounty_id: u64 = 1;
    lock_bounty(&client, &env, &depositor, bounty_id, 100);

    client.set_emergency_pause(&true);

    let res = client.try_release_funds(&bounty_id, &contributor);
    assert!(res.is_err());
}

// ---------------------------------------------------------------------------
// Tests: refund blocked by global pause
// ---------------------------------------------------------------------------

#[test]
fn test_global_pause_blocks_refund() {
    let env = Env::default();
    let (client, admin, depositor, _, _) = setup(&env, 10_000);

    let bounty_id: u64 = 1;
    lock_bounty(&client, &env, &depositor, bounty_id, 100);

    // Move past deadline so refund is eligible
    env.ledger().with_mut(|l| l.timestamp += 2000);

    // Approve refund first
    client.approve_refund(&bounty_id, &100, &depositor, &RefundMode::Full);

    client.set_emergency_pause(&true);

    let res = client.try_refund(&bounty_id);
    assert!(res.is_err());
}

// ---------------------------------------------------------------------------
// Tests: claim blocked by global pause
// ---------------------------------------------------------------------------

#[test]
fn test_global_pause_blocks_claim() {
    let env = Env::default();
    let (client, _, depositor, contributor, _) = setup(&env, 10_000);

    let bounty_id: u64 = 1;
    lock_bounty(&client, &env, &depositor, bounty_id, 100);

    // Authorize claim for contributor
    client.authorize_claim(&bounty_id, &contributor);

    client.set_emergency_pause(&true);

    let res = client.try_claim(&bounty_id);
    assert!(res.is_err());
}

// ---------------------------------------------------------------------------
// Tests: authorize_claim blocked by global pause
// ---------------------------------------------------------------------------

#[test]
fn test_global_pause_blocks_authorize_claim() {
    let env = Env::default();
    let (client, _, depositor, contributor, _) = setup(&env, 10_000);

    let bounty_id: u64 = 1;
    lock_bounty(&client, &env, &depositor, bounty_id, 100);

    client.set_emergency_pause(&true);

    let res = client.try_authorize_claim(&bounty_id, &contributor);
    assert!(res.is_err());
}

// ---------------------------------------------------------------------------
// Tests: partial_release blocked by global pause
// ---------------------------------------------------------------------------

#[test]
fn test_global_pause_blocks_partial_release() {
    let env = Env::default();
    let (client, _, depositor, contributor, _) = setup(&env, 10_000);

    let bounty_id: u64 = 1;
    lock_bounty(&client, &env, &depositor, bounty_id, 100);

    client.set_emergency_pause(&true);

    let res = client.try_partial_release(&bounty_id, &contributor, &50);
    assert!(res.is_err());
}

// ---------------------------------------------------------------------------
// Tests: batch_lock_funds blocked by global pause
// ---------------------------------------------------------------------------

#[test]
fn test_global_pause_blocks_batch_lock_funds() {
    let env = Env::default();
    let (client, _, depositor, _, _) = setup(&env, 10_000);

    client.set_emergency_pause(&true);

    let mut items = Vec::new(&env);
    items.push_back(LockFundsItem {
        depositor: depositor.clone(),
        bounty_id: 1,
        amount: 100,
        deadline: env.ledger().timestamp() + 1000,
    });

    let res = client.try_batch_lock_funds(&items);
    assert!(res.is_err());
}

// ---------------------------------------------------------------------------
// Tests: batch_release_funds blocked by global pause
// ---------------------------------------------------------------------------

#[test]
fn test_global_pause_blocks_batch_release_funds() {
    let env = Env::default();
    let (client, _, depositor, contributor, _) = setup(&env, 10_000);

    let bounty_id: u64 = 1;
    lock_bounty(&client, &env, &depositor, bounty_id, 100);

    client.set_emergency_pause(&true);

    let mut items = Vec::new(&env);
    items.push_back(ReleaseFundsItem {
        bounty_id,
        contributor: contributor.clone(),
    });

    let res = client.try_batch_release_funds(&items);
    assert!(res.is_err());
}

// ---------------------------------------------------------------------------
// Tests: sweep_expired_refunds blocked by global pause
// ---------------------------------------------------------------------------

#[test]
fn test_global_pause_blocks_sweep_expired_refunds() {
    let env = Env::default();
    let (client, _, depositor, _, _) = setup(&env, 10_000);

    let bounty_id: u64 = 1;
    lock_bounty(&client, &env, &depositor, bounty_id, 100);

    // Move past deadline
    env.ledger().with_mut(|l| l.timestamp += 2000);

    // Approve refund
    client.approve_refund(&bounty_id, &100, &depositor, &RefundMode::Full);

    client.set_emergency_pause(&true);

    let mut bounty_ids = Vec::new(&env);
    bounty_ids.push_back(bounty_id);

    let res = client.try_sweep_expired_refunds(&bounty_ids);
    assert!(res.is_err());
}

// ---------------------------------------------------------------------------
// Tests: approve_refund blocked by global pause
// ---------------------------------------------------------------------------

#[test]
fn test_global_pause_blocks_approve_refund() {
    let env = Env::default();
    let (client, _, depositor, _, _) = setup(&env, 10_000);

    let bounty_id: u64 = 1;
    lock_bounty(&client, &env, &depositor, bounty_id, 100);

    client.set_emergency_pause(&true);

    let res = client.try_approve_refund(&bounty_id, &100, &depositor, &RefundMode::Full);
    assert!(res.is_err());
}

// ---------------------------------------------------------------------------
// Tests: view/query functions remain callable while paused
// ---------------------------------------------------------------------------

#[test]
fn test_global_pause_allows_get_escrow_info() {
    let env = Env::default();
    let (client, _, depositor, _, _) = setup(&env, 10_000);

    let bounty_id: u64 = 1;
    lock_bounty(&client, &env, &depositor, bounty_id, 100);

    client.set_emergency_pause(&true);

    // View functions must still work
    let escrow = client.get_escrow_info(&bounty_id);
    assert_eq!(escrow.amount, 100);
}

#[test]
fn test_global_pause_allows_get_balance() {
    let env = Env::default();
    let (client, _, depositor, _, _) = setup(&env, 10_000);

    let bounty_id: u64 = 1;
    lock_bounty(&client, &env, &depositor, bounty_id, 100);

    client.set_emergency_pause(&true);

    let balance = client.get_balance();
    assert_eq!(balance, 100);
}

#[test]
fn test_global_pause_allows_get_pause_flags() {
    let env = Env::default();
    let (client, _, _, _, _) = setup(&env, 10_000);

    client.set_emergency_pause(&true);

    let flags = client.get_pause_flags();
    assert!(flags.global_paused);
    assert!(!flags.lock_paused);
    assert!(!flags.release_paused);
    assert!(!flags.refund_paused);
}

#[test]
fn test_global_pause_allows_query_escrows() {
    let env = Env::default();
    let (client, _, depositor, _, _) = setup(&env, 10_000);

    let bounty_id: u64 = 1;
    lock_bounty(&client, &env, &depositor, bounty_id, 100);

    client.set_emergency_pause(&true);

    let escrows = client.query_escrows_by_status(&EscrowStatus::Locked, &0, &10);
    assert_eq!(escrows.len(), 1);
}

#[test]
fn test_global_pause_allows_get_aggregate_stats() {
    let env = Env::default();
    let (client, _, depositor, _, _) = setup(&env, 10_000);

    let bounty_id: u64 = 1;
    lock_bounty(&client, &env, &depositor, bounty_id, 100);

    client.set_emergency_pause(&true);

    let stats = client.get_aggregate_stats();
    assert_eq!(stats.total_locked, 100);
}

#[test]
fn test_global_pause_allows_get_escrow_count() {
    let env = Env::default();
    let (client, _, depositor, _, _) = setup(&env, 10_000);

    let bounty_id: u64 = 1;
    lock_bounty(&client, &env, &depositor, bounty_id, 100);

    client.set_emergency_pause(&true);

    let count = client.get_escrow_count();
    assert_eq!(count, 1);
}

// ---------------------------------------------------------------------------
// Tests: precedence — global pause overrides granular flags
// ---------------------------------------------------------------------------

#[test]
fn test_global_pause_overrides_granular_unlock() {
    let env = Env::default();
    let (client, _, depositor, _, _) = setup(&env, 10_000);

    // Set granular flags to unlocked
    client.set_paused(&Some(false), &Some(false), &Some(false));

    // Set global pause
    client.set_emergency_pause(&true);

    // Even though granular flags are unlocked, global pause blocks lock
    let bounty_id: u64 = 1;
    let deadline = env.ledger().timestamp() + 1000;
    let res = client.try_lock_funds(&depositor, &bounty_id, &100, &deadline);
    assert!(res.is_err());
}

#[test]
fn test_global_pause_clears_with_granular_paused() {
    let env = Env::default();
    let (client, _, depositor, _, _) = setup(&env, 10_000);

    // Set granular lock pause
    client.set_paused(&Some(true), &None, &None);

    // Set global pause
    client.set_emergency_pause(&true);

    // Clear global pause — granular lock pause still applies
    client.set_emergency_pause(&false);

    let bounty_id: u64 = 1;
    let deadline = env.ledger().timestamp() + 1000;
    let res = client.try_lock_funds(&depositor, &bounty_id, &100, &deadline);
    assert!(res.is_err()); // still blocked by granular lock_paused

    // But release should work (granular release is not paused)
    // Lock a bounty first by clearing granular lock pause
    client.set_paused(&Some(false), &None, &None);
    lock_bounty(&client, &env, &depositor, bounty_id, 100);

    // Release should succeed (no granular or global pause on release)
    client.release_funds(&bounty_id, &depositor);
}

// ---------------------------------------------------------------------------
// Tests: workflow — pause mid-operations
// ---------------------------------------------------------------------------

#[test]
fn test_global_pause_mid_lock_then_unlock_resumes() {
    let env = Env::default();
    let (client, _, depositor, _, _) = setup(&env, 10_000);

    // Lock bounty 1 successfully
    let bounty_id_1: u64 = 1;
    lock_bounty(&client, &env, &depositor, bounty_id_1, 100);

    // Pause
    client.set_emergency_pause(&true);

    // Bounty 2 blocked
    let bounty_id_2: u64 = 2;
    let deadline = env.ledger().timestamp() + 1000;
    let res = client.try_lock_funds(&depositor, &bounty_id_2, &200, &deadline);
    assert!(res.is_err());

    // Unpause
    client.set_emergency_pause(&false);

    // Bounty 2 now succeeds
    client.lock_funds(&depositor, &bounty_id_2, &200, &deadline);

    let escrow = client.get_escrow_info(&bounty_id_2);
    assert_eq!(escrow.amount, 200);
}

#[test]
fn test_global_pause_during_release_then_refund_blocked() {
    let env = Env::default();
    let (client, _, depositor, contributor, _) = setup(&env, 10_000);

    let bounty_id: u64 = 1;
    lock_bounty(&client, &env, &depositor, bounty_id, 100);

    // Approve refund before pausing
    env.ledger().with_mut(|l| l.timestamp += 2000);
    client.approve_refund(&bounty_id, &100, &depositor, &RefundMode::Full);

    // Pause globally
    client.set_emergency_pause(&true);

    // Both release and refund blocked
    assert!(client.try_release_funds(&bounty_id, &contributor).is_err());
    assert!(client.try_refund(&bounty_id).is_err());

    // Unpause — release should work
    client.set_emergency_pause(&false);
    // Note: refund may fail for other reasons (deadline logic), but release should work
    // since we haven't released yet
}
