//! # Bounty Escrow Analytics Module
//!
//! This module provides comprehensive analytics views for bounty escrow contracts,
//! enabling off-chain indexing and monitoring of contract state.
//!
//! ## Features
//! - Track active bounties and their lifecycle states
//! - Monitor total locked and paid out amounts
//! - Query escrows by multiple dimensions (status, amount, deadline, depositor)
//! - Emit state transition events for off-chain indexing
//! - Efficient aggregated statistics
//!
//! ## Events
//! - `BountyStateTransitioned` - Emitted when a bounty status changes
//! - `AnalyticsSnapshot` - Periodic snapshots of contract-wide metrics
//!
//! ## Arithmetic safety
//! All aggregate accumulator updates use `checked_add` / `checked_sub` to
//! prevent silent wrap-around on `i128` fields.  Any overflow returns
//! [`AnalyticsError::Overflow`] rather than panicking or wrapping silently.

use soroban_sdk::{contracttype, symbol_short, Address, Env, Symbol};

/// Analytics version
pub const ANALYTICS_VERSION_V1: u32 = 1;

/// Error type for analytics operations.
///
/// `AnalyticsError::Overflow` is returned whenever a checked arithmetic
/// operation on an aggregate accumulator would overflow `i128` or `u32`.
/// Appended at the end to avoid renumbering any previously-defined variants
/// in the parent contract's `Error` enum.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum AnalyticsError {
    /// Arithmetic overflow in an aggregate accumulator.
    ///
    /// This should never occur during normal contract operation because bounty
    /// amounts are bounded by the token supply, but is returned instead of
    /// panicking or wrapping silently when near-`i128::MAX` inputs are detected.
    Overflow,
}

/// Compact analytics struct for bounty-level summaries
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BountyAnalytics {
    /// Total amount originally locked in this bounty.
    ///
    /// Expected input bound: 0 ≤ total_amount_locked ≤ i128::MAX / 2.
    /// Values above this bound will cause `checked_add` to return
    /// `AnalyticsError::Overflow` on the first accumulation.
    pub total_amount_locked: i128,
    /// Total amount released to contributors.
    ///
    /// Expected input bound: 0 ≤ total_amount_released ≤ total_amount_locked.
    pub total_amount_released: i128,
    /// Total amount refunded to original depositor.
    ///
    /// Expected input bound: 0 ≤ total_amount_refunded ≤ total_amount_locked.
    pub total_amount_refunded: i128,
    /// Current remaining amount in escrow.
    pub remaining_amount: i128,
    /// Bounty creation timestamp
    pub created_at: u64,
    /// Timestamp of last state transition
    pub last_updated: u64,
    /// Number of partial releases performed.
    ///
    /// Expected input bound: partial_releases_count ≤ u32::MAX.
    pub partial_releases_count: u32,
    /// Number of partial refunds performed.
    ///
    /// Expected input bound: partial_refunds_count ≤ u32::MAX.
    pub partial_refunds_count: u32,
}

/// Contract-wide analytics snapshot
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractAnalytics {
    /// Total number of active bounties (Locked or Partially Refunded)
    pub active_bounty_count: u32,
    /// Total number of released bounties
    pub released_bounty_count: u32,
    /// Total number of refunded bounties
    pub refunded_bounty_count: u32,
    /// Total amount currently locked in contract
    pub total_locked: i128,
    /// Total amount released to all contributors
    pub total_released: i128,
    /// Total amount refunded to all depositors
    pub total_refunded: i128,
    /// Average bounty amount
    pub average_bounty_amount: i128,
    /// Timestamp of this snapshot
    pub snapshot_timestamp: u64,
}

/// State transition event for bounty escrow
#[contracttype]
#[derive(Clone, Debug)]
pub struct BountyStateTransitioned {
    /// Analytics version
    pub version: u32,
    /// Bounty ID
    pub bounty_id: u64,
    /// Previous state (e.g., "Locked")
    pub previous_state: Symbol,
    /// New state (e.g., "Released")
    pub new_state: Symbol,
    /// Amount involved in the transition
    pub amount: i128,
    /// Actor performing the transition
    pub actor: Address,
    /// Timestamp of the transition
    pub timestamp: u64,
}

/// Emit state transition event
pub fn emit_bounty_state_transitioned(env: &Env, event: BountyStateTransitioned) {
    env.events().publish(
        (symbol_short!("analytics"), symbol_short!("state_tx")),
        event,
    );
}

