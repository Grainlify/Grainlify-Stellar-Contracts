#![cfg(test)]

//! Comprehensive coverage-boost tests for Issue #177.
//! Exercises error paths, lifecycle edges, and all public getters

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

struct Cx<'a> {
    env: Env,
    admin: Address,
    depositor: Address,
    contributor: Address,
    token_id: Address,
    client: BountyEscrowContractClient<'a>,
}

impl<'a> Cx<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_id = env
            .register_stellar_asset_contract_v2(token_admin.clone())
            .address();
        client.init(&admin, &token_id);
        Self { env, admin, depositor, contributor, token_id, client }
    }
    fn fund(&self, who: &Address, amount: i128) {
        let sac = token::StellarAssetClient::new(&self.env, &self.token_id);
        sac.mint(who, &amount);
    }
    fn lock(&self, bid: u64, amt: i128) {
        let dl = self.env.ledger().timestamp() + 3600;
        self.client.lock_funds(&self.depositor, &bid, &amt, &dl);
    }
}

// ─── error paths ───

#[test]
fn error_amount_below_minimum() {
    let s = Cx::new(); s.fund(&s.depositor, 5000i128);
    s.client.set_amount_policy(&s.admin.clone(), &500i128, &100_000i128);
    let r = s.client.try_lock_funds(&s.depositor, &1u64, &10i128, &(s.env.ledger().timestamp() + 3600));
    assert!(r.is_err());
}
#[test]
fn error_amount_above_maximum() {
    let s = Cx::new(); s.fund(&s.depositor, 500_000i128);
    s.client.set_amount_policy(&s.admin.clone(), &10i128, &1_000i128);
    let r = s.client.try_lock_funds(&s.depositor, &1u64, &5_000i128, &(s.env.ledger().timestamp() + 3600));
    assert!(r.is_err());
}
#[test]
fn error_bounty_not_found() {
    let s = Cx::new();
    let r = s.client.try_get_escrow_info(&99u64);
    assert!(r.is_err());
}
#[test]
fn error_insufficient_balance() {
    let s = Cx::new(); s.fund(&s.depositor, 50i128);
    let r = s.client.try_lock_funds(&s.depositor, &1u64, &1000i128, &(s.env.ledger().timestamp() + 3600));
    assert!(r.is_err());
}
#[test]
fn error_duplicate_bounty_id() {
    let s = Cx::new(); s.fund(&s.depositor, 10_000i128);
    s.lock(1u64, 1000i128);
    let r = s.client.try_lock_funds(&s.depositor, &1u64, &500i128, &(s.env.ledger().timestamp() + 3600));
    assert!(r.is_err());
}

// ─── full lifecycle ───

#[test]
fn full_lifecycle_with_refund_approve_flow() {
    let s = Cx::new(); s.fund(&s.depositor, 10_000i128);
    // claim path
    s.lock(1u64, 1000i128);
    s.client.authorize_claim(&1u64, &s.contributor);
    s.client.claim(&1u64);
    // refund path on separate bounty
    s.lock(2u64, 1000i128);
    s.client.approve_refund(&2u64, &100i128, &s.depositor, &RefundMode::Full);
    s.client.refund(&2u64);
}

#[test]
fn lifecycle_with_amount_policy_restrictions() {
    let s = Cx::new(); s.fund(&s.depositor, 20_000i128);
    s.client.set_amount_policy(&s.admin.clone(), &100i128, &5_000i128);
    s.lock(2u64, 2000i128);
    // cancel pending claim path
    s.client.authorize_claim(&2u64, &s.contributor);
    s.client.cancel_pending_claim(&2u64);
}

// ─── getters: public read APIs ───

