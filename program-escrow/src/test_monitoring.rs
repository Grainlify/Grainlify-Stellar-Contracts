#![cfg(test)]

use crate::{ProgramEscrowContract, ProgramEscrowContractClient};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    Address, Env, String,
};

#[test]
fn test_monitoring_analytics_and_health() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ProgramEscrowContract);
    let client = ProgramEscrowContractClient::new(&env, &contract_id);

    // Initial health check
    let initial_health = client.health_check();
    assert_eq!(initial_health.is_healthy, true);
    assert_eq!(initial_health.total_operations, 0);

    let backend = Address::generate(&env);
    let token = Address::generate(&env);
    let prog_id = String::from_str(&env, "TestHealth");

    // Test init metric
    env.ledger().set_timestamp(100);
    client.init_program(&prog_id, &backend, &token);

    let analytics = client.get_monitoring_analytics();
    assert_eq!(analytics.operation_count, 1);
    assert_eq!(analytics.error_count, 0);

    let stats = client.get_performance_stats(&symbol_short!("init"));
    assert_eq!(stats.call_count, 1);

    // Test lock metric
    env.ledger().set_timestamp(200);
    client.lock_program_funds(&5000);

    let analytics2 = client.get_monitoring_analytics();
    assert_eq!(analytics2.operation_count, 2);

    // Test lock error metric (trigger panic)
    let result = client.try_lock_program_funds(&0);
    assert!(result.is_err());

    let analytics3 = client.get_monitoring_analytics();
    // Two successful operations tracked; error doesn't add to operation_count
    assert_eq!(analytics3.operation_count, 2);
    // Error count reflects the error_count stored from monitoring context
    assert_eq!(analytics3.error_count, 0);

    // Test state snapshot
    let snapshot = client.get_state_snapshot();
    assert_eq!(snapshot.total_operations, 2);
    assert_eq!(snapshot.total_errors, 0);
}
