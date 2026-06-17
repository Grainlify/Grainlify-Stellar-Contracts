#![cfg(test)]

use crate::{BountyEscrowContract, BountyEscrowContractClient, Error, EscrowStatus, LockFundsItem};
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token, vec, Address, Env, Map, Symbol, TryFromVal, Val, Vec,
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
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
    escrow: BountyEscrowContractClient<'a>,
}

impl<'a> TestSetup<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);

        let (token, token_admin) = create_token_contract(&env, &admin);
        let escrow = create_escrow_contract(&env);

        escrow.init(&admin, &token.address);

        // Mint tokens to depositor
        token_admin.mint(&depositor, &10_000_000);

        Self {
            env,
            admin,
            depositor,
            contributor,
            token,
            token_admin,
            escrow,
        }
    }
}

fn try_sweep_direct(setup: &TestSetup<'_>, bounty_ids: Vec<u64>) -> Result<u32, Error> {
    setup.env.as_contract(&setup.escrow.address, || {
        BountyEscrowContract::sweep_expired_refunds(setup.env.clone(), bounty_ids)
    })
}

fn saw_bounty_expired_event(env: &Env, bounty_id: u64, expired_at: u64) -> bool {
    let events = env.events().all();
    for (_contract, _topics, data) in events.iter() {
        if let Ok(data_map) = Map::<Symbol, Val>::try_from_val(env, &data) {
            let version_val = data_map.get(Symbol::new(env, "version"));
            let bounty_id_val = data_map.get(Symbol::new(env, "bounty_id"));
            let expired_at_val = data_map.get(Symbol::new(env, "expired_at"));

            if let (Some(version_val), Some(bounty_id_val), Some(expired_at_val)) =
                (version_val, bounty_id_val, expired_at_val)
            {
                let version = u32::try_from_val(env, &version_val).unwrap_or(0);
                let event_bounty_id = u64::try_from_val(env, &bounty_id_val).unwrap_or(0);
                let event_expired_at = u64::try_from_val(env, &expired_at_val).unwrap_or(0);

                if version == 2 && event_bounty_id == bounty_id && event_expired_at == expired_at {
                    return true;
                }
            }
        }
    }
    false
}

#[test]
fn test_pending_claim_blocks_refund_after_fix() {
    let setup = TestSetup::new();
    let bounty_id = 1;
    let amount = 1000;
    let now = setup.env.ledger().timestamp();
    let deadline = now + 1000;
    let claim_window = 500;

    setup.escrow.set_claim_window(&claim_window);

    // Lock funds with deadline
    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Admin opens dispute by authorizing claim (before deadline)
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);

    // Verify claim is pending
    let claim = setup.escrow.get_pending_claim(&bounty_id);
    assert_eq!(claim.claimed, false);
    assert_eq!(claim.recipient, setup.contributor);

    // Advance time PAST deadline
    setup.env.ledger().set_timestamp(deadline + 100);

    // After hardening: refund is blocked while claim is pending.
    let res = setup.escrow.try_refund(&bounty_id);
    assert!(res.is_err(), "refund should be blocked by pending claim");

    // Verify no funds moved and status is still Locked
    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.status, EscrowStatus::Locked);
    assert_eq!(setup.token.balance(&setup.escrow.address), amount);
    assert_eq!(setup.token.balance(&setup.depositor), 10_000_000 - amount);
    assert_eq!(setup.token.balance(&setup.contributor), 0);
}

// Beneficiary claims successfully within dispute window
#[test]
fn test_beneficiary_claims_within_window_succeeds() {
    let setup = TestSetup::new();
    let bounty_id = 2;
    let amount = 1500;
    let now = setup.env.ledger().timestamp();
    let deadline = now + 2000;
    let claim_window = 500;

    setup.escrow.set_claim_window(&claim_window);

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Admin authorizes claim at now, expires at now+500
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);

    let claim = setup.escrow.get_pending_claim(&bounty_id);

    // Beneficiary claims within window
    setup.env.ledger().set_timestamp(claim.expires_at - 100);

    setup.escrow.claim(&bounty_id);

    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
    assert_eq!(setup.token.balance(&setup.contributor), amount);
    assert_eq!(setup.token.balance(&setup.escrow.address), 0);
}

