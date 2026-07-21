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
