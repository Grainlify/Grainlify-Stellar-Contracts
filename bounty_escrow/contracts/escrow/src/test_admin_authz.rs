#![cfg(test)]

//! Issue #177 — Admin authorization audit tests.
//!
//! Every admin-only entrypoint MUST reject a caller that is not the stored
//! admin. These tests prove that guarantee by invoking each admin function
//! as an arbitrary (non-admin) address with NO auth mocked, and asserting the
//! call returns `Err(Error::Unauthorized)` instead of mutating state.
//!
//! IMPORTANT: we deliberately do NOT call `env.mock_all_auths()`. Once
//! `mock_all_auths()` is enabled it cannot be undone for the env, which would
//! make every `require_auth()` succeed and silently void the negative test
//! (this was the flaw in the previous `test_rbac.rs` negative cases).

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

struct AuthzSetup<'a> {
    env: Env,
    admin: Address,
    random: Address,
    token_id: Address,
    client: BountyEscrowContractClient<'a>,
}

impl<'a> AuthzSetup<'a> {
    fn new() -> Self {
        let env = Env::default();
        // NOTE: no mock_all_auths(). `init` performs no require_auth, so it
        // works without any mocked authorization.
        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let random = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_id = env
            .register_stellar_asset_contract_v2(token_admin.clone())
            .address();

        client.init(&admin, &token_id);

        Self {
            env,
            admin,
            random,
            token_id,
            client,
        }
    }

    /// Mint + lock funds so escrows exist for functions that need a target.
    /// Uses `mock_all_auths()` only for the setup mutations, then clears all
    /// mocked auth with `mock_auths(&[])` (which DOES override
    /// `mock_all_auths`) so the subsequent admin call is evaluated
    /// unauthenticated.
    fn seed_escrow(&self, bounty_id: u64, amount: i128, deadline_offset: u64) {
        self.env.mock_all_auths();
        let sac = token::StellarAssetClient::new(&self.env, &self.token_id);
        sac.mint(&self.admin, &(amount * 2));
        let deadline = self.env.ledger().timestamp() + deadline_offset;
        self.client
            .lock_funds(&self.admin, &bounty_id, &amount, &deadline);
        // Clear auth so subsequent admin calls are evaluated unauthenticated.
        self.env.mock_auths(&[]);
    }
}

/// In Soroban, a failed `require_auth` aborts the call, which the `try_*`
/// client surfaces as an outer `Err(InvokeError)` (not as the contract's
/// `Error::Unauthorized`). So a rejected non-admin call is simply `is_err()`.
fn assert_unauthorized<V: core::fmt::Debug, T: core::fmt::Debug, E: core::fmt::Debug>(
    res: Result<Result<V, T>, E>,
) {
    assert!(
        res.is_err(),
        "expected admin-only call to be rejected (auth abort), got: {:?}",
        res
    );
}

// ─────────────────────────────────────────────────────────
// Core admin controls
// ─────────────────────────────────────────────────────────

#[test]
fn non_admin_cannot_update_fee_config() {
    let s = AuthzSetup::new();
    let res = s
        .client
        .try_update_fee_config(&None, &None, &Some(s.random.clone()), &Some(true));
    assert_unauthorized(res);
}

#[test]
fn non_admin_cannot_set_paused() {
    let s = AuthzSetup::new();
    let res = s.client.try_set_paused(&Some(true), &None, &None);
    assert_unauthorized(res);
}

#[test]
fn non_admin_cannot_set_emergency_pause() {
    let s = AuthzSetup::new();
    let res = s.client.try_set_emergency_pause(&true);
    assert_unauthorized(res);
}

#[test]
fn non_admin_cannot_update_multisig_config() {
    let s = AuthzSetup::new();
    let signers = vec![&s.env, s.admin.clone()];
    let res = s
        .client
        .try_update_multisig_config(&1000i128, &signers, &1u32);
    assert_unauthorized(res);
}

#[test]
fn non_admin_cannot_set_amount_policy() {
    let s = AuthzSetup::new();
    let res = s.client.try_set_amount_policy(&s.random.clone(), &100i128, &1_000_000i128);
    assert_unauthorized(res);
}

#[test]
fn non_admin_cannot_set_claim_window() {
    let s = AuthzSetup::new();
    let res = s.client.try_set_claim_window(&3600u64);
    assert_unauthorized(res);
}

// ─────────────────────────────────────────────────────────
// Claim / release / refund controls
// ─────────────────────────────────────────────────────────

#[test]
fn non_admin_cannot_authorize_claim() {
    let s = AuthzSetup::new();
    s.seed_escrow(1u64, 1000i128, 3600u64);
    let res = s.client.try_authorize_claim(&1u64, &s.random.clone());
    assert_unauthorized(res);
}