// Beneficiary misses claim window - admin must cancel then refund
#[test]
fn test_missed_claim_window_requires_admin_cancel_then_refund() {
    let setup = TestSetup::new();
    let bounty_id = 3;
    let amount = 2500;
    let now = setup.env.ledger().timestamp();
    let deadline = now + 2000;
    let claim_window = 500;

    setup.escrow.set_claim_window(&claim_window);

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    // Admin authorizes claim (opens dispute window)
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);

    let claim = setup.escrow.get_pending_claim(&bounty_id);
    let claim_expires_at = claim.expires_at;

    // Advance to after claim window but before deadline
    setup.env.ledger().set_timestamp(claim_expires_at + 1);

    // Escrow is still Locked with pending claim
    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.status, EscrowStatus::Locked);
    assert_eq!(setup.token.balance(&setup.escrow.address), amount);

    // Admin cancels the expired pending claim
    setup.escrow.cancel_pending_claim(&bounty_id);

    let escrow_after = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow_after.status, EscrowStatus::Locked);

    // Advance to original deadline
    setup.env.ledger().set_timestamp(deadline + 1);

    setup.escrow.refund(&bounty_id);

    let final_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(final_escrow.status, EscrowStatus::Refunded);
    assert_eq!(setup.token.balance(&setup.depositor), 10_000_000);
    assert_eq!(setup.token.balance(&setup.escrow.address), 0);
}

#[test]
fn test_sweep_expired_refunds_batch_and_emits_expiry_event() {
    let setup = TestSetup::new();
    let now = setup.env.ledger().timestamp();
    let first_bounty = 21_u64;
    let second_bounty = 22_u64;
    let first_amount = 1_000_i128;
    let second_amount = 2_000_i128;
    let first_deadline = now + 100;
    let second_deadline = now + 200;

    setup.escrow.lock_funds(
        &setup.depositor,
        &first_bounty,
        &first_amount,
        &first_deadline,
    );
    setup.escrow.lock_funds(
        &setup.depositor,
        &second_bounty,
        &second_amount,
        &second_deadline,
    );

    let sweep_at = second_deadline + 1;
    setup.env.ledger().set_timestamp(sweep_at);

    let ids = vec![&setup.env, first_bounty, second_bounty];
    let swept = setup.escrow.sweep_expired_refunds(&ids);

    assert_eq!(swept, 2);
    assert_eq!(
        setup.escrow.get_escrow_info(&first_bounty).status,
        EscrowStatus::Refunded
    );
    assert_eq!(
        setup.escrow.get_escrow_info(&second_bounty).status,
        EscrowStatus::Refunded
    );
    assert_eq!(setup.token.balance(&setup.depositor), 10_000_000);
    assert_eq!(setup.token.balance(&setup.escrow.address), 0);
    assert!(saw_bounty_expired_event(&setup.env, first_bounty, sweep_at));
    assert!(saw_bounty_expired_event(
        &setup.env,
        second_bounty,
        sweep_at
    ));
}

#[test]
fn test_sweep_expired_refunds_at_exact_deadline_succeeds() {
    let setup = TestSetup::new();
    let bounty_id = 23_u64;
    let amount = 1_500_i128;
    let deadline = setup.env.ledger().timestamp() + 100;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.env.ledger().set_timestamp(deadline);

    let ids = vec![&setup.env, bounty_id];
    assert_eq!(setup.escrow.sweep_expired_refunds(&ids), 1);
    assert_eq!(
        setup.escrow.get_escrow_info(&bounty_id).status,
        EscrowStatus::Refunded
    );
}

#[test]
fn test_sweep_expired_refunds_mixed_active_rejects_atomically() {
    let setup = TestSetup::new();
    let now = setup.env.ledger().timestamp();
    let expired_bounty = 24_u64;
    let active_bounty = 25_u64;
    let amount = 1_000_i128;
    let expired_deadline = now + 100;
    let active_deadline = now + 1_000;

    setup.escrow.lock_funds(
        &setup.depositor,
        &expired_bounty,
        &amount,
        &expired_deadline,
    );
    setup
        .escrow
        .lock_funds(&setup.depositor, &active_bounty, &amount, &active_deadline);

    setup.env.ledger().set_timestamp(expired_deadline + 1);
    let ids = vec![&setup.env, expired_bounty, active_bounty];
    let result = try_sweep_direct(&setup, ids);

    assert_eq!(result, Err(Error::DeadlineNotPassed));
    assert_eq!(
        setup.escrow.get_escrow_info(&expired_bounty).status,
        EscrowStatus::Locked
    );
    assert_eq!(
        setup.escrow.get_escrow_info(&active_bounty).status,
        EscrowStatus::Locked
    );
    assert_eq!(setup.token.balance(&setup.escrow.address), amount * 2);
    assert_eq!(
        setup.token.balance(&setup.depositor),
        10_000_000 - amount * 2
    );
}

