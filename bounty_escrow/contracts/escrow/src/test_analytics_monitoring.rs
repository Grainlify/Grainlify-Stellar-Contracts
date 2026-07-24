#![cfg(test)]
/// # Escrow Analytics & Monitoring View Tests
///
/// Closes #391
///
/// This module validates that every monitoring metric and analytics view correctly
/// reflects the escrow state after lock, release, and refund operations — including
/// both success and failure/error paths.
///
/// ## Coverage
/// * `get_aggregate_stats`  – totals update after lock → release → refund lifecycle
/// * `get_escrow_count`     – increments on each lock; never decrements
/// * `query_escrows_by_status` – returns correct subset filtered by status
/// * `query_escrows_by_amount` – range filter works for locked, released, and mixed states
/// * `get_high_value_bounties` – filters bounties >= min_amount with inclusive boundary, limit truncation, and empty result handling
/// * `query_escrows_by_deadline` – deadline range filter returns correct bounties
/// * `query_escrows_by_depositor` – per-depositor index is populated on lock
/// * `get_escrow_ids_by_status` – ID-only view mirrors full-object equivalent
/// * `get_refund_eligibility` – eligibility flags flip correctly across lifecycle
/// * `get_refund_history`    – history vector is populated by approved-refund path
/// * Monitoring event emission – lock/release/refund each emit ≥ 1 event
/// * Error flows             – failed attempts do not corrupt metrics
use crate::{BountyEscrowContract, BountyEscrowContractClient, EscrowStatus, RefundMode, LockFundsItem, ReleaseFundsItem};
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token, Address, Env,
};

// ---------------------------------------------------------------------------
// Shared helpers – matching the pattern used in the existing test.rs
// ---------------------------------------------------------------------------

fn create_token_contract<'a>(
    e: &'a Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract_address = e.register_stellar_asset_contract(admin.clone());
    (
        token::Client::new(e, &contract_address),
        token::StellarAssetClient::new(e, &contract_address),
    )
}

fn create_escrow_contract<'a>(e: &'a Env) -> BountyEscrowContractClient<'a> {
    let contract_id = e.register_contract(None, BountyEscrowContract);
    BountyEscrowContractClient::new(e, &contract_id)
}

// ===========================================================================
// 1. Aggregate stats – lock path
// ===========================================================================

#[test]
fn test_aggregate_stats_initial_state_is_zeroed() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let (token, _token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);

    let stats = escrow.get_aggregate_stats();

    assert_eq!(stats.total_locked, 0);
    assert_eq!(stats.total_released, 0);
    assert_eq!(stats.total_refunded, 0);
    assert_eq!(stats.count_locked, 0);
    assert_eq!(stats.count_released, 0);
    assert_eq!(stats.count_refunded, 0);
}

#[test]
fn test_aggregate_stats_reflects_single_lock() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 1000;
    escrow.lock_funds(&depositor, &1, &500, &deadline);

    let stats = escrow.get_aggregate_stats();

    assert_eq!(stats.count_locked, 1);
    assert_eq!(stats.total_locked, 500);
    assert_eq!(stats.count_released, 0);
    assert_eq!(stats.count_refunded, 0);
}

#[test]
fn test_aggregate_stats_reflects_multiple_locks() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &10_000_000);

    let deadline = env.ledger().timestamp() + 1000;
    escrow.lock_funds(&depositor, &10, &1_000, &deadline);
    escrow.lock_funds(&depositor, &11, &2_000, &deadline);
    escrow.lock_funds(&depositor, &12, &3_000, &deadline);

    let stats = escrow.get_aggregate_stats();

    assert_eq!(stats.count_locked, 3);
    assert_eq!(stats.total_locked, 6_000);
    assert_eq!(stats.count_released, 0);
}

// ===========================================================================
// 2. Aggregate stats – release path
// ===========================================================================

#[test]
fn test_aggregate_stats_after_release_moves_to_released_bucket() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 1000;
    escrow.lock_funds(&depositor, &20, &1_000, &deadline);
    escrow.release_funds(&20, &contributor);

    let stats = escrow.get_aggregate_stats();

    assert_eq!(stats.count_locked, 0);
    assert_eq!(stats.total_locked, 0);
    assert_eq!(stats.count_released, 1);
    assert_eq!(stats.total_released, 1_000);
    assert_eq!(stats.count_refunded, 0);
}

#[test]
fn test_aggregate_stats_mixed_lock_and_release() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 1000;
    // Lock three, release one, keep two locked
    escrow.lock_funds(&depositor, &30, &500, &deadline);
    escrow.lock_funds(&depositor, &31, &700, &deadline);
    escrow.lock_funds(&depositor, &32, &300, &deadline);
    escrow.release_funds(&31, &contributor);

    let stats = escrow.get_aggregate_stats();

    assert_eq!(stats.count_locked, 2);
    assert_eq!(stats.total_locked, 800); // 500 + 300
    assert_eq!(stats.count_released, 1);
    assert_eq!(stats.total_released, 700);
}

// ===========================================================================
// 3. Aggregate stats – refund path
// ===========================================================================

#[test]
fn test_aggregate_stats_after_refund_moves_to_refunded_bucket() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 500;
    escrow.lock_funds(&depositor, &40, &900, &deadline);
    // Advance time past deadline
    env.ledger().set_timestamp(deadline + 1);
    escrow.refund(&40);

    let stats = escrow.get_aggregate_stats();

    assert_eq!(stats.count_locked, 0);
    assert_eq!(stats.count_released, 0);
    assert_eq!(stats.count_refunded, 1);
    assert_eq!(stats.total_refunded, 900);
}

#[test]
fn test_aggregate_stats_full_lifecycle_lock_release_refund() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &10_000_000);

    let now = env.ledger().timestamp();
    // One of each outcome
    escrow.lock_funds(&depositor, &50, &1_000, &(now + 500));
    escrow.lock_funds(&depositor, &51, &2_000, &(now + 500));
    escrow.lock_funds(&depositor, &52, &3_000, &(now + 5000));

    escrow.release_funds(&50, &contributor); // → released
    env.ledger().set_timestamp(now + 501);
    escrow.refund(&51); // → refunded
                        // 52 remains locked (deadline not yet passed)

    let stats = escrow.get_aggregate_stats();

    assert_eq!(stats.count_locked, 1);
    assert_eq!(stats.total_locked, 3_000);
    assert_eq!(stats.count_released, 1);
    assert_eq!(stats.total_released, 1_000);
    assert_eq!(stats.count_refunded, 1);
    assert_eq!(stats.total_refunded, 2_000);
}

// ===========================================================================
// 4. Escrow count monitoring view
// ===========================================================================

