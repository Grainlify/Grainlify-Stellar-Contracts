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
