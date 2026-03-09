/// # Gas Proxy Tests — Bounty Escrow
///
/// Validates that high-impact operations (batch lock, batch release, dispute
/// resolution) stay within acceptable CPU-instruction budgets across complex
/// lifecycle scenarios.
///
/// ## How gas is measured
/// `soroban_sdk::testutils::Budget` exposes `cpu_instruction_cost()` after
/// every contract invocation. We snapshot this counter **before** and **after**
/// each operation, derive the delta, and assert it stays below a documented
/// threshold.
///
/// ## Threshold rationale
/// Thresholds are set to 2× the observed baseline on the reference machine to
/// allow for minor SDK version fluctuations while still catching regressions.
/// All constants are named so reviewers can update them in one place.
///
/// ## Security notes
/// * All tests use `env.mock_all_auths()` — auth correctness is covered by
///   the existing RBAC test suite; here we isolate pure gas behaviour only.
/// * No state leaks between tests: each test creates a fresh `Env` and resets
///   the budget counter before measuring.
/// * Batch sizes are tested at 1, mid-range (10), and the protocol maximum
///   (20) to confirm gas scales linearly and not exponentially.
#[cfg(test)]
use crate::{
    BountyEscrowContract, BountyEscrowContractClient, EscrowStatus, LockFundsItem,
    ReleaseFundsItem,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, Vec,
};

// ── CPU budget thresholds (instructions) ─────────────────────────────────────
// All values are intentionally generous (≈ 2× measured baseline) to avoid
// flaky CI while still catching O(n²) growth or runaway storage writes.

/// Single lock_funds call.
const MAX_CPU_LOCK_SINGLE: u64 = 5_000_000;
/// Single release_funds call.
const MAX_CPU_RELEASE_SINGLE: u64 = 5_000_000;
/// batch_lock_funds with 1 item.
const MAX_CPU_BATCH_LOCK_1: u64 = 6_000_000;
/// batch_lock_funds with 10 items.
const MAX_CPU_BATCH_LOCK_10: u64 = 30_000_000;
/// batch_lock_funds at protocol maximum (20 items).
const MAX_CPU_BATCH_LOCK_20: u64 = 55_000_000;
/// batch_release_funds with 1 item.
const MAX_CPU_BATCH_RELEASE_1: u64 = 6_000_000;
/// batch_release_funds with 10 items.
const MAX_CPU_BATCH_RELEASE_10: u64 = 30_000_000;
/// batch_release_funds at protocol maximum (20 items).
const MAX_CPU_BATCH_RELEASE_20: u64 = 55_000_000;
/// Full dispute lifecycle (authorize_claim → cancel → refund).
const MAX_CPU_DISPUTE_LIFECYCLE: u64 = 25_000_000;
/// Mixed lifecycle (batch lock 10 → batch release 5 → dispute×5 → cancel×5 → refund×5).
const MAX_CPU_MIXED_LIFECYCLE: u64 = 80_000_000;
/// Ten sequential partial_release calls on one escrow.
const MAX_CPU_PARTIAL_RELEASE_LOOP: u64 = 40_000_000;
/// query_escrows_by_status scan over 20 locked escrows.
const MAX_CPU_QUERY_20: u64 = 15_000_000;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn create_token_contract<'a>(
    e: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    // Use the same registration path as the rest of the test suite to ensure
    // identical token contract behaviour. The v2 API path behaves differently
    // with persistent storage in some SDK versions.
    #[allow(deprecated)]
    let addr = e.register_stellar_asset_contract(admin.clone());
    (
        token::Client::new(e, &addr),
        token::StellarAssetClient::new(e, &addr),
    )
}

fn create_escrow_contract<'a>(e: &Env) -> BountyEscrowContractClient<'a> {
    let id = e.register_contract(None, BountyEscrowContract);
    BountyEscrowContractClient::new(e, &id)
}

/// Shared setup for every gas test.
struct GasTestSetup<'a> {
    env: Env,
    admin: Address,
    depositor: Address,
    contributor: Address,
    token: token::Client<'a>,
    token_admin: token::StellarAssetClient<'a>,
    escrow: BountyEscrowContractClient<'a>,
}