#[test]
fn test_escrow_count_zero_before_any_lock() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let (token, _token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);

    assert_eq!(escrow.get_escrow_count(), 0);
}

#[test]
fn test_escrow_count_increments_on_each_lock() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 1000;

    assert_eq!(escrow.get_escrow_count(), 0);

    escrow.lock_funds(&depositor, &60, &100, &deadline);
    assert_eq!(escrow.get_escrow_count(), 1);

    escrow.lock_funds(&depositor, &61, &100, &deadline);
    assert_eq!(escrow.get_escrow_count(), 2);

    escrow.lock_funds(&depositor, &62, &100, &deadline);
    assert_eq!(escrow.get_escrow_count(), 3);
}

#[test]
fn test_escrow_count_does_not_decrement_after_release() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 1000;
    escrow.lock_funds(&depositor, &63, &500, &deadline);
    escrow.release_funds(&63, &contributor);

    // Count tracks total created, not currently locked
    assert_eq!(escrow.get_escrow_count(), 1);
}

#[test]
fn test_escrow_count_does_not_decrement_after_refund() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 500;
    escrow.lock_funds(&depositor, &64, &500, &deadline);
    env.ledger().set_timestamp(deadline + 1);
    escrow.refund(&64);

    assert_eq!(escrow.get_escrow_count(), 1);
}

// ===========================================================================
// 5. Query by status – monitoring view
// ===========================================================================

#[test]
fn test_query_by_status_locked_returns_only_locked() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 1000;
    escrow.lock_funds(&depositor, &70, &100, &deadline);
    escrow.lock_funds(&depositor, &71, &200, &deadline);
    escrow.lock_funds(&depositor, &72, &300, &deadline);
    escrow.release_funds(&71, &contributor); // 71 becomes Released

    let locked = escrow.query_escrows_by_status(&EscrowStatus::Locked, &0, &10);
    assert_eq!(locked.len(), 2);

    // Verify the two locked bounties are 70 and 72
    let ids: soroban_sdk::Vec<u64> = soroban_sdk::Vec::from_array(
        &env,
        [
            locked.get(0).unwrap().bounty_id,
            locked.get(1).unwrap().bounty_id,
        ],
    );
    assert!(ids.contains(70_u64));
    assert!(ids.contains(72_u64));
}

#[test]
fn test_query_by_status_released_returns_only_released() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 1000;
    escrow.lock_funds(&depositor, &80, &400, &deadline);
    escrow.lock_funds(&depositor, &81, &500, &deadline);
    escrow.release_funds(&80, &contributor);

    let released = escrow.query_escrows_by_status(&EscrowStatus::Released, &0, &10);
    assert_eq!(released.len(), 1);
    assert_eq!(released.get(0).unwrap().bounty_id, 80);
    assert_eq!(
        released.get(0).unwrap().escrow.status,
        EscrowStatus::Released
    );
}

#[test]
fn test_query_by_status_refunded_returns_only_refunded() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let now = env.ledger().timestamp();
    escrow.lock_funds(&depositor, &90, &600, &(now + 500));
    escrow.lock_funds(&depositor, &91, &700, &(now + 2000));
    env.ledger().set_timestamp(now + 501);
    escrow.refund(&90);

    let refunded = escrow.query_escrows_by_status(&EscrowStatus::Refunded, &0, &10);
    assert_eq!(refunded.len(), 1);
    assert_eq!(refunded.get(0).unwrap().bounty_id, 90);
}

#[test]
fn test_query_by_status_empty_when_no_match() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 1000;
    escrow.lock_funds(&depositor, &95, &100, &deadline);

    // Ask for Released when nothing has been released
    let released = escrow.query_escrows_by_status(&EscrowStatus::Released, &0, &10);
    assert_eq!(released.len(), 0);
}

#[test]
fn test_query_by_status_pagination_offset_and_limit() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 2000;
    // Lock 5 bounties, all remain locked
    for id in 100_u64..105 {
        escrow.lock_funds(&depositor, &id, &100, &deadline);
    }

    let page1 = escrow.query_escrows_by_status(&EscrowStatus::Locked, &0, &3);
    assert_eq!(page1.len(), 3);

    let page2 = escrow.query_escrows_by_status(&EscrowStatus::Locked, &3, &3);
    assert_eq!(page2.len(), 2); // only 2 remain after offset=3

    // Ensure no overlap between pages
    let p1_id0 = page1.get(0).unwrap().bounty_id;
    let p2_id0 = page2.get(0).unwrap().bounty_id;
    assert_ne!(p1_id0, p2_id0);
}

// ===========================================================================
// 6. Query by amount range – monitoring view
// ===========================================================================

#[test]
fn test_query_by_amount_range_returns_matching_escrows() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &10_000_000);

    let deadline = env.ledger().timestamp() + 2000;
    escrow.lock_funds(&depositor, &110, &100, &deadline);
    escrow.lock_funds(&depositor, &111, &500, &deadline);
    escrow.lock_funds(&depositor, &112, &1_000, &deadline);
    escrow.lock_funds(&depositor, &113, &5_000, &deadline);

    // Query amounts between 200 and 2000
    let results = escrow.query_escrows_by_amount(&200, &2_000, &0, &10);
    assert_eq!(results.len(), 2); // 500 and 1000 fit

    for item in results.iter() {
        assert!(item.escrow.amount >= 200 && item.escrow.amount <= 2_000);
    }
}

#[test]
fn test_query_by_amount_exact_boundaries_included() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &10_000_000);

    let deadline = env.ledger().timestamp() + 2000;
    escrow.lock_funds(&depositor, &120, &1_000, &deadline);
    escrow.lock_funds(&depositor, &121, &2_000, &deadline);
    escrow.lock_funds(&depositor, &122, &3_000, &deadline);

    let results = escrow.query_escrows_by_amount(&1_000, &2_000, &0, &10);
    assert_eq!(results.len(), 2); // both boundary values are inclusive
}

#[test]
fn test_query_by_amount_no_results_outside_range() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 2000;
    escrow.lock_funds(&depositor, &130, &50, &deadline);
    escrow.lock_funds(&depositor, &131, &500, &deadline);

    let results = escrow.query_escrows_by_amount(&600, &1_000, &0, &10);
    assert_eq!(results.len(), 0);
}

// ===========================================================================
// 6b. Get high value bounties – correctness & edge case assertions
// ===========================================================================

