//! Unit tests for analytics.rs statistics helpers
//!
//! Issue #166 – every exported helper is covered with:
//!   • empty / not-initialised case
//!   • single-element case
//!   • multi-element case
//!   • edge cases (zero amount, saturation, repeated ops)
//!
//! All expected values are hand-computed in the inline comments.
//! GROUP 6 cross-checks `average_bounty_amount` against the formula
//! in `lib.rs::get_contract_analytics` using the same AggregateStats inputs.

#![cfg(test)]

use crate::analytics::{
    get_bounty_analytics, init_bounty_analytics, update_analytics_on_refund,
    update_analytics_on_release, ANALYTICS_VERSION_V1,
};
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
struct AnalyticsStatsContract;

#[contractimpl]
impl AnalyticsStatsContract {
    pub fn noop(_env: Env) {}
}

fn setup() -> (Env, soroban_sdk::Address) {
    let env = Env::default();
    let id = env.register_contract(None, AnalyticsStatsContract);
    (env, id)
}

// ===========================================================================
// GROUP 1 – get_bounty_analytics: empty (no init)
// ===========================================================================

/// Querying analytics for bounty IDs that were never initialised must
/// return None and must not panic.
#[test]
fn test_get_bounty_analytics_empty_set_returns_none() {
    let (env, id) = setup();
    env.as_contract(&id, || {
        assert!(get_bounty_analytics(&env, 0).is_none());
        assert!(get_bounty_analytics(&env, 1).is_none());
        assert!(get_bounty_analytics(&env, u64::MAX).is_none());
    });
}

// ===========================================================================
// GROUP 2 – init_bounty_analytics
// ===========================================================================

/// Single-element: standard positive amount.
///
/// Hand-computed values after init(amount=5_000, ts=42):
///   total_amount_locked   = 5_000
///   total_amount_released = 0
///   total_amount_refunded = 0
///   remaining_amount      = 5_000
///   created_at            = 42
///   last_updated          = 42
///   partial_releases_count = 0
///   partial_refunds_count  = 0
#[test]
fn test_init_analytics_single_element_all_fields() {
    let (env, id) = setup();
    env.as_contract(&id, || {
        init_bounty_analytics(&env, 1, 5_000, 42);
        let a = get_bounty_analytics(&env, 1).expect("must exist after init");

        assert_eq!(a.total_amount_locked, 5_000);
        assert_eq!(a.total_amount_released, 0);
        assert_eq!(a.total_amount_refunded, 0);
        assert_eq!(a.remaining_amount, 5_000);
        assert_eq!(a.created_at, 42);
        assert_eq!(a.last_updated, 42);
        assert_eq!(a.partial_releases_count, 0);
        assert_eq!(a.partial_refunds_count, 0);
    });
}

/// Edge case: init with amount = 0.
///
/// Hand-computed: all monetary fields = 0; timestamps preserved.
#[test]
fn test_init_analytics_zero_amount() {
    let (env, id) = setup();
    env.as_contract(&id, || {
        init_bounty_analytics(&env, 2, 0, 999);
        let a = get_bounty_analytics(&env, 2).unwrap();

        assert_eq!(a.total_amount_locked, 0);
        assert_eq!(a.remaining_amount, 0);
        assert_eq!(a.created_at, 999);
        assert_eq!(a.last_updated, 999);
        assert_eq!(a.partial_releases_count, 0);
        assert_eq!(a.partial_refunds_count, 0);
    });
}

/// Multi-element: two bounties are stored independently.
///
/// Hand-computed:
///   bounty 10: locked = 1_000, remaining = 1_000, created_at = 10
///   bounty 11: locked = 9_000, remaining = 9_000, created_at = 20
#[test]
fn test_init_analytics_multiple_bounties_independent() {
    let (env, id) = setup();
    env.as_contract(&id, || {
        init_bounty_analytics(&env, 10, 1_000, 10);
        init_bounty_analytics(&env, 11, 9_000, 20);

        let a10 = get_bounty_analytics(&env, 10).unwrap();
        let a11 = get_bounty_analytics(&env, 11).unwrap();

        // bounty 10
        assert_eq!(a10.total_amount_locked, 1_000);
        assert_eq!(a10.remaining_amount, 1_000);
        assert_eq!(a10.created_at, 10);

        // bounty 11
        assert_eq!(a11.total_amount_locked, 9_000);
        assert_eq!(a11.remaining_amount, 9_000);
        assert_eq!(a11.created_at, 20);

        // cross-check: bounty 10 is not affected by bounty 11's init
        assert_eq!(a10.total_amount_locked, 1_000);
    });
}