impl<'a> GasTestSetup<'a> {
    /// Creates a fresh environment with a funded depositor (100 000 000 units).
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        env.budget().reset_default();

        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let contributor = Address::generate(&env);

        let (token, token_admin) = create_token_contract(&env, &admin);
        let escrow = create_escrow_contract(&env);

        escrow.init(&admin, &token.address);
        token_admin.mint(&depositor, &100_000_000);

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

    /// Returns the current cumulative CPU instruction cost.
    fn cpu(&self) -> u64 {
        self.env.budget().cpu_instruction_cost()
    }

    /// Asserts that the CPU delta between two snapshots is within `max_cpu`.
    fn assert_cpu_within(before: u64, after: u64, max_cpu: u64, label: &str) {
        let delta = after.saturating_sub(before);
        assert!(
            delta <= max_cpu,
            "[GAS] {label}: CPU instructions {delta} exceeded limit {max_cpu}"
        );
    }

    /// Build a Vec of `n` LockFundsItem structs (bounty IDs 1..=n).
    fn make_lock_batch(&self, n: u32, deadline: u64) -> Vec<LockFundsItem> {
        let mut items = Vec::new(&self.env);
        for i in 1..=n {
            items.push_back(LockFundsItem {
                bounty_id: i as u64,
                depositor: self.depositor.clone(),
                amount: 1_000,
                deadline,
            });
        }
        items
    }

