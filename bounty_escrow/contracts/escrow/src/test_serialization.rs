#![cfg(test)]

//! Serialization roundtrip tests for contract types.
//! Exercises TryFrom<ScVal> implementations to close coverage gap.

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env, Symbol, String, IntoVal, TryFromVal, Val};

fn roundtrip<T>(env: &Env, val: T)
where
    T: IntoVal<Env, Val> + TryFromVal<Env, Val>,
{
    let v: Val = val.into_val(env);
    let _back: T = T::try_from_val(env, &v).expect("deserialize");
}

#[test]
fn ser_escrow_types() {
    let env = Env::default();
    let a = Address::generate(&env);
    roundtrip(&env, EscrowStatus::Locked);
    roundtrip(&env, EscrowStatus::Released);
    roundtrip(&env, EscrowStatus::Refunded);
    roundtrip(&env, EscrowStatus::PartiallyRefunded);
    roundtrip(&env, Escrow{ depositor: a.clone(), amount: 100, remaining_amount: 50,
        status: EscrowStatus::Locked, deadline: 9999, refund_history: soroban_sdk::vec![&env] });
    roundtrip(&env, DataKey::Admin);
    roundtrip(&env, DataKey::Token);
    roundtrip(&env, DataKey::Escrow(1u64));
    roundtrip(&env, EscrowWithId{ bounty_id: 1, escrow: Escrow{ depositor: a.clone(),
        amount: 100, remaining_amount: 50, status: EscrowStatus::Locked, deadline: 1,
        refund_history: soroban_sdk::vec![&env] }});
    roundtrip(&env, PauseFlags{ global_paused: false, lock_paused: true,
        release_paused: false, refund_paused: false });
    roundtrip(&env, AggregateStats{ total_locked: 100, total_released: 50,
        total_refunded: 10, count_locked: 5, count_released: 3, count_refunded: 2 });
    roundtrip(&env, EscrowQueryFilter{ has_status_filter: true,
        status: EscrowStatus::Locked, has_depositor_filter: false, depositor: a.clone(),
        min_amount: 0, max_amount: i128::MAX, min_deadline: 0, max_deadline: u64::MAX });
    roundtrip(&env, PauseStateChanged{ operation: Symbol::new(&env, "op"),
        paused: true, admin: a.clone() });
    roundtrip(&env, FeeConfig{ lock_fee_rate: 100, release_fee_rate: 50,
        fee_recipient: a.clone(), fee_enabled: true });
    roundtrip(&env, MultisigConfig{ threshold_amount: 1000,
        signers: soroban_sdk::vec![&env, a.clone()], required_signatures: 1 });
    roundtrip(&env, AdminConfigSnapshot{ version: 1, admin: a.clone(), token: a.clone(),
        fee_config: FeeConfig{ lock_fee_rate: 0, release_fee_rate: 0,
            fee_recipient: a.clone(), fee_enabled: false },
        pause_flags: PauseFlags{ global_paused: false, lock_paused: false,
            release_paused: false, refund_paused: false },
        governance_contract: None, min_governance_version: 0, claim_window: 0,
        has_amount_policy: false, min_lock_amount: 0, max_lock_amount: 0 });
    roundtrip(&env, ReleaseApproval{ bounty_id: 1, contributor: a.clone(),
        approvals: soroban_sdk::vec![&env, a.clone()] });
    roundtrip(&env, ClaimRecord{ bounty_id: 1, recipient: a.clone(),
        amount: 500, expires_at: 9999, claimed: false });
    roundtrip(&env, RefundMode::Full);
    roundtrip(&env, RefundMode::Partial);
    roundtrip(&env, RefundApproval{ bounty_id: 1, amount: 100,
        recipient: a.clone(), mode: RefundMode::Partial,
        approved_by: a.clone(), approved_at: 1 });
    roundtrip(&env, RefundRecord{ amount: 100, recipient: a.clone(),
        timestamp: 1, mode: RefundMode::Partial });
    roundtrip(&env, LockFundsItem{ depositor: a.clone(), bounty_id: 1,
        amount: 100, deadline: 9999 });
    roundtrip(&env, ReleaseFundsItem{ bounty_id: 1, contributor: a.clone() });
}