#[test]
fn exercise_all_getters() {
    let s = Cx::new(); s.fund(&s.depositor, 20_000i128);
    s.lock(1u64, 1000i128); s.lock(2u64, 2000i128);
    let _pf = s.client.get_pause_flags();
    let _ms = s.client.get_multisig_config();
    let _gv = s.client.get_min_governance_version();
    let _bal = s.client.get_balance();
    let _gov = s.client.get_governance_contract();
    let _agg = s.client.get_aggregate_stats_full_scan();
    let _ids = s.client.get_escrow_ids_by_status(&EscrowStatus::Locked, &0u32, &10u32);
    let _qs = s.client.query_escrows_by_status(&EscrowStatus::Locked, &0u32, &10u32);
    let _ac = s.client.count_bounties_by_status(&EscrowStatus::Locked);
    let _sc = s.client.count_by_status_full_scan(&EscrowStatus::Locked);
    let _vs = s.client.get_volume_by_status(&EscrowStatus::Locked);
    let _vf = s.client.volume_by_status_full_scan(&EscrowStatus::Locked);
    let hv = s.client.get_high_value_bounties(&500i128, &5u32);
    assert_eq!(hv.len(), 2);
    assert_eq!(hv.get(0).unwrap(), 1u64);
    assert_eq!(hv.get(1).unwrap(), 2u64);
    let _ds = s.client.get_depositor_stats(&s.depositor);
    let _ei = s.client.get_escrow_info(&1u64);
    let _rh = s.client.get_refund_history(&1u64);
    let _re = s.client.get_refund_eligibility(&1u64);
    let _ca = s.client.get_contract_analytics();
    s.client.emit_analytics_snapshot_event();
    let _av = s.client.get_admin_audit_view();
}

// ─── anti-abuse rate-limit ───

#[test]
fn anti_abuse_rate_limit_exceed() {
    let s = Cx::new(); s.fund(&s.depositor, 1_000_000i128);
    // disable whitelist for cleaner path
    let random = Address::generate(&s.env);
    s.client.set_whitelist(&random, &false);
    // hammer with minimal amounts to hit default anti-abuse limit (100)
    for i in 0..150u64 {
        let dl = s.env.ledger().timestamp() + 3600;
        let r = s.client.try_lock_funds(&s.depositor, &i, &50i128, &dl);
        if r.is_err() { break; } // limit hit, good
    }
}

// ─── fee with custom circuit-breaker ───

#[test]
fn fee_with_circuit_breaker_exercise() {
    let s = Cx::new(); s.fund(&s.depositor, 10_000i128);
    s.client.update_fee_config(&Some(500i128), &Some(300i128), &Some(s.admin.clone()), &Some(true));
    s.lock(1u64, 1000i128);
    // configure breaker low and attempt retry
    s.client.set_circuit_breaker_config(&3u32, &2u32, &5u32);
    s.client.set_circuit_breaker_admin(&s.admin.clone());
    let _log = s.client.get_circuit_error_log();
}

// ─── status full-scan reconciliation helpers ───
//
// `count_by_status_full_scan` and `volume_by_status_full_scan` are reconciliation
// helpers: they recompute per-status counts and volumes from a full O(N) scan of
// the escrow index, so they can cross-check the O(1) running counters. Previously
// they were only touched by `exercise_all_getters`, which discards both return
// values and only ever passes `Locked` — so neither the correctness of the scan
// nor its agreement with the O(1) counters was ever verified. These tests supply
// the missing assertions.

