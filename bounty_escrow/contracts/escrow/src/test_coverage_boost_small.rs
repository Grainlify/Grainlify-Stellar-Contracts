#![cfg(test)]

//! Targeted coverage-boost tests for Issue #177.
//! Reaches uncovered branches in lib.rs by:
//! - calling public client methods (auto inside contract context), and
//! - calling internal helpers inside `env.as_contract(&contract_id, || ...)`.

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, Symbol,
};

struct Boost<'a> {
    env: Env,
    contract_id: Address,
    admin: Address,
    depositor: Address,
    token_id: Address,
    client: BountyEscrowContractClient<'a>,
}

impl<'a> Boost<'a> {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, BountyEscrowContract);
        let client = BountyEscrowContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let depositor = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_id = env
            .register_stellar_asset_contract_v2(token_admin.clone())
            .address();
        client.init(&admin, &token_id);
        Self { env, contract_id, admin, depositor, token_id, client }
    }

    fn fund(&self, who: &Address, amount: i128) {
        let sac = token::StellarAssetClient::new(&self.env, &self.token_id);
        sac.mint(who, &amount);
    }

    fn lock(&self, bounty_id: u64, amount: i128) {
        let deadline = self.env.ledger().timestamp() + 3600;
        self.client.lock_funds(&self.depositor, &bounty_id, &amount, &deadline);
    }
}

#[test]
fn fee_and_track_branches() {
    let s = Boost::new();
    // Activate nonzero fees: this makes calculate_fire + fee-event branches live.
    s.client.update_fee_config(
        &Some(500i128),
        &Some(300i128),
        &Some(s.admin.clone()),
        &Some(true),
    );
    s.fund(&s.depositor, 10_000i128);
    s.lock(1u64, 1_000i128); // exercises calculate_fee + emit_fee_collected
    s.client.partial_release(&1u64, &s.depositor, &400i128); // release-fee branch too

    // Directly hit track_operation failure branch inside contract context.
    s.env.as_contract(&s.contract_id, || {
        monitoring::track_operation(&s.env, Symbol::new(&s.env, "op_x"), s.admin.clone(), false);
        monitoring::track_operation(&s.env, Symbol::new(&s.env, "op_y"), s.admin.clone(), true);
    });

    // Analytics both branches: before any operation (ops==0) and after.
    let _a0 = s.client.get_contract_analytics();
    s.env.as_contract(&s.contract_id, || {
        let _a = monitoring::get_analytics(&s.env);
        let _h = monitoring::health_check(&s.env);
        let _s = monitoring::get_state_snapshot(&s.env);
        let _p = monitoring::get_performance_stats(&s.env, Symbol::new(&s.env, "lock"));
    });
}

#[test]
fn anti_abuse_and_whitelist_branches() {
    let s = Boost::new();
    let random = Address::generate(&s.env);
    // Toggle whitelist to exercise both branches in check_rate_limit.
    s.client.set_whitelist(&random, &true);
    s.client.set_whitelist(&random, &false);

    // Prepare depositor with lots of funds.
    s.fund(&s.depositor, 200_000i128);

    // Hammer lock_funds to exceed default anti-abuse limits and trigger
    // rate-limit panic branch (or at least exercise the window/cooldown paths).
    for i in 0..30u64 {
        let _ = s.client.try_lock_funds(&s.depositor, &i, &100i128, &(s.env.ledger().timestamp() + 3600));
    }
}

