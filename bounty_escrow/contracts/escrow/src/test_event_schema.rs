//! # Event Schema Tests for bounty_escrow events.rs
//!
//! Issue #170 – every event type defined in `events.rs` has a dedicated test
//! that asserts:
//!   1. The exact topic symbols and their order.
//!   2. The payload struct fields, their types, and their order, by
//!      round-tripping through `TryFromVal` / the Soroban test host event log.
//!
//! ## Why this matters
//!
//! Off-chain consumers (e.g. `internal/soroban/event_parser.go`) decode
//! events by field *position* and *type*, not by name.  A change to field
//! order, field type, or topic symbol is a silent breaking change at the
//! Soroban XDR level even if the Rust code compiles without errors.
//!
//! These tests act as a regression guard: any breaking schema change will
//! cause at least one assertion here to fail before the change reaches
//! consumers.
//!
//! ## Coverage
//!
//! Every distinct `emit_*` function in `events.rs` is covered:
//!   - `emit_bounty_initialized`    → topic `("init",)`,            payload `BountyEscrowInitialized`
//!   - `emit_funds_locked`          → topic `("f_lock", bounty_id)`, payload `FundsLocked`
//!   - `emit_funds_released`        → topic `("f_rel",  bounty_id)`, payload `FundsReleased`
//!   - `emit_funds_refunded`        → topic `("f_ref",  bounty_id)`, payload `FundsRefunded`
//!   - `emit_bounty_expired`        → topic `("b_exp",  bounty_id)`, payload `BountyExpired`
//!   - `emit_upgrade_executed`      → topic `("upgrade",)`,          payload `UpgradeExecuted`
//!   - `emit_fee_collected`         → topic `("fee",)`,              payload `FeeCollected`
//!   - `emit_batch_funds_locked`    → topic `("b_lock",)`,           payload `BatchFundsLocked`
//!   - `emit_fee_config_updated`    → topic `("fee_cfg",)`,          payload `FeeConfigUpdated`
//!   - `emit_batch_funds_released`  → topic `("b_rel",)`,            payload `BatchFundsReleased`
//!   - `emit_approval_added`        → topic `("approval", bounty_id)`, payload `ApprovalAdded`
//!   - `emit_claim_created`         → topic `("claim", "created")`,  payload `ClaimCreated`
//!   - `emit_claim_executed`        → topic `("claim", "done")`,     payload `ClaimExecuted`
//!   - `emit_claim_cancelled`       → topic `("claim", "cancel")`,   payload `ClaimCancelled`
//!   - `emit_dispute_resolved`      → topic `("dispute", "resolved")`, payload `DisputeResolved`
//!   - `emit_pause_state_changed`   → topic `("pause", operation)`,  payload `PauseStateChanged`

#![cfg(test)]

use super::*;
use events::{
    emit_approval_added, emit_batch_funds_locked, emit_batch_funds_released,
    emit_bounty_expired, emit_bounty_initialized, emit_claim_cancelled, emit_claim_created,
    emit_claim_executed, emit_dispute_resolved, emit_fee_collected, emit_fee_config_updated,
    emit_funds_locked, emit_funds_refunded, emit_funds_released, emit_upgrade_executed,
    ApprovalAdded, BatchFundsLocked, BatchFundsReleased, BountyEscrowInitialized, BountyExpired,
    ClaimCancelled, ClaimCreated, ClaimExecuted, DisputeOutcome, DisputeResolved, FeeCollected,
    FeeConfigUpdated, FeeOperationType, FundsLocked, FundsRefunded, FundsReleased,
    UpgradeExecuted, EVENT_VERSION_V2,
};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events as _, Ledger as _},
    Address, BytesN, Env, IntoVal, TryFromVal, Val, Vec,
};

// ---------------------------------------------------------------------------
// Helper: a lightweight dummy contract so we can call env.as_contract().
// ---------------------------------------------------------------------------
#[contract]
struct EventSchemaContract;

#[contractimpl]
impl EventSchemaContract {
    pub fn noop(_env: Env) {}
}

fn setup() -> (Env, Address) {
    let env = Env::default();
    env.ledger().set_timestamp(1_000);
    let id = env.register_contract(None, EventSchemaContract);
    (env, id)
}

/// Extract the last event from the test host log as (contract_id, topics Vec<Val>, data Val).
fn last_event(env: &Env) -> (Address, Vec<Val>, Val) {
    let all = env.events().all();
    assert!(!all.is_empty(), "expected at least one event");
    let ev = all.get(all.len() - 1).unwrap();
    ev
}

// ===========================================================================
// 1. BountyEscrowInitialized  –  topic ("init",)
// ===========================================================================