/// Drive a mix of bounties into all four statuses and assert the full-scan
/// helpers return exactly the manually-computed count and volume for each.
///
/// Volume semantics under scan (mirrors the contract):
/// - `Locked` / `PartiallyRefunded` sum `remaining_amount`.
/// - `Released` / `Refunded` sum the original `amount`.
#[test]
fn full_scan_correctness_all_statuses() {
    let s = Cx::new();
    s.fund(&s.depositor, 1_000_000i128);

    // Two Locked bounties: remaining 1000 + 2000.
    s.lock(1u64, 1000i128);
    s.lock(2u64, 2000i128);

    // Two Released bounties (claimed in full): amount 1000 + 3000.
    s.lock(3u64, 1000i128);
    s.client.authorize_claim(&3u64, &s.contributor);
    s.client.claim(&3u64);
    s.lock(4u64, 3000i128);
    s.client.authorize_claim(&4u64, &s.contributor);
    s.client.claim(&4u64);

    // One fully Refunded bounty: amount 1000.
    s.lock(5u64, 1000i128);
    s.client.approve_refund(&5u64, &1000i128, &s.depositor, &RefundMode::Full);
    s.client.refund(&5u64);

    // One PartiallyRefunded bounty: 2000 locked, 500 refunded, remaining 1500.
    s.lock(6u64, 2000i128);
    s.client.approve_refund(&6u64, &500i128, &s.depositor, &RefundMode::Partial);
    s.client.refund(&6u64);

    assert_eq!(s.client.get_escrow_count(), 6);

    // Locked: exactly the two untouched bounties.
    assert_eq!(s.client.count_by_status_full_scan(&EscrowStatus::Locked), 2);
    assert_eq!(s.client.volume_by_status_full_scan(&EscrowStatus::Locked), 3000i128);

    // Released: both claimed bounties, valued at their original amount.
    assert_eq!(s.client.count_by_status_full_scan(&EscrowStatus::Released), 2);
    assert_eq!(s.client.volume_by_status_full_scan(&EscrowStatus::Released), 4000i128);

    // Refunded: the single fully-refunded bounty.
    assert_eq!(s.client.count_by_status_full_scan(&EscrowStatus::Refunded), 1);
    assert_eq!(s.client.volume_by_status_full_scan(&EscrowStatus::Refunded), 1000i128);

    // PartiallyRefunded: the single partially-refunded bounty, valued at its
    // remaining_amount after the 500 refund.
    assert_eq!(s.client.count_by_status_full_scan(&EscrowStatus::PartiallyRefunded), 1);
    assert_eq!(s.client.volume_by_status_full_scan(&EscrowStatus::PartiallyRefunded), 1500i128);

    // Document the intentional difference from the O(1) counter: the running
    // `count_locked` folds PartiallyRefunded in with Locked, so it reports 3
    // where the exact full scan reports 2. This is exactly the kind of drift the
    // full-scan helper exists to expose.
    assert_eq!(s.client.count_bounties_by_status(&EscrowStatus::Locked), 3);
    assert_eq!(s.client.count_by_status_full_scan(&EscrowStatus::Locked), 2);
}

/// The reconciliation use case: with no partial refunds in play, the full-scan
/// helpers must agree exactly with the contract's own O(1) running counters for
/// Locked, Released, and Refunded — count and volume alike.
#[test]
fn full_scan_agrees_with_o1_counters() {
    let s = Cx::new();
    s.fund(&s.depositor, 1_000_000i128);

    s.lock(1u64, 1000i128);
    s.lock(2u64, 2000i128);

    s.lock(3u64, 1500i128);
    s.client.authorize_claim(&3u64, &s.contributor);
    s.client.claim(&3u64);

    s.lock(4u64, 1000i128);
    s.client.approve_refund(&4u64, &1000i128, &s.depositor, &RefundMode::Full);
    s.client.refund(&4u64);

    for status in [
        EscrowStatus::Locked,
        EscrowStatus::Released,
        EscrowStatus::Refunded,
    ] {
        assert_eq!(
            s.client.count_by_status_full_scan(&status),
            s.client.count_bounties_by_status(&status),
            "count mismatch between full scan and O(1) counter",
        );
        assert_eq!(
            s.client.volume_by_status_full_scan(&status),
            s.client.get_volume_by_status(&status),
            "volume mismatch between full scan and O(1) counter",
        );
    }
}

/// A status with zero matching bounties must return 0 count and 0 volume from
/// the full-scan helpers, both on an empty index and alongside unrelated
/// bounties — never a panic.
#[test]
fn full_scan_zero_match_returns_zero() {
    // Empty index: every status is 0/0.
    let empty = Cx::new();
    for status in [
        EscrowStatus::Locked,
        EscrowStatus::Released,
        EscrowStatus::Refunded,
        EscrowStatus::PartiallyRefunded,
    ] {
        assert_eq!(empty.client.count_by_status_full_scan(&status), 0);
        assert_eq!(empty.client.volume_by_status_full_scan(&status), 0i128);
    }

    // Populated with only Locked bounties: the other statuses stay 0/0.
    let s = Cx::new();
    s.fund(&s.depositor, 1_000_000i128);
    s.lock(1u64, 1000i128);
    s.lock(2u64, 2000i128);

    for status in [
        EscrowStatus::Released,
        EscrowStatus::Refunded,
        EscrowStatus::PartiallyRefunded,
    ] {
        assert_eq!(s.client.count_by_status_full_scan(&status), 0);
        assert_eq!(s.client.volume_by_status_full_scan(&status), 0i128);
    }
}
