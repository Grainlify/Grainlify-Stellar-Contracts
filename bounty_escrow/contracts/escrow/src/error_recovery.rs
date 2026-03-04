#![no_std]

use soroban_sdk::{contracttype, symbol_short, Address, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CircuitBreakerKey {
    State,
    FailureCount,
    LastFailureTimestamp,
    OpenedAt,
    SuccessCount,
    Admin,
    Config,
    ErrorLog,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub max_error_log: u32,
}

impl CircuitBreakerConfig {
    pub fn default() -> Self {
        Self {
            failure_threshold: 3,
            success_threshold: 1,
            max_error_log: 10,
        }
    }
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ErrorEntry {
    pub operation: soroban_sdk::Symbol,
    pub bounty_id: u64,
    pub error_code: u32,
    pub timestamp: u64,
    pub failure_count_at_time: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CircuitBreakerStatus {
    pub state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
    pub last_failure_timestamp: u64,
    pub opened_at: u64,
    pub failure_threshold: u32,
    pub success_threshold: u32,
}

pub const ERR_CIRCUIT_OPEN: u32 = 1001;
pub const ERR_INSUFFICIENT_BALANCE: u32 = 1003;

pub fn get_config(env: &Env) -> CircuitBreakerConfig {
    env.storage()
        .persistent()
        .get(&CircuitBreakerKey::Config)
        .unwrap_or(CircuitBreakerConfig::default())
}

pub fn set_config(env: &Env, config: CircuitBreakerConfig) {
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::Config, &config);
}

pub fn get_state(env: &Env) -> CircuitState {
    env.storage()
        .persistent()
        .get(&CircuitBreakerKey::State)
        .unwrap_or(CircuitState::Closed)
}

pub fn get_failure_count(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&CircuitBreakerKey::FailureCount)
        .unwrap_or(0)
}

pub fn get_success_count(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&CircuitBreakerKey::SuccessCount)
        .unwrap_or(0)
}

pub fn get_status(env: &Env) -> CircuitBreakerStatus {
    let config = get_config(env);
    CircuitBreakerStatus {
        state: get_state(env),
        failure_count: get_failure_count(env),
        success_count: get_success_count(env),
        last_failure_timestamp: env
            .storage()
            .persistent()
            .get(&CircuitBreakerKey::LastFailureTimestamp)
            .unwrap_or(0),
        opened_at: env
            .storage()
            .persistent()
            .get(&CircuitBreakerKey::OpenedAt)
            .unwrap_or(0),
        failure_threshold: config.failure_threshold,
        success_threshold: config.success_threshold,
    }
}

pub fn check_and_allow(env: &Env) -> Result<(), u32> {
    match get_state(env) {
        CircuitState::Open => {
            emit_circuit_event(env, symbol_short!("cb_reject"), get_failure_count(env));
            Err(ERR_CIRCUIT_OPEN)
        }
        CircuitState::Closed | CircuitState::HalfOpen => Ok(()),
    }
}

pub fn record_success(env: &Env) {
    let state = get_state(env);
    match state {
        CircuitState::Closed => {
            env.storage()
                .persistent()
                .set(&CircuitBreakerKey::FailureCount, &0u32);
            env.storage()
                .persistent()
                .set(&CircuitBreakerKey::SuccessCount, &0u32);
        }
        CircuitState::HalfOpen => {
            let config = get_config(env);
            let successes = get_success_count(env) + 1;
            env.storage()
                .persistent()
                .set(&CircuitBreakerKey::SuccessCount, &successes);
            if successes >= config.success_threshold {
                close_circuit(env);
            }
        }
        CircuitState::Open => {}
    }
}

pub fn record_failure(env: &Env, bounty_id: u64, operation: soroban_sdk::Symbol, error_code: u32) {
    let config = get_config(env);
    let failures = get_failure_count(env) + 1;
    let now = env.ledger().timestamp();

    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::FailureCount, &failures);
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::LastFailureTimestamp, &now);

    let mut log: soroban_sdk::Vec<ErrorEntry> = env
        .storage()
        .persistent()
        .get(&CircuitBreakerKey::ErrorLog)
        .unwrap_or(soroban_sdk::Vec::new(env));

    log.push_back(ErrorEntry {
        operation: operation.clone(),
        bounty_id,
        error_code,
        timestamp: now,
        failure_count_at_time: failures,
    });

    while log.len() > config.max_error_log {
        log.remove(0);
    }

    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::ErrorLog, &log);

    emit_circuit_event(env, symbol_short!("cb_fail"), failures);

    if failures >= config.failure_threshold {
        open_circuit(env);
    }
}

pub fn open_circuit(env: &Env) {
    let now = env.ledger().timestamp();
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::State, &CircuitState::Open);
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::OpenedAt, &now);
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::SuccessCount, &0u32);

    emit_circuit_event(env, symbol_short!("cb_open"), get_failure_count(env));
}

pub fn half_open_circuit(env: &Env) {
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::State, &CircuitState::HalfOpen);
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::SuccessCount, &0u32);

    emit_circuit_event(env, symbol_short!("cb_half"), get_failure_count(env));
}

pub fn close_circuit(env: &Env) {
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::State, &CircuitState::Closed);
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::FailureCount, &0u32);
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::SuccessCount, &0u32);
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::OpenedAt, &0u64);

    emit_circuit_event(env, symbol_short!("cb_close"), 0);
}

pub fn set_circuit_admin(env: &Env, new_admin: Address) {
    env.storage()
        .persistent()
        .set(&CircuitBreakerKey::Admin, &new_admin);
}

pub fn get_circuit_admin(env: &Env) -> Option<Address> {
    env.storage().persistent().get(&CircuitBreakerKey::Admin)
}

pub fn get_error_log(env: &Env) -> soroban_sdk::Vec<ErrorEntry> {
    env.storage()
        .persistent()
        .get(&CircuitBreakerKey::ErrorLog)
        .unwrap_or(soroban_sdk::Vec::new(env))
}

pub fn reset_circuit_breaker(env: &Env) {
    match get_state(env) {
        CircuitState::Open => half_open_circuit(env),
        CircuitState::HalfOpen | CircuitState::Closed => close_circuit(env),
    }
}

pub fn set_default_config_if_missing(env: &Env) {
    if !env.storage().persistent().has(&CircuitBreakerKey::Config) {
        set_config(env, CircuitBreakerConfig::default());
    }
}

pub fn init_error_log_if_missing(env: &Env) {
    if !env.storage().persistent().has(&CircuitBreakerKey::ErrorLog) {
        env.storage()
            .persistent()
            .set(&CircuitBreakerKey::ErrorLog, &soroban_sdk::Vec::<ErrorEntry>::new(env));
    }
}

fn emit_circuit_event(env: &Env, event_type: soroban_sdk::Symbol, value: u32) {
    env.events().publish(
        (symbol_short!("circuit"), event_type),
        (value, env.ledger().timestamp()),
    );
}