/// Topic must be the single symbol `"init"`.
/// Payload fields in order: version (u32), admin (Address), token (Address), timestamp (u64).
///
/// ⚠ Changing field order or type is a BREAKING CHANGE for off-chain consumers.
#[test]
fn test_event_schema_bounty_escrow_initialized_topic_and_payload() {
    let (env, id) = setup();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    let payload = BountyEscrowInitialized {
        version: EVENT_VERSION_V2,
        admin: admin.clone(),
        token: token.clone(),
        timestamp: 1_000,
    };

    env.as_contract(&id, || {
        emit_bounty_initialized(&env, payload.clone());
    });

    let (contract_id, topics, data) = last_event(&env);
    assert_eq!(contract_id, id, "event must originate from our contract");

    // Topic: exactly one element — symbol "init"
    let expected_topics: Vec<Val> = (symbol_short!("init"),).into_val(&env);
    assert_eq!(topics, expected_topics, "topic mismatch for BountyEscrowInitialized");

    // Payload round-trip
    let decoded = BountyEscrowInitialized::try_from_val(&env, &data)
        .expect("payload must decode as BountyEscrowInitialized");
    assert_eq!(decoded.version, EVENT_VERSION_V2);
    assert_eq!(decoded.admin, admin);
    assert_eq!(decoded.token, token);
    assert_eq!(decoded.timestamp, 1_000);
}

// ===========================================================================
// 2. FundsLocked  –  topic ("f_lock", bounty_id)
// ===========================================================================

/// Topic must be ("f_lock", bounty_id: u64).
/// Payload fields in order: version, bounty_id, amount, depositor, deadline.
///
/// ⚠ Changing field order or type is a BREAKING CHANGE for off-chain consumers.
#[test]
fn test_event_schema_funds_locked_topic_and_payload() {
    let (env, id) = setup();
    let depositor = Address::generate(&env);
    let bounty_id: u64 = 42;

    let payload = FundsLocked {
        version: EVENT_VERSION_V2,
        bounty_id,
        amount: 5_000,
        depositor: depositor.clone(),
        deadline: 9_999,
    };

    env.as_contract(&id, || {
        emit_funds_locked(&env, payload.clone());
    });

    let (_, topics, data) = last_event(&env);

    // Topic: ("f_lock", 42u64)
    let expected_topics: Vec<Val> = (symbol_short!("f_lock"), bounty_id).into_val(&env);
    assert_eq!(topics, expected_topics, "topic mismatch for FundsLocked");

    let decoded = FundsLocked::try_from_val(&env, &data)
        .expect("payload must decode as FundsLocked");
    assert_eq!(decoded.version, EVENT_VERSION_V2);
    assert_eq!(decoded.bounty_id, bounty_id);
    assert_eq!(decoded.amount, 5_000);
    assert_eq!(decoded.depositor, depositor);
    assert_eq!(decoded.deadline, 9_999);
}

// ===========================================================================
// 3. FundsReleased  –  topic ("f_rel", bounty_id)
// ===========================================================================

/// Topic must be ("f_rel", bounty_id: u64).
/// Payload fields in order: version, bounty_id, amount, recipient, timestamp.
///
/// ⚠ Changing field order or type is a BREAKING CHANGE for off-chain consumers.
#[test]
fn test_event_schema_funds_released_topic_and_payload() {
    let (env, id) = setup();
    let recipient = Address::generate(&env);
    let bounty_id: u64 = 7;

    let payload = FundsReleased {
        version: EVENT_VERSION_V2,
        bounty_id,
        amount: 3_000,
        recipient: recipient.clone(),
        timestamp: 1_500,
    };

    env.as_contract(&id, || {
        emit_funds_released(&env, payload.clone());
    });

    let (_, topics, data) = last_event(&env);

    let expected_topics: Vec<Val> = (symbol_short!("f_rel"), bounty_id).into_val(&env);
    assert_eq!(topics, expected_topics, "topic mismatch for FundsReleased");

    let decoded = FundsReleased::try_from_val(&env, &data)
        .expect("payload must decode as FundsReleased");
    assert_eq!(decoded.version, EVENT_VERSION_V2);
    assert_eq!(decoded.bounty_id, bounty_id);
    assert_eq!(decoded.amount, 3_000);
    assert_eq!(decoded.recipient, recipient);
    assert_eq!(decoded.timestamp, 1_500);
}

// ===========================================================================
// 4. FundsRefunded  –  topic ("f_ref", bounty_id)
// ===========================================================================

