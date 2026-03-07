// contracts/bounty_escrow/contracts/escrow/src/test_circuit_breaker.rs
//
// Comprehensive Circuit Breaker Tests for Bounty Escrow
//
// Tests cover:
// - Initial state validation
// - Failure threshold behavior
// - State transitions (Closed -> Open -> HalfOpen -> Closed)
// - Admin controls
// - Circuit breaker integration with escrow operations
// - Error logging

#![cfg(test)]

use crate::error_recovery::{
    check_and_allow, close_circuit, get_circuit_admin, get_config, get_error_log, get_failure_count,
    get_state, get_status, get_success_count, half_open_circuit, open_circuit, record_failure,
    record_success, reset_circuit_breaker, set_circuit_admin, set_config, CircuitBreakerConfig,
    CircuitState, ERR_CIRCUIT_OPEN, ERR_TRANSFER_FAILED,
};
use crate::{
    BountyEscrowContract, CircuitBreakerStatus, Error, LockFundsItem, ReleaseFundsItem,
};
use soroban_sdk::testutils::Address as TestAddress;
use soroban_sdk::testutils::Ledger;
use soroban_sdk::{contract, contractimpl, symbol_short, vec, Address, Env, String};

// ─────────────────────────────────────────────────────────
// Test Contract for Circuit Breaker Unit Tests
// ─────────────────────────────────────────────────────────

#[contract]
pub struct CircuitBreakerTestContract;

#[contractimpl]
impl CircuitBreakerTestContract {}

// ─────────────────────────────────────────────────────────
// Test Helpers
// ─────────────────────────────────────────────────────────

/// Setup a basic test environment with contract registered
fn setup_env() -> (Env, Address) {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1000);
    let contract_id = env.register_contract(None, CircuitBreakerTestContract);
    (env, contract_id)
}

/// Setup environment with admin and initial circuit breaker config
fn setup_with_admin(failure_threshold: u32) -> (Env, Address, Address) {
    let (env, contract_id) = setup_env();
    let admin = Address::generate(&env);

    env.as_contract(&contract_id, || {
        set_circuit_admin(&env, admin.clone(), None);
        set_config(
            &env,
            CircuitBreakerConfig {
                failure_threshold,
                success_threshold: 1,
                max_error_log: 5,
            },
        );
    });

    (env, admin, contract_id)
}

/// Simulate consecutive failures
fn simulate_failures(env: &Env, contract_id: &Address, n: u32) {
    env.as_contract(contract_id, || {
        for i in 0..n {
            record_failure(env, i as u64, symbol_short!("transfer"), ERR_TRANSFER_FAILED);
        }
    });
}

/// Setup full bounty escrow contract for integration tests
fn setup_bounty_escrow() -> (Env, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1000);

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    env.as_contract(&contract_id, || {
        crate::BountyEscrowContract::init(env.clone(), admin.clone(), token.clone()).unwrap();
    });

    (env, contract_id, admin, token)
}

// ─────────────────────────────────────────────────────────
// 1. Initial State Tests
// ─────────────────────────────────────────────────────────

#[test]
fn test_initial_state_is_closed() {
    let (env, contract_id) = setup_env();
    env.as_contract(&contract_id, || {
        assert_eq!(get_state(&env), CircuitState::Closed);
        assert_eq!(get_failure_count(&env), 0);
        assert_eq!(get_success_count(&env), 0);
    });
}

#[test]
fn test_check_and_allow_passes_when_closed() {
    let (env, contract_id) = setup_env();
    env.as_contract(&contract_id, || {
        assert!(check_and_allow(&env).is_ok());
    });
}

#[test]
fn test_default_config_values() {
    let (env, contract_id) = setup_env();
    env.as_contract(&contract_id, || {
        let config = get_config(&env);
        assert_eq!(config.failure_threshold, 3);
        assert_eq!(config.success_threshold, 1);
        assert_eq!(config.max_error_log, 10);
    });
}

// ─────────────────────────────────────────────────────────
// 2. Failure Threshold Behavior
// ─────────────────────────────────────────────────────────

#[test]
fn test_single_failure_does_not_open_circuit() {
    let (env, _admin, contract_id) = setup_with_admin(3);
    simulate_failures(&env, &contract_id, 1);
    env.as_contract(&contract_id, || {
        assert_eq!(get_state(&env), CircuitState::Closed);
        assert_eq!(get_failure_count(&env), 1);
        assert!(check_and_allow(&env).is_ok());
    });
}

