#![cfg(test)]
//! Analytics Epoch Boundary Tests
//!
//! Validates how analytics events and metrics handle period boundaries
//! (e.g. days/weeks) to ensure no double counting or dropped events.

use soroban_sdk::{
    testutils::Events, Env, Symbol, TryFromVal
};
use crate::analytics::{
    init_bounty_analytics, update_analytics_on_release, update_analytics_on_refund, 
    emit_bounty_activity, BountyActivityEvent, ANALYTICS_VERSION_V1, get_bounty_analytics
};
use soroban_sdk::{contract, contractimpl};

#[contract]
struct DummyContract;

#[contractimpl]
impl DummyContract {
    pub fn noop(_env: Env) {}
}

const PERIOD_SECONDS: u64 = 86400; // 1 day epoch

// Simple off-chain indexer simulation for testing
fn get_period(timestamp: u64) -> u64 {
    timestamp / PERIOD_SECONDS
}

#[test]
fn test_event_epoch_boundary_attribution() {
    let env = Env::default();
    let contract_id = env.register_contract(None, DummyContract);

    env.as_contract(&contract_id, || {
        let before_boundary = PERIOD_SECONDS - 1;
        let boundary = PERIOD_SECONDS;
        let after_boundary = PERIOD_SECONDS + 1;

        emit_bounty_activity(&env, BountyActivityEvent {
            version: ANALYTICS_VERSION_V1,
            bounty_id: 1,
            activity_type: Symbol::new(&env, "created"),
            amount: 100,
            timestamp: before_boundary,
        });

        emit_bounty_activity(&env, BountyActivityEvent {
            version: ANALYTICS_VERSION_V1,
            bounty_id: 2,
            activity_type: Symbol::new(&env, "created"),
            amount: 200,
            timestamp: boundary,
        });

        emit_bounty_activity(&env, BountyActivityEvent {
            version: ANALYTICS_VERSION_V1,
            bounty_id: 3,
            activity_type: Symbol::new(&env, "created"),
            amount: 300,
            timestamp: after_boundary,
        });
    });

    let events = env.events().all();
    let mut p0_amount = 0;
    let mut p1_amount = 0;

    for (_contract_id, _topics, data) in events.iter() {
        if let Ok(event) = BountyActivityEvent::try_from_val(&env, &data) {
            let period = get_period(event.timestamp);
            if period == 0 {
                p0_amount += event.amount;
            } else if period == 1 {
                p1_amount += event.amount;
            }
        }
    }

    assert_eq!(p0_amount, 100);
    // Boundary event correctly attributed to period 1
    assert_eq!(p1_amount, 500); 
}

#[test]
fn test_cross_period_aggregation() {
    let env = Env::default();
    let contract_id = env.register_contract(None, DummyContract);

    env.as_contract(&contract_id, || {
        let period_0_ts = 1000;
        let gap_ts = PERIOD_SECONDS * 5; // Large ledger jump
        let period_5_ts = gap_ts + 1000;

        // Activity in period 0
        emit_bounty_activity(&env, BountyActivityEvent {
            version: ANALYTICS_VERSION_V1, bounty_id: 1, activity_type: Symbol::new(&env, "released"),
            amount: 50, timestamp: period_0_ts,
        });
        emit_bounty_activity(&env, BountyActivityEvent {
            version: ANALYTICS_VERSION_V1, bounty_id: 2, activity_type: Symbol::new(&env, "released"),
            amount: 75, timestamp: period_0_ts + 10,
        });

        // Activity in period 5 (after large ledger gap)
        emit_bounty_activity(&env, BountyActivityEvent {
            version: ANALYTICS_VERSION_V1, bounty_id: 1, activity_type: Symbol::new(&env, "released"),
            amount: 25, timestamp: period_5_ts,
        });
    });

    let events = env.events().all();
    let mut p0_amount = 0;
    let mut p5_amount = 0;
    let mut p1_amount = 0; // The gap

    for (_, _, data) in events.iter() {
        if let Ok(event) = BountyActivityEvent::try_from_val(&env, &data) {
            let p = get_period(event.timestamp);
            if p == 0 { p0_amount += event.amount; }
            if p == 5 { p5_amount += event.amount; }
            if p == 1 { p1_amount += event.amount; }
        }
    }

    assert_eq!(p0_amount, 125);
    assert_eq!(p5_amount, 25);
    assert_eq!(p1_amount, 0); // No events in gap
}