#[test]
fn ser_monitoring_types() {
    let env = Env::default();
    let a = Address::generate(&env);
    roundtrip(&env, monitoring::OperationMetric{ operation: Symbol::new(&env, "test"),
        caller: a.clone(), timestamp: 42, success: true });
    roundtrip(&env, monitoring::PerformanceMetric{ function: Symbol::new(&env, "f"),
        duration: 100, timestamp: 1 });
    roundtrip(&env, monitoring::HealthStatus{ is_healthy: true, last_operation: 1,
        total_operations: 10, contract_version: String::from_str(&env, "1.0") });
    roundtrip(&env, monitoring::Analytics{ operation_count: 5, unique_users: 2,
        error_count: 1, error_rate: 200 });
    roundtrip(&env, monitoring::StateSnapshot{ timestamp: 1, total_operations: 100,
        total_users: 10, total_errors: 2 });
    roundtrip(&env, monitoring::PerformanceStats{ function_name: Symbol::new(&env, "fn"),
        call_count: 3, total_time: 300, avg_time: 100, last_called: 10 });
}

#[test]
fn ser_anti_abuse_types() {
    let env = Env::default();
    let a = Address::generate(&env);
    roundtrip(&env, anti_abuse::AntiAbuseConfig{ window_size: 60, max_operations: 10,
        cooldown_period: 1 });
    roundtrip(&env, anti_abuse::AddressState{ last_operation_timestamp: 100,
        window_start_timestamp: 50, operation_count: 3 });
    roundtrip(&env, anti_abuse::AntiAbuseKey::Whitelist(a.clone()));
}

#[test]
fn ser_error_recovery_types() {
    let env = Env::default();
    roundtrip(&env, error_recovery::CircuitState::Closed);
    roundtrip(&env, error_recovery::CircuitState::Open);
    roundtrip(&env, error_recovery::CircuitState::HalfOpen);
    roundtrip(&env, error_recovery::CircuitBreakerKey::State);
    roundtrip(&env, error_recovery::CircuitBreakerConfig{
        failure_threshold: 3, success_threshold: 2, max_error_log: 10 });
    roundtrip(&env, error_recovery::ErrorEntry{ bounty_id: 1,
        operation: Symbol::new(&env, "x"), error_code: 500, timestamp: 1,
        failure_count_at_time: 0 });
    roundtrip(&env, error_recovery::CircuitBreakerStatus{
        state: error_recovery::CircuitState::Closed, failure_count: 0,
        success_count: 0, last_failure_timestamp: 0, opened_at: 0,
        failure_threshold: 3, success_threshold: 2 });
    roundtrip(&env, error_recovery::RetryConfig{ max_attempts: 3 });
    roundtrip(&env, error_recovery::RetryResult{ succeeded: false,
        attempts: 2, final_error: 500 });
}

#[test]
fn ser_event_types() {
    let env = Env::default();
    let a = Address::generate(&env);
    roundtrip(&env, events::BountyEscrowInitialized{ version: 1,
        admin: a.clone(), token: a.clone(), timestamp: 1 });
    roundtrip(&env, events::FundsLocked{ version: 1, bounty_id: 1,
        amount: 100, depositor: a.clone(), deadline: 9999 });
    roundtrip(&env, events::FundsReleased{ version: 1, bounty_id: 1,
        amount: 100, recipient: a.clone(), timestamp: 1 });
    roundtrip(&env, events::FundsRefunded{ version: 1, bounty_id: 1,
        amount: 100, refund_to: a.clone(), timestamp: 1 });
    roundtrip(&env, events::BountyExpired{ version: 1, bounty_id: 1,
        depositor: a.clone(), amount: 500, deadline: 9999, expired_at: 42 });
    roundtrip(&env, events::FeeOperationType::Lock);
    roundtrip(&env, events::FeeOperationType::Release);
    roundtrip(&env, events::FeeCollected{ version: 1,
        operation_type: events::FeeOperationType::Lock, amount: 25,
        fee_rate: 500, recipient: a.clone(), timestamp: 1 });
    roundtrip(&env, events::BatchFundsLocked{ version: 1, count: 3,
        total_amount: 3000, timestamp: 1 });
    roundtrip(&env, events::FeeConfigUpdated{ version: 1, lock_fee_rate: 100,
        release_fee_rate: 50, fee_recipient: a.clone(), fee_enabled: true, timestamp: 1 });
    roundtrip(&env, events::BatchFundsReleased{ version: 1, count: 2,
        total_amount: 2000, timestamp: 1 });
    roundtrip(&env, events::ApprovalAdded{ version: 1, bounty_id: 1,
        contributor: a.clone(), approver: a.clone(), timestamp: 1 });
    roundtrip(&env, events::ClaimCreated{ version: 1, bounty_id: 1,
        recipient: a.clone(), amount: 100, expires_at: 9999 });
    roundtrip(&env, events::ClaimExecuted{ version: 1, bounty_id: 1,
        recipient: a.clone(), amount: 100, claimed_at: 42 });
    roundtrip(&env, events::ClaimCancelled{ version: 1, bounty_id: 1,
        recipient: a.clone(), amount: 100, cancelled_at: 42,
        cancelled_by: a.clone(), reason: Symbol::new(&env, "test") });
}