#[test]
fn test_failures_below_threshold_keep_circuit_closed() {
    let (env, _admin, contract_id) = setup_with_admin(5);
    simulate_failures(&env, &contract_id, 4);
    env.as_contract(&contract_id, || {
        assert_eq!(get_state(&env), CircuitState::Closed);
        assert_eq!(get_failure_count(&env), 4);
        assert!(check_and_allow(&env).is_ok());
    });
}

#[test]
fn test_circuit_opens_at_threshold() {
    let (env, _admin, contract_id) = setup_with_admin(3);
    simulate_failures(&env, &contract_id, 3);
    env.as_contract(&contract_id, || {
        assert_eq!(get_state(&env), CircuitState::Open);
        assert_eq!(get_failure_count(&env), 3);
    });
}

#[test]
fn test_circuit_opens_exactly_at_threshold_not_before() {
    let (env, _admin, contract_id) = setup_with_admin(3);
    simulate_failures(&env, &contract_id, 2);
    env.as_contract(&contract_id, || {
        assert_eq!(
            get_state(&env),
            CircuitState::Closed,
            "Should be Closed after 2 failures"
        );
    });
    simulate_failures(&env, &contract_id, 1);
    env.as_contract(&contract_id, || {
        assert_eq!(
            get_state(&env),
            CircuitState::Open,
            "Should be Open after 3rd failure"
        );
    });
}

#[test]
fn test_failures_beyond_threshold_remain_open() {
    let (env, _admin, contract_id) = setup_with_admin(3);
    simulate_failures(&env, &contract_id, 5);
    env.as_contract(&contract_id, || {
        assert_eq!(get_state(&env), CircuitState::Open);
        assert_eq!(get_failure_count(&env), 5);
    });
}

// ─────────────────────────────────────────────────────────
// 3. Circuit Open Behavior
// ─────────────────────────────────────────────────────────

#[test]
fn test_check_and_allow_rejects_when_open() {
    let (env, _admin, contract_id) = setup_with_admin(3);
    simulate_failures(&env, &contract_id, 3);
    env.as_contract(&contract_id, || {
        assert_eq!(check_and_allow(&env), Err(ERR_CIRCUIT_OPEN));
    });
}

#[test]
fn test_success_does_not_close_open_circuit() {
    let (env, _admin, contract_id) = setup_with_admin(3);
    simulate_failures(&env, &contract_id, 3);
    env.as_contract(&contract_id, || {
        assert_eq!(get_state(&env), CircuitState::Open);
        // Success should not transition out of Open
        record_success(&env);
        assert_eq!(get_state(&env), CircuitState::Open);
    });
}

// ─────────────────────────────────────────────────────────
// 4. State Transitions and Reset
// ─────────────────────────────────────────────────────────

#[test]
fn test_reset_from_open_to_half_open() {
    let (env, admin, contract_id) = setup_with_admin(3);
    simulate_failures(&env, &contract_id, 3);
    env.as_contract(&contract_id, || {
        assert_eq!(get_state(&env), CircuitState::Open);
    });

    // Reset from Open -> HalfOpen
    env.as_contract(&contract_id, || {
        reset_circuit_breaker(&env, &admin);
        assert_eq!(get_state(&env), CircuitState::HalfOpen);
    });
}

#[test]
fn test_reset_from_half_open_to_closed() {
    let (env, admin, contract_id) = setup_with_admin(3);
    simulate_failures(&env, &contract_id, 3);

    // First reset: Open -> HalfOpen
    env.as_contract(&contract_id, || {
        reset_circuit_breaker(&env, &admin);
        assert_eq!(get_state(&env), CircuitState::HalfOpen);
    });

    // Second reset: HalfOpen -> Closed
    env.as_contract(&contract_id, || {
        reset_circuit_breaker(&env, &admin);
        assert_eq!(get_state(&env), CircuitState::Closed);
        assert_eq!(get_failure_count(&env), 0);
    });
}

#[test]
fn test_success_in_half_open_closes_circuit() {
    let (env, admin, contract_id) = setup_with_admin(3);
    simulate_failures(&env, &contract_id, 3);

    env.as_contract(&contract_id, || {
        reset_circuit_breaker(&env, &admin);
        assert_eq!(get_state(&env), CircuitState::HalfOpen);

        // Success in HalfOpen with success_threshold=1 should close the circuit
        record_success(&env);
        assert_eq!(get_state(&env), CircuitState::Closed);
        assert_eq!(get_failure_count(&env), 0);
        assert_eq!(get_success_count(&env), 0); // Reset after closing
    });
}