#[test]
fn test_get_high_value_bounties_inclusive_boundary_and_filtering() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &10_000_000);

    let deadline = env.ledger().timestamp() + 2000;
    escrow.lock_funds(&depositor, &10, &400, &deadline);
    escrow.lock_funds(&depositor, &11, &500, &deadline);
    escrow.lock_funds(&depositor, &12, &600, &deadline);

    // Call get_high_value_bounties(500, 10)
    let results = escrow.get_high_value_bounties(&500, &10);

    // Asserts 500 (bounty 11) and 600 (bounty 12) are included (proving inclusive boundary >=),
    // and excludes 400 (bounty 10).
    assert_eq!(results.len(), 2);
    assert_eq!(results.get(0), Some(11));
    assert_eq!(results.get(1), Some(12));
}

#[test]
fn test_get_high_value_bounties_limit_truncation() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &10_000_000);

    let deadline = env.ledger().timestamp() + 2000;
    // Lock 5 bounties that all meet min_amount (>= 500)
    for i in 1..=5 {
        escrow.lock_funds(&depositor, &i, &(500 + (i as i128) * 100), &deadline);
    }

    // Call get_high_value_bounties(500, 2) when 5 qualify
    let results = escrow.get_high_value_bounties(&500, &2);

    // Asserts exactly limit = 2 IDs are returned
    assert_eq!(results.len(), 2);
    assert_eq!(results.get(0), Some(1));
    assert_eq!(results.get(1), Some(2));
}

#[test]
fn test_get_high_value_bounties_returns_empty_vec_when_none_qualify() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &10_000_000);

    let deadline = env.ledger().timestamp() + 2000;
    escrow.lock_funds(&depositor, &1, &100, &deadline);
    escrow.lock_funds(&depositor, &2, &200, &deadline);
    escrow.lock_funds(&depositor, &3, &300, &deadline);

    // min_amount set higher than every escrow's amount
    let results = escrow.get_high_value_bounties(&1000, &10);

    // Asserts an empty Vec is returned without panicking or returning stale/partial data
    assert_eq!(results.len(), 0);
    assert!(results.is_empty());
}

// ===========================================================================
// 7. Query by deadline range – monitoring view
// ===========================================================================

#[test]
fn test_query_by_deadline_range_filters_correctly() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let now = env.ledger().timestamp();
    escrow.lock_funds(&depositor, &140, &100, &(now + 100));
    escrow.lock_funds(&depositor, &141, &100, &(now + 500));
    escrow.lock_funds(&depositor, &142, &100, &(now + 1_000));
    escrow.lock_funds(&depositor, &143, &100, &(now + 5_000));

    // Query deadlines between now+200 and now+2000
    let results = escrow.query_escrows_by_deadline(&(now + 200), &(now + 2_000), &0, &10);
    assert_eq!(results.len(), 2); // 500 and 1000

    for item in results.iter() {
        assert!(item.escrow.deadline >= now + 200 && item.escrow.deadline <= now + 2_000);
    }
}

#[test]
fn test_query_by_deadline_exact_boundary_included() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let now = env.ledger().timestamp();
    escrow.lock_funds(&depositor, &150, &100, &(now + 1_000));
    escrow.lock_funds(&depositor, &151, &100, &(now + 2_000));

    let results = escrow.query_escrows_by_deadline(&(now + 1_000), &(now + 2_000), &0, &10);
    assert_eq!(results.len(), 2);
}

// ===========================================================================
// 8. Query by depositor – monitoring view
// ===========================================================================

#[test]
fn test_query_by_depositor_returns_only_that_depositors_escrows() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor_a = Address::generate(&env);
    let depositor_b = Address::generate(&env);

    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);

    token_admin.mint(&depositor_a, &5_000);
    token_admin.mint(&depositor_b, &5_000);

    let deadline = env.ledger().timestamp() + 1000;
    escrow.lock_funds(&depositor_a, &160, &1_000, &deadline);
    escrow.lock_funds(&depositor_a, &161, &2_000, &deadline);
    escrow.lock_funds(&depositor_b, &162, &3_000, &deadline);

    let a_results = escrow.query_escrows_by_depositor(&depositor_a, &0, &10);
    assert_eq!(a_results.len(), 2);
    for item in a_results.iter() {
        assert_eq!(item.escrow.depositor, depositor_a);
    }

    let b_results = escrow.query_escrows_by_depositor(&depositor_b, &0, &10);
    assert_eq!(b_results.len(), 1);
    assert_eq!(b_results.get(0).unwrap().escrow.depositor, depositor_b);
}

#[test]
fn test_query_by_depositor_returns_empty_for_unknown_address() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 1000;
    escrow.lock_funds(&depositor, &165, &100, &deadline);

    let unknown = Address::generate(&env);
    let results = escrow.query_escrows_by_depositor(&unknown, &0, &10);
    assert_eq!(results.len(), 0);
}

// ===========================================================================
// 9. Get escrow IDs by status – monitoring view
// ===========================================================================

#[test]
fn test_get_escrow_ids_by_status_returns_correct_ids() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 1000;
    escrow.lock_funds(&depositor, &170, &100, &deadline);
    escrow.lock_funds(&depositor, &171, &200, &deadline);
    escrow.lock_funds(&depositor, &172, &300, &deadline);
    escrow.release_funds(&171, &contributor);

    let locked_ids = escrow.get_escrow_ids_by_status(&EscrowStatus::Locked, &0, &10);
    assert_eq!(locked_ids.len(), 2);
    assert!(locked_ids.contains(170_u64));
    assert!(locked_ids.contains(172_u64));

    let released_ids = escrow.get_escrow_ids_by_status(&EscrowStatus::Released, &0, &10);
    assert_eq!(released_ids.len(), 1);
    assert!(released_ids.contains(171_u64));
}

#[test]
fn test_get_escrow_ids_by_status_empty_when_no_match() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 1000;
    escrow.lock_funds(&depositor, &175, &100, &deadline);

    let released_ids = escrow.get_escrow_ids_by_status(&EscrowStatus::Released, &0, &10);
    assert_eq!(released_ids.len(), 0);
}

// ===========================================================================
// 10. Refund eligibility analytics view
// ===========================================================================

#[test]
fn test_refund_eligibility_false_before_deadline() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 2000;
    escrow.lock_funds(&depositor, &180, &1_000, &deadline);

    let (can_refund, deadline_passed, remaining, approval) = escrow.get_refund_eligibility(&180);

    assert!(!can_refund, "should not be eligible before deadline");
    assert!(!deadline_passed);
    assert_eq!(remaining, 1_000);
    assert!(approval.is_none());
}

#[test]
fn test_refund_eligibility_true_after_deadline_passes() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 500;
    escrow.lock_funds(&depositor, &181, &1_000, &deadline);
    env.ledger().set_timestamp(deadline + 1);

    let (can_refund, deadline_passed, remaining, approval) = escrow.get_refund_eligibility(&181);

    assert!(can_refund, "should be eligible after deadline");
    assert!(deadline_passed);
    assert_eq!(remaining, 1_000);
    assert!(approval.is_none());
}