/// Analytics snapshot event for contract-wide metrics
#[contracttype]
#[derive(Clone, Debug)]
pub struct AnalyticsSnapshot {
    /// Analytics version
    pub version: u32,
    /// Contract-wide metrics
    pub metrics: ContractAnalytics,
}

/// Emit analytics snapshot event
pub fn emit_analytics_snapshot(env: &Env, event: AnalyticsSnapshot) {
    env.events()
        .publish((symbol_short!("analytics"), symbol_short!("snap")), event);
}

/// Bounty lifecycle event - records major state changes
#[contracttype]
#[derive(Clone, Debug)]
pub struct BountyActivityEvent {
    /// Analytics version
    pub version: u32,
    /// Bounty ID
    pub bounty_id: u64,
    /// Activity type: "created", "released", "refunded", "disputed"
    pub activity_type: Symbol,
    /// Amount affected
    pub amount: i128,
    /// Timestamp
    pub timestamp: u64,
}

/// Emit bounty activity event
pub fn emit_bounty_activity(env: &Env, event: BountyActivityEvent) {
    env.events().publish(
        (symbol_short!("analytics"), symbol_short!("activity")),
        event,
    );
}

/// Storage keys for analytics
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnalyticsKey {
    /// Per-bounty analytics: AnalyticsKey::BountyMetrics(bounty_id) -> BountyAnalytics
    BountyMetrics(u64),
}

/// Initialize bounty analytics on lock
pub fn init_bounty_analytics(env: &Env, bounty_id: u64, amount: i128, timestamp: u64) {
    let analytics = BountyAnalytics {
        total_amount_locked: amount,
        total_amount_released: 0,
        total_amount_refunded: 0,
        remaining_amount: amount,
        created_at: timestamp,
        last_updated: timestamp,
        partial_releases_count: 0,
        partial_refunds_count: 0,
    };

    env.storage()
        .persistent()
        .set(&AnalyticsKey::BountyMetrics(bounty_id), &analytics);
}

/// Update analytics on release.
///
/// # Input bounds
/// `release_amount` must satisfy `0 ≤ release_amount ≤ i128::MAX − analytics.total_amount_released`.
/// Values outside this range cause `Err(AnalyticsError::Overflow)` to be returned instead
/// of panicking or allowing silent wrap-around of the accumulator.
///
/// # Errors
/// Returns `Err(AnalyticsError::Overflow)` when the checked addition of
/// `release_amount` into `total_amount_released` or `partial_releases_count`
/// would exceed the type's maximum value.
pub fn update_analytics_on_release(
    env: &Env,
    bounty_id: u64,
    release_amount: i128,
    timestamp: u64,
) -> Result<(), AnalyticsError> {
    if let Some(mut analytics) = env
        .storage()
        .persistent()
        .get::<AnalyticsKey, BountyAnalytics>(&AnalyticsKey::BountyMetrics(bounty_id))
    {
        // Use checked_add to detect overflow rather than panicking or wrapping.
        // Changing field order or types here is a BREAKING CHANGE for off-chain
        // consumers (e.g. internal/soroban/event_parser.go) that decode
        // BountyAnalytics by field position — update those consumers before
        // deploying any schema change.
        analytics.total_amount_released = analytics
            .total_amount_released
            .checked_add(release_amount)
            .ok_or(AnalyticsError::Overflow)?;
        analytics.remaining_amount = analytics.remaining_amount.saturating_sub(release_amount);
        analytics.last_updated = timestamp;
        analytics.partial_releases_count = analytics
            .partial_releases_count
            .checked_add(1)
            .ok_or(AnalyticsError::Overflow)?;

        env.storage()
            .persistent()
            .set(&AnalyticsKey::BountyMetrics(bounty_id), &analytics);
    }
    Ok(())
}