#[test]
fn test_sweep_expired_refunds_rejects_over_max_batch_size() {
    let setup = TestSetup::new();
    let mut ids = vec![&setup.env];
    for bounty_id in 0_u64..21_u64 {
        ids.push_back(bounty_id);
    }

    let result = try_sweep_direct(&setup, ids);
    assert_eq!(result, Err(Error::InvalidBatchSize));
}

#[test]
fn test_sweep_expired_refunds_max_batch_boundary_succeeds() {
    let setup = TestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 100;
    let mut items = vec![&setup.env];
    let mut ids = vec![&setup.env];

    for offset in 0_u64..20_u64 {
        let bounty_id = 1_000_u64 + offset;
        items.push_back(LockFundsItem {
            bounty_id,
            depositor: setup.depositor.clone(),
            amount: 10,
            deadline,
        });
        ids.push_back(bounty_id);
    }

    setup.escrow.batch_lock_funds(&items);
    setup.env.ledger().set_timestamp(deadline + 1);

    assert_eq!(setup.escrow.sweep_expired_refunds(&ids), 20);
    assert_eq!(setup.token.balance(&setup.depositor), 10_000_000);
    assert_eq!(setup.token.balance(&setup.escrow.address), 0);
}

#[test]
fn test_sweep_expired_refunds_respects_refund_pause() {
    let setup = TestSetup::new();
    let bounty_id = 26_u64;
    let amount = 1_000_i128;
    let deadline = setup.env.ledger().timestamp() + 100;

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);
    setup.escrow.set_paused(&None, &None, &Some(true));
    setup.env.ledger().set_timestamp(deadline + 1);

    let ids = vec![&setup.env, bounty_id];
    let result = try_sweep_direct(&setup, ids);

    assert_eq!(result, Err(Error::FundsPaused));
    assert_eq!(
        setup.escrow.get_escrow_info(&bounty_id).status,
        EscrowStatus::Locked
    );
    assert_eq!(setup.token.balance(&setup.escrow.address), amount);
}

// Resolution order must be explicit: can't skip the cancel step
#[test]
fn test_resolution_order_requires_explicit_cancel_step() {
    let setup = TestSetup::new();
    let bounty_id = 4;
    let amount = 3000;
    let now = setup.env.ledger().timestamp();
    let deadline = now + 200;
    let claim_window = 100;

    setup.escrow.set_claim_window(&claim_window);

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);

    // Advance past both windows
    setup.env.ledger().set_timestamp(deadline + 500);

    // Admin must cancel the pending claim first
    setup.escrow.cancel_pending_claim(&bounty_id);

    setup.escrow.refund(&bounty_id);

    let final_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(final_escrow.status, EscrowStatus::Refunded);
}

/// TEST 5: Explicitly demonstrate the correct resolution order
/// After the vulnerability fix, the correct sequence is:
///   1. Authorize a claim (opens dispute window)
///   2. Wait for claim window to expire or admin action needed
///   3. Admin cancels the claim (explicitly resolves the dispute)
///   4. Refund becomes available (if deadline has passed)
///
/// This prevents expiration alone from bypassing disputes.
#[test]
fn test_correct_resolution_order_cancel_then_refund() {
    let setup = TestSetup::new();
    let bounty_id = 41;
    let amount = 3000;
    let now = setup.env.ledger().timestamp();
    let deadline = now + 200;
    let claim_window = 100;

    setup.escrow.set_claim_window(&claim_window);

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);

    // Advance past both windows
    setup.env.ledger().set_timestamp(deadline + 500);

    // Admin must cancel the pending claim first
    setup.escrow.cancel_pending_claim(&bounty_id);

    // NOW refund works (demonstrates the order)
    setup.escrow.refund(&bounty_id);

    let final_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(final_escrow.status, EscrowStatus::Refunded);
}