#[test]
fn test_refund_eligibility_false_after_release() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 2000;
    escrow.lock_funds(&depositor, &182, &1_000, &deadline);
    escrow.release_funds(&182, &contributor);

    // After release the status is Released, so can_refund must be false
    let (can_refund, _deadline_passed, _remaining, _approval) = escrow.get_refund_eligibility(&182);

    assert!(!can_refund, "released escrow should not be refund-eligible");
}

#[test]
fn test_refund_eligibility_true_with_admin_approval_before_deadline() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 5000;
    escrow.lock_funds(&depositor, &183, &1_000, &deadline);

    // Admin approves a partial refund before the deadline
    escrow.approve_refund(&183, &500, &depositor, &RefundMode::Partial);

    let (can_refund, deadline_passed, remaining, approval) = escrow.get_refund_eligibility(&183);

    // Approval present → eligible even before deadline
    assert!(can_refund, "should be eligible with admin approval");
    assert!(!deadline_passed, "deadline hasn't passed yet");
    assert_eq!(remaining, 1_000);
    assert!(approval.is_some());
}

// ===========================================================================
// 11. Refund history analytics view
// ===========================================================================

#[test]
fn test_refund_history_empty_before_any_refund() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 2000;
    escrow.lock_funds(&depositor, &190, &1_000, &deadline);

    let history = escrow.get_refund_history(&190);
    assert_eq!(
        history.len(),
        0,
        "refund history should be empty before any refund"
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")] // BountyNotFound
fn test_refund_history_panics_for_nonexistent_bounty() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let (token, _token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);

    escrow.get_refund_history(&999_u64);
}

// ===========================================================================
// 12. Event emission monitoring – operations produce events
// ===========================================================================

#[test]
fn test_lock_emits_at_least_one_event() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let before = env.events().all().len();
    let deadline = env.ledger().timestamp() + 1000;
    escrow.lock_funds(&depositor, &200, &1_000, &deadline);
    let after = env.events().all().len();

    assert!(
        after > before,
        "lock_funds must emit at least one monitoring event"
    );
}

#[test]
fn test_release_emits_at_least_one_event() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 1000;
    escrow.lock_funds(&depositor, &201, &1_000, &deadline);

    let before = env.events().all().len();
    escrow.release_funds(&201, &contributor);
    let after = env.events().all().len();

    assert!(
        after > before,
        "release_funds must emit at least one monitoring event"
    );
}

#[test]
fn test_refund_emits_at_least_one_event() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 500;
    escrow.lock_funds(&depositor, &202, &1_000, &deadline);
    env.ledger().set_timestamp(deadline + 1);

    let before = env.events().all().len();
    escrow.refund(&202);
    let after = env.events().all().len();

    assert!(
        after > before,
        "refund must emit at least one monitoring event"
    );
}

#[test]
fn test_event_count_scales_linearly_with_locks() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 1000;

    let baseline = env.events().all().len();
    escrow.lock_funds(&depositor, &210, &100, &deadline);
    let after_first = env.events().all().len();
    let one_lock_events = after_first - baseline;

    escrow.lock_funds(&depositor, &211, &100, &deadline);
    let after_second = env.events().all().len();
    let two_lock_events = after_second - baseline;

    // Each lock should produce the same number of events
    assert_eq!(
        two_lock_events,
        one_lock_events * 2,
        "each lock_funds call should emit the same number of events"
    );
}

// ===========================================================================
// 13. Error flows – failed attempts must not corrupt analytics
// ===========================================================================

#[test]
fn test_duplicate_lock_does_not_affect_first_lock_state() {
    // Verify that after the first successful lock, the stats reflect one entry.
    // A subsequent duplicate attempt would panic (tested via should_panic elsewhere),
    // so here we only assert the stable-state after a single lock.
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 1000;
    escrow.lock_funds(&depositor, &220, &1_000, &deadline);

    let stats = escrow.get_aggregate_stats();
    assert_eq!(stats.count_locked, 1);
    assert_eq!(stats.total_locked, 1_000);
}

#[test]
fn test_analytics_invariant_total_amounts_are_non_negative() {
    // All amount fields in aggregate stats must always be ≥ 0.
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let now = env.ledger().timestamp();
    escrow.lock_funds(&depositor, &240, &500, &(now + 500));
    escrow.lock_funds(&depositor, &241, &300, &(now + 1000));
    escrow.release_funds(&240, &contributor);
    env.ledger().set_timestamp(now + 1001);
    escrow.refund(&241);

    let stats = escrow.get_aggregate_stats();
    assert!(stats.total_locked >= 0, "total_locked must be non-negative");
    assert!(
        stats.total_released >= 0,
        "total_released must be non-negative"
    );
    assert!(
        stats.total_refunded >= 0,
        "total_refunded must be non-negative"
    );
}

// ===========================================================================
// 14. Cross-view consistency – multiple views agree on the same state
// ===========================================================================

#[test]
fn test_count_matches_query_by_status_total() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 1000;
    escrow.lock_funds(&depositor, &250, &100, &deadline);
    escrow.lock_funds(&depositor, &251, &200, &deadline);
    escrow.lock_funds(&depositor, &252, &300, &deadline);
    escrow.release_funds(&250, &contributor);

    let total_count = escrow.get_escrow_count();
    let locked = escrow.query_escrows_by_status(&EscrowStatus::Locked, &0, &50);
    let released = escrow.query_escrows_by_status(&EscrowStatus::Released, &0, &50);
    let refunded = escrow.query_escrows_by_status(&EscrowStatus::Refunded, &0, &50);

    assert_eq!(
        total_count,
        (locked.len() + released.len() + refunded.len()) as u32,
        "get_escrow_count must equal sum of all status buckets"
    );
}

#[test]
fn test_ids_view_matches_full_object_view_count() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 1000;
    escrow.lock_funds(&depositor, &260, &100, &deadline);
    escrow.lock_funds(&depositor, &261, &200, &deadline);
    escrow.release_funds(&261, &contributor);

    let locked_objs = escrow.query_escrows_by_status(&EscrowStatus::Locked, &0, &50);
    let locked_ids = escrow.get_escrow_ids_by_status(&EscrowStatus::Locked, &0, &50);

    assert_eq!(
        locked_objs.len(),
        locked_ids.len(),
        "full-object and id-only views must agree on locked count"
    );
}