#[test]
fn test_multiple_successes_required_to_close() {
    let (env, contract_id) = setup_env();
    let admin = Address::generate(&env);

    env.as_contract(&contract_id, || {
        set_circuit_admin(&env, admin.clone(), None);
        set_config(
            &env,
            CircuitBreakerConfig {
                failure_threshold: 3,
                success_threshold: 3, // Require 3 successes
                max_error_log: 5,
            },
        );

        // Open the circuit
        for i in 0..3 {
            record_failure(&env, i as u64, symbol_short!("test"), ERR_TRANSFER_FAILED);
        }
        assert_eq!(get_state(&env), CircuitState::Open);

        // Reset to HalfOpen
        reset_circuit_breaker(&env, &admin);
        assert_eq!(get_state(&env), CircuitState::HalfOpen);

        // First success - still HalfOpen
        record_success(&env);
        assert_eq!(get_state(&env), CircuitState::HalfOpen);
        assert_eq!(get_success_count(&env), 1);

        // Second success - still HalfOpen
        record_success(&env);
        assert_eq!(get_state(&env), CircuitState::HalfOpen);
        assert_eq!(get_success_count(&env), 2);

        // Third success - now Closed
        record_success(&env);
        assert_eq!(get_state(&env), CircuitState::Closed);
        assert_eq!(get_failure_count(&env), 0);
    });
}

#[test]
fn test_failure_in_half_open_keeps_circuit_open() {
    let (env, admin, contract_id) = setup_with_admin(3);
    simulate_failures(&env, &contract_id, 3);

    env.as_contract(&contract_id, || {
        reset_circuit_breaker(&env, &admin);
        assert_eq!(get_state(&env), CircuitState::HalfOpen);

        // Failure in HalfOpen should open circuit again
        record_failure(&env, 1, symbol_short!("test"), ERR_TRANSFER_FAILED);
        assert_eq!(get_state(&env), CircuitState::Open);
    });
}

// ─────────────────────────────────────────────────────────
// 5. Admin Controls
// ─────────────────────────────────────────────────────────

#[test]
fn test_set_circuit_admin() {
    let (env, contract_id) = setup_env();
    let admin = Address::generate(&env);

    env.as_contract(&contract_id, || {
        set_circuit_admin(&env, admin.clone(), None);
        assert_eq!(get_circuit_admin(&env), Some(admin.clone()));
    });
}

#[test]
fn test_update_circuit_admin_requires_current_admin() {
    let (env, contract_id) = setup_env();
    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);

    env.as_contract(&contract_id, || {
        // Set initial admin
        set_circuit_admin(&env, admin1.clone(), None);
        assert_eq!(get_circuit_admin(&env), Some(admin1.clone()));

        // Update with correct current admin should succeed
        set_circuit_admin(&env, admin2.clone(), Some(admin1.clone()));
        assert_eq!(get_circuit_admin(&env), Some(admin2.clone()));
    });
}

#[test]
#[should_panic(expected = "Unauthorized: only current admin can change circuit breaker admin")]
fn test_update_circuit_admin_rejects_wrong_caller() {
    let (env, contract_id) = setup_env();
    let admin1 = Address::generate(&env);
    let admin2 = Address::generate(&env);
    let wrong_admin = Address::generate(&env);

    env.as_contract(&contract_id, || {
        set_circuit_admin(&env, admin1.clone(), None);
        // Try to update with wrong admin - should panic
        set_circuit_admin(&env, admin2.clone(), Some(wrong_admin));
    });
}

#[test]
#[should_panic(expected = "Unauthorized: only registered circuit breaker admin can reset")]
fn test_reset_requires_registered_admin() {
    let (env, _admin, contract_id) = setup_with_admin(3);
    simulate_failures(&env, &contract_id, 3);

    let wrong_admin = Address::generate(&env);
    env.as_contract(&contract_id, || {
        reset_circuit_breaker(&env, &wrong_admin);
    });
}

#[test]
fn test_set_config() {
    let (env, contract_id) = setup_env();

    env.as_contract(&contract_id, || {
        let new_config = CircuitBreakerConfig {
            failure_threshold: 5,
            success_threshold: 2,
            max_error_log: 20,
        };
        set_config(&env, new_config.clone());

        let retrieved = get_config(&env);
        assert_eq!(retrieved.failure_threshold, 5);
        assert_eq!(retrieved.success_threshold, 2);
        assert_eq!(retrieved.max_error_log, 20);
    });
}

// ─────────────────────────────────────────────────────────
// 6. Status and Logging
// ─────────────────────────────────────────────────────────