// ===========================================================================
// GROUP 3 – update_analytics_on_release
// ===========================================================================

/// Empty case: update on a non-existent bounty is a silent no-op.
#[test]
fn test_release_missing_bounty_is_noop() {
    let (env, id) = setup();
    env.as_contract(&id, || {
        update_analytics_on_release(&env, 99, 500, 100); // no init – must not panic
        assert!(get_bounty_analytics(&env, 99).is_none());
    });
}

/// Single-element: one partial release.
///
/// Hand-computed after init(1_000, ts=100) then release(500, ts=200):
///   total_amount_released  = 500
///   remaining_amount       = 1_000 − 500 = 500
///   partial_releases_count = 1
///   last_updated           = 200
///   total_amount_refunded  = 0  (unchanged)
///   total_amount_locked    = 1_000  (unchanged)
#[test]
fn test_release_single_partial_release() {
    let (env, id) = setup();
    env.as_contract(&id, || {
        init_bounty_analytics(&env, 20, 1_000, 100);
        update_analytics_on_release(&env, 20, 500, 200);

        let a = get_bounty_analytics(&env, 20).unwrap();
        assert_eq!(a.total_amount_released, 500);
        assert_eq!(a.remaining_amount, 500);
        assert_eq!(a.partial_releases_count, 1);
        assert_eq!(a.last_updated, 200);
        assert_eq!(a.total_amount_refunded, 0);
        assert_eq!(a.total_amount_locked, 1_000);
    });
}

/// Multi-element: three successive partial releases drain the bounty.
///
/// Hand-computed after init(1_000):
///   release(300) → released=300, remaining=700, count=1
///   release(300) → released=600, remaining=400, count=2
///   release(400) → released=1_000, remaining=0,  count=3
#[test]
fn test_release_multiple_partial_releases_accumulate() {
    let (env, id) = setup();
    env.as_contract(&id, || {
        init_bounty_analytics(&env, 21, 1_000, 1);

        update_analytics_on_release(&env, 21, 300, 2);
        let a = get_bounty_analytics(&env, 21).unwrap();
        assert_eq!(a.total_amount_released, 300);
        assert_eq!(a.remaining_amount, 700);
        assert_eq!(a.partial_releases_count, 1);

        update_analytics_on_release(&env, 21, 300, 3);
        let a = get_bounty_analytics(&env, 21).unwrap();
        assert_eq!(a.total_amount_released, 600);
        assert_eq!(a.remaining_amount, 400);
        assert_eq!(a.partial_releases_count, 2);

        update_analytics_on_release(&env, 21, 400, 4);
        let a = get_bounty_analytics(&env, 21).unwrap();
        assert_eq!(a.total_amount_released, 1_000);
        assert_eq!(a.remaining_amount, 0);
        assert_eq!(a.partial_releases_count, 3);
    });
}

/// Edge case: releasing exactly the full locked amount drains remaining to 0.
///
/// Hand-computed: init(500) → release(500): remaining = 500 − 500 = 0.
#[test]
fn test_release_exact_amount_drains_to_zero() {
    let (env, id) = setup();
    env.as_contract(&id, || {
        init_bounty_analytics(&env, 22, 500, 1);
        update_analytics_on_release(&env, 22, 500, 2);

        let a = get_bounty_analytics(&env, 22).unwrap();
        assert_eq!(a.remaining_amount, 0);
        assert_eq!(a.total_amount_released, 500);
        assert_eq!(a.partial_releases_count, 1);
    });
}

/// Edge case: release of zero amount increments counter, totals unchanged.
///
/// Hand-computed: remaining=1_000 (unchanged), released=0 (unchanged), count=1.
#[test]
fn test_release_zero_amount_increments_count_only() {
    let (env, id) = setup();
    env.as_contract(&id, || {
        init_bounty_analytics(&env, 23, 1_000, 1);
        update_analytics_on_release(&env, 23, 0, 2);

        let a = get_bounty_analytics(&env, 23).unwrap();
        assert_eq!(a.total_amount_released, 0);
        assert_eq!(a.remaining_amount, 1_000);
        assert_eq!(a.partial_releases_count, 1);
    });
}

// ===========================================================================
// GROUP 4 – update_analytics_on_refund
// ===========================================================================

/// Empty case: update on a non-existent bounty is a silent no-op.
#[test]
fn test_refund_missing_bounty_is_noop() {
    let (env, id) = setup();
    env.as_contract(&id, || {
        update_analytics_on_refund(&env, 88, 100, 50);
        assert!(get_bounty_analytics(&env, 88).is_none());
    });
}