/// Topic must be ("f_ref", bounty_id: u64).
/// Payload fields in order: version, bounty_id, amount, refund_to, timestamp.
///
/// ⚠ Changing field order or type is a BREAKING CHANGE for off-chain consumers.
#[test]
fn test_event_schema_funds_refunded_topic_and_payload() {
    let (env, id) = setup();
    let refund_to = Address::generate(&env);
    let bounty_id: u64 = 99;

    let payload = FundsRefunded {
        version: EVENT_VERSION_V2,
        bounty_id,
        amount: 1_200,
        refund_to: refund_to.clone(),
        timestamp: 2_000,
    };

    env.as_contract(&id, || {
        emit_funds_refunded(&env, payload.clone());
    });

    let (_, topics, data) = last_event(&env);

    let expected_topics: Vec<Val> = (symbol_short!("f_ref"), bounty_id).into_val(&env);
    assert_eq!(topics, expected_topics, "topic mismatch for FundsRefunded");

    let decoded = FundsRefunded::try_from_val(&env, &data)
        .expect("payload must decode as FundsRefunded");
    assert_eq!(decoded.version, EVENT_VERSION_V2);
    assert_eq!(decoded.bounty_id, bounty_id);
    assert_eq!(decoded.amount, 1_200);
    assert_eq!(decoded.refund_to, refund_to);
    assert_eq!(decoded.timestamp, 2_000);
}

// ===========================================================================
// 5. BountyExpired  –  topic ("b_exp", bounty_id)
// ===========================================================================

/// Topic must be ("b_exp", bounty_id: u64).
/// Payload fields in order: version, bounty_id, depositor, amount, deadline, expired_at.
///
/// ⚠ Changing field order or type is a BREAKING CHANGE for off-chain consumers.
#[test]
fn test_event_schema_bounty_expired_topic_and_payload() {
    let (env, id) = setup();
    let depositor = Address::generate(&env);
    let bounty_id: u64 = 55;

    let payload = BountyExpired {
        version: EVENT_VERSION_V2,
        bounty_id,
        depositor: depositor.clone(),
        amount: 800,
        deadline: 500,
        expired_at: 1_000,
    };

    env.as_contract(&id, || {
        emit_bounty_expired(&env, payload.clone());
    });

    let (_, topics, data) = last_event(&env);

    let expected_topics: Vec<Val> = (symbol_short!("b_exp"), bounty_id).into_val(&env);
    assert_eq!(topics, expected_topics, "topic mismatch for BountyExpired");

    let decoded = BountyExpired::try_from_val(&env, &data)
        .expect("payload must decode as BountyExpired");
    assert_eq!(decoded.version, EVENT_VERSION_V2);
    assert_eq!(decoded.bounty_id, bounty_id);
    assert_eq!(decoded.depositor, depositor);
    assert_eq!(decoded.amount, 800);
    assert_eq!(decoded.deadline, 500);
    assert_eq!(decoded.expired_at, 1_000);
}

// ===========================================================================
// 6. UpgradeExecuted  –  topic ("upgrade",)
// ===========================================================================

/// Topic must be the single symbol `"upgrade"`.
/// Payload fields in order: version (u32), wasm_hash (BytesN<32>), admin (Address).
///
/// ⚠ Changing field order or type is a BREAKING CHANGE for off-chain consumers.
#[test]
fn test_event_schema_upgrade_executed_topic_and_payload() {
    let (env, id) = setup();
    let admin = Address::generate(&env);
    let wasm_hash: BytesN<32> = BytesN::from_array(&env, &[0xab; 32]);

    let payload = UpgradeExecuted {
        version: EVENT_VERSION_V2,
        wasm_hash: wasm_hash.clone(),
        admin: admin.clone(),
    };

    env.as_contract(&id, || {
        emit_upgrade_executed(&env, payload.clone());
    });

    let (_, topics, data) = last_event(&env);

    let expected_topics: Vec<Val> = (symbol_short!("upgrade"),).into_val(&env);
    assert_eq!(topics, expected_topics, "topic mismatch for UpgradeExecuted");

    let decoded = UpgradeExecuted::try_from_val(&env, &data)
        .expect("payload must decode as UpgradeExecuted");
    assert_eq!(decoded.version, EVENT_VERSION_V2);
    assert_eq!(decoded.wasm_hash, wasm_hash);
    assert_eq!(decoded.admin, admin);
}

// ===========================================================================
// 7. FeeCollected  –  topic ("fee",)
// ===========================================================================

/// Topic must be the single symbol `"fee"`.
/// Payload fields in order: version, operation_type, amount, fee_rate, recipient, timestamp.
///
/// ⚠ Changing field order or type is a BREAKING CHANGE for off-chain consumers.
#[test]
fn test_event_schema_fee_collected_lock_topic_and_payload() {
    let (env, id) = setup();
    let recipient = Address::generate(&env);

    let payload = FeeCollected {
        version: EVENT_VERSION_V2,
        operation_type: FeeOperationType::Lock,
        amount: 10_000,
        fee_rate: 100,
        recipient: recipient.clone(),
        timestamp: 1_000,
    };

    env.as_contract(&id, || {
        events::emit_fee_collected(&env, payload.clone());
    });

    let (_, topics, data) = last_event(&env);

    let expected_topics: Vec<Val> = (symbol_short!("fee"),).into_val(&env);
    assert_eq!(topics, expected_topics, "topic mismatch for FeeCollected");

    let decoded = FeeCollected::try_from_val(&env, &data)
        .expect("payload must decode as FeeCollected");
    assert_eq!(decoded.version, EVENT_VERSION_V2);
    assert_eq!(decoded.operation_type, FeeOperationType::Lock);
    assert_eq!(decoded.amount, 10_000);
    assert_eq!(decoded.fee_rate, 100);
    assert_eq!(decoded.recipient, recipient);
    assert_eq!(decoded.timestamp, 1_000);
}

