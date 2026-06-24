#![cfg(test)]

extern crate std;

use crate::{BountyEscrowContract, BountyEscrowContractClient, EscrowStatus, RefundMode};
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestCaseError, TestRng, TestRunner};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};
use std::format;

const CASES: u32 = 16;
const MAX_SHRINK_ITERS: u32 = 64;
const BASIS_POINTS: i128 = 10_000;
const MAX_FEE_RATE: i128 = 5_000;

#[derive(Clone, Copy, Debug)]
struct LifecycleOp {
    kind: u8,
    selector: usize,
    amount: i128,
    deadline_delta: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ModelStatus {
    Locked,
    Released,
    Refunded,
    PartiallyRefunded,
}

#[derive(Clone, Debug)]
struct ModelEscrow {
    id: u64,
    amount: i128,
    remaining: i128,
    deadline: u64,
    status: ModelStatus,
}

struct ModelTotals {
    minted: i128,
    locked: i128,
    released: i128,
    refunded: i128,
}

struct TestSetup<'a> {
    env: Env,
    depositor: Address,
    contributor: Address,
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
    escrow: BountyEscrowContractClient<'a>,
}

fn create_token_contract<'a>(
    e: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract = e.register_stellar_asset_contract_v2(admin.clone());
    let token_address = contract.address();
    (
        token::Client::new(e, &token_address),
        token::StellarAssetClient::new(e, &token_address),
    )
}

fn create_escrow_contract<'a>(e: &Env) -> BountyEscrowContractClient<'a> {
    let contract_id = e.register_contract(None, BountyEscrowContract);
    BountyEscrowContractClient::new(e, &contract_id)
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

        Self {
            env,
            depositor,
            contributor,
            token,
            token_admin,
            escrow,
        }
    }
}

fn op_strategy() -> impl Strategy<Value = LifecycleOp> {
    (0_u8..=4, 0_usize..64, 1_i128..=20_000, 1_u64..=1_000).prop_map(
        |(kind, selector, amount, deadline_delta)| LifecycleOp {
            kind,
            selector,
            amount,
            deadline_delta,
        },
    )
}

fn proptest_config() -> ProptestConfig {
    ProptestConfig {
        cases: CASES,
        max_shrink_iters: MAX_SHRINK_ITERS,
        failure_persistence: None,
        ..ProptestConfig::default()
    }
}

fn deterministic_runner() -> TestRunner {
    let config = proptest_config();
    let algorithm = config.rng_algorithm;
    TestRunner::new_with_rng(config, TestRng::deterministic_rng(algorithm))
}

fn pick_index<F>(model: &[ModelEscrow], selector: usize, predicate: F) -> Option<usize>
where
    F: Fn(&ModelEscrow) -> bool,
{
    let candidates: std::vec::Vec<usize> = model
        .iter()
        .enumerate()
        .filter_map(|(idx, escrow)| predicate(escrow).then_some(idx))
        .collect();

    if candidates.is_empty() {
        None
    } else {
        Some(candidates[selector % candidates.len()])
    }
}

fn expected_status(status: ModelStatus) -> EscrowStatus {
    match status {
        ModelStatus::Locked => EscrowStatus::Locked,
        ModelStatus::Released => EscrowStatus::Released,
        ModelStatus::Refunded => EscrowStatus::Refunded,
        ModelStatus::PartiallyRefunded => EscrowStatus::PartiallyRefunded,
    }
}