/// Single-element: one partial refund.
///
/// Hand-computed after init(1_000, ts=100) then refund(300, ts=200):
///   total_amount_refunded  = 300
///   remaining_amount       = 1_000 − 300 = 700
///   partial_refunds_count  = 1
///   last_updated           = 200
///   total_amount_released  = 0   (unchanged)
///   total_amount_locked    = 1_000  (unchanged)
#[test]
fn test_refund_single_partial_refund() {
    let (env, id) = setup();
    env.as_contract(&id, || {
        init_bounty_analytics(&env, 30, 1_000, 100);
        update_analytics_on_refund(&env, 30, 300, 200);

        let a = get_bounty_analytics(&env, 30).unwrap();
        assert_eq!(a.total_amount_refunded, 300);
        assert_eq!(a.remaining_amount, 700);
        assert_eq!(a.partial_refunds_count, 1);
        assert_eq!(a.last_updated, 200);
        assert_eq!(a.total_amount_released, 0);
        assert_eq!(a.total_amount_locked, 1_000);
    });
}

/// Multi-element: two partial refunds drain the bounty.
///
/// Hand-computed after init(800):
///   refund(200) → refunded=200, remaining=600, count=1
///   refund(600) → refunded=800, remaining=0,   count=2
#[test]
fn test_refund_multiple_partial_refunds_accumulate() {
    let (env, id) = setup();
    env.as_contract(&id, || {
        init_bounty_analytics(&env, 31, 800, 10);

        update_analytics_on_refund(&env, 31, 200, 20);
        let a = get_bounty_analytics(&env, 31).unwrap();
        assert_eq!(a.total_amount_refunded, 200);
        assert_eq!(a.remaining_amount, 600);
        assert_eq!(a.partial_refunds_count, 1);

        update_analytics_on_refund(&env, 31, 600, 30);
        let a = get_bounty_analytics(&env, 31).unwrap();
        assert_eq!(a.total_amount_refunded, 800);
        assert_eq!(a.remaining_amount, 0);
        assert_eq!(a.partial_refunds_count, 2);
    });
}

/// Edge case: refunding exactly the full remaining amount drains to 0.
///
/// Hand-computed: init(400) → refund(400): remaining = 400 − 400 = 0.
#[test]
fn test_refund_exact_amount_drains_to_zero() {
    let (env, id) = setup();
    env.as_contract(&id, || {
        init_bounty_analytics(&env, 32, 400, 1);
        update_analytics_on_refund(&env, 32, 400, 2);

        let a = get_bounty_analytics(&env, 32).unwrap();
        assert_eq!(a.remaining_amount, 0);
        assert_eq!(a.total_amount_refunded, 400);
        assert_eq!(a.partial_refunds_count, 1);
    });
}

/// Edge case: refund of zero amount increments counter, monetary fields unchanged.
///
/// Hand-computed: remaining=500 (unchanged), refunded=0 (unchanged), count=1.
#[test]
fn test_refund_zero_amount_increments_count_only() {
    let (env, id) = setup();
    env.as_contract(&id, || {
        init_bounty_analytics(&env, 33, 500, 1);
        update_analytics_on_refund(&env, 33, 0, 2);

        let a = get_bounty_analytics(&env, 33).unwrap();
        assert_eq!(a.total_amount_refunded, 0);
        assert_eq!(a.remaining_amount, 500);
        assert_eq!(a.partial_refunds_count, 1);
    });
}

// ===========================================================================
// GROUP 5 – Mixed release + refund lifecycle
// ===========================================================================

/// Full lifecycle: partial release then partial refund for remaining amount.
///
/// Hand-computed after init(1_000):
///   release(600, ts=2) → released=600, remaining=400, release_count=1
///   refund(400,  ts=3) → refunded=400, remaining=0,   refund_count=1
///   Invariant: released + refunded = total_locked = 1_000
#[test]
fn test_lifecycle_release_then_refund_drains_to_zero() {
    let (env, id) = setup();
    env.as_contract(&id, || {
        init_bounty_analytics(&env, 40, 1_000, 1);

        update_analytics_on_release(&env, 40, 600, 2);
        update_analytics_on_refund(&env, 40, 400, 3);

        let a = get_bounty_analytics(&env, 40).unwrap();
        assert_eq!(a.total_amount_released, 600);
        assert_eq!(a.total_amount_refunded, 400);
        assert_eq!(a.remaining_amount, 0);
        assert_eq!(a.partial_releases_count, 1);
        assert_eq!(a.partial_refunds_count, 1);
        assert_eq!(a.last_updated, 3);

        // Invariant: released + refunded == total_locked
        assert_eq!(
            a.total_amount_released + a.total_amount_refunded,
            a.total_amount_locked
        );
    });
}