/// Update analytics on refund.
///
/// # Input bounds
/// `refund_amount` must satisfy `0 ≤ refund_amount ≤ i128::MAX − analytics.total_amount_refunded`.
/// Values outside this range cause `Err(AnalyticsError::Overflow)` to be returned instead
/// of panicking or allowing silent wrap-around of the accumulator.
///
/// # Errors
/// Returns `Err(AnalyticsError::Overflow)` when the checked addition of
/// `refund_amount` into `total_amount_refunded` or `partial_refunds_count`
/// would exceed the type's maximum value.
pub fn update_analytics_on_refund(
    env: &Env,
    bounty_id: u64,
    refund_amount: i128,
    timestamp: u64,
) -> Result<(), AnalyticsError> {
    if let Some(mut analytics) = env
        .storage()
        .persistent()
        .get::<AnalyticsKey, BountyAnalytics>(&AnalyticsKey::BountyMetrics(bounty_id))
    {
        // Use checked_add to detect overflow rather than panicking or wrapping.
        // Changing field order or types here is a BREAKING CHANGE for off-chain
        // consumers (e.g. internal/soroban/event_parser.go) that decode
        // BountyAnalytics by field position — update those consumers before
        // deploying any schema change.
        analytics.total_amount_refunded = analytics
            .total_amount_refunded
            .checked_add(refund_amount)
            .ok_or(AnalyticsError::Overflow)?;
        analytics.remaining_amount = analytics.remaining_amount.saturating_sub(refund_amount);
        analytics.last_updated = timestamp;
        analytics.partial_refunds_count = analytics
            .partial_refunds_count
            .checked_add(1)
            .ok_or(AnalyticsError::Overflow)?;

        env.storage()
            .persistent()
            .set(&AnalyticsKey::BountyMetrics(bounty_id), &analytics);
    }
    Ok(())
}