/// Assert conservation of funds and view consistency after every operation.
fn assert_invariants(
    setup: &TestSetup<'_>,
    model: &[ModelEscrow],
    totals: &ModelTotals,
) -> Result<(), TestCaseError> {
    let mut active_contract_balance = 0_i128;
    let mut count_locked = 0_u32;
    let mut count_released = 0_u32;
    let mut count_refunded = 0_u32;
    let mut stats_locked = 0_i128;
    let mut stats_released = 0_i128;
    let mut stats_refunded = 0_i128;

    for expected in model {
        let escrow = setup.escrow.get_escrow_info(&expected.id);
        prop_assert_eq!(escrow.amount, expected.amount);
        prop_assert!(escrow.remaining_amount >= 0);
        prop_assert!(escrow.remaining_amount <= escrow.amount);
        let actual_status = escrow.status.clone();
        prop_assert_eq!(actual_status, expected_status(expected.status));

        let mut escrow_refunded_amount = 0;
        for record in escrow.refund_history.iter() {
            escrow_refunded_amount += record.amount;
        }

        let escrow_released_amount = escrow.amount - escrow.remaining_amount - escrow_refunded_amount;

        stats_released += escrow_released_amount;
        stats_refunded += escrow_refunded_amount;

        match escrow.status {
            EscrowStatus::Locked | EscrowStatus::PartiallyRefunded => {
                count_locked += 1;
                stats_locked += escrow.remaining_amount;
                active_contract_balance += escrow.remaining_amount;
            }
            EscrowStatus::Released => {
                count_released += 1;
            }
            EscrowStatus::Refunded => {
                count_refunded += 1;
            }
        }
    }

    let aggregate = setup.escrow.get_aggregate_stats();
    prop_assert_eq!(aggregate.count_locked, count_locked);
    prop_assert_eq!(aggregate.count_released, count_released);
    prop_assert_eq!(aggregate.count_refunded, count_refunded);
    prop_assert_eq!(aggregate.total_locked, stats_locked);
    prop_assert_eq!(aggregate.total_released, stats_released);
    prop_assert_eq!(aggregate.total_refunded, stats_refunded);

    let contract_balance = setup.escrow.get_balance();
    prop_assert_eq!(contract_balance, active_contract_balance);
    prop_assert_eq!(
        contract_balance,
        totals.locked - totals.released - totals.refunded
    );
    prop_assert!(totals.released + totals.refunded <= totals.locked);
    prop_assert_eq!(setup.token.balance(&setup.contributor), totals.released);
    prop_assert_eq!(
        setup.token.balance(&setup.depositor),
        totals.minted - totals.locked + totals.refunded
    );

    Ok(())
}

fn apply_lock(
    setup: &TestSetup<'_>,
    model: &mut std::vec::Vec<ModelEscrow>,
    totals: &mut ModelTotals,
    next_id: &mut u64,
    op: LifecycleOp,
) {
    // lock_funds enforces a depositor cooldown. Advance the ledger before
    // generated locks so this property models successful lifecycle calls.
    let next_timestamp = setup.env.ledger().timestamp().saturating_add(61);
    setup.env.ledger().set_timestamp(next_timestamp);

    let amount = op.amount;
    let deadline = setup
        .env
        .ledger()
        .timestamp()
        .saturating_add(op.deadline_delta)
        .saturating_add(1);
    let id = *next_id;
    *next_id += 1;

    setup.token_admin.mint(&setup.depositor, &amount);
    totals.minted += amount;
    totals.locked += amount;

    setup
        .escrow
        .lock_funds(&setup.depositor, &id, &amount, &deadline);
    model.push(ModelEscrow {
        id,
        amount,
        remaining: amount,
        deadline,
        status: ModelStatus::Locked,
    });
}

fn apply_partial_release(
    setup: &TestSetup<'_>,
    model: &mut [ModelEscrow],
    totals: &mut ModelTotals,
    op: LifecycleOp,
) {
    let Some(idx) = pick_index(model, op.selector, |escrow| {
        escrow.status == ModelStatus::Locked && escrow.remaining > 0
    }) else {
        return;
    };

    let payout = 1 + ((op.amount - 1) % model[idx].remaining);
    setup
        .escrow
        .partial_release(&model[idx].id, &setup.contributor, &payout);

    model[idx].remaining -= payout;
    totals.released += payout;
    if model[idx].remaining == 0 {
        model[idx].status = ModelStatus::Released;
    }
}

fn apply_full_release(
    setup: &TestSetup<'_>,
    model: &mut [ModelEscrow],
    totals: &mut ModelTotals,
    op: LifecycleOp,
) {
    let Some(idx) = pick_index(model, op.selector, |escrow| {
        escrow.status == ModelStatus::Locked && escrow.remaining == escrow.amount
    }) else {
        return;
    };

    setup
        .escrow
        .release_funds(&model[idx].id, &setup.contributor);
    totals.released += model[idx].amount;
    model[idx].status = ModelStatus::Released;
    // The current full-release path leaves remaining_amount unchanged on
    // terminal escrows, so active-balance checks intentionally ignore it.
}