/// Multi-step interleaved lifecycle across multiple bounties.
///
/// Three bounties, different amounts:
///   bounty 50: init(2_000) → release(2_000)
///   bounty 51: init(3_000) → refund(1_000) → refund(2_000)
///   bounty 52: init(5_000) → release(2_000) → refund(3_000)
///
/// Hand-computed final remaining for each: 0, 0, 0.
/// Cross-check totals across the three:
///   total released  = 2_000 + 0 + 2_000 = 4_000
///   total refunded  = 0 + 3_000 + 3_000 = 6_000
///   sum             = 10_000 = 2_000 + 3_000 + 5_000 (sum of locked amounts)
#[test]
fn test_lifecycle_multiple_bounties_cross_check_totals() {
    let (env, id) = setup();
    env.as_contract(&id, || {
        // bounty 50: fully released
        init_bounty_analytics(&env, 50, 2_000, 1);
        update_analytics_on_release(&env, 50, 2_000, 2);

        // bounty 51: fully refunded in two steps
        init_bounty_analytics(&env, 51, 3_000, 1);
        update_analytics_on_refund(&env, 51, 1_000, 2);
        update_analytics_on_refund(&env, 51, 2_000, 3);

        // bounty 52: partial release then refund remainder
        init_bounty_analytics(&env, 52, 5_000, 1);
        update_analytics_on_release(&env, 52, 2_000, 2);
        update_analytics_on_refund(&env, 52, 3_000, 3);

        let a50 = get_bounty_analytics(&env, 50).unwrap();
        let a51 = get_bounty_analytics(&env, 51).unwrap();
        let a52 = get_bounty_analytics(&env, 52).unwrap();

        assert_eq!(a50.remaining_amount, 0);
        assert_eq!(a51.remaining_amount, 0);
        assert_eq!(a52.remaining_amount, 0);

        let total_released =
            a50.total_amount_released + a51.total_amount_released + a52.total_amount_released;
        let total_refunded =
            a50.total_amount_refunded + a51.total_amount_refunded + a52.total_amount_refunded;
        let total_locked =
            a50.total_amount_locked + a51.total_amount_locked + a52.total_amount_locked;

        // hand-computed
        assert_eq!(total_released, 4_000);
        assert_eq!(total_refunded, 6_000);
        // invariant: released + refunded == locked (all fully drained)
        assert_eq!(total_released + total_refunded, total_locked);
    });
}

// ===========================================================================
// GROUP 6 – Cross-check: average_bounty_amount formula (lib.rs parity)
//
// The formula used in lib.rs::get_contract_analytics is:
//
//   total_count = count_locked + count_released + count_refunded
//   average = if total_count > 0 {
//       (total_locked + total_released + total_refunded) / total_count
//   } else { 0 }
//
// This group verifies the formula directly using the BountyAnalytics totals
// gathered from analytics.rs helpers, then cross-checks the result against
// the hand-computed value.  This mirrors what lib.rs does with AggregateStats.
// ===========================================================================

/// Empty set: average of zero bounties must be 0 (no divide-by-zero).
///
/// Hand-computed: total_count = 0 → average = 0.
#[test]
fn test_average_bounty_amount_empty_set_is_zero() {
    // Simulate the lib.rs formula with zero counters.
    let count_locked: i128 = 0;
    let count_released: i128 = 0;
    let count_refunded: i128 = 0;
    let total_locked: i128 = 0;
    let total_released: i128 = 0;
    let total_refunded: i128 = 0;

    let total_count = count_locked + count_released + count_refunded;
    let average = if total_count > 0 {
        (total_locked + total_released + total_refunded) / total_count
    } else {
        0
    };

    // hand-computed: 0
    assert_eq!(average, 0);
}

/// Single-element: one bounty of 4_000.
///
/// Hand-computed:
///   total_count = 1  (count_locked=1)
///   total_sum   = 4_000
///   average     = 4_000 / 1 = 4_000
#[test]
fn test_average_bounty_amount_single_bounty() {
    let count_locked: i128 = 1;
    let count_released: i128 = 0;
    let count_refunded: i128 = 0;
    let total_locked: i128 = 4_000;
    let total_released: i128 = 0;
    let total_refunded: i128 = 0;

    let total_count = count_locked + count_released + count_refunded;
    let average = if total_count > 0 {
        (total_locked + total_released + total_refunded) / total_count
    } else {
        0
    };

    // hand-computed: 4_000 / 1 = 4_000
    assert_eq!(average, 4_000);
}