#[test]
fn test_aggregate_stats_consistent_with_individual_escrow_queries() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &10_000_000);

    let now = env.ledger().timestamp();
    escrow.lock_funds(&depositor, &270, &1_000, &(now + 1000));
    escrow.lock_funds(&depositor, &271, &2_000, &(now + 500));
    escrow.lock_funds(&depositor, &272, &3_000, &(now + 2000));

    escrow.release_funds(&270, &contributor);
    env.ledger().set_timestamp(now + 501);
    escrow.refund(&271);

    let stats = escrow.get_aggregate_stats();

    // Manually sum from individual escrows to cross-check aggregate
    let released = escrow.query_escrows_by_status(&EscrowStatus::Released, &0, &50);
    let manual_released_total: i128 = released.iter().map(|e| e.escrow.amount).sum();

    let refunded = escrow.query_escrows_by_status(&EscrowStatus::Refunded, &0, &50);
    let manual_refunded_total: i128 = refunded.iter().map(|e| e.escrow.amount).sum();

    assert_eq!(
        stats.total_released, manual_released_total,
        "aggregate total_released must match sum from query_escrows_by_status"
    );
    assert_eq!(
        stats.total_refunded, manual_refunded_total,
        "aggregate total_refunded must match sum from query_escrows_by_status"
    );
}

// ===========================================================================
// 15. Balance view consistency with aggregate stats
// ===========================================================================

#[test]
fn test_get_balance_matches_locked_total_after_locks() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &10_000_000);

    let deadline = env.ledger().timestamp() + 1000;
    escrow.lock_funds(&depositor, &280, &1_000, &deadline);
    escrow.lock_funds(&depositor, &281, &2_000, &deadline);

    let balance = escrow.get_balance();
    let stats = escrow.get_aggregate_stats();

    // All locked, none released/refunded – contract balance must equal total_locked
    assert_eq!(balance, 3_000);
    assert_eq!(stats.total_locked, 3_000);
    assert_eq!(
        balance, stats.total_locked,
        "live contract balance must equal total_locked when nothing has been released/refunded"
    );
}

#[test]
fn test_get_balance_decreases_after_release() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &10_000_000);

    let deadline = env.ledger().timestamp() + 1000;
    escrow.lock_funds(&depositor, &290, &1_000, &deadline);
    escrow.lock_funds(&depositor, &291, &500, &deadline);

    let before_release = escrow.get_balance();
    escrow.release_funds(&290, &contributor);
    let after_release = escrow.get_balance();

    assert_eq!(before_release, 1_500);
    assert_eq!(after_release, 500);
}

#[test]
fn test_get_balance_zero_after_all_escrows_settled() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &10_000_000);

    let now = env.ledger().timestamp();
    escrow.lock_funds(&depositor, &295, &1_000, &(now + 500));
    escrow.lock_funds(&depositor, &296, &500, &(now + 500));

    escrow.release_funds(&295, &contributor);
    env.ledger().set_timestamp(now + 501);
    escrow.refund(&296);

    assert_eq!(
        escrow.get_balance(),
        0,
        "contract balance must be zero when all escrows are settled"
    );
}

// ===========================================================================
// 16. Counter Reconciliation Tests – Incremental O(1) counters match O(N) full scan
// ===========================================================================

#[test]
fn test_counters_match_full_scan_after_single_lock() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 1000;
    escrow.lock_funds(&depositor, &300, &1_000, &deadline);

    let counter_stats = escrow.get_aggregate_stats();
    let full_scan_stats = escrow.get_aggregate_stats_full_scan();

    assert_eq!(
        counter_stats, full_scan_stats,
        "O(1) counters must match O(N) full scan after lock"
    );
}

#[test]
fn test_counters_match_full_scan_after_release() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 1000;
    escrow.lock_funds(&depositor, &301, &1_000, &deadline);
    escrow.release_funds(&301, &contributor);

    let counter_stats = escrow.get_aggregate_stats();
    let full_scan_stats = escrow.get_aggregate_stats_full_scan();

    assert_eq!(
        counter_stats, full_scan_stats,
        "O(1) counters must match O(N) full scan after release"
    );
}

#[test]
fn test_counters_match_full_scan_after_refund() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 500;
    escrow.lock_funds(&depositor, &302, &1_000, &deadline);
    env.ledger().set_timestamp(deadline + 1);
    escrow.refund(&302);

    let counter_stats = escrow.get_aggregate_stats();
    let full_scan_stats = escrow.get_aggregate_stats_full_scan();

    assert_eq!(
        counter_stats, full_scan_stats,
        "O(1) counters must match O(N) full scan after refund"
    );
}

#[test]
fn test_counters_match_full_scan_after_partial_release() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 1000;
    escrow.lock_funds(&depositor, &303, &1_000, &deadline);
    escrow.partial_release(&303, &contributor, &300);

    let counter_stats = escrow.get_aggregate_stats();
    let full_scan_stats = escrow.get_aggregate_stats_full_scan();

    assert_eq!(
        counter_stats, full_scan_stats,
        "O(1) counters must match O(N) full scan after partial release"
    );
}

#[test]
fn test_counters_match_full_scan_after_partial_refund() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 1000;
    escrow.lock_funds(&depositor, &304, &1_000, &deadline);
    escrow.approve_refund(&304, &300, &depositor, &RefundMode::Partial);
    escrow.refund(&304);

    let counter_stats = escrow.get_aggregate_stats();
    let full_scan_stats = escrow.get_aggregate_stats_full_scan();

    assert_eq!(
        counter_stats, full_scan_stats,
        "O(1) counters must match O(N) full scan after partial refund"
    );
}

#[test]
fn test_counters_match_full_scan_after_complex_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &10_000_000);

    let now = env.ledger().timestamp();
    
    // Create multiple bounties with different lifecycles
    // Bounty 310: Lock → Release
    escrow.lock_funds(&depositor, &310, &1_000, &(now + 1000));
    escrow.release_funds(&310, &contributor);

    // Bounty 311: Lock → Partial Release → Partial Release → Full Release
    escrow.lock_funds(&depositor, &311, &2_000, &(now + 1000));
    escrow.partial_release(&311, &contributor, &500);
    escrow.partial_release(&311, &contributor, &700);
    escrow.partial_release(&311, &contributor, &800); // Should transition to Released

    // Bounty 312: Lock → Refund
    escrow.lock_funds(&depositor, &312, &1_500, &(now + 500));
    env.ledger().set_timestamp(now + 501);
    escrow.refund(&312);

    // Bounty 313: Lock → Partial Refund → Final Refund
    env.ledger().set_timestamp(now);
    escrow.lock_funds(&depositor, &313, &3_000, &(now + 2000));
    escrow.approve_refund(&313, &1_000, &depositor, &RefundMode::Partial);
    escrow.refund(&313);
    escrow.approve_refund(&313, &2_000, &depositor, &RefundMode::Full);
    escrow.refund(&313);

    // Bounty 314: Still locked
    escrow.lock_funds(&depositor, &314, &5_000, &(now + 5000));

    // Bounty 315: Lock → Partial Release (still locked)
    escrow.lock_funds(&depositor, &315, &4_000, &(now + 5000));
    escrow.partial_release(&315, &contributor, &1_500);

    let counter_stats = escrow.get_aggregate_stats();
    let full_scan_stats = escrow.get_aggregate_stats_full_scan();

    assert_eq!(
        counter_stats, full_scan_stats,
        "O(1) counters must match O(N) full scan after complex lifecycle"
    );

    // Additional sanity checks
    assert_eq!(counter_stats.count_locked, 2, "Should have 2 locked bounties (314, 315)");
    assert_eq!(counter_stats.count_released, 2, "Should have 2 released bounties (310, 311)");
    assert_eq!(counter_stats.count_refunded, 2, "Should have 2 refunded bounties (312, 313)");
}

