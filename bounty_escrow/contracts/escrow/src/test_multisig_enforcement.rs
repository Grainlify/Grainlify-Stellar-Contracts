#![cfg(test)]

//! Tests for multisig large-release enforcement in `release_funds` and
//! `partial_release`.
//!
//! Verifies that when `escrow.amount >= threshold_amount`, the release
//! requires `required_signatures` distinct signers to have called
//! `approve_large_release` before funds can move, and that consumed
//! approvals are cleared.

use super::*;
use soroban_sdk::{
    testutils::Address as _,
    vec, Address, Env,
};

fn create_token_contract<'a>(
    e: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract_address = e.register_stellar_asset_contract(admin.clone());
    (
        token::Client::new(e, &contract_address),
        token::StellarAssetClient::new(e, &contract_address),
    )
}

fn create_escrow_contract<'a>(e: &Env) -> BountyEscrowContractClient<'a> {
    let contract_id = e.register_contract(None, BountyEscrowContract);
    BountyEscrowContractClient::new(e, &contract_id)
}

struct TestSetup<'a> {
    env: Env,
    admin: Address,
    depositor: Address,
    contributor: Address,
    signer_a: Address,
    signer_b: Address,
    escrow: BountyEscrowContractClient<'a>,
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
}

impl<'a> TestSetup<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);
        let signer_a = Address::generate(&env);
        let signer_b = Address::generate(&env);

        let (token, token_admin) = create_token_contract(&env, &admin);
        let escrow = create_escrow_contract(&env);

        escrow.init(&admin, &token.address);
        token_admin.mint(&depositor, &10_000_000);
        token_admin.mint(&admin, &10_000_000);

        Self {
            env,
            admin,
            depositor,
            contributor,
            signer_a,
            signer_b,
            escrow,
            token,
            token_admin,
        }
    }

    /// Lock a bounty.
    fn lock(&self, bounty_id: u64, amount: i128) {
        let deadline = self.env.ledger().timestamp() + 10_000;
        self.escrow
            .lock_funds(&self.depositor, &bounty_id, &amount, &deadline);
    }

    /// Configure multisig with threshold_amount, signers, required_signatures.
    fn configure_multisig(&self, threshold: i128, required: u32) {
        self.escrow.update_multisig_config(
            &threshold,
            &vec![&self.env, self.signer_a.clone(), self.signer_b.clone()],
            &required,
        );
    }

    /// Read the persisted ReleaseApproval from contract storage.
    fn read_release_approval(&self, bounty_id: u64) -> Option<ReleaseApproval> {
        self.env.as_contract(&self.escrow.address, || {
            self.env
                .storage()
                .persistent()
                .get(&DataKey::ReleaseApproval(bounty_id))
        })
    }

    /// Get the contract's token balance.
    fn contract_balance(&self) -> i128 {
        self.token.balance(&self.escrow.address)
    }

    /// Get the contributor's token balance.
    fn contributor_balance(&self) -> i128 {
        self.token.balance(&self.contributor)
    }
}

// ─────────────────────────────────────────────────────────
// 1. Above-threshold release_funds requires multisig approval
// ─────────────────────────────────────────────────────────

#[test]
fn release_funds_above_threshold_rejected_without_approvals() {
    let s = TestSetup::new();
    s.lock(1, 1_000);

    // threshold 500, need 2-of-2 signatures
    s.configure_multisig(500, 2);

    // No approvals have been collected — must be rejected.
    let result = s
        .escrow
        .try_release_funds(&1, &s.contributor);
    assert_eq!(result, Err(Ok(Error::ApprovalRequired)));
}

#[test]
fn release_funds_above_threshold_succeeds_with_enough_approvals() {
    let s = TestSetup::new();
    s.lock(1, 1_000);

    s.configure_multisig(500, 2);

    // Both signers approve.
    s.escrow
        .approve_large_release(&1, &s.contributor, &s.signer_a);
    s.escrow
        .approve_large_release(&1, &s.contributor, &s.signer_b);

    let approval = s.read_release_approval(1).unwrap();
    assert_eq!(approval.approvals.len(), 2);

    let contract_before = s.contract_balance();
    let contributor_before = s.contributor_balance();

    // Now release must succeed.
    let result = s.escrow.try_release_funds(&1, &s.contributor);
    assert_eq!(result, Ok(Ok(())));

    // Funds moved.
    assert_eq!(s.contract_balance(), contract_before - 1_000);
    assert_eq!(s.contributor_balance(), contributor_before + 1_000);

    // Approval record is consumed.
    assert!(s.read_release_approval(1).is_none());
}

#[test]
fn release_funds_above_threshold_rejected_with_one_of_two_approvals() {
    let s = TestSetup::new();
    s.lock(1, 1_000);

    s.configure_multisig(500, 2);

    // Only one signer approves — insufficient.
    s.escrow
        .approve_large_release(&1, &s.contributor, &s.signer_a);

    let result = s
        .escrow
        .try_release_funds(&1, &s.contributor);
    assert_eq!(result, Err(Ok(Error::ApprovalRequired)));

    // Second signer approves — now it should work.
    s.escrow
        .approve_large_release(&1, &s.contributor, &s.signer_b);

    let result = s.escrow.try_release_funds(&1, &s.contributor);
    assert_eq!(result, Ok(Ok(())));
}