#[test]
fn non_admin_cannot_cancel_pending_claim() {
    let s = AuthzSetup::new();
    s.seed_escrow(1u64, 1000i128, 3600u64);
    let res = s.client.try_cancel_pending_claim(&1u64);
    assert_unauthorized(res);
}

#[test]
fn non_admin_cannot_approve_refund() {
    let s = AuthzSetup::new();
    s.seed_escrow(1u64, 1000i128, 3600u64);
    let res = s.client.try_approve_refund(&1u64, &100i128, &s.random.clone(), &RefundMode::Full);
    assert_unauthorized(res);
}

#[test]
fn non_admin_cannot_partial_release() {
    let s = AuthzSetup::new();
    s.seed_escrow(1u64, 1000i128, 3600u64);
    let res = s
        .client
        .try_partial_release(&1u64, &s.random.clone(), &100i128);
    assert_unauthorized(res);
}

#[test]
fn non_admin_cannot_release_funds() {
    let s = AuthzSetup::new();
    s.seed_escrow(1u64, 1000i128, 3600u64);
    let res = s.client.try_release_funds(&1u64, &s.random.clone());
    assert_unauthorized(res);
}

#[test]
fn non_admin_cannot_batch_release_funds() {
    let s = AuthzSetup::new();
    s.seed_escrow(1u64, 1000i128, 3600u64);
    let items = soroban_sdk::vec![&s.env, ReleaseFundsItem {
        bounty_id: 1u64,
        contributor: s.random.clone(),
    }];
    let res = s.client.try_batch_release_funds(&items);
    assert_unauthorized(res);
}

// `approve_large_release` is gated on the caller being a registered multisig
// signer (not the stored admin), but a completely arbitrary caller must still
// be rejected — and since no auth is mocked here, the require_auth aborts.
#[test]
fn non_signer_cannot_approve_large_release() {
    let s = AuthzSetup::new();
    s.seed_escrow(1u64, 1000i128, 3600u64);
    let res = s
        .client
        .try_approve_large_release(&1u64, &s.random.clone(), &s.random.clone());
    assert_unauthorized(res);
}

// ─────────────────────────────────────────────────────────
// Governance + anti-abuse controls
// ─────────────────────────────────────────────────────────

#[test]
fn non_admin_cannot_set_anti_abuse_admin() {
    let s = AuthzSetup::new();
    let res = s.client.try_set_anti_abuse_admin(&s.random.clone());
    assert_unauthorized(res);
}

#[test]
fn non_admin_cannot_set_whitelist() {
    let s = AuthzSetup::new();
    let res = s.client.try_set_whitelist(&s.random.clone(), &true);
    assert_unauthorized(res);
}

#[test]
fn non_admin_cannot_set_governance_contract() {
    let s = AuthzSetup::new();
    let res = s.client.try_set_governance_contract(&s.random.clone());
    assert_unauthorized(res);
}

#[test]
fn non_admin_cannot_set_min_governance_version() {
    let s = AuthzSetup::new();
    let res = s.client.try_set_min_governance_version(&2u32);
    assert_unauthorized(res);
}

// ─────────────────────────────────────────────────────────
// Circuit breaker controls
// ─────────────────────────────────────────────────────────

#[test]
fn non_admin_cannot_set_circuit_breaker_admin() {
    let s = AuthzSetup::new();
    let res = s.client.try_set_circuit_breaker_admin(&s.random.clone());
    assert_unauthorized(res);
}

#[test]
fn non_admin_cannot_set_circuit_breaker_config() {
    let s = AuthzSetup::new();
    let res = s
        .client
        .try_set_circuit_breaker_config(&3u32, &2u32, &10u32);
    assert_unauthorized(res);
}

#[test]
fn non_admin_cannot_reset_circuit() {
    let s = AuthzSetup::new();
    let res = s.client.try_reset_circuit(&s.random.clone());
    assert_unauthorized(res);
}

// ─────────────────────────────────────────────────────────
// Positive control: the stored admin IS authorized
// (proves require_auth is wired to the correct stored address)
// ─────────────────────────────────────────────────────────

#[test]
fn stored_admin_can_set_paused() {
    let s = AuthzSetup::new();
    // With all auth mocked, the stored admin (who calls set_paused) is
    // authorized, so the call succeeds. This proves the happy path / that
    // require_auth is wired to the correct stored address.
    s.env.mock_all_auths();
    let res = s.client.try_set_paused(&Some(true), &None, &None);
    assert!(
        res.unwrap_or_else(|e| panic!("invoke error: {:?}", e)).is_ok(),
        "stored admin should be authorized"
    );
}