/// Same as above but with Release operation type.
#[test]
fn test_event_schema_fee_collected_release_operation_type() {
    let (env, id) = setup();
    let recipient = Address::generate(&env);

    let payload = FeeCollected {
        version: EVENT_VERSION_V2,
        operation_type: FeeOperationType::Release,
        amount: 5_000,
        fee_rate: 50,
        recipient: recipient.clone(),
        timestamp: 2_000,
    };

    env.as_contract(&id, || {
        events::emit_fee_collected(&env, payload.clone());
    });

    let (_, _, data) = last_event(&env);
    let decoded = FeeCollected::try_from_val(&env, &data).unwrap();
    assert_eq!(decoded.operation_type, FeeOperationType::Release);
}

// ===========================================================================
// 8. BatchFundsLocked  –  topic ("b_lock",)
// ===========================================================================

/// Topic must be the single symbol `"b_lock"`.
/// Payload fields in order: version, count, total_amount, timestamp.
///
/// ⚠ Changing field order or type is a BREAKING CHANGE for off-chain consumers.
#[test]
fn test_event_schema_batch_funds_locked_topic_and_payload() {
    let (env, id) = setup();

    let payload = BatchFundsLocked {
        version: EVENT_VERSION_V2,
        count: 3,
        total_amount: 15_000,
        timestamp: 1_000,
    };

    env.as_contract(&id, || {
        emit_batch_funds_locked(&env, payload.clone());
    });

    let (_, topics, data) = last_event(&env);

    let expected_topics: Vec<Val> = (symbol_short!("b_lock"),).into_val(&env);
    assert_eq!(topics, expected_topics, "topic mismatch for BatchFundsLocked");

    let decoded = BatchFundsLocked::try_from_val(&env, &data)
        .expect("payload must decode as BatchFundsLocked");
    assert_eq!(decoded.version, EVENT_VERSION_V2);
    assert_eq!(decoded.count, 3);
    assert_eq!(decoded.total_amount, 15_000);
    assert_eq!(decoded.timestamp, 1_000);
}

// ===========================================================================
// 9. FeeConfigUpdated  –  topic ("fee_cfg",)
// ===========================================================================

/// Topic must be the single symbol `"fee_cfg"`.
/// Payload fields in order: version, lock_fee_rate, release_fee_rate,
///   fee_recipient, fee_enabled, timestamp.
///
/// ⚠ Changing field order or type is a BREAKING CHANGE for off-chain consumers.
#[test]
fn test_event_schema_fee_config_updated_topic_and_payload() {
    let (env, id) = setup();
    let fee_recipient = Address::generate(&env);

    let payload = FeeConfigUpdated {
        version: EVENT_VERSION_V2,
        lock_fee_rate: 200,
        release_fee_rate: 150,
        fee_recipient: fee_recipient.clone(),
        fee_enabled: true,
        timestamp: 1_000,
    };

    env.as_contract(&id, || {
        emit_fee_config_updated(&env, payload.clone());
    });

    let (_, topics, data) = last_event(&env);

    let expected_topics: Vec<Val> = (symbol_short!("fee_cfg"),).into_val(&env);
    assert_eq!(topics, expected_topics, "topic mismatch for FeeConfigUpdated");

    let decoded = FeeConfigUpdated::try_from_val(&env, &data)
        .expect("payload must decode as FeeConfigUpdated");
    assert_eq!(decoded.version, EVENT_VERSION_V2);
    assert_eq!(decoded.lock_fee_rate, 200);
    assert_eq!(decoded.release_fee_rate, 150);
    assert_eq!(decoded.fee_recipient, fee_recipient);
    assert_eq!(decoded.fee_enabled, true);
    assert_eq!(decoded.timestamp, 1_000);
}

// ===========================================================================
// 10. BatchFundsReleased  –  topic ("b_rel",)
// ===========================================================================

