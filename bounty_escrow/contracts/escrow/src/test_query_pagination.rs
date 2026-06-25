#![cfg(test)]

extern crate std;

use crate::{
    BountyEscrowContract, BountyEscrowContractClient, EscrowQueryFilter, EscrowStatus,
};
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestCaseError, TestRng, TestRunner};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};
use std::format;

const CASES: u32 = 16;
const MAX_SHRINK_ITERS: u32 = 64;

#[derive(Clone, Debug)]
struct EscrowDef {
    depositor_idx: u8,
    amount: i128,
    deadline: u64,
    action: u8, // 0 = Lock, 1 = Lock then Release, 2 = Lock then Refund
}

fn escrow_def_strategy() -> impl Strategy<Value = EscrowDef> {
    (
        0_u8..3,
        100_i128..=1000_i128,
        1000_u64..=2000_u64,
        0_u8..=2,
    )
        .prop_map(|(depositor_idx, amount, deadline, action)| EscrowDef {
            depositor_idx,
            amount,
            deadline,
            action,
        })
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

fn create_escrow<'a>(env: &Env) -> BountyEscrowContractClient<'a> {
    let id = env.register_contract(None, BountyEscrowContract);
    BountyEscrowContractClient::new(env, &id)
}

struct Setup<'a> {
    env: Env,
    depositors: std::vec::Vec<Address>,
    contributor: Address,
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
    escrow: BountyEscrowContractClient<'a>,
}

impl<'a> Setup<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.budget().reset_unlimited();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let contributor = Address::generate(&env);
        let mut depositors = std::vec::Vec::new();
        for _ in 0..3 {
            depositors.push(Address::generate(&env));
        }
        let (token, token_admin) = create_token(&env, &admin);
        let escrow = create_escrow(&env);
        escrow.init(&admin, &token.address);
        for dep in depositors.iter() {
            token_admin.mint(dep, &10_000_000);
            escrow.set_whitelist(dep, &true);
        }
        Setup {
            env,
            depositors,
            contributor,
            token,
            token_admin,
            escrow,
        }
    }
}

fn assert_pagination_invariants<T, F>(
    total_matches: usize,
    fetch_page: F,
    extract_id: impl Fn(&T) -> u64,
) -> Result<(), TestCaseError>
where
    F: Fn(u32, u32) -> std::vec::Vec<T>,
{
    // 1. Stable ordering, uniqueness, and completeness check over varying page sizes
    for limit in [1, 2, 5, 10, 100] {
        let mut collected_ids = std::vec::Vec::new();
        let mut offset = 0u32;
        loop {
            let page = fetch_page(offset, limit as u32);
            if page.is_empty() {
                break;
            }
            prop_assert!(page.len() <= limit);

            for item in page.iter() {
                let id = extract_id(item);
                // Non-overlapping check: each matching escrow must be yielded exactly once
                prop_assert!(!collected_ids.contains(&id));
                collected_ids.push(id);
            }
            offset += page.len() as u32;
        }

        // Full scan (offset=0, limit=MAX) serves as the ground-truth order/set
        let full_scan = fetch_page(0, 1000);
        let expected_ids: std::vec::Vec<u64> = full_scan.iter().map(|item| extract_id(item)).collect();
        prop_assert_eq!(&expected_ids, &collected_ids);
    }

    // 2. Limit 0 boundary case
    let page_limit_zero = fetch_page(0, 0);
    prop_assert!(page_limit_zero.is_empty());

    // 3. Offset >= total boundary case
    let page_offset_large = fetch_page(total_matches as u32, 10);
    prop_assert!(page_offset_large.is_empty());

    // 4. Limit > total boundary case
    let page_limit_large = fetch_page(0, (total_matches + 10) as u32);
    let full_scan = fetch_page(0, 1000);
    prop_assert_eq!(full_scan.len(), page_limit_large.len());
    for (i, item) in page_limit_large.iter().enumerate() {
        prop_assert_eq!(extract_id(item), extract_id(&full_scan[i]));
    }

    Ok(())
}

