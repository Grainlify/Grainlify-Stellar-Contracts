// bounty_escrow/contracts/escrow/src/test_balance_invariant.rs
//
// # Token-Balance Invariant Tests — BountyEscrow
//
// ## The Invariant
//
//   contract_sac_balance == Σ remaining_amount  for every tracked bounty
//
// This invariant must hold after every mutating operation.  A violation
// indicates either an over-payout (funds can be drained beyond what escrows
// account for) or a lockup (funds stuck in the contract with no escrow to
// claim them).
//
// ## Flows covered
//
// 1. lock → partial_release (multi-milestone) → full release
// 2. lock → approve_refund (partial) → refund → approve_refund (full) → refund
// 3. lock → deadline passes → refund (no approval needed)
// 4. full release via release_funds
// 5. batch_lock_funds → batch_release_funds
// 6. Over-payout attempt: revert must not change balances
// 7. Insufficient-funds lock: no phantom escrow is created
// 8. Multi-bounty: Σ remaining_amounts == contract balance across mixed states

#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, vec, Address, Env,
};

fn create_token<'a>(
    env: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    (
        token::Client::new(env, &addr),
        token::StellarAssetClient::new(env, &addr),
    )
}

fn create_escrow_client<'a>(env: &Env) -> BountyEscrowContractClient<'a> {
    let cid = env.register_contract(None, BountyEscrowContract);
    BountyEscrowContractClient::new(env, &cid)
}