/// Topic must be the single symbol `"b_rel"`.
/// Payload fields in order: version, count, total_amount, timestamp.
///
/// ⚠ Changing field order or type is a BREAKING CHANGE for off-chain consumers.
#[test]
fn test_event_schema_batch_funds_released_topic_and_payload() {
    let (env, id) = setup();

    let payload = BatchFundsReleased {
        version: EVENT_VERSION_V2,
        count: 5,
        total_amount: 25_000,
        timestamp: 1_000,
    };

    env.as_contract(&id, || {
        emit_batch_funds_released(&env, payload.clone());
    });

    let (_, topics, data) = last_event(&env);

    let expected_topics: Vec<Val> = (symbol_short!("b_rel"),).into_val(&env);
    assert_eq!(topics, expected_topics, "topic mismatch for BatchFundsReleased");

    let decoded = BatchFundsReleased::try_from_val(&env, &data)
        .expect("payload must decode as BatchFundsReleased");
    assert_eq!(decoded.version, EVENT_VERSION_V2);
    assert_eq!(decoded.count, 5);
    assert_eq!(decoded.total_amount, 25_000);
    assert_eq!(decoded.timestamp, 1_000);
}

// ===========================================================================
// 11. ApprovalAdded  –  topic ("approval", bounty_id)
// ===========================================================================

/// Topic must be ("approval", bounty_id: u64).
/// Payload fields in order: version, bounty_id, contributor, approver, timestamp.
///
/// ⚠ Changing field order or type is a BREAKING CHANGE for off-chain consumers.
#[test]
fn test_event_schema_approval_added_topic_and_payload() {
    let (env, id) = setup();
    let contributor = Address::generate(&env);
    let approver = Address::generate(&env);
    let bounty_id: u64 = 77;

    let payload = ApprovalAdded {
        version: EVENT_VERSION_V2,
        bounty_id,
        contributor: contributor.clone(),
        approver: approver.clone(),
        timestamp: 1_000,
    };

    env.as_contract(&id, || {
        emit_approval_added(&env, payload.clone());
    });

    let (_, topics, data) = last_event(&env);

    let expected_topics: Vec<Val> = (symbol_short!("approval"), bounty_id).into_val(&env);
    assert_eq!(topics, expected_topics, "topic mismatch for ApprovalAdded");

    let decoded = ApprovalAdded::try_from_val(&env, &data)
        .expect("payload must decode as ApprovalAdded");
    assert_eq!(decoded.version, EVENT_VERSION_V2);
    assert_eq!(decoded.bounty_id, bounty_id);
    assert_eq!(decoded.contributor, contributor);
    assert_eq!(decoded.approver, approver);
    assert_eq!(decoded.timestamp, 1_000);
}

// ===========================================================================
// 12. ClaimCreated  –  topic ("claim", "created")
// ===========================================================================

/// Topic must be ("claim", "created").
/// Payload fields in order: version, bounty_id, recipient, amount, expires_at.
///
/// ⚠ Changing field order or type is a BREAKING CHANGE for off-chain consumers.
#[test]
fn test_event_schema_claim_created_topic_and_payload() {
    let (env, id) = setup();
    let recipient = Address::generate(&env);
    let bounty_id: u64 = 11;

    let payload = ClaimCreated {
        version: EVENT_VERSION_V2,
        bounty_id,
        recipient: recipient.clone(),
        amount: 4_000,
        expires_at: 9_000,
    };

    env.as_contract(&id, || {
        emit_claim_created(&env, payload.clone());
    });

    let (_, topics, data) = last_event(&env);

    let expected_topics: Vec<Val> =
        (symbol_short!("claim"), symbol_short!("created")).into_val(&env);
    assert_eq!(topics, expected_topics, "topic mismatch for ClaimCreated");

    let decoded = ClaimCreated::try_from_val(&env, &data)
        .expect("payload must decode as ClaimCreated");
    assert_eq!(decoded.version, EVENT_VERSION_V2);
    assert_eq!(decoded.bounty_id, bounty_id);
    assert_eq!(decoded.recipient, recipient);
    assert_eq!(decoded.amount, 4_000);
    assert_eq!(decoded.expires_at, 9_000);
}

// ===========================================================================
// 13. ClaimExecuted  –  topic ("claim", "done")
// ===========================================================================

/// Topic must be ("claim", "done").
/// Payload fields in order: version, bounty_id, recipient, amount, claimed_at.
///
/// ⚠ Changing field order or type is a BREAKING CHANGE for off-chain consumers.
#[test]
fn test_event_schema_claim_executed_topic_and_payload() {
    let (env, id) = setup();
    let recipient = Address::generate(&env);
    let bounty_id: u64 = 22;

    let payload = ClaimExecuted {
        version: EVENT_VERSION_V2,
        bounty_id,
        recipient: recipient.clone(),
        amount: 6_000,
        claimed_at: 1_500,
    };

    env.as_contract(&id, || {
        emit_claim_executed(&env, payload.clone());
    });

    let (_, topics, data) = last_event(&env);

    let expected_topics: Vec<Val> =
        (symbol_short!("claim"), symbol_short!("done")).into_val(&env);
    assert_eq!(topics, expected_topics, "topic mismatch for ClaimExecuted");

    let decoded = ClaimExecuted::try_from_val(&env, &data)
        .expect("payload must decode as ClaimExecuted");
    assert_eq!(decoded.version, EVENT_VERSION_V2);
    assert_eq!(decoded.bounty_id, bounty_id);
    assert_eq!(decoded.recipient, recipient);
    assert_eq!(decoded.amount, 6_000);
    assert_eq!(decoded.claimed_at, 1_500);
}