fn run_pagination_test(defs: std::vec::Vec<EscrowDef>) -> Result<(), TestCaseError> {
    let s = Setup::new();
    let mut expected_locked = 0;
    let mut expected_released = 0;
    let mut expected_refunded = 0;

    let mut bounty_id = 1u64;
    for def in defs.iter() {
        let depositor = &s.depositors[def.depositor_idx as usize % s.depositors.len()];
        s.escrow.lock_funds(depositor, &bounty_id, &def.amount, &def.deadline);

        if def.action == 1 {
            s.escrow.release_funds(&bounty_id, &s.contributor);
            expected_released += 1;
        } else if def.action == 2 {
            // Set ledger timestamp past the deadline to allow refund
            s.env.ledger().with_mut(|li| li.timestamp = def.deadline + 1);
            s.escrow.refund(&bounty_id);
            expected_refunded += 1;
        } else {
            expected_locked += 1;
        }
        bounty_id += 1;
    }
    let total_escrows = defs.len();

    // Reset ledger time to 0 for deadline queries
    s.env.ledger().with_mut(|li| li.timestamp = 0);

    // --- 1. Query: query_escrows (Unfiltered full scan) ---
    assert_pagination_invariants(
        total_escrows,
        |offset, limit| {
            let filter = EscrowQueryFilter {
                has_status_filter: false,
                status: EscrowStatus::Locked,
                has_depositor_filter: false,
                depositor: Address::generate(&s.env),
                min_amount: 0,
                max_amount: i128::MAX,
                min_deadline: 0,
                max_deadline: u64::MAX,
            };
            let results = s.escrow.query_escrows(&filter, &offset, &limit);
            let mut page = std::vec::Vec::new();
            for r in results.iter() {
                page.push(r);
            }
            page
        },
        |r| r.bounty_id,
    )?;

    // --- 2. Query: query_escrows_by_status (Locked) ---
    assert_pagination_invariants(
        expected_locked,
        |offset, limit| {
            let results = s.escrow.query_escrows_by_status(&EscrowStatus::Locked, &offset, &limit);
            let mut page = std::vec::Vec::new();
            for r in results.iter() {
                page.push(r);
            }
            page
        },
        |r| r.bounty_id,
    )?;

    // --- 3. Query: query_escrows_by_amount (Varied ranges) ---
    // Test filter stability under amounts between 200 and 800
    let expected_in_amount_range = defs
        .iter()
        .filter(|d| d.amount >= 200 && d.amount <= 800)
        .count();
    assert_pagination_invariants(
        expected_in_amount_range,
        |offset, limit| {
            let results = s.escrow.query_escrows_by_amount(&200, &800, &offset, &limit);
            let mut page = std::vec::Vec::new();
            for r in results.iter() {
                page.push(r);
            }
            page
        },
        |r| r.bounty_id,
    )?;

    // --- 4. Query: query_escrows_by_deadline (Varied ranges) ---
    let expected_in_deadline_range = defs
        .iter()
        .filter(|d| d.deadline >= 1200 && d.deadline <= 1800)
        .count();
    assert_pagination_invariants(
        expected_in_deadline_range,
        |offset, limit| {
            let results = s.escrow.query_escrows_by_deadline(&1200, &1800, &offset, &limit);
            let mut page = std::vec::Vec::new();
            for r in results.iter() {
                page.push(r);
            }
            page
        },
        |r| r.bounty_id,
    )?;

    // --- 5. Query: query_escrows_by_depositor (Depositor index path) ---
    for (dep_idx, depositor) in s.depositors.iter().enumerate() {
        let expected_dep_count = defs.iter().filter(|d| d.depositor_idx as usize % s.depositors.len() == dep_idx).count();
        assert_pagination_invariants(
            expected_dep_count,
            |offset, limit| {
                let results = s.escrow.query_escrows_by_depositor(depositor, &offset, &limit);
                let mut page = std::vec::Vec::new();
                for r in results.iter() {
                    page.push(r);
                }
                page
            },
            |r| r.bounty_id,
        )?;
    }

    // --- 6. Query: get_escrow_ids_by_status (Locked) ---
    assert_pagination_invariants(
        expected_locked,
        |offset, limit| {
            let results = s.escrow.get_escrow_ids_by_status(&EscrowStatus::Locked, &offset, &limit);
            let mut page = std::vec::Vec::new();
            for id in results.iter() {
                page.push(id);
            }
            page
        },
        |&id| id,
    )?;

    // --- 7. Query: query_expiring_bounties (Deadline filtering) ---
    let expected_expiring = defs.iter().filter(|d| d.deadline <= 1500 && d.action == 0).count();
    assert_pagination_invariants(
        expected_expiring,
        |offset, limit| {
            let results = s.escrow.query_expiring_bounties(&1500, &offset, &limit);
            let mut page = std::vec::Vec::new();
            for id in results.iter() {
                page.push(id);
            }
            page
        },
        |&id| id,
    )?;

    Ok(())
}

#[test]
fn proptest_query_pagination_invariants_hold() {
    let mut runner = deterministic_runner();
    let strategy = proptest::collection::vec(escrow_def_strategy(), 1..=40);
    runner
        .run(&strategy, |defs| run_pagination_test(defs))
        .expect("query pagination invariants must hold across all query functions");
}