/// Assert the core invariant:
///
/// ```text
/// get_balance() == Σ get_escrow_info(id).remaining_amount  for id in bounty_ids
/// ```
///
/// NOTE: Soroban's generated client methods return `T` directly (panicking on
/// error); they do NOT return `Option<T>` or `Result<T, E>`.
fn assert_balance_invariant(
    client: &BountyEscrowContractClient,
    bounty_ids: &[u64],
    label: &str,
) {
    let contract_balance = client.get_balance();
    let sum_remaining: i128 = bounty_ids
        .iter()
        .map(|id| client.get_escrow_info(id).remaining_amount)
        .sum();
    assert_eq!(
        contract_balance,
        sum_remaining,
        "[{}] INVARIANT VIOLATED — SAC balance ({}) ≠ Σ remaining_amount ({})",
        label,
        contract_balance,
        sum_remaining,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. lock → partial_release (milestones) → full release
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_invariant_partial_release_multi_milestone() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);

    let (token_client, token_sac) = create_token(&env, &admin);
    let client = create_escrow_client(&env);
    client.init(&admin, &token_client.address);

    let total = 9_000i128;
    token_sac.mint(&depositor, &total);

    let bounty_id = 1u64;
    let deadline = 100_000u64;
    client.lock_funds(&depositor, &bounty_id, &total, &deadline);
    assert_balance_invariant(&client, &[bounty_id], "after lock");

    client.partial_release(&bounty_id, &contributor, &3_000i128);
    assert_eq!(client.get_escrow_info(&bounty_id).remaining_amount, 6_000);
    assert_balance_invariant(&client, &[bounty_id], "after milestone 1");

    client.partial_release(&bounty_id, &contributor, &3_000i128);
    assert_eq!(client.get_escrow_info(&bounty_id).remaining_amount, 3_000);
    assert_balance_invariant(&client, &[bounty_id], "after milestone 2");

    client.partial_release(&bounty_id, &contributor, &3_000i128);
    let info = client.get_escrow_info(&bounty_id);
    assert_eq!(info.remaining_amount, 0);
    assert_eq!(info.status, EscrowStatus::Released);
    assert_balance_invariant(&client, &[bounty_id], "after final milestone");

    assert_eq!(token_client.balance(&contributor), total);
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. lock → approve_refund (partial) → refund → approve_refund (full) → refund
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_invariant_partial_then_full_refund() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);

    let (token_client, token_sac) = create_token(&env, &admin);
    let client = create_escrow_client(&env);
    client.init(&admin, &token_client.address);

    let total = 5_000i128;
    token_sac.mint(&depositor, &total);

    let bounty_id = 2u64;
    let deadline = 100_000u64;
    client.lock_funds(&depositor, &bounty_id, &total, &deadline);
    assert_balance_invariant(&client, &[bounty_id], "after lock");

    client.approve_refund(&bounty_id, &2_000i128, &depositor, &RefundMode::Partial);
    client.refund(&bounty_id);

    let info = client.get_escrow_info(&bounty_id);
    assert_eq!(info.status, EscrowStatus::PartiallyRefunded);
    assert_eq!(info.remaining_amount, 3_000);
    assert_balance_invariant(&client, &[bounty_id], "after partial refund");

    let remaining = info.remaining_amount;
    client.approve_refund(&bounty_id, &remaining, &depositor, &RefundMode::Full);
    client.refund(&bounty_id);

    let info = client.get_escrow_info(&bounty_id);
    assert_eq!(info.status, EscrowStatus::Refunded);
    assert_eq!(info.remaining_amount, 0);
    assert_balance_invariant(&client, &[bounty_id], "after full refund");

    assert_eq!(token_client.balance(&depositor), total);
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. lock → deadline passes → refund (no approval needed)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_invariant_deadline_based_refund() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);

    let (token_client, token_sac) = create_token(&env, &admin);
    let client = create_escrow_client(&env);
    client.init(&admin, &token_client.address);

    let total = 4_000i128;
    token_sac.mint(&depositor, &total);

    let deadline = 2_000u64;
    let bounty_id = 3u64;
    client.lock_funds(&depositor, &bounty_id, &total, &deadline);
    assert_balance_invariant(&client, &[bounty_id], "after lock");

    let result = client.try_refund(&bounty_id);
    assert!(result.is_err(), "refund before deadline must fail");
    assert_balance_invariant(&client, &[bounty_id], "after failed refund attempt");

    env.ledger().set_timestamp(deadline + 1);
    client.refund(&bounty_id);

    let info = client.get_escrow_info(&bounty_id);
    assert_eq!(info.status, EscrowStatus::Refunded);
    assert_eq!(info.remaining_amount, 0);
    assert_balance_invariant(&client, &[bounty_id], "after deadline refund");

    assert_eq!(token_client.balance(&depositor), total);
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. full release via release_funds
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_invariant_full_release_funds() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);

    let (token_client, token_sac) = create_token(&env, &admin);
    let client = create_escrow_client(&env);
    client.init(&admin, &token_client.address);

    let total = 7_500i128;
    token_sac.mint(&depositor, &total);

    let bounty_id = 4u64;
    let deadline = 100_000u64;
    client.lock_funds(&depositor, &bounty_id, &total, &deadline);
    assert_balance_invariant(&client, &[bounty_id], "after lock");

    client.release_funds(&bounty_id, &contributor);

    let info = client.get_escrow_info(&bounty_id);
    assert_eq!(info.status, EscrowStatus::Released);
    assert_eq!(info.remaining_amount, 0);
    assert_balance_invariant(&client, &[bounty_id], "after full release");

    assert_eq!(token_client.balance(&contributor), total);
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. batch_lock_funds → batch_release_funds
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_invariant_batch_lock_and_batch_release() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let admin = Address::generate(&env);
    let dep1 = Address::generate(&env);
    let dep2 = Address::generate(&env);
    let dep3 = Address::generate(&env);
    let c1 = Address::generate(&env);
    let c2 = Address::generate(&env);
    let c3 = Address::generate(&env);

    let (token_client, token_sac) = create_token(&env, &admin);
    let client = create_escrow_client(&env);
    client.init(&admin, &token_client.address);

    let (a1, a2, a3) = (1_000i128, 2_000i128, 3_000i128);
    token_sac.mint(&dep1, &a1);
    token_sac.mint(&dep2, &a2);
    token_sac.mint(&dep3, &a3);

    let deadline = 100_000u64;
    let (id1, id2, id3) = (10u64, 11u64, 12u64);

    let lock_items = vec![
        &env,
        LockFundsItem { bounty_id: id1, depositor: dep1.clone(), amount: a1, deadline },
        LockFundsItem { bounty_id: id2, depositor: dep2.clone(), amount: a2, deadline },
        LockFundsItem { bounty_id: id3, depositor: dep3.clone(), amount: a3, deadline },
    ];
    let locked = client.batch_lock_funds(&lock_items);
    assert_eq!(locked, 3);
    assert_balance_invariant(&client, &[id1, id2, id3], "after batch lock");

    let release_items = vec![
        &env,
        ReleaseFundsItem { bounty_id: id1, contributor: c1.clone() },
        ReleaseFundsItem { bounty_id: id2, contributor: c2.clone() },
        ReleaseFundsItem { bounty_id: id3, contributor: c3.clone() },
    ];
    let released = client.batch_release_funds(&release_items);
    assert_eq!(released, 3);
    assert_balance_invariant(&client, &[id1, id2, id3], "after batch release");

    assert_eq!(token_client.balance(&c1), a1);
    assert_eq!(token_client.balance(&c2), a2);
    assert_eq!(token_client.balance(&c3), a3);
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. Over-payout attempt — revert must not mutate balances
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_invariant_overpayout_reverts_without_balance_change() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);

    let (token_client, token_sac) = create_token(&env, &admin);
    let client = create_escrow_client(&env);
    client.init(&admin, &token_client.address);

    let total = 3_000i128;
    token_sac.mint(&depositor, &total);

    let bounty_id = 20u64;
    let deadline = 100_000u64;
    client.lock_funds(&depositor, &bounty_id, &total, &deadline);
    assert_balance_invariant(&client, &[bounty_id], "after lock");

    client.partial_release(&bounty_id, &contributor, &1_000i128);
    assert_balance_invariant(&client, &[bounty_id], "after valid partial release");

    let over = client.try_partial_release(&bounty_id, &contributor, &5_000i128);
    assert!(over.is_err(), "over-release must be rejected");
    assert_balance_invariant(&client, &[bounty_id], "after failed over-release");

    assert_eq!(client.get_balance(), 2_000);
    assert_eq!(client.get_escrow_info(&bounty_id).remaining_amount, 2_000);
}