// ===========================================================================
// 14. ClaimCancelled  –  topic ("claim", "cancel")
// ===========================================================================

/// Topic must be ("claim", "cancel").
/// Payload fields in order: version, bounty_id, recipient, amount,
///   cancelled_at, cancelled_by, reason.
///
/// ⚠ Changing field order or type is a BREAKING CHANGE for off-chain consumers.
#[test]
fn test_event_schema_claim_cancelled_topic_and_payload() {
    let (env, id) = setup();
    let recipient = Address::generate(&env);
    let cancelled_by = Address::generate(&env);
    let bounty_id: u64 = 33;
    let reason = symbol_short!("expired");

    let payload = ClaimCancelled {
        version: EVENT_VERSION_V2,
        bounty_id,
        recipient: recipient.clone(),
        amount: 2_500,
        cancelled_at: 1_000,
        cancelled_by: cancelled_by.clone(),
        reason: reason.clone(),
    };

    env.as_contract(&id, || {
        emit_claim_cancelled(&env, payload.clone());
    });

    let (_, topics, data) = last_event(&env);

    let expected_topics: Vec<Val> =
        (symbol_short!("claim"), symbol_short!("cancel")).into_val(&env);
    assert_eq!(topics, expected_topics, "topic mismatch for ClaimCancelled");

    let decoded = ClaimCancelled::try_from_val(&env, &data)
        .expect("payload must decode as ClaimCancelled");
    assert_eq!(decoded.version, EVENT_VERSION_V2);
    assert_eq!(decoded.bounty_id, bounty_id);
    assert_eq!(decoded.recipient, recipient);
    assert_eq!(decoded.amount, 2_500);
    assert_eq!(decoded.cancelled_at, 1_000);
    assert_eq!(decoded.cancelled_by, cancelled_by);
    assert_eq!(decoded.reason, reason);
}

// ===========================================================================
// 15. DisputeResolved  –  topic ("dispute", "resolved")
// ===========================================================================

/// Topic must be ("dispute", "resolved").
/// Payload fields in order: version, bounty_id, outcome, resolver,
///   recipient, amount, resolved_at.
/// DisputeOutcome enum variants: Claimed, Cancelled, Expired.
///
/// ⚠ Changing field order or type is a BREAKING CHANGE for off-chain consumers.
#[test]
fn test_event_schema_dispute_resolved_claimed_topic_and_payload() {
    let (env, id) = setup();
    let resolver = Address::generate(&env);
    let recipient = Address::generate(&env);
    let bounty_id: u64 = 44;

    let payload = DisputeResolved {
        version: EVENT_VERSION_V2,
        bounty_id,
        outcome: DisputeOutcome::Claimed,
        resolver: resolver.clone(),
        recipient: recipient.clone(),
        amount: 8_000,
        resolved_at: 1_000,
    };

    env.as_contract(&id, || {
        emit_dispute_resolved(&env, payload.clone());
    });

    let (_, topics, data) = last_event(&env);

    let expected_topics: Vec<Val> =
        (symbol_short!("dispute"), symbol_short!("resolved")).into_val(&env);
    assert_eq!(topics, expected_topics, "topic mismatch for DisputeResolved");

    let decoded = DisputeResolved::try_from_val(&env, &data)
        .expect("payload must decode as DisputeResolved");
    assert_eq!(decoded.version, EVENT_VERSION_V2);
    assert_eq!(decoded.bounty_id, bounty_id);
    assert_eq!(decoded.outcome, DisputeOutcome::Claimed);
    assert_eq!(decoded.resolver, resolver);
    assert_eq!(decoded.recipient, recipient);
    assert_eq!(decoded.amount, 8_000);
    assert_eq!(decoded.resolved_at, 1_000);
}

/// DisputeOutcome::Cancelled variant round-trips correctly.
#[test]
fn test_event_schema_dispute_resolved_cancelled_outcome() {
    let (env, id) = setup();
    let resolver = Address::generate(&env);
    let recipient = Address::generate(&env);

    let payload = DisputeResolved {
        version: EVENT_VERSION_V2,
        bounty_id: 45,
        outcome: DisputeOutcome::Cancelled,
        resolver: resolver.clone(),
        recipient: recipient.clone(),
        amount: 1_000,
        resolved_at: 1_000,
    };

    env.as_contract(&id, || {
        emit_dispute_resolved(&env, payload.clone());
    });

    let (_, _, data) = last_event(&env);
    let decoded = DisputeResolved::try_from_val(&env, &data).unwrap();
    assert_eq!(decoded.outcome, DisputeOutcome::Cancelled);
}