#[test]
fn test_state_lifecycle_cross_period() {
    let env = Env::default();
    let contract_id = env.register_contract(None, DummyContract);

    env.as_contract(&contract_id, || {
        let p0_ts = 100;
        let p1_ts = PERIOD_SECONDS + 100;
        let p2_ts = PERIOD_SECONDS * 2 + 100;

        init_bounty_analytics(&env, 1, 1000, p0_ts);
        update_analytics_on_release(&env, 1, 200, p1_ts);
        update_analytics_on_refund(&env, 1, 800, p2_ts);

        let analytics = get_bounty_analytics(&env, 1).unwrap();
        assert_eq!(analytics.created_at, p0_ts);
        assert_eq!(analytics.last_updated, p2_ts);
        assert_eq!(analytics.total_amount_released, 200);
        assert_eq!(analytics.total_amount_refunded, 800);
        assert_eq!(analytics.remaining_amount, 0);
    });

}

// ============================================================
// Direct unit tests for analytics helper functions
// ============================================================

/// Test init_bounty_analytics directly with full field assertions
#[test]
fn test_direct_init_bounty_analytics_full_assertions() {
    let env = Env::default();
    let contract_id = env.register_contract(None, DummyContract);

    env.as_contract(&contract_id, || {
        let bounty_id = 10u64;
        let amount = 5000i128;
        let timestamp = 12345678u64;

        // Call the function directly
        init_bounty_analytics(&env, bounty_id, amount, timestamp);

        // Retrieve and assert every field
        let analytics = get_bounty_analytics(&env, bounty_id).expect("should exist");
        assert_eq!(analytics.total_amount_locked, amount, "total_amount_locked mismatch");
        assert_eq!(analytics.total_amount_released, 0, "total_amount_released should be 0");
        assert_eq!(analytics.total_amount_refunded, 0, "total_amount_refunded should be 0");
        assert_eq!(analytics.remaining_amount, amount, "remaining_amount should equal locked amount");
        assert_eq!(analytics.created_at, timestamp, "created_at mismatch");
        assert_eq!(analytics.last_updated, timestamp, "last_updated mismatch");
        assert_eq!(analytics.partial_releases_count, 0, "partial_releases_count should be 0");
        assert_eq!(analytics.partial_refunds_count, 0, "partial_refunds_count should be 0");
    });
}

/// Test multiple sequential releases update counters and remaining_amount correctly
#[test]
fn test_direct_sequential_releases() {
    let env = Env::default();
    let contract_id = env.register_contract(None, DummyContract);

    env.as_contract(&contract_id, || {
        let bounty_id = 11u64;
        let amount = 1000i128;

        init_bounty_analytics(&env, bounty_id, amount, 100);

        // First partial release
        update_analytics_on_release(&env, bounty_id, 200, 200);
        let a = get_bounty_analytics(&env, bounty_id).unwrap();
        assert_eq!(a.total_amount_released, 200);
        assert_eq!(a.remaining_amount, 800);
        assert_eq!(a.partial_releases_count, 1);

        // Second partial release
        update_analytics_on_release(&env, bounty_id, 300, 300);
        let a = get_bounty_analytics(&env, bounty_id).unwrap();
        assert_eq!(a.total_amount_released, 500);
        assert_eq!(a.remaining_amount, 500);
        assert_eq!(a.partial_releases_count, 2);

        // Third partial release
        update_analytics_on_release(&env, bounty_id, 100, 400);
        let a = get_bounty_analytics(&env, bounty_id).unwrap();
        assert_eq!(a.total_amount_released, 600);
        assert_eq!(a.remaining_amount, 400);
        assert_eq!(a.partial_releases_count, 3);

        // Refund count should remain untouched
        assert_eq!(a.partial_refunds_count, 0);
        assert_eq!(a.total_amount_refunded, 0);
    });
}