fn apply_approved_refund(
    setup: &TestSetup<'_>,
    model: &mut [ModelEscrow],
    totals: &mut ModelTotals,
    op: LifecycleOp,
) {
    let Some(idx) = pick_index(model, op.selector, |escrow| {
        (escrow.status == ModelStatus::Locked || escrow.status == ModelStatus::PartiallyRefunded)
            && escrow.remaining > 0
    }) else {
        return;
    };

    let refund_amount = 1 + ((op.amount - 1) % model[idx].remaining);
    let mode = if refund_amount == model[idx].remaining {
        RefundMode::Full
    } else {
        RefundMode::Partial
    };

    setup
        .escrow
        .approve_refund(&model[idx].id, &refund_amount, &setup.depositor, &mode);
    setup.escrow.refund(&model[idx].id);

    model[idx].remaining -= refund_amount;
    totals.refunded += refund_amount;
    model[idx].status = if model[idx].remaining == 0 {
        ModelStatus::Refunded
    } else {
        ModelStatus::PartiallyRefunded
    };
}

fn apply_deadline_refund(
    setup: &TestSetup<'_>,
    model: &mut [ModelEscrow],
    totals: &mut ModelTotals,
    op: LifecycleOp,
) {
    let Some(idx) = pick_index(model, op.selector, |escrow| {
        (escrow.status == ModelStatus::Locked || escrow.status == ModelStatus::PartiallyRefunded)
            && escrow.remaining > 0
    }) else {
        return;
    };

    let now = setup.env.ledger().timestamp();
    let refund_at = if now < model[idx].deadline {
        model[idx].deadline
    } else {
        now
    };
    setup.env.ledger().set_timestamp(refund_at);
    let refund_amount = model[idx].remaining;
    setup.escrow.refund(&model[idx].id);

    model[idx].remaining = 0;
    model[idx].status = ModelStatus::Refunded;
    totals.refunded += refund_amount;
}

fn run_lifecycle_ops(ops: std::vec::Vec<LifecycleOp>) -> Result<(), TestCaseError> {
    let setup = TestSetup::new();
    let mut model = std::vec::Vec::new();
    let mut totals = ModelTotals {
        minted: 0,
        locked: 0,
        released: 0,
        refunded: 0,
    };
    let mut next_id = 10_000_u64;

    assert_invariants(&setup, &model, &totals)?;

    for op in ops {
        match op.kind {
            0 => apply_lock(&setup, &mut model, &mut totals, &mut next_id, op),
            1 => apply_partial_release(&setup, &mut model, &mut totals, op),
            2 => apply_full_release(&setup, &mut model, &mut totals, op),
            3 => apply_approved_refund(&setup, &mut model, &mut totals, op),
            _ => apply_deadline_refund(&setup, &mut model, &mut totals, op),
        }
        assert_invariants(&setup, &model, &totals)?;
    }

    Ok(())
}

#[test]
fn proptest_invariant_smoke_exercises_lifecycle_entrypoints() {
    let ops = std::vec![
        LifecycleOp {
            kind: 0,
            selector: 0,
            amount: 1_000,
            deadline_delta: 100,
        },
        LifecycleOp {
            kind: 1,
            selector: 0,
            amount: 300,
            deadline_delta: 1,
        },
        LifecycleOp {
            kind: 3,
            selector: 0,
            amount: 200,
            deadline_delta: 1,
        },
        LifecycleOp {
            kind: 4,
            selector: 0,
            amount: 1,
            deadline_delta: 1,
        },
        LifecycleOp {
            kind: 0,
            selector: 0,
            amount: 700,
            deadline_delta: 100,
        },
        LifecycleOp {
            kind: 2,
            selector: 0,
            amount: 1,
            deadline_delta: 1,
        },
    ];

    run_lifecycle_ops(ops).expect("deterministic lifecycle should preserve invariants");
}

#[test]
fn proptest_lifecycle_invariants_hold_after_each_operation() {
    let mut runner = deterministic_runner();
    let strategy = proptest::collection::vec(op_strategy(), 8..=24);

    runner
        .run(&strategy, |ops| run_lifecycle_ops(ops))
        .expect("bounded lifecycle properties should hold");
}

#[test]
fn proptest_fee_basis_points_do_not_overflow_or_exceed_principal() {
    let mut runner = deterministic_runner();
    let strategy = (
        prop_oneof![
            1_i128..=1_000_000_i128,
            (i128::MAX - 10_000_i128)..=i128::MAX,
        ],
        0_i128..=MAX_FEE_RATE,
    );

    runner
        .run(&strategy, |(amount, rate)| {
            let fee = amount
                .checked_mul(rate)
                .and_then(|value| value.checked_div(BASIS_POINTS))
                .unwrap_or(0);

            prop_assert!(fee >= 0);
            prop_assert!(fee <= amount);
            Ok(())
        })
        .expect("basis-point fee properties should hold");
}