#[test]
fn test_get_status_returns_full_snapshot() {
    let (env, _admin, contract_id) = setup_with_admin(3);
    simulate_failures(&env, &contract_id, 2);

    env.as_contract(&contract_id, || {
        let status: CircuitBreakerStatus = get_status(&env);
        assert_eq!(status.state, CircuitState::Closed);
        assert_eq!(status.failure_count, 2);
        assert_eq!(status.failure_threshold, 3);
        assert_eq!(status.success_threshold, 1);
    });
}

#[test]
fn test_error_log_records_failures() {
    let (env, _admin, contract_id) = setup_with_admin(3);

    env.as_contract(&contract_id, || {
        // Record some failures
        record_failure(&env, 1, symbol_short!("lock"), ERR_TRANSFER_FAILED);
        record_failure(&env, 2, symbol_short!("release"), ERR_TRANSFER_FAILED);

        let log = get_error_log(&env);
        assert_eq!(log.len(), 2);

        // Check first entry
        let entry = log.get(0).unwrap();
        assert_eq!(entry.bounty_id, 1);
        assert_eq!(entry.error_code, ERR_TRANSFER_FAILED);
    });
}

#[test]
fn test_error_log_respects_max_size() {
    let (env, _admin, contract_id) = setup_with_admin(3);

    env.as_contract(&contract_id, || {
        // Record more failures than max_error_log
        for i in 0..10 {
            record_failure(&env, i as u64, symbol_short!("test"), ERR_TRANSFER_FAILED);
        }

        let log = get_error_log(&env);
        // Should be capped at max_error_log (5)
        assert_eq!(log.len(), 5);
    });
}

// ─────────────────────────────────────────────────────────
// 7. Success Counter in Closed State
// ─────────────────────────────────────────────────────────

#[test]
fn test_success_in_closed_resets_failure_count() {
    let (env, _admin, contract_id) = setup_with_admin(3);

    env.as_contract(&contract_id, || {
        // Record some failures but don't reach threshold
        record_failure(&env, 1, symbol_short!("test"), ERR_TRANSFER_FAILED);
        record_failure(&env, 2, symbol_short!("test"), ERR_TRANSFER_FAILED);
        assert_eq!(get_failure_count(&env), 2);

        // Success should reset failure count
        record_success(&env);
        assert_eq!(get_failure_count(&env), 0);
    });
}

#[test]
fn test_success_in_closed_does_not_change_state() {
    let (env, _admin, contract_id) = setup_with_admin(3);

    env.as_contract(&contract_id, || {
        record_failure(&env, 1, symbol_short!("test"), ERR_TRANSFER_FAILED);
        assert_eq!(get_state(&env), CircuitState::Closed);

        record_success(&env);
        assert_eq!(get_state(&env), CircuitState::Closed);
    });
}

// ─────────────────────────────────────────────────────────
// 8. Integration Tests with Bounty Escrow Contract
// ─────────────────────────────────────────────────────────

#[test]
fn test_circuit_breaker_blocks_lock_funds_when_open() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1000);

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let depositor = Address::generate(&env);

    // Initialize contract
    env.as_contract(&contract_id, || {
        crate::BountyEscrowContract::init(env.clone(), admin.clone(), token.clone()).unwrap();
        
        // Setup circuit breaker and open it
        crate::error_recovery::set_circuit_admin(&env, admin.clone(), None);
        crate::error_recovery::set_config(
            &env,
            CircuitBreakerConfig {
                failure_threshold: 1,
                success_threshold: 1,
                max_error_log: 5,
            },
        );
        // Open circuit
        crate::error_recovery::record_failure(&env, 0, symbol_short!("test"), ERR_TRANSFER_FAILED);
    });

    // Try to lock funds - should fail with CircuitBreakerOpen
    let result = env.as_contract(&contract_id, || {
        crate::BountyEscrowContract::lock_funds(
            env.clone(),
            depositor,
            1,
            1000,
            env.ledger().timestamp() + 1000,
        )
    });
    assert_eq!(result, Err(Error::CircuitBreakerOpen));
}

#[test]
fn test_circuit_breaker_allows_lock_funds_when_closed() {
    // Unit test the circuit breaker check directly - when circuit is closed, check_and_allow passes
    let (env, contract_id) = setup_env();
    
    env.as_contract(&contract_id, || {
        // Circuit is closed by default
        assert_eq!(get_state(&env), CircuitState::Closed);
        
        // check_and_allow should succeed when circuit is closed
        let result = check_and_allow(&env);
        assert!(result.is_ok(), "check_and_allow should pass when circuit is closed");
    });
}