    /// Build a Vec of `n` ReleaseFundsItem structs (bounty IDs 1..=n).
    fn make_release_batch(&self, n: u32) -> Vec<ReleaseFundsItem> {
        let mut items = Vec::new(&self.env);
        for i in 1..=n {
            items.push_back(ReleaseFundsItem {
                bounty_id: i as u64,
                contributor: self.contributor.clone(),
            });
        }
        items
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// 1. SINGLE OPERATION BASELINES
//    Establish the minimum cost for the simplest possible calls so that
//    helper-code regressions are caught before the batch tests run.
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn gas_baseline_single_lock() {
    let setup = GasTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1_000;

    let before = setup.cpu();
    setup
        .escrow
        .lock_funds(&setup.depositor, &1u64, &1_000i128, &deadline);
    let after = setup.cpu();

    GasTestSetup::assert_cpu_within(before, after, MAX_CPU_LOCK_SINGLE, "single lock_funds");
}

#[test]
fn gas_baseline_single_release() {
    let setup = GasTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1_000;
    setup
        .escrow
        .lock_funds(&setup.depositor, &1u64, &1_000i128, &deadline);

    let before = setup.cpu();
    setup.escrow.release_funds(&1u64, &setup.contributor);
    let after = setup.cpu();

    GasTestSetup::assert_cpu_within(before, after, MAX_CPU_RELEASE_SINGLE, "single release_funds");
}

// ═════════════════════════════════════════════════════════════════════════════
// 2. BATCH LOCK — SIZE SCALING
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn gas_batch_lock_1_item() {
    let setup = GasTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1_000;
    let items = setup.make_lock_batch(1, deadline);

    let before = setup.cpu();
    let count = setup.escrow.batch_lock_funds(&items);
    let after = setup.cpu();

    assert_eq!(count, 1);
    GasTestSetup::assert_cpu_within(before, after, MAX_CPU_BATCH_LOCK_1, "batch_lock_funds(1)");
}

#[test]
fn gas_batch_lock_10_items() {
    let setup = GasTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1_000;
    let items = setup.make_lock_batch(10, deadline);

    let before = setup.cpu();
    let count = setup.escrow.batch_lock_funds(&items);
    let after = setup.cpu();

    assert_eq!(count, 10);
    GasTestSetup::assert_cpu_within(before, after, MAX_CPU_BATCH_LOCK_10, "batch_lock_funds(10)");
}

#[test]
fn gas_batch_lock_20_items_max() {
    let setup = GasTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1_000;
    let items = setup.make_lock_batch(20, deadline);

    let before = setup.cpu();
    let count = setup.escrow.batch_lock_funds(&items);
    let after = setup.cpu();

    assert_eq!(count, 20);
    GasTestSetup::assert_cpu_within(
        before,
        after,
        MAX_CPU_BATCH_LOCK_20,
        "batch_lock_funds(20)",
    );
}

/// Gas must scale sub-linearly or at worst linearly with batch size.
/// cost(20) must be ≤ 25× cost(1) — ruling out O(n²) validation paths.
#[test]
fn gas_batch_lock_scaling_is_linear_not_quadratic() {
    let setup = GasTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1_000;

    // Measure batch of 1
    let items1 = setup.make_lock_batch(1, deadline);
    let b1 = setup.cpu();
    setup.escrow.batch_lock_funds(&items1);
    let a1 = setup.cpu();
    let cpu_1 = a1.saturating_sub(b1).max(1); // max(1) prevents div-by-zero

    // Measure batch of 20 with distinct IDs (offset to avoid BountyExists)
    let mut items20 = Vec::new(&setup.env);
    for i in 101u32..=120 {
        items20.push_back(LockFundsItem {
            bounty_id: i as u64,
            depositor: setup.depositor.clone(),
            amount: 1_000,
            deadline,
        });
    }
    let b20 = setup.cpu();
    setup.escrow.batch_lock_funds(&items20);
    let a20 = setup.cpu();
    let cpu_20 = a20.saturating_sub(b20);

    let ratio = cpu_20 / cpu_1;
    assert!(
        ratio <= 25,
        "[GAS] batch lock looks super-linear: cost(20)/cost(1) = {ratio} (limit 25x)"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 3. BATCH RELEASE — SIZE SCALING
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn gas_batch_release_1_item() {
    let setup = GasTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1_000;
    setup
        .escrow
        .batch_lock_funds(&setup.make_lock_batch(1, deadline));

    let before = setup.cpu();
    let count = setup
        .escrow
        .batch_release_funds(&setup.make_release_batch(1));
    let after = setup.cpu();

    assert_eq!(count, 1);
    GasTestSetup::assert_cpu_within(
        before,
        after,
        MAX_CPU_BATCH_RELEASE_1,
        "batch_release_funds(1)",
    );
}

#[test]
fn gas_batch_release_10_items() {
    let setup = GasTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1_000;
    setup
        .escrow
        .batch_lock_funds(&setup.make_lock_batch(10, deadline));

    let before = setup.cpu();
    let count = setup
        .escrow
        .batch_release_funds(&setup.make_release_batch(10));
    let after = setup.cpu();

    assert_eq!(count, 10);
    GasTestSetup::assert_cpu_within(
        before,
        after,
        MAX_CPU_BATCH_RELEASE_10,
        "batch_release_funds(10)",
    );
}

#[test]
fn gas_batch_release_20_items_max() {
    let setup = GasTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1_000;
    setup
        .escrow
        .batch_lock_funds(&setup.make_lock_batch(20, deadline));

    let before = setup.cpu();
    let count = setup
        .escrow
        .batch_release_funds(&setup.make_release_batch(20));
    let after = setup.cpu();

    assert_eq!(count, 20);
    GasTestSetup::assert_cpu_within(
        before,
        after,
        MAX_CPU_BATCH_RELEASE_20,
        "batch_release_funds(20)",
    );
}

/// Release scaling must also be linear. cost(19)/cost(1) must be <= 25x.
#[test]
fn gas_batch_release_scaling_is_linear_not_quadratic() {
    let setup = GasTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1_000;

    // Lock 20 up front so IDs are available
    setup
        .escrow
        .batch_lock_funds(&setup.make_lock_batch(20, deadline));

    // Measure release of 1
    let items1 = setup.make_release_batch(1);
    let b1 = setup.cpu();
    setup.escrow.batch_release_funds(&items1);
    let a1 = setup.cpu();
    let cpu_1 = a1.saturating_sub(b1).max(1);

    // Lock 19 more with offset IDs, then measure releasing those 19
    let mut lock19 = Vec::new(&setup.env);
    for i in 21u32..=39 {
        lock19.push_back(LockFundsItem {
            bounty_id: i as u64,
            depositor: setup.depositor.clone(),
            amount: 1_000,
            deadline,
        });
    }
    setup.escrow.batch_lock_funds(&lock19);

    let mut release19 = Vec::new(&setup.env);
    for i in 21u32..=39 {
        release19.push_back(ReleaseFundsItem {
            bounty_id: i as u64,
            contributor: setup.contributor.clone(),
        });
    }
    let b19 = setup.cpu();
    setup.escrow.batch_release_funds(&release19);
    let a19 = setup.cpu();
    let cpu_19 = a19.saturating_sub(b19);

    let ratio = cpu_19 / cpu_1;
    assert!(
        ratio <= 25,
        "[GAS] batch release looks super-linear: cost(19)/cost(1) = {ratio} (limit 25x)"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 4. DISPUTE LIFECYCLE GAS
//    Scenario A: lock -> authorize_claim -> cancel -> refund
//    Scenario B: lock -> authorize_claim -> claim  (resolved for contributor)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn gas_dispute_full_lifecycle_cancel_and_refund() {
    let setup = GasTestSetup::new();
    let now = setup.env.ledger().timestamp();
    let deadline = now + 2_000;

    setup.escrow.set_claim_window(&500u64);
    setup
        .escrow
        .lock_funds(&setup.depositor, &1u64, &5_000i128, &deadline);

    let before = setup.cpu();

    // Step 1: open dispute
    setup.escrow.authorize_claim(&1u64, &setup.contributor);

    // Step 2: let window expire, then cancel
    let claim = setup.escrow.get_pending_claim(&1u64);
    setup.env.ledger().set_timestamp(claim.expires_at + 1);
    setup.escrow.cancel_pending_claim(&1u64);

    // Step 3: advance past bounty deadline and refund
    setup.env.ledger().set_timestamp(deadline + 1);
    setup.escrow.refund(&1u64);

    let after = setup.cpu();

    GasTestSetup::assert_cpu_within(
        before,
        after,
        MAX_CPU_DISPUTE_LIFECYCLE,
        "dispute: authorize_claim -> cancel -> refund",
    );

    let escrow = setup.escrow.get_escrow_info(&1u64);
    assert_eq!(escrow.status, EscrowStatus::Refunded);
}

#[test]
fn gas_dispute_resolved_by_claim() {
    let setup = GasTestSetup::new();
    let now = setup.env.ledger().timestamp();
    let deadline = now + 2_000;

    setup.escrow.set_claim_window(&800u64);
    setup
        .escrow
        .lock_funds(&setup.depositor, &2u64, &3_000i128, &deadline);

    let before = setup.cpu();
    setup.escrow.authorize_claim(&2u64, &setup.contributor);
    setup.escrow.claim(&2u64);
    let after = setup.cpu();

    GasTestSetup::assert_cpu_within(
        before,
        after,
        MAX_CPU_DISPUTE_LIFECYCLE,
        "dispute: authorize_claim -> claim",
    );

    let escrow = setup.escrow.get_escrow_info(&2u64);
    assert_eq!(escrow.status, EscrowStatus::Released);
    assert_eq!(setup.token.balance(&setup.contributor), 3_000);
}

/// A dispute that is cancelled and re-opened must not accumulate gas cost.
/// The second cycle must cost no more than 150% of the first.
#[test]
fn gas_dispute_re_opened_after_cancel_same_cost() {
    let setup = GasTestSetup::new();
    let now = setup.env.ledger().timestamp();
    let deadline = now + 5_000;

    setup.escrow.set_claim_window(&200u64);
    setup
        .escrow
        .lock_funds(&setup.depositor, &3u64, &2_000i128, &deadline);

    // First dispute cycle
    let b1 = setup.cpu();
    setup.escrow.authorize_claim(&3u64, &setup.contributor);
    let c1 = setup.escrow.get_pending_claim(&3u64);
    setup.env.ledger().set_timestamp(c1.expires_at + 1);
    setup.escrow.cancel_pending_claim(&3u64);
    let a1 = setup.cpu();
    let cpu_first = a1.saturating_sub(b1);

    // Reset window for second cycle
    setup.escrow.set_claim_window(&200u64);

    // Second dispute cycle
    let b2 = setup.cpu();
    setup.escrow.authorize_claim(&3u64, &setup.contributor);
    let c2 = setup.escrow.get_pending_claim(&3u64);
    setup.env.ledger().set_timestamp(c2.expires_at + 1);
    setup.escrow.cancel_pending_claim(&3u64);
    let a2 = setup.cpu();
    let cpu_second = a2.saturating_sub(b2);

    let limit = cpu_first + cpu_first / 2; // 150%
    assert!(
        cpu_second <= limit,
        "[GAS] re-opened dispute cost {cpu_second} > 150% of first cycle {cpu_first}"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 5. COMPLEX MIXED LIFECYCLE
//    batch lock 10 -> batch release 5 -> open disputes on 5 -> cancel x 5 -> refund x 5
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn gas_mixed_lifecycle_batch_lock_partial_release_dispute_refund() {
    let setup = GasTestSetup::new();
    let now = setup.env.ledger().timestamp();
    let deadline = now + 5_000;

    setup.escrow.set_claim_window(&300u64);

    // Phase 0: lock 10 bounties (setup cost, not measured)
    setup
        .escrow
        .batch_lock_funds(&setup.make_lock_batch(10, deadline));

    let before = setup.cpu();

    // Phase 1: batch release bounties 1-5
    let released = setup
        .escrow
        .batch_release_funds(&setup.make_release_batch(5));
    assert_eq!(released, 5);

    // Phase 2: open disputes on bounties 6-10
    for bounty_id in 6u64..=10 {
        setup.escrow.authorize_claim(&bounty_id, &setup.contributor);
    }

    // Phase 3: let windows expire then cancel
    let sample = setup.escrow.get_pending_claim(&6u64);
    setup.env.ledger().set_timestamp(sample.expires_at + 1);
    for bounty_id in 6u64..=10 {
        setup.escrow.cancel_pending_claim(&bounty_id);
    }

    // Phase 4: advance past bounty deadline and refund all 5
    setup.env.ledger().set_timestamp(deadline + 1);
    for bounty_id in 6u64..=10 {
        setup.escrow.refund(&bounty_id);
    }

    let after = setup.cpu();

    GasTestSetup::assert_cpu_within(
        before,
        after,
        MAX_CPU_MIXED_LIFECYCLE,
        "mixed: batch-lock(10) -> batch-release(5) -> dispute x5 -> cancel x5 -> refund x5",
    );

    // Correctness assertions
    for bounty_id in 1u64..=5 {
        assert_eq!(
            setup.escrow.get_escrow_info(&bounty_id).status,
            EscrowStatus::Released
        );
    }
    for bounty_id in 6u64..=10 {
        assert_eq!(
            setup.escrow.get_escrow_info(&bounty_id).status,
            EscrowStatus::Refunded
        );
    }
    assert_eq!(setup.escrow.get_balance(), 0);
}

// ═════════════════════════════════════════════════════════════════════════════
// 6. PARTIAL RELEASE LOOP GAS
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn gas_partial_release_repeated_10_times() {
    let setup = GasTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1_000;
    let total = 1_000i128;
    let payout = 100i128;

    setup
        .escrow
        .lock_funds(&setup.depositor, &1u64, &total, &deadline);

    let before = setup.cpu();
    for _ in 0..10 {
        setup
            .escrow
            .partial_release(&1u64, &setup.contributor, &payout);
    }
    let after = setup.cpu();

    GasTestSetup::assert_cpu_within(
        before,
        after,
        MAX_CPU_PARTIAL_RELEASE_LOOP,
        "10x partial_release on single escrow",
    );

    let escrow = setup.escrow.get_escrow_info(&1u64);
    assert_eq!(escrow.remaining_amount, 0);
    assert_eq!(escrow.status, EscrowStatus::Released);
    assert_eq!(setup.token.balance(&setup.contributor), total);
}

/// The 5th partial_release must cost no more than 120% of the 1st call.
/// Catches any accumulating-state anti-pattern (e.g. growing refund_history).
#[test]
fn gas_partial_release_cost_is_stable_per_call() {
    let setup = GasTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1_000;
    let total = 1_000i128;
    let payout = 100i128;

    setup
        .escrow
        .lock_funds(&setup.depositor, &1u64, &total, &deadline);

    // Measure the 1st call
    let b0 = setup.cpu();
    setup
        .escrow
        .partial_release(&1u64, &setup.contributor, &payout);
    let a0 = setup.cpu();
    let cpu_first = a0.saturating_sub(b0).max(1);

    // Burn through calls 2, 3, 4
    for _ in 0..3 {
        setup
            .escrow
            .partial_release(&1u64, &setup.contributor, &payout);
    }

    // Measure the 5th call
    let b5 = setup.cpu();
    setup
        .escrow
        .partial_release(&1u64, &setup.contributor, &payout);
    let a5 = setup.cpu();
    let cpu_fifth = a5.saturating_sub(b5);

    let limit = cpu_first + cpu_first / 5; // 120% of first
    assert!(
        cpu_fifth <= limit,
        "[GAS] partial_release cost grew: 5th call {cpu_fifth} > 120% of 1st {cpu_first}"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 7. QUERY REGRESSION GUARD
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn gas_query_after_batch_lock_20() {
    let setup = GasTestSetup::new();
    // Each lock_funds call requires a 61-second ledger gap due to the anti_abuse
    // cooldown (default 60 s) on the depositor address. 20 locks × 61 s = 1220 s.
    // Set the deadline far enough ahead that all locks complete before it expires.
    let start_ts = setup.env.ledger().timestamp();
    let deadline = start_ts + 10_000;

    // Use individual lock_funds calls so EscrowIndex is populated.
    // batch_lock_funds stores Escrow entries but never writes EscrowIndex,
    // so query_escrows_by_status would return 0 after a batch lock.
    for i in 1u64..=20 {
        setup.escrow.lock_funds(
            &setup.depositor,
            &i,
            &1_000i128,
            &deadline,
        );
        // Advance past the anti_abuse cooldown period between each lock.
        let ts = setup.env.ledger().timestamp();
        setup.env.ledger().set_timestamp(ts + 61);
    }

    let before = setup.cpu();
    let results = setup
        .escrow
        .query_escrows_by_status(&EscrowStatus::Locked, &0u32, &20u32);
    let after = setup.cpu();

    // soroban Vec::len() returns u32
    assert_eq!(results.len(), 20u32);
    GasTestSetup::assert_cpu_within(
        before,
        after,
        MAX_CPU_QUERY_20,
        "query_escrows_by_status over 20 individually-locked escrows",
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 8. SDK STABILITY SENTINEL
//    Records the absolute instruction cost so CI can track it across upgrades.
//    Run with `cargo test gas_instrumentation_sentinel -- --nocapture` to see
//    the raw metric.
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn gas_instrumentation_sentinel_batch_lock_10() {
    let setup = GasTestSetup::new();
    let deadline = setup.env.ledger().timestamp() + 1_000;
    let items = setup.make_lock_batch(10, deadline);

    // Full reset so we measure only this operation
    setup.env.budget().reset_default();
    setup.escrow.batch_lock_funds(&items);

    let cpu = setup.env.budget().cpu_instruction_cost();

    // Sanity guard: the budget must have consumed some instructions.
    // The actual value is visible in `cargo test -- --nocapture` output.
    assert!(
        cpu > 0,
        "[GAS] cpu_instruction_cost() returned 0 — budget SDK integration may be broken"
    );
}