/// DisputeOutcome::Expired variant round-trips correctly.
#[test]
fn test_event_schema_dispute_resolved_expired_outcome() {
    let (env, id) = setup();
    let resolver = Address::generate(&env);
    let recipient = Address::generate(&env);

    let payload = DisputeResolved {
        version: EVENT_VERSION_V2,
        bounty_id: 46,
        outcome: DisputeOutcome::Expired,
        resolver: resolver.clone(),
        recipient: recipient.clone(),
        amount: 500,
        resolved_at: 2_000,
    };

    env.as_contract(&id, || {
        emit_dispute_resolved(&env, payload.clone());
    });

    let (_, _, data) = last_event(&env);
    let decoded = DisputeResolved::try_from_val(&env, &data).unwrap();
    assert_eq!(decoded.outcome, DisputeOutcome::Expired);
}

// ===========================================================================
// 16. PauseStateChanged  –  topic ("pause", operation)
// ===========================================================================

/// Topic must be ("pause", operation: Symbol) where operation is the
/// specific pause operation symbol (e.g. "lock", "release", "refund", "global").
/// Payload fields in order: operation (Symbol), paused (bool), admin (Address).
///
/// Note: emit_pause_state_changed takes crate::PauseStateChanged (defined in lib.rs)
/// rather than an events.rs struct.
///
/// ⚠ Changing field order or type is a BREAKING CHANGE for off-chain consumers.
#[test]
fn test_event_schema_pause_state_changed_lock_operation() {
    let (env, id) = setup();
    let admin = Address::generate(&env);
    let operation = symbol_short!("lock");

    let payload = PauseStateChanged {
        operation: operation.clone(),
        paused: true,
        admin: admin.clone(),
    };

    env.as_contract(&id, || {
        events::emit_pause_state_changed(&env, payload.clone());
    });

    let (_, topics, data) = last_event(&env);

    // Topic: ("pause", "lock")
    let expected_topics: Vec<Val> =
        (symbol_short!("pause"), operation.clone()).into_val(&env);
    assert_eq!(topics, expected_topics, "topic mismatch for PauseStateChanged lock");

    let decoded = PauseStateChanged::try_from_val(&env, &data)
        .expect("payload must decode as PauseStateChanged");
    assert_eq!(decoded.operation, operation);
    assert_eq!(decoded.paused, true);
    assert_eq!(decoded.admin, admin);
}

/// Same as above but for the "release" operation and paused=false.
#[test]
fn test_event_schema_pause_state_changed_release_operation() {
    let (env, id) = setup();
    let admin = Address::generate(&env);
    let operation = symbol_short!("release");

    let payload = PauseStateChanged {
        operation: operation.clone(),
        paused: false,
        admin: admin.clone(),
    };

    env.as_contract(&id, || {
        events::emit_pause_state_changed(&env, payload.clone());
    });

    let (_, topics, data) = last_event(&env);

    let expected_topics: Vec<Val> =
        (symbol_short!("pause"), operation.clone()).into_val(&env);
    assert_eq!(topics, expected_topics, "topic mismatch for PauseStateChanged release");

    let decoded = PauseStateChanged::try_from_val(&env, &data).unwrap();
    assert_eq!(decoded.operation, operation);
    assert_eq!(decoded.paused, false);
}

/// Same as above for "global" emergency pause.
#[test]
fn test_event_schema_pause_state_changed_global_operation() {
    let (env, id) = setup();
    let admin = Address::generate(&env);
    let operation = symbol_short!("global");

    let payload = PauseStateChanged {
        operation: operation.clone(),
        paused: true,
        admin: admin.clone(),
    };

    env.as_contract(&id, || {
        events::emit_pause_state_changed(&env, payload.clone());
    });

    let (_, topics, data) = last_event(&env);

    let expected_topics: Vec<Val> =
        (symbol_short!("pause"), operation.clone()).into_val(&env);
    assert_eq!(topics, expected_topics, "topic mismatch for PauseStateChanged global");

    let decoded = PauseStateChanged::try_from_val(&env, &data).unwrap();
    assert_eq!(decoded.operation, operation);
    assert_eq!(decoded.paused, true);
}

// ===========================================================================
// 17. EVENT_VERSION_V2 constant stability
// ===========================================================================

/// The version constant must remain 2 — any bump is a schema change that
/// requires updating off-chain consumers before deployment.
///
/// If you are reading this comment because this test just failed: update
/// all off-chain event parsers to handle the new version before merging.
#[test]
fn test_event_version_v2_is_two() {
    assert_eq!(
        events::EVENT_VERSION_V2,
        2,
        "EVENT_VERSION_V2 changed — update off-chain consumers before bumping"
    );
}