/// Test multiple sequential refunds update counters and remaining_amount correctly
#[test]
fn test_direct_sequential_refunds() {
    let env = Env::default();
    let contract_id = env.register_contract(None, DummyContract);

    env.as_contract(&contract_id, || {
        let bounty_id = 12u64;
        let amount = 1000i128;

        init_bounty_analytics(&env, bounty_id, amount, 100);

        // First partial refund
        update_analytics_on_refund(&env, bounty_id, 150, 200);
        let a = get_bounty_analytics(&env, bounty_id).unwrap();
        assert_eq!(a.total_amount_refunded, 150);
        assert_eq!(a.remaining_amount, 850);
        assert_eq!(a.partial_refunds_count, 1);

        // Second partial refund
        update_analytics_on_refund(&env, bounty_id, 250, 300);
        let a = get_bounty_analytics(&env, bounty_id).unwrap();
        assert_eq!(a.total_amount_refunded, 400);
        assert_eq!(a.remaining_amount, 600);
        assert_eq!(a.partial_refunds_count, 2);

        // Release count should remain untouched
        assert_eq!(a.partial_releases_count, 0);
        assert_eq!(a.total_amount_released, 0);
    });
}

/// Test saturating_sub boundary — release/refund exceeding remaining can go negative
/// because saturating_sub on i128 prevents integer overflow (not negative balances).
/// remaining_amount going negative is a known quirk worth surfacing.
#[test]
fn test_direct_saturating_sub_boundary() {
    let env = Env::default();
    let contract_id = env.register_contract(None, DummyContract);

    env.as_contract(&contract_id, || {
        let bounty_id = 13u64;
        let amount = 100i128;

        // Release more than remaining — saturating_sub prevents overflow but NOT underflow below 0
        init_bounty_analytics(&env, bounty_id, amount, 100);
        update_analytics_on_release(&env, bounty_id, 200, 200);
        let a = get_bounty_analytics(&env, bounty_id).unwrap();
        // saturating_sub(100, 200) on i128 = -100 — does NOT floor at zero
        assert_eq!(a.remaining_amount, -100, "saturating_sub on i128 allows negative values");
        assert_eq!(a.total_amount_released, 200, "total_released accumulates via raw +=");
        assert_eq!(a.partial_releases_count, 1);

        // Refund on a different bounty — refund more than remaining
        let bounty_id2 = 14u64;
        init_bounty_analytics(&env, bounty_id2, 50, 100);
        update_analytics_on_refund(&env, bounty_id2, 500, 200);
        let a2 = get_bounty_analytics(&env, bounty_id2).unwrap();
        assert_eq!(a2.remaining_amount, -450, "refund excess goes negative via saturating_sub");
        assert_eq!(a2.total_amount_refunded, 500, "total_refunded accumulates via raw +=");
        assert_eq!(a2.partial_refunds_count, 1);
    });
}

/// Test that calling update_analytics_on_release on uninitialized bounty is a silent no-op
#[test]
fn test_direct_release_on_uninitialized_bounty() {
    let env = Env::default();
    let contract_id = env.register_contract(None, DummyContract);

    env.as_contract(&contract_id, || {
        let bounty_id = 9999u64;

        // This should not panic — just silent no-op
        update_analytics_on_release(&env, bounty_id, 100, 200);

        // Verify no record was created
        let result = get_bounty_analytics(&env, bounty_id);
        assert!(result.is_none(), "no analytics record should exist for uninitialized bounty");
    });
}

/// Test that calling update_analytics_on_refund on uninitialized bounty is a silent no-op
#[test]
fn test_direct_refund_on_uninitialized_bounty() {
    let env = Env::default();
    let contract_id = env.register_contract(None, DummyContract);

    env.as_contract(&contract_id, || {
        let bounty_id = 9998u64;

        // This should not panic — just silent no-op
        update_analytics_on_refund(&env, bounty_id, 100, 200);

        // Verify no record was created
        let result = get_bounty_analytics(&env, bounty_id);
        assert!(result.is_none(), "no analytics record should exist for uninitialized bounty");
    });
}