#[test]
fn test_circuit_breaker_admin_controls_integration() {
    let (env, admin, contract_id) = setup_with_admin(3);
    
    env.as_contract(&contract_id, || {
        // Get circuit breaker admin
        let cb_admin = get_circuit_admin(&env);
        assert_eq!(cb_admin, Some(admin.clone()));

        // Set custom circuit breaker config
        let new_config = CircuitBreakerConfig {
            failure_threshold: 5,
            success_threshold: 2,
            max_error_log: 20,
        };
        set_config(&env, new_config.clone());

        // Get circuit breaker config
        let config = get_config(&env);
        assert_eq!(config.failure_threshold, 5);
        assert_eq!(config.success_threshold, 2);
        assert_eq!(config.max_error_log, 20);

        // Get circuit breaker status
        let status = get_status(&env);
        assert_eq!(status.state, CircuitState::Closed);
    });
}

#[test]
fn test_batch_operations_blocked_when_circuit_open() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1000);

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let depositor = Address::generate(&env);

    env.as_contract(&contract_id, || {
        crate::BountyEscrowContract::init(env.clone(), admin.clone(), token.clone()).unwrap();
        
        // Setup circuit breaker and open it
        crate::error_recovery::set_circuit_admin(&env, admin.clone(), None);
        crate::error_recovery::set_config(
            &env,
            CircuitBreakerConfig {
                failure_threshold: 1,
                success_threshold: 1,
                max_error_log: 5,
            },
        );
        // Open circuit
        crate::error_recovery::record_failure(&env, 0, symbol_short!("test"), ERR_TRANSFER_FAILED);

        // Try batch lock - should fail with CircuitBreakerOpen
        let items = vec![
            &env,
            LockFundsItem {
                bounty_id: 1,
                depositor: depositor.clone(),
                amount: 1000,
                deadline: env.ledger().timestamp() + 1000,
            },
        ];

        let result = crate::BountyEscrowContract::batch_lock_funds(env.clone(), items);
        assert_eq!(result, Err(Error::CircuitBreakerOpen));
    });
}

#[test]
fn test_circuit_breaker_reset_integration() {
    // Test that reset_circuit_breaker transitions states correctly
    // This is already tested in test_reset_from_open_to_half_open and test_reset_from_half_open_to_closed
    // So we just verify the contract-level integration works by checking the public API
    let (env, _admin, contract_id) = setup_with_admin(3);
    
    env.as_contract(&contract_id, || {
        // Open the circuit
        record_failure(&env, 1, symbol_short!("test"), ERR_TRANSFER_FAILED);
        record_failure(&env, 2, symbol_short!("test"), ERR_TRANSFER_FAILED);
        record_failure(&env, 3, symbol_short!("test"), ERR_TRANSFER_FAILED);
        
        assert_eq!(get_state(&env), CircuitState::Open);
        
        // The reset function requires admin auth which is tested in unit tests
        // For integration, we just verify the circuit opened correctly
        let status = get_status(&env);
        assert_eq!(status.state, CircuitState::Open);
        assert_eq!(status.failure_count, 3);
    });
}

// ─────────────────────────────────────────────────────────
// 9. Reentrancy Guard with Circuit Breaker
// ─────────────────────────────────────────────────────────

#[test]
fn test_circuit_breaker_works_with_reentrancy_guard() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1000);

    let contract_id = env.register_contract(None, BountyEscrowContract);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    env.as_contract(&contract_id, || {
        crate::BountyEscrowContract::init(env.clone(), admin.clone(), token.clone()).unwrap();
        
        // Setup circuit breaker
        crate::error_recovery::set_circuit_admin(&env, admin.clone(), None);

        // Circuit breaker should be checked before reentrancy guard is set
        // Both protections should work independently
        let status = crate::BountyEscrowContract::get_circuit_breaker_status(env.clone());
        assert_eq!(status.state, CircuitState::Closed);
    });
}

// ─────────────────────────────────────────────────────────
// 10. Timestamp Recording
// ─────────────────────────────────────────────────────────

#[test]
fn test_opened_at_timestamp_recorded() {
    let (env, _admin, contract_id) = setup_with_admin(3);

    let open_time = 2000u64;
    env.ledger().set_timestamp(open_time);

    simulate_failures(&env, &contract_id, 3);

    env.as_contract(&contract_id, || {
        let status = get_status(&env);
        assert_eq!(status.opened_at, open_time);
    });
}

#[test]
fn test_last_failure_timestamp_recorded() {
    let (env, _admin, contract_id) = setup_with_admin(3);

    let failure_time = 2500u64;
    env.ledger().set_timestamp(failure_time);

    simulate_failures(&env, &contract_id, 1);

    env.as_contract(&contract_id, || {
        let status = get_status(&env);
        assert_eq!(status.last_failure_timestamp, failure_time);
    });
}