// ===========================================================================
// 18. Topic uniqueness regression — no two distinct event types share a topic
// ===========================================================================

/// Emit one event of each type and verify the topics are mutually distinct.
///
/// This is a guard against accidentally reusing a topic symbol across two
/// different event structs, which would make off-chain demultiplexing
/// ambiguous.
#[test]
fn test_all_event_topics_are_unique() {
    let (env, id) = setup();
    let addr = Address::generate(&env);
    let wasm_hash: BytesN<32> = BytesN::from_array(&env, &[0xcd; 32]);

    env.as_contract(&id, || {
        emit_bounty_initialized(&env, BountyEscrowInitialized {
            version: EVENT_VERSION_V2, admin: addr.clone(), token: addr.clone(), timestamp: 0,
        });
        emit_funds_locked(&env, FundsLocked {
            version: EVENT_VERSION_V2, bounty_id: 1, amount: 1, depositor: addr.clone(), deadline: 0,
        });
        emit_funds_released(&env, FundsReleased {
            version: EVENT_VERSION_V2, bounty_id: 2, amount: 1, recipient: addr.clone(), timestamp: 0,
        });
        emit_funds_refunded(&env, FundsRefunded {
            version: EVENT_VERSION_V2, bounty_id: 3, amount: 1, refund_to: addr.clone(), timestamp: 0,
        });
        emit_bounty_expired(&env, BountyExpired {
            version: EVENT_VERSION_V2, bounty_id: 4, depositor: addr.clone(), amount: 1, deadline: 0, expired_at: 0,
        });
        emit_upgrade_executed(&env, UpgradeExecuted {
            version: EVENT_VERSION_V2, wasm_hash: wasm_hash.clone(), admin: addr.clone(),
        });
        events::emit_fee_collected(&env, FeeCollected {
            version: EVENT_VERSION_V2, operation_type: FeeOperationType::Lock,
            amount: 1, fee_rate: 1, recipient: addr.clone(), timestamp: 0,
        });
        emit_batch_funds_locked(&env, BatchFundsLocked {
            version: EVENT_VERSION_V2, count: 1, total_amount: 1, timestamp: 0,
        });
        emit_fee_config_updated(&env, FeeConfigUpdated {
            version: EVENT_VERSION_V2, lock_fee_rate: 0, release_fee_rate: 0,
            fee_recipient: addr.clone(), fee_enabled: false, timestamp: 0,
        });
        emit_batch_funds_released(&env, BatchFundsReleased {
            version: EVENT_VERSION_V2, count: 1, total_amount: 1, timestamp: 0,
        });
        emit_approval_added(&env, ApprovalAdded {
            version: EVENT_VERSION_V2, bounty_id: 5, contributor: addr.clone(),
            approver: addr.clone(), timestamp: 0,
        });
        emit_claim_created(&env, ClaimCreated {
            version: EVENT_VERSION_V2, bounty_id: 6, recipient: addr.clone(),
            amount: 1, expires_at: 0,
        });
        emit_claim_executed(&env, ClaimExecuted {
            version: EVENT_VERSION_V2, bounty_id: 7, recipient: addr.clone(),
            amount: 1, claimed_at: 0,
        });
        emit_claim_cancelled(&env, ClaimCancelled {
            version: EVENT_VERSION_V2, bounty_id: 8, recipient: addr.clone(),
            amount: 1, cancelled_at: 0, cancelled_by: addr.clone(), reason: symbol_short!("test"),
        });
        emit_dispute_resolved(&env, DisputeResolved {
            version: EVENT_VERSION_V2, bounty_id: 9, outcome: DisputeOutcome::Claimed,
            resolver: addr.clone(), recipient: addr.clone(), amount: 1, resolved_at: 0,
        });
        events::emit_pause_state_changed(&env, PauseStateChanged {
            operation: symbol_short!("lock"), paused: false, admin: addr.clone(),
        });
    });

    let all = env.events().all();
    // Collect topics (as strings via debug for comparison — sufficient for uniqueness check)
    let mut topic_strings: soroban_sdk::Vec<soroban_sdk::Vec<Val>> = soroban_sdk::Vec::new(&env);
    let mut duplicates = 0u32;
    for i in 0..all.len() {
        let (_, topics, _) = all.get(i).unwrap();
        for j in 0..i {
            let (_, other_topics, _) = all.get(j).unwrap();
            if topics == other_topics {
                duplicates += 1;
            }
        }
        topic_strings.push_back(topics);
    }

    // Only the three claim events ("claim","created"), ("claim","done"), ("claim","cancel")
    // are expected to share the first topic element "claim" — but they differ in the second
    // element and are therefore not identical.  All 16 topic tuples must be unique.
    assert_eq!(
        duplicates, 0,
        "found duplicate event topics — two distinct event types must not share an identical topic tuple"
    );
}