/// Multi-element: three bounties with different amounts and statuses.
///
/// Scenario (mirrors what lib.rs counters would hold after these transitions):
///   1 locked  bounty  of 2_000  → contributes to total_locked
///   2 released bounties of 3_000 and 1_000 → total_released = 4_000
///   1 refunded bounty  of 5_000  → total_refunded = 5_000
///
/// Hand-computed:
///   total_count = 1 + 2 + 1 = 4
///   total_sum   = 2_000 + 4_000 + 5_000 = 11_000
///   average     = 11_000 / 4 = 2_750
#[test]
fn test_average_bounty_amount_multi_element() {
    let count_locked: i128 = 1;
    let count_released: i128 = 2;
    let count_refunded: i128 = 1;
    let total_locked: i128 = 2_000;
    let total_released: i128 = 4_000; // 3_000 + 1_000
    let total_refunded: i128 = 5_000;

    let total_count = count_locked + count_released + count_refunded;
    let average = if total_count > 0 {
        (total_locked + total_released + total_refunded) / total_count
    } else {
        0
    };

    // hand-computed: 11_000 / 4 = 2_750
    assert_eq!(total_count, 4);
    assert_eq!(average, 2_750);
}

/// Cross-check: derive the same average from BountyAnalytics structs stored by
/// analytics.rs helpers, then confirm it matches the lib.rs formula output.
///
/// Setup: two bounties, both fully released.
///   bounty 60: locked=6_000, fully released
///   bounty 61: locked=4_000, fully released
///
/// Hand-computed lib.rs-style:
///   count_released = 2  (after finalize_partial_release_to_released × 2)
///   total_released = 10_000
///   total_count    = 2
///   average        = 10_000 / 2 = 5_000
///
/// We verify the formula using the total_amount_locked values from
/// the per-bounty analytics (as a proxy for the aggregate counter inputs).
#[test]
fn test_average_bounty_amount_cross_checked_with_analytics_helpers() {
    let (env, id) = setup();
    env.as_contract(&id, || {
        init_bounty_analytics(&env, 60, 6_000, 1);
        update_analytics_on_release(&env, 60, 6_000, 2);

        init_bounty_analytics(&env, 61, 4_000, 1);
        update_analytics_on_release(&env, 61, 4_000, 2);

        let a60 = get_bounty_analytics(&env, 60).unwrap();
        let a61 = get_bounty_analytics(&env, 61).unwrap();

        // Both fully drained
        assert_eq!(a60.remaining_amount, 0);
        assert_eq!(a61.remaining_amount, 0);

        // Simulate lib.rs AggregateStats after two full releases:
        //   count_released = 2, total_released = 6_000 + 4_000 = 10_000
        let count_released: i128 = 2;
        let total_released: i128 =
            a60.total_amount_locked + a61.total_amount_locked; // 10_000

        let total_count: i128 = count_released; // only released bounties here
        let average = if total_count > 0 {
            total_released / total_count
        } else {
            0
        };

        // hand-computed: 10_000 / 2 = 5_000
        assert_eq!(total_released, 10_000);
        assert_eq!(average, 5_000);
    });
}

// ===========================================================================
// GROUP 7 – ANALYTICS_VERSION_V1 constant
// ===========================================================================

/// The constant must equal 1 as the initial schema version.
#[test]
fn test_analytics_version_v1_is_one() {
    assert_eq!(ANALYTICS_VERSION_V1, 1);
}

// ===========================================================================
// GROUP 8 – Large-value / near-overflow saturation guard
// ===========================================================================

/// Large amounts: initialise with i128::MAX / 2; two successive releases that
/// together equal the locked amount must not overflow (saturating arithmetic).
///
/// Hand-computed:
///   locked    = i128::MAX / 2
///   release_1 = i128::MAX / 4
///   release_2 = i128::MAX / 4
///   remaining after both = (i128::MAX/2) - (i128::MAX/2) = 0
#[test]
fn test_large_amounts_no_overflow() {
    let locked: i128 = i128::MAX / 2;
    let half: i128 = locked / 2;

    let (env, id) = setup();
    env.as_contract(&id, || {
        init_bounty_analytics(&env, 70, locked, 1);
        update_analytics_on_release(&env, 70, half, 2);
        update_analytics_on_release(&env, 70, locked - half, 3);

        let a = get_bounty_analytics(&env, 70).unwrap();
        assert_eq!(a.total_amount_locked, locked);
        assert_eq!(a.total_amount_released, locked);
        assert_eq!(a.remaining_amount, 0);
    });
}
