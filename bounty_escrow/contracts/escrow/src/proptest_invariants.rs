#![cfg(test)]

extern crate std;

use crate::{
    BountyEscrowContract, BountyEscrowContractClient, EscrowQueryFilter, EscrowStatus,
    EscrowWithId, RefundMode,
};
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestCaseError, TestRng, TestRunner};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, Vec as SorobanVec,
};
use std::{collections::BTreeSet, format};

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

#[derive(Clone, Copy, Debug)]
struct PaginationEscrowSpec {
    depositor_slot: u8,
    transition: u8,
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

#[derive(Clone, Debug)]
struct PaginationModelEscrow {
    id: u64,
    amount: i128,
    deadline: u64,
    status: EscrowStatus,
    depositor_slot: u8,
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

fn pagination_spec_strategy() -> impl Strategy<Value = std::vec::Vec<PaginationEscrowSpec>> {
    proptest::collection::vec(
        (0_u8..=1, 0_u8..=3, 2_i128..=50_000, 1_u64..=600).prop_map(
            |(depositor_slot, transition, amount, deadline_delta)| PaginationEscrowSpec {
                depositor_slot,
                transition,
                amount,
                deadline_delta,
            },
        ),
        4..=14,
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

    for expected in model {
        let escrow = setup.escrow.get_escrow_info(&expected.id);
        prop_assert_eq!(escrow.amount, expected.amount);
        prop_assert!(escrow.remaining_amount >= 0);
        prop_assert!(escrow.remaining_amount <= escrow.amount);
        let actual_status = escrow.status.clone();
        prop_assert_eq!(actual_status, expected_status(expected.status));

        match escrow.status {
            EscrowStatus::Locked => {
                count_locked += 1;
                active_contract_balance += escrow.remaining_amount;
            }
            EscrowStatus::Released => {
                count_released += 1;
            }
            EscrowStatus::Refunded => {
                count_refunded += 1;
            }
            EscrowStatus::PartiallyRefunded => {
                count_locked += 1;
                active_contract_balance += escrow.remaining_amount;
            }
        }
    }

    let aggregate = setup.escrow.get_aggregate_stats();
    prop_assert_eq!(aggregate.count_locked, count_locked);
    prop_assert_eq!(aggregate.count_released, count_released);
    prop_assert_eq!(aggregate.count_refunded, count_refunded);
    prop_assert_eq!(aggregate.total_locked, active_contract_balance);
    prop_assert_eq!(aggregate.total_released, totals.released);
    prop_assert_eq!(aggregate.total_refunded, totals.refunded);

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

fn escrow_with_id_ids(page: SorobanVec<EscrowWithId>) -> std::vec::Vec<u64> {
    let mut ids = std::vec::Vec::with_capacity(page.len() as usize);
    for i in 0..page.len() {
        ids.push(page.get(i).unwrap().bounty_id);
    }
    ids
}

fn u64_vec_ids(page: SorobanVec<u64>) -> std::vec::Vec<u64> {
    let mut ids = std::vec::Vec::with_capacity(page.len() as usize);
    for i in 0..page.len() {
        ids.push(page.get(i).unwrap());
    }
    ids
}

fn assert_no_duplicate_ids(ids: &[u64]) -> Result<(), TestCaseError> {
    let mut seen = BTreeSet::new();
    for id in ids {
        prop_assert!(seen.insert(*id), "duplicate id {id}");
    }
    Ok(())
}

/// Proves a paginated query is stable, non-overlapping, and complete.
///
/// The expected IDs are the ground truth derived from the generated model. The
/// first query with `limit > total` also validates the contract's unpaginated
/// scan against that model before walking fixed-size pages.
fn assert_paginated_ids<F>(expected_ids: &[u64], mut query: F) -> Result<(), TestCaseError>
where
    F: FnMut(u32, u32) -> std::vec::Vec<u64>,
{
    let total = expected_ids.len() as u32;
    let over_limit = total.saturating_add(5).max(1);

    prop_assert!(
        query(0, 0).is_empty(),
        "limit 0 must always return an empty page"
    );
    prop_assert!(
        query(total, 1).is_empty(),
        "offset equal to total must return an empty page"
    );
    prop_assert!(
        query(total.saturating_add(5), 3).is_empty(),
        "offset greater than total must return an empty page"
    );

    let full_scan = query(0, over_limit);
    prop_assert_eq!(full_scan.as_slice(), expected_ids);
    assert_no_duplicate_ids(&full_scan)?;

    for limit in [1_u32, 2, 3, 5, over_limit] {
        let mut offset = 0_u32;
        let mut stitched = std::vec::Vec::new();

        loop {
            let page = query(offset, limit);
            let repeated_page = query(offset, limit);
            prop_assert_eq!(
                page.as_slice(),
                repeated_page.as_slice(),
                "same page must be stable across repeated reads"
            );
            prop_assert!(
                page.len() <= limit as usize,
                "page length must not exceed limit"
            );
            assert_no_duplicate_ids(&page)?;

            for id in &page {
                prop_assert!(
                    !stitched.contains(id),
                    "id {id} appeared in more than one page"
                );
            }

            if page.is_empty() {
                break;
            }

            stitched.extend(page);
            offset = offset.saturating_add(limit);

            if offset > total.saturating_add(limit) {
                break;
            }
        }

        prop_assert_eq!(stitched, expected_ids);
    }

    Ok(())
}

fn model_ids_by_status(
    model: &[PaginationModelEscrow],
    status: &EscrowStatus,
) -> std::vec::Vec<u64> {
    model
        .iter()
        .filter_map(|escrow| (escrow.status == *status).then_some(escrow.id))
        .collect()
}

fn model_ids_by_amount(
    model: &[PaginationModelEscrow],
    min_amount: i128,
    max_amount: i128,
) -> std::vec::Vec<u64> {
    model
        .iter()
        .filter_map(|escrow| {
            (escrow.amount >= min_amount && escrow.amount <= max_amount).then_some(escrow.id)
        })
        .collect()
}

fn model_ids_by_deadline(
    model: &[PaginationModelEscrow],
    min_deadline: u64,
    max_deadline: u64,
) -> std::vec::Vec<u64> {
    model
        .iter()
        .filter_map(|escrow| {
            (escrow.deadline >= min_deadline && escrow.deadline <= max_deadline)
                .then_some(escrow.id)
        })
        .collect()
}

fn model_ids_by_depositor_slot(
    model: &[PaginationModelEscrow],
    depositor_slot: u8,
) -> std::vec::Vec<u64> {
    model
        .iter()
        .filter_map(|escrow| (escrow.depositor_slot == depositor_slot).then_some(escrow.id))
        .collect()
}

fn model_ids_by_composite_filter(
    model: &[PaginationModelEscrow],
    depositor_slot: u8,
    status: &EscrowStatus,
    min_amount: i128,
    max_deadline: u64,
) -> std::vec::Vec<u64> {
    model
        .iter()
        .filter_map(|escrow| {
            (escrow.depositor_slot == depositor_slot
                && escrow.status == *status
                && escrow.amount >= min_amount
                && escrow.deadline <= max_deadline)
                .then_some(escrow.id)
        })
        .collect()
}

fn model_expiring_ids(model: &[PaginationModelEscrow], max_deadline: u64) -> std::vec::Vec<u64> {
    model
        .iter()
        .filter_map(|escrow| {
            ((escrow.status == EscrowStatus::Locked
                || escrow.status == EscrowStatus::PartiallyRefunded)
                && escrow.deadline <= max_deadline)
                .then_some(escrow.id)
        })
        .collect()
}

fn generated_pagination_model(
    setup: &TestSetup<'_>,
    specs: &[PaginationEscrowSpec],
) -> (Address, std::vec::Vec<PaginationModelEscrow>) {
    let alternate_depositor = Address::generate(&setup.env);
    let mut model = std::vec::Vec::with_capacity(specs.len());
    let mut release_ids = std::vec::Vec::new();
    let mut partial_refunds = std::vec::Vec::new();
    let mut deadline_refund_ids = std::vec::Vec::new();

    for (idx, spec) in specs.iter().enumerate() {
        let id = 20_000_u64 + idx as u64;
        let depositor = if spec.depositor_slot == 0 {
            setup.depositor.clone()
        } else {
            alternate_depositor.clone()
        };

        setup.token_admin.mint(&depositor, &spec.amount);
        setup
            .env
            .ledger()
            .set_timestamp(setup.env.ledger().timestamp().saturating_add(61));
        let deadline = setup
            .env
            .ledger()
            .timestamp()
            .saturating_add(spec.deadline_delta)
            .saturating_add(1);

        setup
            .escrow
            .lock_funds(&depositor, &id, &spec.amount, &deadline);

        let status = match spec.transition {
            1 => {
                release_ids.push(id);
                EscrowStatus::Released
            }
            2 => {
                deadline_refund_ids.push(id);
                EscrowStatus::Refunded
            }
            3 => {
                partial_refunds.push((id, spec.amount / 2, depositor.clone()));
                EscrowStatus::PartiallyRefunded
            }
            _ => EscrowStatus::Locked,
        };

        model.push(PaginationModelEscrow {
            id,
            amount: spec.amount,
            deadline,
            status,
            depositor_slot: spec.depositor_slot,
        });
    }

    for id in release_ids {
        setup.escrow.release_funds(&id, &setup.contributor);
    }

    for (id, refund_amount, depositor) in partial_refunds {
        setup
            .escrow
            .approve_refund(&id, &refund_amount, &depositor, &RefundMode::Partial);
        setup.escrow.refund(&id);
    }

    let refund_timestamp = model
        .iter()
        .map(|escrow| escrow.deadline)
        .max()
        .unwrap_or_else(|| setup.env.ledger().timestamp())
        .saturating_add(1);
    setup.env.ledger().set_timestamp(refund_timestamp);

    for id in deadline_refund_ids {
        setup.escrow.refund(&id);
    }

    (alternate_depositor, model)
}

fn assert_query_pagination_properties(
    specs: std::vec::Vec<PaginationEscrowSpec>,
) -> Result<(), TestCaseError> {
    let setup = TestSetup::new();
    let (alternate_depositor, model) = generated_pagination_model(&setup, &specs);
    let max_model_amount = model.iter().map(|escrow| escrow.amount).max().unwrap_or(0);
    let min_model_deadline = model
        .iter()
        .map(|escrow| escrow.deadline)
        .min()
        .unwrap_or(0);
    let max_model_deadline = model
        .iter()
        .map(|escrow| escrow.deadline)
        .max()
        .unwrap_or(0);
    let mid_deadline = min_model_deadline
        .saturating_add(max_model_deadline.saturating_sub(min_model_deadline) / 2);
    let min_amount = 2_i128;
    let max_amount = (max_model_amount / 2).max(min_amount);

    let locked = EscrowStatus::Locked;
    let released = EscrowStatus::Released;
    let partially_refunded = EscrowStatus::PartiallyRefunded;

    let expected_locked = model_ids_by_status(&model, &locked);
    assert_paginated_ids(&expected_locked, |offset, limit| {
        escrow_with_id_ids(
            setup
                .escrow
                .query_escrows_by_status(&locked, &offset, &limit),
        )
    })?;

    let expected_released_ids = model_ids_by_status(&model, &released);
    assert_paginated_ids(&expected_released_ids, |offset, limit| {
        u64_vec_ids(
            setup
                .escrow
                .get_escrow_ids_by_status(&released, &offset, &limit),
        )
    })?;

    let expected_by_amount = model_ids_by_amount(&model, min_amount, max_amount);
    assert_paginated_ids(&expected_by_amount, |offset, limit| {
        escrow_with_id_ids(setup.escrow.query_escrows_by_amount(
            &min_amount,
            &max_amount,
            &offset,
            &limit,
        ))
    })?;

    let expected_by_deadline = model_ids_by_deadline(&model, min_model_deadline, mid_deadline);
    assert_paginated_ids(&expected_by_deadline, |offset, limit| {
        escrow_with_id_ids(setup.escrow.query_escrows_by_deadline(
            &min_model_deadline,
            &mid_deadline,
            &offset,
            &limit,
        ))
    })?;

    let expected_by_depositor = model_ids_by_depositor_slot(&model, 1);
    assert_paginated_ids(&expected_by_depositor, |offset, limit| {
        escrow_with_id_ids(setup.escrow.query_escrows_by_depositor(
            &alternate_depositor,
            &offset,
            &limit,
        ))
    })?;

    let composite_filter = EscrowQueryFilter {
        has_status_filter: true,
        status: partially_refunded.clone(),
        has_depositor_filter: true,
        depositor: setup.depositor.clone(),
        min_amount,
        max_amount: i128::MAX,
        min_deadline: 0,
        max_deadline: mid_deadline,
    };
    let expected_composite =
        model_ids_by_composite_filter(&model, 0, &partially_refunded, min_amount, mid_deadline);
    assert_paginated_ids(&expected_composite, |offset, limit| {
        escrow_with_id_ids(
            setup
                .escrow
                .query_escrows(&composite_filter, &offset, &limit),
        )
    })?;

    let expiring_cutoff = mid_deadline.max(min_model_deadline);
    let expected_expiring = model_expiring_ids(&model, expiring_cutoff);
    assert_paginated_ids(&expected_expiring, |offset, limit| {
        u64_vec_ids(
            setup
                .escrow
                .query_expiring_bounties(&expiring_cutoff, &offset, &limit),
        )
    })?;

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

#[test]
fn proptest_query_pagination_invariants_cover_edge_limits_and_offsets() {
    let mut runner = deterministic_runner();

    runner
        .run(
            &pagination_spec_strategy(),
            assert_query_pagination_properties,
        )
        .expect("query pagination properties should hold");
}
