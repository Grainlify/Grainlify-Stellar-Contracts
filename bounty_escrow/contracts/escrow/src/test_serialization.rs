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