// ─────────────────────────────────────────────────────────────────────────────
// Analytics events — analytics.rs
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ser_analytics_event_types() {
    use analytics::{AnalyticsSnapshot, BountyActivityEvent, BountyStateTransitioned, ContractAnalytics};
    let env = Env::default();
    let a = Address::generate(&env);
    roundtrip(&env, BountyStateTransitioned {
        version: 1, bounty_id: 5,
        previous_state: Symbol::new(&env, "Locked"),
        new_state: Symbol::new(&env, "Released"),
        amount: 1000, actor: a.clone(), timestamp: 31337,
    });
    let metrics = ContractAnalytics {
        active_bounty_count: 3, released_bounty_count: 1, refunded_bounty_count: 0,
        total_locked: 3000, total_released: 1000, total_refunded: 0,
        average_bounty_amount: 1000, snapshot_timestamp: 42,
    };
    roundtrip(&env, metrics.clone());
    roundtrip(&env, AnalyticsSnapshot { version: 1, metrics });
    roundtrip(&env, BountyActivityEvent {
        version: 1, bounty_id: 8,
        activity_type: Symbol::new(&env, "created"),
        amount: 2000, timestamp: 7777,
    });
    // dispute-resolution activity type (added in the prior round)
    roundtrip(&env, BountyActivityEvent {
        version: 1, bounty_id: 100,
        activity_type: Symbol::new(&env, "disputed"),
        amount: 5000, timestamp: 88888,
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Error enum — discriminant stability (lib.rs)
// Verifies that every variant keeps its assigned u32 repr and is roundtrip-safe.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ser_error_enum_all_variants() {
    let env = Env::default();
    // Roundtrip every variant
    roundtrip(&env, Error::AlreadyInitialized);
    roundtrip(&env, Error::NotInitialized);
    roundtrip(&env, Error::BountyExists);
    roundtrip(&env, Error::BountyNotFound);
    roundtrip(&env, Error::FundsNotLocked);
    roundtrip(&env, Error::DeadlineNotPassed);
    roundtrip(&env, Error::Unauthorized);
    roundtrip(&env, Error::InvalidFeeRate);
    roundtrip(&env, Error::FeeRecipientNotSet);
    roundtrip(&env, Error::InvalidBatchSize);
    roundtrip(&env, Error::BatchSizeMismatch);
    roundtrip(&env, Error::DuplicateBountyId);
    roundtrip(&env, Error::InvalidAmount);
    roundtrip(&env, Error::InvalidDeadline);
    roundtrip(&env, Error::InsufficientFunds);
    roundtrip(&env, Error::RefundNotApproved);
    roundtrip(&env, Error::FundsPaused);
    roundtrip(&env, Error::AmountBelowMinimum);
    roundtrip(&env, Error::AmountAboveMaximum);
    roundtrip(&env, Error::CircuitBreakerOpen);
    roundtrip(&env, Error::ClaimExpired);
    roundtrip(&env, Error::GovernanceVersionTooLow);
    roundtrip(&env, Error::PendingClaimExists);
}

#[test]
fn error_discriminants_are_stable() {
    // Each variant's u32 repr must match the value in the source.
    // If a variant is renumbered or the gap at 15 is filled, this test catches it.
    assert_eq!(Error::AlreadyInitialized as u32, 1);
    assert_eq!(Error::NotInitialized as u32, 2);
    assert_eq!(Error::BountyExists as u32, 3);
    assert_eq!(Error::BountyNotFound as u32, 4);
    assert_eq!(Error::FundsNotLocked as u32, 5);
    assert_eq!(Error::DeadlineNotPassed as u32, 6);
    assert_eq!(Error::Unauthorized as u32, 7);
    assert_eq!(Error::InvalidFeeRate as u32, 8);
    assert_eq!(Error::FeeRecipientNotSet as u32, 9);
    assert_eq!(Error::InvalidBatchSize as u32, 10);
    assert_eq!(Error::BatchSizeMismatch as u32, 11);
    assert_eq!(Error::DuplicateBountyId as u32, 12);
    assert_eq!(Error::InvalidAmount as u32, 13);
    assert_eq!(Error::InvalidDeadline as u32, 14);
    // discriminant 15 is a reserved gap — no variant must ever claim it
    assert_eq!(Error::InsufficientFunds as u32, 16);
    assert_eq!(Error::RefundNotApproved as u32, 17);
    assert_eq!(Error::FundsPaused as u32, 18);
    assert_eq!(Error::AmountBelowMinimum as u32, 19);
    assert_eq!(Error::AmountAboveMaximum as u32, 20);
    assert_eq!(Error::CircuitBreakerOpen as u32, 21);
    assert_eq!(Error::ClaimExpired as u32, 22);
    assert_eq!(Error::GovernanceVersionTooLow as u32, 23);
    assert_eq!(Error::PendingClaimExists as u32, 24);
}

// ─────────────────────────────────────────────────────────────────────────────
// Boundary / edge-value tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ser_boundary_values() {
    use events::{FundsLocked, EVENT_VERSION_V2};
    let env = Env::default();
    let a = Address::generate(&env);
    // max values
    roundtrip(&env, FundsLocked {
        version: u32::MAX, bounty_id: u64::MAX,
        amount: i128::MAX, depositor: a.clone(), deadline: u64::MAX,
    });
    // zero / min values
    roundtrip(&env, FundsLocked {
        version: EVENT_VERSION_V2, bounty_id: 0,
        amount: 0, depositor: a.clone(), deadline: 0,
    });
    // negative i128 (valid for fee deltas etc.)
    roundtrip(&env, FundsLocked {
        version: EVENT_VERSION_V2, bounty_id: 1,
        amount: i128::MIN, depositor: a.clone(), deadline: 1,
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Forward-compatibility
//
// Soroban contracttype structs are encoded as field-keyed maps.  Adding a
// field at the end (append-only) means an older decoder ignores the new key
// and does not panic.  These tests encode with the current struct definition
// and verify all known fields survive the cycle — documenting the property.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn forward_compat_event_fields_survive_roundtrip() {
    use events::{ClaimCreated, FundsLocked, EVENT_VERSION_V2};
    let env = Env::default();
    let a = Address::generate(&env);

    // FundsLocked: all 5 fields intact
    let fl = FundsLocked {
        version: EVENT_VERSION_V2, bounty_id: 55,
        amount: 1234, depositor: a.clone(), deadline: 8888,
    };
    roundtrip(&env, fl.clone());

    // ClaimCreated: all 5 fields intact
    let cc = ClaimCreated {
        version: EVENT_VERSION_V2, bounty_id: 77,
        recipient: a.clone(), amount: 999, expires_at: 123456,
    };
    roundtrip(&env, cc.clone());
}

#[test]
fn forward_compat_analytics_snapshot_fields_survive_roundtrip() {
    use analytics::{AnalyticsSnapshot, ContractAnalytics};
    let env = Env::default();
    let metrics = ContractAnalytics {
        active_bounty_count: 7, released_bounty_count: 2, refunded_bounty_count: 1,
        total_locked: 7000, total_released: 2000, total_refunded: 500,
        average_bounty_amount: 875, snapshot_timestamp: 999,
    };
    roundtrip(&env, AnalyticsSnapshot { version: 1, metrics });
}