#[test]
fn circuit_breaker_retry_branches() {
    let s = Boost::new();
    s.client.set_circuit_breaker_config(&3u32, &2u32, &5u32);
    s.client.set_circuit_breaker_admin(&s.admin.clone());
    let _log = s.client.get_circuit_error_log();

    // execute_with_retry with failing closure -> drs record_failure + retry loop exhaustion.
    let fail = s.env.as_contract(&s.contract_id, || {
        error_recovery::execute_with_retry(
            &s.env,
            &error_recovery::RetryConfig { max_attempts: 2 },
            1u64,
            Symbol::new(&s.env, "fail"),
            || Err(1u32),
        )
    });
    assert!(!fail.succeeded);
    assert_eq!(fail.attempts, 2);
    // Exhausting the attempt budget is a distinct terminal outcome, not the
    // last attempt's own (recoverable-looking) error code.
    assert_eq!(fail.final_error, error_recovery::ERR_RETRIES_EXHAUSTED);
    assert_ne!(fail.final_error, 1u32);

    // Success closure -> record_success + immediate return.
    let ok = s.env.as_contract(&s.contract_id, || {
        error_recovery::execute_with_retry(
            &s.env,
            &error_recovery::RetryConfig { max_attempts: 1 },
            2u64,
            Symbol::new(&s.env, "ok"),
            || Ok(()),
        )
    });
    assert!(ok.succeeded);
}

#[test]
fn circuit_breaker_retry_exhausted_uses_default_cap_and_distinct_terminal_error() {
    let s = Boost::new();
    s.client.set_circuit_breaker_config(&100u32, &1u32, &5u32);
    s.client.set_circuit_breaker_admin(&s.admin.clone());

    // A high failure_threshold keeps the circuit Closed throughout, so this
    // exercises retry-budget exhaustion specifically, not a circuit trip.
    let config = error_recovery::RetryConfig::default();
    assert_eq!(config.max_attempts, error_recovery::DEFAULT_MAX_RETRY_ATTEMPTS);

    let result = s.env.as_contract(&s.contract_id, || {
        error_recovery::execute_with_retry(
            &s.env,
            &config,
            3u64,
            Symbol::new(&s.env, "always_fails"),
            || Err(error_recovery::ERR_TRANSFER_FAILED),
        )
    });

    assert!(!result.succeeded);
    assert_eq!(result.attempts, error_recovery::DEFAULT_MAX_RETRY_ATTEMPTS);
    assert_eq!(result.final_error, error_recovery::ERR_RETRIES_EXHAUSTED);
    // The distinct terminal code is never confused with the recoverable
    // code every individual attempt actually failed with.
    assert_ne!(result.final_error, error_recovery::ERR_TRANSFER_FAILED);

    // The underlying per-attempt error remains discoverable via the error
    // log even though the RetryResult itself now reports the terminal code.
    let log = s.env.as_contract(&s.contract_id, || error_recovery::get_error_log(&s.env));
    assert_eq!(log.len(), error_recovery::DEFAULT_MAX_RETRY_ATTEMPTS);
    assert_eq!(log.get(0).unwrap().error_code, error_recovery::ERR_TRANSFER_FAILED);
}

#[test]
fn circuit_breaker_open_short_circuits_retry_with_its_own_terminal_error() {
    let s = Boost::new();
    // A low failure_threshold means the circuit opens after the 1st failure,
    // so the 2nd configured attempt never runs.
    s.client.set_circuit_breaker_config(&1u32, &1u32, &5u32);
    s.client.set_circuit_breaker_admin(&s.admin.clone());

    let result = s.env.as_contract(&s.contract_id, || {
        error_recovery::execute_with_retry(
            &s.env,
            &error_recovery::RetryConfig { max_attempts: 5 },
            4u64,
            Symbol::new(&s.env, "trips_breaker"),
            || Err(error_recovery::ERR_TRANSFER_FAILED),
        )
    });

    assert!(!result.succeeded);
    assert_eq!(result.attempts, 1);
    // Circuit-open is its own terminal error, distinct from exhausting the
    // retry budget — the loop stopped early, not because it ran out of
    // configured attempts.
    assert_eq!(result.final_error, error_recovery::ERR_CIRCUIT_OPEN);
    assert_ne!(result.final_error, error_recovery::ERR_RETRIES_EXHAUSTED);
}