// ─────────────────────────────────────────────────────────
// 2. Below-threshold release_funds is unaffected
// ─────────────────────────────────────────────────────────

#[test]
fn release_funds_below_threshold_does_not_require_approval() {
    let s = TestSetup::new();
    s.lock(1, 100);

    s.configure_multisig(500, 2);

    // amount (100) < threshold (500) — no approval needed, works with admin only.
    let result = s.escrow.try_release_funds(&1, &s.contributor);
    assert_eq!(result, Ok(Ok(())));
}

#[test]
fn release_funds_default_threshold_never_requires_approval() {
    let s = TestSetup::new();
    s.lock(1, 1_000_000);

    // `update_multisig_config` is never called — default threshold is i128::MAX,
    // so every release is below threshold and proceeds on admin auth alone.
    let result = s.escrow.try_release_funds(&1, &s.contributor);
    assert_eq!(result, Ok(Ok(())));
}

// ─────────────────────────────────────────────────────────
// 3. partial_release above threshold requires multisig
// ─────────────────────────────────────────────────────────

#[test]
fn partial_release_above_threshold_rejected_without_approvals() {
    let s = TestSetup::new();
    s.lock(1, 1_000);

    s.configure_multisig(500, 2);

    // Even a small payout_amount (10) from a large (1_000) escrow
    // is gated because we check against escrow.amount.
    let result = s
        .escrow
        .try_partial_release(&1, &s.contributor, &10);
    assert_eq!(result, Err(Ok(Error::ApprovalRequired)));
}

#[test]
fn partial_release_above_threshold_succeeds_with_enough_approvals() {
    let s = TestSetup::new();
    s.lock(1, 1_000);

    s.configure_multisig(500, 2);

    s.escrow
        .approve_large_release(&1, &s.contributor, &s.signer_a);
    s.escrow
        .approve_large_release(&1, &s.contributor, &s.signer_b);

    let contract_before = s.contract_balance();
    let contributor_before = s.contributor_balance();

    let result = s
        .escrow
        .try_partial_release(&1, &s.contributor, &10);
    assert_eq!(result, Ok(Ok(())));

    // Funds moved.
    assert_eq!(s.contract_balance(), contract_before - 10);
    assert_eq!(s.contributor_balance(), contributor_before + 10);

    // Approval record is consumed after a gated partial release.
    assert!(s.read_release_approval(1).is_none());
}

#[test]
fn partial_release_below_threshold_not_affected() {
    let s = TestSetup::new();
    s.lock(1, 100);

    s.configure_multisig(500, 2);

    // amount (100) < threshold (500) — no approval needed.
    let result = s
        .escrow
        .try_partial_release(&1, &s.contributor, &50);
    assert_eq!(result, Ok(Ok(())));
}

// ─────────────────────────────────────────────────────────
// 4. Consumed approvals cannot be reused
// ─────────────────────────────────────────────────────────

#[test]
fn consumed_approval_cannot_be_replayed_for_second_release() {
    let s = TestSetup::new();
    // First bounty: above threshold.
    s.lock(1, 1_000);
    s.configure_multisig(500, 2);

    // Gather approvals and release.
    s.escrow
        .approve_large_release(&1, &s.contributor, &s.signer_a);
    s.escrow
        .approve_large_release(&1, &s.contributor, &s.signer_b);

    let result = s.escrow.try_release_funds(&1, &s.contributor);
    assert_eq!(result, Ok(Ok(())));
    assert!(s.read_release_approval(1).is_none());

    // Second bounty (same admin, new bounty_id): approvals are fresh again.
    // Use a different depositor with fresh funds since the first one's funds went to contributor.
    let depositor2 = Address::generate(&s.env);
    s.token_admin.mint(&depositor2, &10_000_000);
    let deadline = s.env.ledger().timestamp() + 10_000;
    s.escrow
        .lock_funds(&depositor2, &2, &1_000, &deadline);

    // Without fresh approvals, release must fail.
    let result = s.escrow.try_release_funds(&2, &s.contributor);
    assert_eq!(result, Err(Ok(Error::ApprovalRequired)));

    // After fresh approvals, it works.
    s.escrow
        .approve_large_release(&2, &s.contributor, &s.signer_a);
    s.escrow
        .approve_large_release(&2, &s.contributor, &s.signer_b);

    let result = s.escrow.try_release_funds(&2, &s.contributor);
    assert_eq!(result, Ok(Ok(())));
}

// ─────────────────────────────────────────────────────────
// 5. Threshold of zero — all releases require multisig
// ─────────────────────────────────────────────────────────

#[test]
fn threshold_zero_requires_approval_for_any_release() {
    let s = TestSetup::new();
    s.lock(1, 1);

    // threshold 0 means any non-negative amount >= 0, so every release is gated.
    s.configure_multisig(0, 1);

    let result = s
        .escrow
        .try_release_funds(&1, &s.contributor);
    assert_eq!(result, Err(Ok(Error::ApprovalRequired)));

    s.escrow
        .approve_large_release(&1, &s.contributor, &s.signer_a);

    let result = s.escrow.try_release_funds(&1, &s.contributor);
    assert_eq!(result, Ok(Ok(())));
}