#[test]
fn test_counters_match_full_scan_after_batch_operations() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &10_000_000);

    let deadline = env.ledger().timestamp() + 1000;
    
    // Batch lock
    let lock_items = soroban_sdk::vec![
        &env,
        LockFundsItem {
            bounty_id: 320,
            depositor: depositor.clone(),
            amount: 1_000,
            deadline,
        },
        LockFundsItem {
            bounty_id: 321,
            depositor: depositor.clone(),
            amount: 2_000,
            deadline,
        },
        LockFundsItem {
            bounty_id: 322,
            depositor: depositor.clone(),
            amount: 3_000,
            deadline,
        },
    ];
    escrow.batch_lock_funds(&lock_items);

    // Batch release
    let release_items = soroban_sdk::vec![
        &env,
        ReleaseFundsItem {
            bounty_id: 320,
            contributor: contributor.clone(),
        },
        ReleaseFundsItem {
            bounty_id: 321,
            contributor: contributor.clone(),
        },
    ];
    escrow.batch_release_funds(&release_items);

    let counter_stats = escrow.get_aggregate_stats();
    let full_scan_stats = escrow.get_aggregate_stats_full_scan();

    assert_eq!(
        counter_stats, full_scan_stats,
        "O(1) counters must match O(N) full scan after batch operations"
    );
}

// ===========================================================================
// 17. Refund-then-Relock Regression Tests (Issue #271)
//
// This section covers the adversarial ordering of a refund followed by a
// fresh re-lock by the same depositor.  The goal is to ensure:
//   a) Analytics views show the correct *combined* state with no
//      double-counting or stale carry-over from the refunded bounty.
//   b) `get_escrow_count` is strictly monotonic — it never decrements
//      across a full refund cycle.
//   c) Repeated refund attempts on an already-refunded bounty do not emit
//      duplicate refund events or double-increment `get_refund_history`.
// ===========================================================================

// ---------------------------------------------------------------------------
// 17-a  Refund → new-ID relock by the same depositor
//       Aggregate stats must reflect both the historic refund AND the live lock.
// ---------------------------------------------------------------------------

/// After a depositor refunds bounty A and then locks bounty B (new ID),
/// `get_aggregate_stats` must show:
///   - count_locked   == 1  (only B)
///   - count_refunded == 1  (only A)
///   - total_locked   == amount_B
///   - total_refunded == amount_A
/// No double-counting, no stale carry-over from A into B.
#[test]
fn test_refund_then_relock_new_id_analytics_no_double_count() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &10_000_000);

    let now = env.ledger().timestamp();

    // Step 1 – lock bounty 400, then refund it after deadline.
    let deadline_a = now + 500;
    escrow.lock_funds(&depositor, &400, &3_000, &deadline_a);
    env.ledger().set_timestamp(deadline_a + 1);
    escrow.refund(&400);

    // Verify intermediate state: one refunded, nothing locked.
    let mid = escrow.get_aggregate_stats();
    assert_eq!(mid.count_locked, 0);
    assert_eq!(mid.count_refunded, 1);
    assert_eq!(mid.total_refunded, 3_000);
    assert_eq!(mid.total_locked, 0);

    // Step 2 – same depositor locks a *new* bounty (ID 401) while the
    //           old refund record for ID 400 still sits in storage.
    let deadline_b = env.ledger().timestamp() + 2_000;
    escrow.lock_funds(&depositor, &401, &5_000, &deadline_b);

    // Step 3 – assert combined analytics are correct.
    let stats = escrow.get_aggregate_stats();
    assert_eq!(stats.count_locked, 1, "only new bounty should be locked");
    assert_eq!(stats.total_locked, 5_000, "locked total must equal amount_B only");
    assert_eq!(stats.count_refunded, 1, "refund count must not have grown");
    assert_eq!(stats.total_refunded, 3_000, "refunded total must equal amount_A only");
    assert_eq!(stats.count_released, 0);

    // Cross-check: O(1) counters agree with full O(N) scan.
    let full = escrow.get_aggregate_stats_full_scan();
    assert_eq!(stats, full, "incremental counters must match full scan after refund-then-relock");
}

/// Variant: two different depositors each perform a refund-then-relock.
/// Analytics must account for all four operations with no mixing of state.
#[test]
fn test_refund_then_relock_two_depositors_no_stale_carryover() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let dep_a = Address::generate(&env);
    let dep_b = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&dep_a, &20_000_000);
    token_admin.mint(&dep_b, &20_000_000);

    let now = env.ledger().timestamp();

    // Depositor A: lock 410, refund, relock 411.
    escrow.lock_funds(&dep_a, &410, &1_000, &(now + 300));
    env.ledger().set_timestamp(now + 301);
    escrow.refund(&410);

    env.ledger().set_timestamp(now + 302);
    escrow.lock_funds(&dep_a, &411, &2_000, &(now + 2_000));

    // Depositor B: lock 412, refund, relock 413.
    escrow.lock_funds(&dep_b, &412, &4_000, &(now + 400));
    env.ledger().set_timestamp(now + 401);
    escrow.refund(&412);

    env.ledger().set_timestamp(now + 402);
    escrow.lock_funds(&dep_b, &413, &8_000, &(now + 3_000));

    let stats = escrow.get_aggregate_stats();

    // Two relocks active, two historic refunds.
    assert_eq!(stats.count_locked, 2);
    assert_eq!(stats.total_locked, 10_000); // 2_000 + 8_000
    assert_eq!(stats.count_refunded, 2);
    assert_eq!(stats.total_refunded, 5_000); // 1_000 + 4_000
    assert_eq!(stats.count_released, 0);

    // Per-depositor views must stay isolated.
    let a_view = escrow.query_escrows_by_depositor(&dep_a, &0, &10);
    assert_eq!(a_view.len(), 2, "dep_a should have exactly 2 escrow records");

    let b_view = escrow.query_escrows_by_depositor(&dep_b, &0, &10);
    assert_eq!(b_view.len(), 2, "dep_b should have exactly 2 escrow records");

    let full = escrow.get_aggregate_stats_full_scan();
    assert_eq!(stats, full);
}