// ─────────────────────────────────────────────────────────────────────────────
// 7. Insufficient-funds lock attempt does not create a phantom escrow
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_invariant_failed_lock_does_not_create_escrow() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);

    let (token_client, token_sac) = create_token(&env, &admin);
    let client = create_escrow_client(&env);
    client.init(&admin, &token_client.address);

    token_sac.mint(&depositor, &100i128);

    let bounty_id = 30u64;
    let deadline = 100_000u64;
    let result = client.try_lock_funds(&depositor, &bounty_id, &500i128, &deadline);
    assert!(result.is_err(), "lock with insufficient token balance must fail");

    assert_eq!(client.get_balance(), 0);
    assert_eq!(token_client.balance(&depositor), 100);
}

// ─────────────────────────────────────────────────────────────────────────────
// 8. Multi-bounty: Σ remaining_amounts == contract balance across mixed states
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_invariant_multi_bounty_mixed_states() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let admin = Address::generate(&env);
    let dep_a = Address::generate(&env);
    let dep_b = Address::generate(&env);
    let dep_c = Address::generate(&env);
    let dep_d = Address::generate(&env);
    let contributor = Address::generate(&env);

    let (token_client, token_sac) = create_token(&env, &admin);
    let client = create_escrow_client(&env);
    client.init(&admin, &token_client.address);

    token_sac.mint(&dep_a, &2_000i128);
    token_sac.mint(&dep_b, &3_000i128);
    token_sac.mint(&dep_c, &1_500i128);
    token_sac.mint(&dep_d, &4_000i128);

    let (id_a, id_b, id_c, id_d) = (41u64, 42u64, 43u64, 44u64);
    let far_deadline = 999_999u64;
    let near_deadline = 1_100u64;

    client.lock_funds(&dep_a, &id_a, &2_000i128, &far_deadline);
    client.lock_funds(&dep_b, &id_b, &3_000i128, &far_deadline);
    client.lock_funds(&dep_c, &id_c, &1_500i128, &near_deadline);
    client.lock_funds(&dep_d, &id_d, &4_000i128, &far_deadline);
    assert_balance_invariant(&client, &[id_a, id_b, id_c, id_d], "after all locks");

    client.partial_release(&id_a, &contributor, &500i128);
    assert_balance_invariant(&client, &[id_a, id_b, id_c, id_d], "A: after milestone 1");

    client.partial_release(&id_a, &contributor, &1_500i128);
    assert_balance_invariant(&client, &[id_a, id_b, id_c, id_d], "A: after milestone 2");
    assert_eq!(client.get_escrow_info(&id_a).status, EscrowStatus::Released);

    client.approve_refund(&id_b, &1_000i128, &dep_b, &RefundMode::Partial);
    client.refund(&id_b);
    assert_balance_invariant(&client, &[id_a, id_b, id_c, id_d], "B: after partial refund");
    assert_eq!(client.get_escrow_info(&id_b).remaining_amount, 2_000);

    env.ledger().set_timestamp(near_deadline + 1);
    client.refund(&id_c);
    assert_balance_invariant(&client, &[id_a, id_b, id_c, id_d], "C: after deadline refund");
    assert_eq!(client.get_escrow_info(&id_c).status, EscrowStatus::Refunded);

    assert_eq!(client.get_escrow_info(&id_d).remaining_amount, 4_000);
    assert_balance_invariant(&client, &[id_a, id_b, id_c, id_d], "final state");

    // B(2000) + D(4000) = 6000
    assert_eq!(client.get_balance(), 6_000);
}