/// Get per-bounty analytics
pub fn get_bounty_analytics(env: &Env, bounty_id: u64) -> Option<BountyAnalytics> {
    env.storage()
        .persistent()
        .get::<AnalyticsKey, BountyAnalytics>(&AnalyticsKey::BountyMetrics(bounty_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{contract, contractimpl};

    #[contract]
    struct DummyAnalyticsContract;

    #[contractimpl]
    impl DummyAnalyticsContract {
        pub fn noop(_env: Env) {}
    }

    #[test]
    fn test_bounty_analytics_initialization() {
        let env = Env::default();
        let contract_id = env.register_contract(None, DummyAnalyticsContract);

        env.as_contract(&contract_id, || {
            let bounty_id = 1u64;
            let amount = 1000i128;
            let timestamp = 100u64;

            init_bounty_analytics(&env, bounty_id, amount, timestamp);

            let analytics = get_bounty_analytics(&env, bounty_id);
            assert!(analytics.is_some());

            let analytics = analytics.unwrap();
            assert_eq!(analytics.total_amount_locked, amount);
            assert_eq!(analytics.total_amount_released, 0);
            assert_eq!(analytics.total_amount_refunded, 0);
            assert_eq!(analytics.remaining_amount, amount);
            assert_eq!(analytics.created_at, timestamp);
            assert_eq!(analytics.last_updated, timestamp);
            assert_eq!(analytics.partial_releases_count, 0);
            assert_eq!(analytics.partial_refunds_count, 0);
        });
    }

    #[test]
    fn test_analytics_on_release() {
        let env = Env::default();
        let contract_id = env.register_contract(None, DummyAnalyticsContract);

        env.as_contract(&contract_id, || {
            let bounty_id = 2u64;
            let amount = 1000i128;

            init_bounty_analytics(&env, bounty_id, amount, 100);
            update_analytics_on_release(&env, bounty_id, 500, 200).unwrap();

            let analytics = get_bounty_analytics(&env, bounty_id).unwrap();
            assert_eq!(analytics.total_amount_released, 500);
            assert_eq!(analytics.remaining_amount, 500);
            assert_eq!(analytics.partial_releases_count, 1);
            assert_eq!(analytics.last_updated, 200);
        });
    }

    #[test]
    fn test_analytics_on_refund() {
        let env = Env::default();
        let contract_id = env.register_contract(None, DummyAnalyticsContract);

        env.as_contract(&contract_id, || {
            let bounty_id = 3u64;
            let amount = 1000i128;

            init_bounty_analytics(&env, bounty_id, amount, 100);
            update_analytics_on_refund(&env, bounty_id, 300, 200).unwrap();

            let analytics = get_bounty_analytics(&env, bounty_id).unwrap();
            assert_eq!(analytics.total_amount_refunded, 300);
            assert_eq!(analytics.remaining_amount, 700);
            assert_eq!(analytics.partial_refunds_count, 1);
            assert_eq!(analytics.last_updated, 200);
        });
    }

    #[test]
    fn test_analytics_lifecycle() {
        let env = Env::default();
        let contract_id = env.register_contract(None, DummyAnalyticsContract);

        env.as_contract(&contract_id, || {
            let bounty_id = 4u64;
            let amount = 1000i128;

            // Initialize
            init_bounty_analytics(&env, bounty_id, amount, 100);

            // Partial release
            update_analytics_on_release(&env, bounty_id, 300, 200).unwrap();
            let analytics = get_bounty_analytics(&env, bounty_id).unwrap();
            assert_eq!(analytics.remaining_amount, 700);
            assert_eq!(analytics.total_amount_released, 300);

            // Another release
            update_analytics_on_release(&env, bounty_id, 300, 300).unwrap();
            let analytics = get_bounty_analytics(&env, bounty_id).unwrap();
            assert_eq!(analytics.remaining_amount, 400);
            assert_eq!(analytics.total_amount_released, 600);
            assert_eq!(analytics.partial_releases_count, 2);

            // Final refund for remaining
            update_analytics_on_refund(&env, bounty_id, 400, 400).unwrap();
            let analytics = get_bounty_analytics(&env, bounty_id).unwrap();
            assert_eq!(analytics.remaining_amount, 0);
            assert_eq!(analytics.total_amount_refunded, 400);
            assert_eq!(analytics.partial_refunds_count, 1);
        });
    }

    #[test]
    fn test_get_nonexistent_bounty_analytics() {
        let env = Env::default();
        let contract_id = env.register_contract(None, DummyAnalyticsContract);

        env.as_contract(&contract_id, || {
            let analytics = get_bounty_analytics(&env, 999u64);
            assert!(analytics.is_none());
        });
    }

    // -----------------------------------------------------------------------
    // Overflow / checked-arithmetic tests (Issue #165)
    // -----------------------------------------------------------------------

    /// release: near-max value that still fits must succeed.
    ///
    /// Hand-computed:
    ///   total_amount_released = 0 before
    ///   release_amount        = i128::MAX − 1
    ///   result                = i128::MAX − 1  (no overflow)
    #[test]
    fn test_release_near_max_no_overflow() {
        let env = Env::default();
        let contract_id = env.register_contract(None, DummyAnalyticsContract);

        env.as_contract(&contract_id, || {
            let bounty_id = 100u64;
            // Initialize with a very large locked amount so the accumulator
            // starts at 0 and we can push it close to i128::MAX.
            init_bounty_analytics(&env, bounty_id, i128::MAX, 1);

            let near_max = i128::MAX - 1;
            let result = update_analytics_on_release(&env, bounty_id, near_max, 2);
            assert!(result.is_ok(), "near-max release must not overflow");

            let a = get_bounty_analytics(&env, bounty_id).unwrap();
            assert_eq!(a.total_amount_released, near_max);
        });
    }

    /// release: adding to an accumulator already at i128::MAX must return Overflow.
    ///
    /// Hand-computed:
    ///   total_amount_released = i128::MAX before the second call
    ///   release_amount        = 1
    ///   i128::MAX + 1 overflows → AnalyticsError::Overflow
    #[test]
    fn test_release_overflow_returns_error_not_panic() {
        let env = Env::default();
        let contract_id = env.register_contract(None, DummyAnalyticsContract);

        env.as_contract(&contract_id, || {
            let bounty_id = 101u64;
            init_bounty_analytics(&env, bounty_id, i128::MAX, 1);

            // First call: fill the accumulator to i128::MAX.
            update_analytics_on_release(&env, bounty_id, i128::MAX, 2)
                .expect("first call must succeed");

            // Second call: any positive increment must now overflow.
            let overflow_result = update_analytics_on_release(&env, bounty_id, 1, 3);
            assert_eq!(
                overflow_result,
                Err(AnalyticsError::Overflow),
                "overflow must return Err(AnalyticsError::Overflow), not panic"
            );
        });
    }

    /// refund: near-max value that still fits must succeed.
    ///
    /// Hand-computed:
    ///   total_amount_refunded = 0 before
    ///   refund_amount         = i128::MAX − 1
    ///   result                = i128::MAX − 1  (no overflow)
    #[test]
    fn test_refund_near_max_no_overflow() {
        let env = Env::default();
        let contract_id = env.register_contract(None, DummyAnalyticsContract);

        env.as_contract(&contract_id, || {
            let bounty_id = 102u64;
            init_bounty_analytics(&env, bounty_id, i128::MAX, 1);

            let near_max = i128::MAX - 1;
            let result = update_analytics_on_refund(&env, bounty_id, near_max, 2);
            assert!(result.is_ok(), "near-max refund must not overflow");

            let a = get_bounty_analytics(&env, bounty_id).unwrap();
            assert_eq!(a.total_amount_refunded, near_max);
        });
    }

    /// refund: adding to an accumulator already at i128::MAX must return Overflow.
    ///
    /// Hand-computed:
    ///   total_amount_refunded = i128::MAX before the second call
    ///   refund_amount         = 1
    ///   i128::MAX + 1 overflows → AnalyticsError::Overflow
    #[test]
    fn test_refund_overflow_returns_error_not_panic() {
        let env = Env::default();
        let contract_id = env.register_contract(None, DummyAnalyticsContract);

        env.as_contract(&contract_id, || {
            let bounty_id = 103u64;
            init_bounty_analytics(&env, bounty_id, i128::MAX, 1);

            // First call: fill the accumulator to i128::MAX.
            update_analytics_on_refund(&env, bounty_id, i128::MAX, 2)
                .expect("first call must succeed");

            // Second call: any positive increment must now overflow.
            let overflow_result = update_analytics_on_refund(&env, bounty_id, 1, 3);
            assert_eq!(
                overflow_result,
                Err(AnalyticsError::Overflow),
                "overflow must return Err(AnalyticsError::Overflow), not panic"
            );
        });
    }

    /// release: state must not be mutated after an overflow is detected.
    ///
    /// The accumulator and counter should remain at their pre-call values when
    /// the checked_add detects an overflow and returns early.
    #[test]
    fn test_release_overflow_state_not_mutated() {
        let env = Env::default();
        let contract_id = env.register_contract(None, DummyAnalyticsContract);

        env.as_contract(&contract_id, || {
            let bounty_id = 104u64;
            init_bounty_analytics(&env, bounty_id, i128::MAX, 1);

            // Fill the accumulator to i128::MAX.
            update_analytics_on_release(&env, bounty_id, i128::MAX, 2).unwrap();

            let before = get_bounty_analytics(&env, bounty_id).unwrap();

            // This must fail without mutating state.
            let _ = update_analytics_on_release(&env, bounty_id, 1, 3);

            let after = get_bounty_analytics(&env, bounty_id).unwrap();
            assert_eq!(
                before.total_amount_released, after.total_amount_released,
                "total_amount_released must not change after overflow"
            );
            assert_eq!(
                before.partial_releases_count, after.partial_releases_count,
                "partial_releases_count must not change after overflow"
            );
        });
    }

    /// refund: state must not be mutated after an overflow is detected.
    #[test]
    fn test_refund_overflow_state_not_mutated() {
        let env = Env::default();
        let contract_id = env.register_contract(None, DummyAnalyticsContract);

        env.as_contract(&contract_id, || {
            let bounty_id = 105u64;
            init_bounty_analytics(&env, bounty_id, i128::MAX, 1);

            // Fill the accumulator to i128::MAX.
            update_analytics_on_refund(&env, bounty_id, i128::MAX, 2).unwrap();

            let before = get_bounty_analytics(&env, bounty_id).unwrap();

            // This must fail without mutating state.
            let _ = update_analytics_on_refund(&env, bounty_id, 1, 3);

            let after = get_bounty_analytics(&env, bounty_id).unwrap();
            assert_eq!(
                before.total_amount_refunded, after.total_amount_refunded,
                "total_amount_refunded must not change after overflow"
            );
            assert_eq!(
                before.partial_refunds_count, after.partial_refunds_count,
                "partial_refunds_count must not change after overflow"
            );
        });
    }

    /// Missing bounty: update on a non-existent bounty must return Ok(()) (silent no-op).
    #[test]
    fn test_release_missing_bounty_is_noop_ok() {
        let env = Env::default();
        let contract_id = env.register_contract(None, DummyAnalyticsContract);

        env.as_contract(&contract_id, || {
            let result = update_analytics_on_release(&env, 999, 500, 100);
            assert_eq!(result, Ok(()), "missing bounty must return Ok(())");
            assert!(get_bounty_analytics(&env, 999).is_none());
        });
    }

    /// Missing bounty: update on a non-existent bounty must return Ok(()) (silent no-op).
    #[test]
    fn test_refund_missing_bounty_is_noop_ok() {
        let env = Env::default();
        let contract_id = env.register_contract(None, DummyAnalyticsContract);

        env.as_contract(&contract_id, || {
            let result = update_analytics_on_refund(&env, 999, 300, 50);
            assert_eq!(result, Ok(()), "missing bounty must return Ok(())");
            assert!(get_bounty_analytics(&env, 999).is_none());
        });
    }
}