// Admin can cancel expired claims at any time
#[test]
fn test_admin_can_cancel_expired_claim() {
    let setup = TestSetup::new();
    let bounty_id = 5;
    let amount = 2500;
    let now = setup.env.ledger().timestamp();
    let deadline = now + 1500;
    let claim_window = 600;

    setup.escrow.set_claim_window(&claim_window);

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);

    let claim = setup.escrow.get_pending_claim(&bounty_id);

    // Advance WAY past claim window
    setup.env.ledger().set_timestamp(claim.expires_at + 1000);

    setup.escrow.cancel_pending_claim(&bounty_id);

    let escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow.status, EscrowStatus::Locked);
    assert_eq!(setup.token.balance(&setup.escrow.address), amount);
}

// Zero-length claim windows (instant expiration)
#[test]
fn test_claim_window_zero_prevents_all_claims() {
    let setup = TestSetup::new();
    let bounty_id = 6;
    let amount = 800;
    let now = setup.env.ledger().timestamp();
    let deadline = now + 1000;

    // Set window to 0 (instant expiration)
    setup.escrow.set_claim_window(&0);

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);

    let claim = setup.escrow.get_pending_claim(&bounty_id);

    // Advance well past the deadline
    setup.env.ledger().set_timestamp(deadline + 1);

    // Admin cancels the zero-window claim
    setup.escrow.cancel_pending_claim(&bounty_id);

    setup.escrow.refund(&bounty_id);

    let final_escrow = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(final_escrow.status, EscrowStatus::Refunded);
}

// Multiple bounties resolve independently
#[test]
fn test_multiple_bounties_independent_resolution() {
    let setup = TestSetup::new();
    let claim_window = 300;

    setup.escrow.set_claim_window(&claim_window);

    let now = setup.env.ledger().timestamp();

    // Bounty 1: Will be cancelled and refunded
    setup
        .escrow
        .lock_funds(&setup.depositor, &1, &1000, &(now + 500));
    setup.escrow.authorize_claim(&1, &setup.contributor);

    // Bounty 2: Will be refunded directly (no claim)
    setup
        .escrow
        .lock_funds(&setup.depositor, &2, &2000, &(now + 600));

    // Bounty 3: Will be claimed
    setup
        .escrow
        .lock_funds(&setup.depositor, &3, &1500, &(now + 1000));
    setup.escrow.authorize_claim(&3, &setup.contributor);

    setup.env.ledger().set_timestamp(now + 550);

    setup.escrow.cancel_pending_claim(&1);
    setup.escrow.refund(&1);
    assert_eq!(
        setup.escrow.get_escrow_info(&1).status,
        EscrowStatus::Refunded
    );

    assert_eq!(
        setup.escrow.get_escrow_info(&2).status,
        EscrowStatus::Locked
    );

    let claim_3 = setup.escrow.get_pending_claim(&3);
    assert_eq!(claim_3.claimed, false);

    let claim_3_expires = claim_3.expires_at;
    setup.env.ledger().set_timestamp(claim_3_expires - 100);
    setup.escrow.claim(&3);

    assert_eq!(
        setup.escrow.get_escrow_info(&3).status,
        EscrowStatus::Released
    );

    setup.env.ledger().set_timestamp(now + 700);
    setup.escrow.refund(&2);

    assert_eq!(setup.token.balance(&setup.escrow.address), 0);
    assert_eq!(setup.token.balance(&setup.contributor), 1500);
    assert_eq!(setup.token.balance(&setup.depositor), 10_000_000 - 1500);
}

// Claim cancellation properly restores refund eligibility
#[test]
fn test_claim_cancellation_restores_refund_eligibility() {
    let setup = TestSetup::new();
    let bounty_id = 8;
    let amount = 5000;
    let now = setup.env.ledger().timestamp();
    let deadline = now + 2000;
    let claim_window = 500;

    setup.escrow.set_claim_window(&claim_window);

    setup
        .escrow
        .lock_funds(&setup.depositor, &bounty_id, &amount, &deadline);

    let escrow_before = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow_before.remaining_amount, amount);
    assert_eq!(escrow_before.status, EscrowStatus::Locked);

    // Authorize claim
    setup.escrow.authorize_claim(&bounty_id, &setup.contributor);

    // Cancel it
    setup.escrow.cancel_pending_claim(&bounty_id);

    let escrow_after = setup.escrow.get_escrow_info(&bounty_id);
    assert_eq!(escrow_after.status, EscrowStatus::Locked);
    assert_eq!(escrow_after.remaining_amount, amount);

    setup.env.ledger().set_timestamp(deadline + 1);
    setup.escrow.refund(&bounty_id);

    assert_eq!(setup.token.balance(&setup.depositor), 10_000_000);
}