// ─────────────────────────────────────────────────────────────────────────────
// 9. Multi-step scripted sequence of 10+ mixed operations
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_invariant_multistep_scripted_sequence() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let admin = Address::generate(&env);
    let dep1 = Address::generate(&env);
    let dep2 = Address::generate(&env);
    let c1 = Address::generate(&env);
    let c2 = Address::generate(&env);

    let (token_client, token_sac) = create_token(&env, &admin);
    let client = create_escrow_client(&env);
    client.init(&admin, &token_client.address);

    let init_balance = 20_000i128;
    token_sac.mint(&dep1, &init_balance);
    token_sac.mint(&dep2, &init_balance);

    let b1 = 101u64;
    let b2 = 102u64;
    let b3 = 103u64;
    let b4 = 104u64;
    let far_deadline = 100_000u64;
    let all_bounties = &[b1, b2, b3, b4];

    // Step 1: lock_funds b1
    client.lock_funds(&dep1, &b1, &5_000i128, &far_deadline);
    assert_balance_invariant(&client, all_bounties, "step 1: lock b1");

    // Step 2: lock_funds b2
    client.lock_funds(&dep2, &b2, &8_000i128, &far_deadline);
    assert_balance_invariant(&client, all_bounties, "step 2: lock b2");

    // Step 3: lock_funds b3
    client.lock_funds(&dep1, &b3, &3_000i128, &2_000u64); // short deadline
    assert_balance_invariant(&client, all_bounties, "step 3: lock b3");

    // Step 4: partial_release b1
    client.partial_release(&b1, &c1, &2_000i128);
    assert_balance_invariant(&client, all_bounties, "step 4: partial release b1");

    // Step 5: approve_refund b2 (partial)
    client.approve_refund(&b2, &4_000i128, &dep2, &RefundMode::Partial);
    assert_balance_invariant(&client, all_bounties, "step 5: approve partial refund b2");

    // Step 6: refund b2 (partial)
    client.refund(&b2);
    assert_balance_invariant(&client, all_bounties, "step 6: execute partial refund b2");

    // Step 7: open dispute on b1 (authorize claim)
    client.authorize_claim(&b1, &c1);
    assert_balance_invariant(&client, all_bounties, "step 7: authorize claim b1");

    // Step 8: claim b1 (resolves dispute in favor of contributor)
    client.claim(&b1);
    assert_balance_invariant(&client, all_bounties, "step 8: claim b1");

    // Step 9: lock_funds b4
    client.lock_funds(&dep2, &b4, &4_000i128, &far_deadline);
    assert_balance_invariant(&client, all_bounties, "step 9: lock b4");

    // Step 10: release_funds b4
    client.release_funds(&b4, &c2);
    assert_balance_invariant(&client, all_bounties, "step 10: release b4");

    // Step 11: advance time past b3 deadline and refund
    env.ledger().set_timestamp(2_001);
    client.refund(&b3);
    assert_balance_invariant(&client, all_bounties, "step 11: deadline refund b3");

    // Step 12: approve and refund remainder of b2
    let rem_b2 = client.get_escrow_info(&b2).remaining_amount;
    client.approve_refund(&b2, &rem_b2, &dep2, &RefundMode::Full);
    client.refund(&b2);
    assert_balance_invariant(&client, all_bounties, "step 12: full refund b2");

    // Final checks
    let contract_bal = client.get_balance();
    assert_eq!(contract_bal, 0, "contract should be fully drained");
}
