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
use crate::RefundMode;
use soroban_sdk::{contract, contractimpl, Address};

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
        update_analytics_on_release(&env, 1, 200, p1_ts).unwrap();
        update_analytics_on_refund(&env, 1, 800, p2_ts).unwrap();

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
        update_analytics_on_release(&env, bounty_id, 200, 200).unwrap();
        let a = get_bounty_analytics(&env, bounty_id).unwrap();
        assert_eq!(a.total_amount_released, 200);
        assert_eq!(a.remaining_amount, 800);
        assert_eq!(a.partial_releases_count, 1);

        // Second partial release
        update_analytics_on_release(&env, bounty_id, 300, 300).unwrap();
        let a = get_bounty_analytics(&env, bounty_id).unwrap();
        assert_eq!(a.total_amount_released, 500);
        assert_eq!(a.remaining_amount, 500);
        assert_eq!(a.partial_releases_count, 2);

        // Third partial release
        update_analytics_on_release(&env, bounty_id, 100, 400).unwrap();
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
        update_analytics_on_refund(&env, bounty_id, 150, 200).unwrap();
        let a = get_bounty_analytics(&env, bounty_id).unwrap();
        assert_eq!(a.total_amount_refunded, 150);
        assert_eq!(a.remaining_amount, 850);
        assert_eq!(a.partial_refunds_count, 1);

        // Second partial refund
        update_analytics_on_refund(&env, bounty_id, 250, 300).unwrap();
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
        update_analytics_on_release(&env, bounty_id, 200, 200).unwrap();
        let a = get_bounty_analytics(&env, bounty_id).unwrap();
        // saturating_sub(100, 200) on i128 = -100 — does NOT floor at zero
        assert_eq!(a.remaining_amount, -100, "saturating_sub on i128 allows negative values");
        assert_eq!(a.total_amount_released, 200, "total_released accumulates via raw +=");
        assert_eq!(a.partial_releases_count, 1);

        // Refund on a different bounty — refund more than remaining
        let bounty_id2 = 14u64;
        init_bounty_analytics(&env, bounty_id2, 50, 100);
        update_analytics_on_refund(&env, bounty_id2, 500, 200).unwrap();
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
        update_analytics_on_release(&env, bounty_id, 100, 200).unwrap();

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
        update_analytics_on_refund(&env, bounty_id, 100, 200).unwrap();

        // Verify no record was created
        let result = get_bounty_analytics(&env, bounty_id);
        assert!(result.is_none(), "no analytics record should exist for uninitialized bounty");
    });
}

// ============================================================
// End-to-end coverage for get_bounty_analytics (Issue #399)
//
// The tests above exercise the analytics helper functions directly
// (init_bounty_analytics / update_analytics_on_release / update_analytics_on_refund)
// against a dummy contract. None of them drive the *real* bounty escrow
// contract's public entrypoints, so a bug where e.g. release_funds and
// approve_refund/refund disagree on how they update remaining_amount vs.
// total_amount_released/total_amount_refunded could slip through even
// though every isolated field assertion above passes. The tests below
// close that gap by driving `lock_funds` -> `partial_release` ->
// `approve_refund`/`refund` (x2) through the generated contract client
// and then asserting every field of get_bounty_analytics at once.
// ============================================================

fn create_token<'a>(
    env: &Env,
    admin: &Address,
) -> (
    soroban_sdk::token::Client<'a>,
    soroban_sdk::token::StellarAssetClient<'a>,
) {
    let addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    (
        soroban_sdk::token::Client::new(env, &addr),
        soroban_sdk::token::StellarAssetClient::new(env, &addr),
    )
}

fn create_escrow<'a>(env: &Env) -> crate::BountyEscrowContractClient<'a> {
    let contract_id = env.register_contract(None, crate::BountyEscrowContract);
    crate::BountyEscrowContractClient::new(env, &contract_id)
}

