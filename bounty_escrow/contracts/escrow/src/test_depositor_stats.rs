#![cfg(test)]
/// # `get_depositor_stats` Correctness Tests
///
/// Closes #315
///
/// `get_depositor_stats(depositor)` returns a 6-tuple:
///   (locked_count, locked_amount, released_count, released_amount,
///    refunded_count, refunded_amount)
///
/// Key documented behaviour verified here:
///   - `Locked` bounties contribute `remaining_amount` to `locked_amount`.
///   - `PartiallyRefunded` bounties also contribute `remaining_amount`
///     (not the original `amount`) to `locked_amount` — the central bug
///     risk called out in issue #315.
///   - `Released` bounties contribute their original `amount` to
///     `released_amount` (remaining_amount is 0 at that point).
///   - `Refunded` bounties contribute their original `amount` to
///     `refunded_amount`.
///   - A depositor with no bounties gets all-zero stats without panicking.
///   - Stats for depositor A are not affected by depositor B's activity.
use crate::{BountyEscrowContract, BountyEscrowContractClient, RefundMode};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup<'a>(env: &'a Env) -> (BountyEscrowContractClient<'a>, Address, Address) {
    env.mock_all_auths();
    let admin = Address::generate(env);
    let token_admin = Address::generate(env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(env, &contract_id);
    client.init(&admin, &token_id);
    (client, admin, token_id)
}

fn mint(env: &Env, token_id: &Address, who: &Address, amount: i128) {
    token::StellarAssetClient::new(env, token_id).mint(who, &amount);
}

/// Lock a bounty with a far-future deadline so it stays `Locked`.
fn lock(client: &BountyEscrowContractClient, env: &Env, depositor: &Address, id: u64, amt: i128) {
    let dl = env.ledger().timestamp() + 7_200;
    client.lock_funds(depositor, &id, &amt, &dl);
}

// ===========================================================================
// 1. Zero-bounty depositor — all values must be 0, no panic
// ===========================================================================

#[test]
fn test_depositor_stats_zero_bounties_all_zero() {
    let env = Env::default();
    let (client, _admin, _token_id) = setup(&env);
    let stranger = Address::generate(&env);

    let (lc, la, rc, ra, fc, fa) = client.get_depositor_stats(&stranger);

    assert_eq!(lc, 0, "locked_count must be 0 for a depositor with no bounties");
    assert_eq!(la, 0, "locked_amount must be 0 for a depositor with no bounties");
    assert_eq!(rc, 0, "released_count must be 0 for a depositor with no bounties");
    assert_eq!(ra, 0, "released_amount must be 0 for a depositor with no bounties");
    assert_eq!(fc, 0, "refunded_count must be 0 for a depositor with no bounties");
    assert_eq!(fa, 0, "refunded_amount must be 0 for a depositor with no bounties");
}

// ===========================================================================
// 2. Single Locked bounty
// ===========================================================================

#[test]
fn test_depositor_stats_single_locked_bounty() {
    let env = Env::default();
    let (client, _admin, token_id) = setup(&env);
    let depositor = Address::generate(&env);
    mint(&env, &token_id, &depositor, 10_000);

    lock(&client, &env, &depositor, 1, 3_000);

    let (lc, la, rc, ra, fc, fa) = client.get_depositor_stats(&depositor);

    assert_eq!(lc, 1);
    assert_eq!(la, 3_000);
    assert_eq!(rc, 0);
    assert_eq!(ra, 0);
    assert_eq!(fc, 0);
    assert_eq!(fa, 0);
}

// ===========================================================================
// 3. Mixed-status bounties — Locked + Released + Refunded
// ===========================================================================

/// Lock three bounties:
///   - ID 10: stays Locked (1 000)
///   - ID 11: released to contributor (2 000)
///   - ID 12: expired and refunded (3 000)
///
/// Expected stats:
///   locked_count=1, locked_amount=1_000
///   released_count=1, released_amount=2_000
///   refunded_count=1, refunded_amount=3_000
#[test]
fn test_depositor_stats_mixed_status_buckets() {
    let env = Env::default();
    let (client, _admin, token_id) = setup(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    mint(&env, &token_id, &depositor, 100_000);

    let now = env.ledger().timestamp();

    // Bounty 10 — stays Locked
    client.lock_funds(&depositor, &10, &1_000, &(now + 7_200));

    // Bounty 11 — Released immediately
    client.lock_funds(&depositor, &11, &2_000, &(now + 7_200));
    client.release_funds(&11, &contributor);

    // Bounty 12 — Refunded after deadline
    client.lock_funds(&depositor, &12, &3_000, &(now + 60));
    env.ledger().set_timestamp(now + 61);
    client.refund(&12);

    let (lc, la, rc, ra, fc, fa) = client.get_depositor_stats(&depositor);

    assert_eq!(lc, 1, "exactly 1 bounty should be Locked");
    assert_eq!(la, 1_000, "locked_amount must be the Locked bounty's amount");
    assert_eq!(rc, 1, "exactly 1 bounty should be Released");
    assert_eq!(ra, 2_000, "released_amount must be the Released bounty's original amount");
    assert_eq!(fc, 1, "exactly 1 bounty should be Refunded");
    assert_eq!(fa, 3_000, "refunded_amount must be the Refunded bounty's original amount");
}

// ===========================================================================
// 4. Multiple bounties in the same bucket
// ===========================================================================

#[test]
fn test_depositor_stats_multiple_locked_bounties_summed() {
    let env = Env::default();
    let (client, _admin, token_id) = setup(&env);
    let depositor = Address::generate(&env);
    mint(&env, &token_id, &depositor, 100_000);

    let dl = env.ledger().timestamp() + 7_200;
    client.lock_funds(&depositor, &20, &1_000, &dl);
    client.lock_funds(&depositor, &21, &2_500, &dl);
    client.lock_funds(&depositor, &22, &4_000, &dl);

    let (lc, la, rc, ra, fc, fa) = client.get_depositor_stats(&depositor);

    assert_eq!(lc, 3);
    assert_eq!(la, 7_500); // 1_000 + 2_500 + 4_000
    assert_eq!(rc, 0);
    assert_eq!(ra, 0);
    assert_eq!(fc, 0);
    assert_eq!(fa, 0);
}

// ===========================================================================
// 5. PartiallyRefunded bounty — remaining_amount, NOT original amount
//
// This is the core regression described in issue #315.
// A PartiallyRefunded bounty has had some amount refunded already.
// get_depositor_stats must count it toward the *locked* bucket and use
// remaining_amount (what is still locked) — not the original amount.
// ===========================================================================

/// Bounty 30: original amount = 5 000
///   approve_refund 2 000 (Partial) → refund → remaining = 3 000, status = PartiallyRefunded
///
/// Expected: locked_count=1, locked_amount=3_000  (not 5_000)
///           refunded_count=0, refunded_amount=0
#[test]
fn test_depositor_stats_partially_refunded_uses_remaining_amount() {
    let env = Env::default();
    let (client, admin, token_id) = setup(&env);
    let depositor = Address::generate(&env);
    mint(&env, &token_id, &depositor, 50_000);

    let dl = env.ledger().timestamp() + 7_200;
    client.lock_funds(&depositor, &30, &5_000, &dl);

    // Partial refund of 2_000 — leaves 3_000 still locked
    client.approve_refund(&30, &2_000, &depositor, &RefundMode::Partial);
    client.refund(&30);

    let (lc, la, rc, ra, fc, fa) = client.get_depositor_stats(&depositor);

    assert_eq!(lc, 1,
        "PartiallyRefunded bounty must still count as locked (1 bounty)");
    assert_eq!(la, 3_000,
        "locked_amount must be remaining_amount (3_000), not original amount (5_000)");
    assert_eq!(rc, 0,
        "PartiallyRefunded bounty must NOT appear in released bucket");
    assert_eq!(ra, 0);
    assert_eq!(fc, 0,
        "PartiallyRefunded bounty must NOT appear in refunded bucket");
    assert_eq!(fa, 0);

    // Sanity: the depositor still has an active, partially-refunded bounty.
    // The locked_amount should be strictly less than the original 5_000.
    assert!(la < 5_000, "locked_amount must be less than the original amount after partial refund");
}

/// Two partial refunds on the same bounty (cumulative reduction).
///   original = 9 000
///   first partial refund  = 3 000  → remaining = 6 000
///   second partial refund = 2 000  → remaining = 4 000
///
/// Expected: locked_count=1, locked_amount=4_000
#[test]
fn test_depositor_stats_two_partial_refunds_cumulative_remaining() {
    let env = Env::default();
    let (client, _admin, token_id) = setup(&env);
    let depositor = Address::generate(&env);
    mint(&env, &token_id, &depositor, 50_000);

    let dl = env.ledger().timestamp() + 7_200;
    client.lock_funds(&depositor, &31, &9_000, &dl);

    // First partial refund
    client.approve_refund(&31, &3_000, &depositor, &RefundMode::Partial);
    client.refund(&31); // remaining = 6_000

    // Second partial refund — need a fresh approval
    client.approve_refund(&31, &2_000, &depositor, &RefundMode::Partial);
    client.refund(&31); // remaining = 4_000

    let (lc, la, ..) = client.get_depositor_stats(&depositor);

    assert_eq!(lc, 1);
    assert_eq!(la, 4_000,
        "locked_amount must reflect cumulative remaining after two partial refunds");
}

/// Full refund of a PartiallyRefunded bounty transitions it to Refunded.
/// At that point it must move from the locked bucket to the refunded bucket,
/// and locked_amount must drop to 0.
///
///   original = 6 000
///   partial refund 2 000  → remaining = 4 000, status = PartiallyRefunded
///   full refund 4 000     → remaining = 0, status = Refunded
///
/// Expected: locked_count=0, locked_amount=0
///           refunded_count=1, refunded_amount=6_000 (original)
#[test]
fn test_depositor_stats_partial_then_full_refund_moves_to_refunded_bucket() {
    let env = Env::default();
    let (client, _admin, token_id) = setup(&env);
    let depositor = Address::generate(&env);
    mint(&env, &token_id, &depositor, 50_000);

    let dl = env.ledger().timestamp() + 7_200;
    client.lock_funds(&depositor, &32, &6_000, &dl);

    // Partial: 2_000 refunded, 4_000 remains
    client.approve_refund(&32, &2_000, &depositor, &RefundMode::Partial);
    client.refund(&32);

    // Full: remaining 4_000 refunded, status becomes Refunded
    client.approve_refund(&32, &4_000, &depositor, &RefundMode::Full);
    client.refund(&32);

    let (lc, la, rc, ra, fc, fa) = client.get_depositor_stats(&depositor);

    assert_eq!(lc, 0, "fully refunded bounty must not appear in locked bucket");
    assert_eq!(la, 0);
    assert_eq!(rc, 0);
    assert_eq!(ra, 0);
    assert_eq!(fc, 1, "fully refunded bounty must appear in refunded bucket");
    assert_eq!(fa, 6_000, "refunded_amount must be the bounty's original amount");
}

// ===========================================================================
// 6. PartiallyRefunded + other statuses coexist correctly
// ===========================================================================

/// Depositor has:
///   ID 40 — Locked          (amount=1_000, remaining=1_000)
///   ID 41 — PartiallyRefunded (amount=8_000, remaining=5_000 after 3_000 refunded)
///   ID 42 — Released        (amount=2_000)
///   ID 43 — Refunded        (amount=4_000)
///
/// Expected:
///   locked_count=2, locked_amount=6_000  (1_000 + 5_000)
///   released_count=1, released_amount=2_000
///   refunded_count=1, refunded_amount=4_000
#[test]
fn test_depositor_stats_all_four_statuses_coexist() {
    let env = Env::default();
    let (client, _admin, token_id) = setup(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    mint(&env, &token_id, &depositor, 500_000);

    let now = env.ledger().timestamp();

    // ID 40 — stays Locked
    client.lock_funds(&depositor, &40, &1_000, &(now + 7_200));

    // ID 41 — PartiallyRefunded: original 8_000, partial refund 3_000 → remaining 5_000
    client.lock_funds(&depositor, &41, &8_000, &(now + 7_200));
    client.approve_refund(&41, &3_000, &depositor, &RefundMode::Partial);
    client.refund(&41);

    // ID 42 — Released
    client.lock_funds(&depositor, &42, &2_000, &(now + 7_200));
    client.release_funds(&42, &contributor);

    // ID 43 — Refunded after deadline
    client.lock_funds(&depositor, &43, &4_000, &(now + 60));
    env.ledger().set_timestamp(now + 61);
    client.refund(&43);

    let (lc, la, rc, ra, fc, fa) = client.get_depositor_stats(&depositor);

    assert_eq!(lc, 2,
        "Locked + PartiallyRefunded both count toward locked_count");
    assert_eq!(la, 6_000,
        "locked_amount = 1_000 (Locked) + 5_000 (PartiallyRefunded remaining)");
    assert_eq!(rc, 1);
    assert_eq!(ra, 2_000);
    assert_eq!(fc, 1);
    assert_eq!(fa, 4_000);
}

// ===========================================================================
// 7. Stats are per-depositor — other depositors' activity has no effect
// ===========================================================================

#[test]
fn test_depositor_stats_isolated_from_other_depositors() {
    let env = Env::default();
    let (client, _admin, token_id) = setup(&env);
    let dep_a = Address::generate(&env);
    let dep_b = Address::generate(&env);
    let contributor = Address::generate(&env);
    mint(&env, &token_id, &dep_a, 100_000);
    mint(&env, &token_id, &dep_b, 100_000);

    let dl = env.ledger().timestamp() + 7_200;

    // dep_a: one lock
    client.lock_funds(&dep_a, &50, &3_000, &dl);

    // dep_b: many different-status bounties
    client.lock_funds(&dep_b, &51, &10_000, &dl);
    client.lock_funds(&dep_b, &52, &20_000, &dl);
    client.release_funds(&52, &contributor);

    // dep_a's stats must only reflect their own bounty
    let (lc, la, rc, ra, fc, fa) = client.get_depositor_stats(&dep_a);
    assert_eq!(lc, 1);
    assert_eq!(la, 3_000);
    assert_eq!(rc, 0);
    assert_eq!(ra, 0);
    assert_eq!(fc, 0);
    assert_eq!(fa, 0);

    // dep_b's stats must only reflect their own bounties
    let (lc_b, la_b, rc_b, ra_b, fc_b, fa_b) = client.get_depositor_stats(&dep_b);
    assert_eq!(lc_b, 1);   // ID 51 is still locked
    assert_eq!(la_b, 10_000);
    assert_eq!(rc_b, 1);   // ID 52 was released
    assert_eq!(ra_b, 20_000);
    assert_eq!(fc_b, 0);
    assert_eq!(fa_b, 0);
}

// ===========================================================================
// 8. Stats update correctly as a bounty progresses through lifecycle
// ===========================================================================

/// Walk one bounty through Locked → PartiallyRefunded → Refunded,
/// asserting stats at each transition.
#[test]
fn test_depositor_stats_update_across_lifecycle_transitions() {
    let env = Env::default();
    let (client, _admin, token_id) = setup(&env);
    let depositor = Address::generate(&env);
    mint(&env, &token_id, &depositor, 50_000);

    let dl = env.ledger().timestamp() + 7_200;
    client.lock_funds(&depositor, &60, &10_000, &dl);

    // --- Step 1: Locked ---
    let (lc, la, rc, ra, fc, fa) = client.get_depositor_stats(&depositor);
    assert_eq!((lc, la, rc, ra, fc, fa), (1, 10_000, 0, 0, 0, 0));

    // --- Step 2: PartiallyRefunded (4_000 refunded, 6_000 remaining) ---
    client.approve_refund(&60, &4_000, &depositor, &RefundMode::Partial);
    client.refund(&60);
    let (lc, la, rc, ra, fc, fa) = client.get_depositor_stats(&depositor);
    assert_eq!(lc, 1, "still in locked bucket after partial refund");
    assert_eq!(la, 6_000, "locked_amount must be remaining after partial refund");
    assert_eq!((rc, ra, fc, fa), (0, 0, 0, 0));

    // --- Step 3: Fully Refunded ---
    client.approve_refund(&60, &6_000, &depositor, &RefundMode::Full);
    client.refund(&60);
    let (lc, la, rc, ra, fc, fa) = client.get_depositor_stats(&depositor);
    assert_eq!((lc, la), (0, 0), "fully refunded bounty exits locked bucket");
    assert_eq!((rc, ra), (0, 0));
    assert_eq!(fc, 1, "fully refunded bounty enters refunded bucket");
    assert_eq!(fa, 10_000, "refunded_amount is the original amount");
}

/// Walk one bounty Locked → Released via release_funds.
#[test]
fn test_depositor_stats_update_locked_to_released() {
    let env = Env::default();
    let (client, _admin, token_id) = setup(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    mint(&env, &token_id, &depositor, 50_000);

    let dl = env.ledger().timestamp() + 7_200;
    client.lock_funds(&depositor, &61, &5_000, &dl);

    // Before release
    let (lc, la, ..) = client.get_depositor_stats(&depositor);
    assert_eq!((lc, la), (1, 5_000));

    // After release
    client.release_funds(&61, &contributor);
    let (lc, la, rc, ra, fc, fa) = client.get_depositor_stats(&depositor);
    assert_eq!((lc, la), (0, 0), "released bounty must leave locked bucket");
    assert_eq!(rc, 1);
    assert_eq!(ra, 5_000);
    assert_eq!((fc, fa), (0, 0));
}

// ===========================================================================
// 9. Counts and amounts are summed correctly across many bounties
// ===========================================================================

#[test]
fn test_depositor_stats_large_mixed_portfolio() {
    let env = Env::default();
    let (client, _admin, token_id) = setup(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    mint(&env, &token_id, &depositor, 10_000_000);

    let now = env.ledger().timestamp();

    // Lock 5 bounties that will stay Locked
    let locked_ids: [(u64, i128); 5] = [(70, 100), (71, 200), (72, 300), (73, 400), (74, 500)];
    for (id, amt) in locked_ids {
        client.lock_funds(&depositor, &id, &amt, &(now + 7_200));
    }

    // Release 3 bounties
    let released_ids: [(u64, i128); 3] = [(75, 1_000), (76, 2_000), (77, 3_000)];
    for (id, amt) in released_ids {
        client.lock_funds(&depositor, &id, &amt, &(now + 7_200));
        client.release_funds(&id, &contributor);
    }

    // Refund 2 bounties after deadline
    let refunded_amounts: [(u64, i128); 2] = [(78, 5_000), (79, 7_000)];
    for (id, amt) in refunded_amounts {
        client.lock_funds(&depositor, &id, &amt, &(now + 60));
    }
    env.ledger().set_timestamp(now + 61);
    client.refund(&78);
    client.refund(&79);

    let (lc, la, rc, ra, fc, fa) = client.get_depositor_stats(&depositor);

    assert_eq!(lc, 5);
    assert_eq!(la, 100 + 200 + 300 + 400 + 500); // 1_500
    assert_eq!(rc, 3);
    assert_eq!(ra, 1_000 + 2_000 + 3_000);        // 6_000
    assert_eq!(fc, 2);
    assert_eq!(fa, 5_000 + 7_000);                // 12_000
}