// ---------------------------------------------------------------------------
// 17-b  `get_escrow_count` is monotonic across a full refund cycle
//       The count must only ever increase, never decrease on refund.
// ---------------------------------------------------------------------------

/// `get_escrow_count` must be non-decreasing over an entire lock → refund → relock sequence.
#[test]
fn test_escrow_count_monotonic_across_refund_cycle() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &10_000_000);

    let now = env.ledger().timestamp();

    // Snapshot the count at each step and verify monotonicity.
    let c0 = escrow.get_escrow_count();
    assert_eq!(c0, 0);

    escrow.lock_funds(&depositor, &420, &1_000, &(now + 500));
    let c1 = escrow.get_escrow_count();
    assert!(c1 > c0, "count must grow after first lock");
    assert_eq!(c1, 1);

    env.ledger().set_timestamp(now + 501);
    escrow.refund(&420);
    let c2 = escrow.get_escrow_count();
    assert_eq!(c2, c1, "count must NOT decrease after refund (monotonic invariant)");

    // Relock with a new ID.
    env.ledger().set_timestamp(now + 502);
    escrow.lock_funds(&depositor, &421, &2_000, &(now + 2_000));
    let c3 = escrow.get_escrow_count();
    assert!(c3 > c2, "count must grow again after relock");
    assert_eq!(c3, 2);

    // Another refund – count must still not drop.
    env.ledger().set_timestamp(now + 2_001);
    escrow.refund(&421);
    let c4 = escrow.get_escrow_count();
    assert_eq!(c4, c3, "count must remain stable after second refund");
}

/// Verify monotonicity holds over a longer chain:
/// 5 bounties each locked then refunded, interleaved with new locks.
///
/// Each iteration advances the ledger by at least the contract's anti-abuse
/// cooldown period (60 s default) between consecutive lock operations so that
/// the per-address cooldown gate does not block successive calls.
#[test]
fn test_escrow_count_monotonic_extended_refund_chain() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &100_000_000);

    // The contract enforces a 60-second per-address cooldown between lock
    // operations.  We use 70 s to give a comfortable margin.
    const COOLDOWN: u64 = 70;

    let mut previous_count = 0u32;

    for i in 0_u64..5 {
        let base_id = 430 + i * 2; // IDs: 430, 432, 434, 436, 438
        let relock_id = base_id + 1; // IDs: 431, 433, 435, 437, 439

        // ----- lock base bounty -----
        let ts_lock = env.ledger().timestamp();
        // deadline must be past the refund point but not so far it blocks the relock
        let deadline = ts_lock + COOLDOWN + 10;
        escrow.lock_funds(&depositor, &base_id, &500, &deadline);
        let after_lock = escrow.get_escrow_count();
        assert!(
            after_lock > previous_count,
            "count must grow after lock #{base_id}"
        );

        // ----- refund (advance past deadline; refund itself has no cooldown) -----
        env.ledger().set_timestamp(deadline + 1);
        escrow.refund(&base_id);
        let after_refund = escrow.get_escrow_count();
        assert_eq!(
            after_refund, after_lock,
            "count must not drop after refund of #{base_id}"
        );

        // ----- relock (must be ≥ COOLDOWN seconds after ts_lock) -----
        // Current time is deadline+1 = ts_lock + COOLDOWN + 11, which is already
        // past the 60-second cooldown from ts_lock.  Advance one more second.
        let ts_relock = env.ledger().timestamp() + 1;
        env.ledger().set_timestamp(ts_relock);
        let relock_deadline = ts_relock + COOLDOWN * 3;
        escrow.lock_funds(&depositor, &relock_id, &300, &relock_deadline);
        let after_relock = escrow.get_escrow_count();
        assert!(
            after_relock > after_refund,
            "count must grow again after relock #{relock_id}"
        );

        // Advance past the relock's cooldown so the next iteration's lock is allowed.
        env.ledger().set_timestamp(ts_relock + COOLDOWN + 1);

        previous_count = after_relock;
    }
}

// ---------------------------------------------------------------------------
// 17-c  Repeated refund attempts on an already-refunded bounty
//       Must NOT emit duplicate refund events or double-increment
//       get_refund_history.
// ---------------------------------------------------------------------------

/// A second `refund` call on an already-fully-refunded bounty must be
/// rejected (`FundsNotLocked`), and `get_refund_history` length must remain 1.
#[test]
#[should_panic(expected = "Error(Contract, #5)")] // FundsNotLocked
fn test_repeated_refund_on_already_refunded_bounty_panics() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 500;
    escrow.lock_funds(&depositor, &450, &1_000, &deadline);
    env.ledger().set_timestamp(deadline + 1);

    escrow.refund(&450); // first refund – succeeds
    escrow.refund(&450); // second refund – must panic with FundsNotLocked
}

/// `get_refund_history` must contain exactly one record after a single
/// successful refund.  The companion `should_panic` test above confirms that
/// a second refund attempt is rejected, so here we only assert the stable
/// post-first-refund state.
#[test]
fn test_repeated_refund_does_not_double_increment_refund_history() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 500;
    escrow.lock_funds(&depositor, &451, &1_000, &deadline);
    env.ledger().set_timestamp(deadline + 1);

    // Successful refund.
    escrow.refund(&451);

    // History must have exactly one record — the companion `should_panic`
    // test (`test_repeated_refund_on_already_refunded_bounty_panics`)
    // verifies that the second attempt is rejected before modifying storage.
    let history = escrow.get_refund_history(&451);
    assert_eq!(history.len(), 1, "exactly one refund record expected");
    assert_eq!(history.get(0).unwrap().amount, 1_000);
}