/// Drives a realistic mixed lifecycle — lock, a partial release, then a
/// partial refund followed by a final refund that fully drains the
/// remainder — through the real contract entrypoints, and asserts every
/// field of the returned `BountyAnalytics` snapshot against hand-computed
/// values in one shot.
///
/// Note: once a refund leaves any bounty in `PartiallyRefunded` status,
/// both `release_funds` and `partial_release` reject it with
/// `FundsNotLocked` (they require `Locked`), so a "release" cannot follow
/// a partial refund. The mixed sequence below is the realistic analogue:
/// a partial release while still `Locked`, followed by a partial refund
/// and then a second, fully-draining refund — covering both counters
/// (`partial_releases_count`, `partial_refunds_count`) and all three
/// amount fields in a single interleaved run.
#[test]
fn test_get_bounty_analytics_mixed_release_and_refund_sequence() {
    use soroban_sdk::testutils::{Address as _, Ledger};

    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);

    let (token_client, token_sac) = create_token(&env, &admin);
    let client = create_escrow(&env);
    client.init(&admin, &token_client.address);

    let locked_amount = 10_000i128;
    token_sac.mint(&depositor, &locked_amount);

    let bounty_id = 500u64;
    let deadline = 1_000_000u64;
    let locked_at = env.ledger().timestamp();
    client.lock_funds(&depositor, &bounty_id, &locked_amount, &deadline);

    // Partial release #1: 4,000 to the contributor. Escrow stays Locked
    // since 6,000 remains.
    env.ledger().set_timestamp(2_000);
    client.partial_release(&bounty_id, &contributor, &4_000i128);

    // Partial refund #1: 3,000 back to the depositor. Leaves 3,000
    // remaining, transitioning the escrow to PartiallyRefunded.
    env.ledger().set_timestamp(3_000);
    client.approve_refund(&bounty_id, &3_000i128, &depositor, &RefundMode::Partial);
    client.refund(&bounty_id);

    // Final refund: drains the last 3,000, completing the bounty.
    let final_refund_at = 4_000u64;
    env.ledger().set_timestamp(final_refund_at);
    client.approve_refund(&bounty_id, &3_000i128, &depositor, &RefundMode::Full);
    client.refund(&bounty_id);

    // Sanity check on the escrow itself before inspecting analytics.
    let info = client.get_escrow_info(&bounty_id);
    assert_eq!(info.status, crate::EscrowStatus::Refunded);
    assert_eq!(info.remaining_amount, 0);

    // Now assert every analytics field simultaneously.
    let analytics = client.get_bounty_analytics(&bounty_id);
    assert_eq!(analytics.total_amount_locked, 10_000, "total_amount_locked");
    assert_eq!(analytics.total_amount_released, 4_000, "total_amount_released");
    assert_eq!(analytics.total_amount_refunded, 6_000, "total_amount_refunded");
    assert_eq!(analytics.remaining_amount, 0, "remaining_amount");
    assert_eq!(analytics.created_at, locked_at, "created_at");
    assert_eq!(analytics.last_updated, final_refund_at, "last_updated");
    assert_eq!(analytics.partial_releases_count, 1, "partial_releases_count");
    assert_eq!(analytics.partial_refunds_count, 2, "partial_refunds_count");

    // Cross-field consistency: nothing may drift out of sync with the
    // originally locked amount after a mixed release/refund sequence.
    assert_eq!(
        analytics.total_amount_locked,
        analytics.total_amount_released + analytics.total_amount_refunded + analytics.remaining_amount,
        "total_amount_locked must equal released + refunded + remaining after the full sequence"
    );
}

/// get_bounty_analytics must return None for a bounty ID that was never
/// locked — not a default/zeroed BountyAnalytics record, and not a panic —
/// even when the contract is live and already tracking other bounties.
#[test]
fn test_get_bounty_analytics_none_for_never_locked_bounty_on_live_contract() {
    use soroban_sdk::testutils::{Address as _, Ledger};

    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);

    let (token_client, token_sac) = create_token(&env, &admin);
    let client = create_escrow(&env);
    client.init(&admin, &token_client.address);

    // Lock a real, unrelated bounty so the contract has live analytics
    // state, then query an ID that was never locked.
    token_sac.mint(&depositor, &1_000i128);
    client.lock_funds(&depositor, &1u64, &1_000i128, &1_000_000u64);

    let never_locked_id = 777u64;

    // The public entrypoint maps the missing record to a typed error
    // rather than panicking or fabricating a zeroed struct.
    let result = client.try_get_bounty_analytics(&never_locked_id);
    assert!(
        result.is_err(),
        "expected an error for a never-locked bounty id, got {:?}",
        result
    );

    // The underlying analytics module function returns None directly.
    let contract_id = client.address.clone();
    env.as_contract(&contract_id, || {
        assert!(
            get_bounty_analytics(&env, never_locked_id).is_none(),
            "no analytics record should exist for a never-locked bounty id"
        );
        // The bounty that *was* locked is unaffected and still present.
        assert!(get_bounty_analytics(&env, 1u64).is_some());
    });
}