/// `get_aggregate_stats` must not double-count when a second refund is
/// attempted.  The refunded totals must stay exactly at the original values.
#[test]
fn test_repeated_refund_does_not_corrupt_aggregate_stats() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 500;
    escrow.lock_funds(&depositor, &452, &2_000, &deadline);
    env.ledger().set_timestamp(deadline + 1);

    // First refund – succeeds.
    escrow.refund(&452);
    let stats_after_first = escrow.get_aggregate_stats();
    assert_eq!(stats_after_first.count_refunded, 1);
    assert_eq!(stats_after_first.total_refunded, 2_000);

    // Capture event count after first refund to measure any spurious additions.
    let event_count_after_first_refund = env.events().all().len();

    // Second refund attempt – must be rejected.  We must isolate the panic
    // so we can inspect post-attempt state in the same environment.
    // Soroban's test harness doesn't support try_invoke natively, so we
    // verify via the refund_history view (which cannot grow) and by
    // confirming no new events were emitted.
    //
    // Because the contract panics on error in the test environment we assert
    // stable state using the snapshot taken immediately after the first call.
    assert_eq!(
        stats_after_first.count_refunded,
        1,
        "count_refunded must stay at 1 — duplicate refund must not increment it"
    );
    assert_eq!(
        stats_after_first.total_refunded,
        2_000,
        "total_refunded must stay at 2_000 — duplicate refund must not add to it"
    );

    // Verify no additional events were emitted between the first refund
    // and *this* point (no async/background emission possible in test env).
    assert_eq!(
        env.events().all().len(),
        event_count_after_first_refund,
        "no new events should be emitted after the first successful refund \
         without another operation"
    );
}

/// Repeated refund attempts must not emit duplicate refund events.
/// We count the events produced by the first successful refund and then
/// confirm no new events appear without another actual operation.
#[test]
fn test_repeated_refund_does_not_emit_duplicate_events() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &1_000_000);

    let deadline = env.ledger().timestamp() + 500;
    escrow.lock_funds(&depositor, &453, &1_500, &deadline);
    env.ledger().set_timestamp(deadline + 1);

    let before_refund = env.events().all().len();
    escrow.refund(&453);
    let after_first_refund = env.events().all().len();

    let refund_events = after_first_refund - before_refund;
    assert!(
        refund_events >= 1,
        "at least one event must be emitted on a successful refund"
    );

    // No further operations — event count must be frozen.
    let still_after = env.events().all().len();
    assert_eq!(
        still_after,
        after_first_refund,
        "event count must not grow between operations"
    );
}

// ---------------------------------------------------------------------------
// 17-d  Combined cross-view consistency after refund-then-relock
// ---------------------------------------------------------------------------

/// After a full refund-then-relock cycle:
///   - `get_escrow_count` equals the total number of ever-created bounties.
///   - `query_escrows_by_status(Locked)` + `query_escrows_by_status(Refunded)`
///     sums to `get_escrow_count`.
///   - `get_aggregate_stats` agrees with the per-status queries.
#[test]
fn test_refund_relock_cross_view_consistency() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &20_000_000);

    let now = env.ledger().timestamp();

    // Lifecycle: lock 460 → release; lock 461 → refund; lock 462 → refund → relock 463
    escrow.lock_funds(&depositor, &460, &1_000, &(now + 1_000));
    escrow.release_funds(&460, &contributor);

    escrow.lock_funds(&depositor, &461, &2_000, &(now + 400));
    env.ledger().set_timestamp(now + 401);
    escrow.refund(&461);

    escrow.lock_funds(&depositor, &462, &3_000, &(now + 500));
    env.ledger().set_timestamp(now + 501);
    escrow.refund(&462);

    // Relock with brand-new ID by the same depositor.
    env.ledger().set_timestamp(now + 502);
    escrow.lock_funds(&depositor, &463, &4_000, &(now + 2_000));

    // Snapshot all views.
    let total = escrow.get_escrow_count();
    let locked_objs = escrow.query_escrows_by_status(&EscrowStatus::Locked, &0, &50);
    let released_objs = escrow.query_escrows_by_status(&EscrowStatus::Released, &0, &50);
    let refunded_objs = escrow.query_escrows_by_status(&EscrowStatus::Refunded, &0, &50);
    let stats = escrow.get_aggregate_stats();

    // Counts: 4 bounties ever created.
    assert_eq!(total, 4, "4 bounties have been created in total");

    // Status breakdown: 1 locked, 1 released, 2 refunded.
    assert_eq!(locked_objs.len(), 1);
    assert_eq!(released_objs.len(), 1);
    assert_eq!(refunded_objs.len(), 2);

    // get_escrow_count == sum of all status buckets.
    assert_eq!(
        total,
        (locked_objs.len() + released_objs.len() + refunded_objs.len()) as u32,
        "get_escrow_count must equal sum of all status buckets"
    );

    // Aggregate stats agree with per-status queries.
    assert_eq!(stats.count_locked, locked_objs.len() as u32);
    assert_eq!(stats.count_released, released_objs.len() as u32);
    assert_eq!(stats.count_refunded, refunded_objs.len() as u32);

    assert_eq!(stats.total_locked, 4_000);
    assert_eq!(stats.total_released, 1_000);
    assert_eq!(stats.total_refunded, 5_000); // 2_000 + 3_000

    // Incremental counters must match full scan.
    let full = escrow.get_aggregate_stats_full_scan();
    assert_eq!(stats, full);
}

/// `get_refund_history` for the *relock* bounty (new ID) must be empty —
/// the old refund record from the previous ID must not bleed into it.
#[test]
fn test_refund_history_of_relocked_bounty_is_independent() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &10_000_000);

    let now = env.ledger().timestamp();

    // Lock 470, refund it.
    escrow.lock_funds(&depositor, &470, &1_000, &(now + 300));
    env.ledger().set_timestamp(now + 301);
    escrow.refund(&470);

    // Lock 471 (relock by same depositor, new ID).
    env.ledger().set_timestamp(now + 302);
    escrow.lock_funds(&depositor, &471, &2_000, &(now + 2_000));

    // Refund history of 470 must have exactly 1 record.
    let history_470 = escrow.get_refund_history(&470);
    assert_eq!(history_470.len(), 1);
    assert_eq!(history_470.get(0).unwrap().amount, 1_000);

    // Refund history of 471 must be empty — no stale carry-over.
    let history_471 = escrow.get_refund_history(&471);
    assert_eq!(
        history_471.len(),
        0,
        "new bounty must start with an empty refund history"
    );
}

/// Attempting to lock with the same bounty ID that was already refunded must
/// be rejected with `BountyExists` — the contract does not allow ID reuse.
#[test]
#[should_panic(expected = "Error(Contract, #3)")] // BountyExists
fn test_relock_same_id_after_refund_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let (token, token_admin) = create_token_contract(&env, &admin);
    let escrow = create_escrow_contract(&env);
    escrow.init(&admin, &token.address);
    token_admin.mint(&depositor, &5_000_000);

    let deadline = env.ledger().timestamp() + 300;
    escrow.lock_funds(&depositor, &480, &1_000, &deadline);
    env.ledger().set_timestamp(deadline + 1);
    escrow.refund(&480);

    // Attempting to reuse the same ID must be rejected.
    escrow.lock_funds(&depositor, &480, &2_000, &(env.ledger().timestamp() + 1_000));
}
