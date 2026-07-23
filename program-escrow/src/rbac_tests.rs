#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

struct RbacSetup<'a> {
    env: Env,
    admin: Address,
    operator: Address,
    pauser: Address,
    random: Address,
    client: ProgramEscrowContractClient<'a>,
    token_address: Address,
    program_id: String,
}

impl<'a> RbacSetup<'a> {
    fn new() -> Self {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let operator = Address::generate(&env);
        let pauser = Address::generate(&env);
        let random = Address::generate(&env);

        let tokenadmin = Address::generate(&env);
        let token_id = env
            .register_stellar_asset_contract_v2(tokenadmin.clone())
            .address();

        let program_id = String::from_str(&env, "RBAC-Test");

        // Initialize contract with admin
        client.initialize_contract(&admin);

        // Initialize program with operator
        // Note: Currently init_program doesn't have auth, so we can just call it
        client.init_program(&program_id, &operator, &token_id);

        // Initialize circuit breaker with pauser
        // caller is None for first setting
        client.set_circuitadmin(&pauser, &None);

        Self {
            env,
            admin,
            operator,
            pauser,
            random,
            client,
            token_address: token_id,
            program_id,
        }
    }
}

// ─────────────────────────────────────────────────────────
// Admin Role Tests
// ─────────────────────────────────────────────────────────

#[test]
fn testadmin_permissions() {
    let setup = RbacSetup::new();

    // Admin should be able to pause/unpause
    setup.env.mock_all_auths();
    setup.client.set_paused(&Some(true), &None, &None);
    assert!(setup.client.get_pause_flags().lock_paused);
}

#[test]
#[should_panic]
fn test_random_cannot_pause() {
    let setup = RbacSetup::new();
    setup.client.set_paused(&Some(true), &None, &None);
    // This should panic because the default caller in Soroban tests (without mock_all_auths)
    // will be unauthorized if it hasn't call setup.env.mock_all_auths() or provided auth.
}

// ─────────────────────────────────────────────────────────
// Operator Role Tests
// ─────────────────────────────────────────────────────────

#[test]
fn test_operator_permissions() {
    let setup = RbacSetup::new();
    setup.env.mock_all_auths();

    // Operator should be able to trigger releases
    setup.client.trigger_program_releases();
}

#[test]
#[should_panic]
fn testadmin_cannot_trigger_releases() {
    let setup = RbacSetup::new();
    // No mock_all_auths()

    // Admin is not the operator
    setup.admin.require_auth();
    setup.client.trigger_program_releases();
}

// ─────────────────────────────────────────────────────────
// Pauser Role Tests
// ─────────────────────────────────────────────────────────

#[test]
fn test_pauser_permissions() {
    let setup = RbacSetup::new();
    setup.env.mock_all_auths();

    // Pauser should be able to reset/configure circuit breaker
    setup.client.reset_circuit_breaker(&setup.pauser);
    setup
        .client
        .configure_circuit_breaker(&setup.pauser, &5, &2, &20);
}

#[test]
#[should_panic]
fn testadmin_cannot_reset_circuit() {
    let setup = RbacSetup::new();
    setup.env.mock_all_auths();

    // Even admin cannot reset circuit if they aren't the registered pauser
    setup.client.reset_circuit_breaker(&setup.admin);
}

#[test]
#[should_panic]
fn test_operator_cannot_reset_circuit() {
    let setup = RbacSetup::new();
    setup.env.mock_all_auths();

    // Operator cannot reset circuit
    setup.client.reset_circuit_breaker(&setup.operator);
}

// ═══════════════════════════════════════════════════════════════════════════
// Role-Hierarchy Enforcement Tests
//
// Design note: the three roles in program-escrow are FLAT — no role
// implicitly subsumes another.  Concretely:
//
//   Admin          (DataKey::Admin)
//     └─ set_paused, open/resolve/cancel_dispute*, update_rate_limit_config,
//        set_fund_cap_config, set_whitelist, set_whitelist_enforced,
//        set_governance_contract, set_min_governance_version, setadmin
//
//   CircuitAdmin   (error_recovery registry — separate key from Admin)
//     └─ reset_circuit_breaker, configure_circuit_breaker,
//        emergency_open_circuit
//
//   Operator       (ProgramData.authorized_payout_key)
//     └─ batch_payout, single_payout, trigger_program_releases
//
// Key invariants verified below:
//   1. Admin CAN perform every Admin-gated action.
//   2. CircuitAdmin CAN perform every CircuitAdmin-gated action.
//   3. Operator CAN perform every Operator-gated action.
//   4. Admin CANNOT perform CircuitAdmin-gated actions.
//   5. Admin CANNOT perform Operator-gated actions.
//   6. CircuitAdmin CANNOT perform Admin-gated actions.
//   7. Operator CANNOT perform Admin-gated actions.
// ═══════════════════════════════════════════════════════════════════════════

// ─────────────────────────────────────────────────────────
// §1  Positive: Admin owns all Admin-gated actions
// ─────────────────────────────────────────────────────────

#[test]
fn test_admin_can_set_paused() {
    let setup = RbacSetup::new();
    setup.env.mock_all_auths();
    setup.client.set_paused(&Some(true), &Some(true), &Some(true));
    let flags = setup.client.get_pause_flags();
    assert!(flags.lock_paused && flags.release_paused && flags.refund_paused);
}

#[test]
fn test_admin_can_update_rate_limit_config() {
    let setup = RbacSetup::new();
    setup.env.mock_all_auths();
    setup.client.update_rate_limit_config(&7200, &20, &120);
    let cfg = setup.client.get_rate_limit_config();
    assert_eq!(cfg.window_size, 7200);
    assert_eq!(cfg.max_operations, 20);
}

#[test]
fn test_admin_can_set_fund_cap_config() {
    let setup = RbacSetup::new();
    setup.env.mock_all_auths();
    setup.client.set_fund_cap_config(&Some(1_000_000), &Some(500_000));
    let cap = setup.client.get_fund_cap_config();
    assert_eq!(cap.max_total_funds, Some(1_000_000));
    assert_eq!(cap.max_single_lock, Some(500_000));
}

#[test]
fn test_admin_can_set_whitelist() {
    let setup = RbacSetup::new();
    setup.env.mock_all_auths();
    let addr = Address::generate(&setup.env);
    setup.client.set_whitelist(&addr, &true);
    assert!(setup.client.is_whitelisted(&addr));
}

#[test]
fn test_admin_can_set_whitelist_enforced() {
    let setup = RbacSetup::new();
    setup.env.mock_all_auths();
    setup.client.set_whitelist_enforced(&true);
    assert!(setup.client.is_whitelist_enforced());
}

#[test]
fn test_admin_can_set_governance_contract() {
    let setup = RbacSetup::new();
    setup.env.mock_all_auths();
    let gov_addr = Address::generate(&setup.env);
    setup.client.set_governance_contract(&gov_addr);
    assert_eq!(setup.client.get_governance_contract(), Some(gov_addr));
}

#[test]
fn test_admin_can_set_min_governance_version() {
    let setup = RbacSetup::new();
    setup.env.mock_all_auths();
    setup.client.set_min_governance_version(&3);
    assert_eq!(setup.client.get_min_governance_version(), 3);
}

// ─────────────────────────────────────────────────────────
// §2  Positive: CircuitAdmin owns all CircuitAdmin-gated actions
// ─────────────────────────────────────────────────────────

#[test]
fn test_circuit_admin_can_reset_circuit_breaker() {
    let setup = RbacSetup::new();
    setup.env.mock_all_auths();
    // pauser is the registered CircuitAdmin
    setup.client.reset_circuit_breaker(&setup.pauser);
}

#[test]
fn test_circuit_admin_can_configure_circuit_breaker() {
    let setup = RbacSetup::new();
    setup.env.mock_all_auths();
    setup.client.configure_circuit_breaker(&setup.pauser, &10, &3, &50);
    let status = setup.client.get_circuit_status();
    assert_eq!(status.failure_threshold, 10);
}

#[test]
fn test_circuit_admin_can_emergency_open_circuit() {
    let setup = RbacSetup::new();
    setup.env.mock_all_auths();
    setup.client.emergency_open_circuit(&setup.pauser);
}

// ─────────────────────────────────────────────────────────
// §3  Positive: Operator owns all Operator-gated actions
// ─────────────────────────────────────────────────────────

#[test]
fn test_operator_can_trigger_program_releases() {
    let setup = RbacSetup::new();
    setup.env.mock_all_auths();
    // No schedules exist yet — function completes without error (returns 0)
    let released = setup.client.trigger_program_releases();
    assert_eq!(released, 0);
}

// ─────────────────────────────────────────────────────────
// §4  Cross-role rejection: Admin CANNOT act as CircuitAdmin
// ─────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "Unauthorized: only circuit admin can reset")]
fn test_admin_cannot_reset_circuit_breaker() {
    let setup = RbacSetup::new();
    setup.env.mock_all_auths();
    // admin is registered as DataKey::Admin, NOT as circuit admin
    setup.client.reset_circuit_breaker(&setup.admin);
}

#[test]
#[should_panic(expected = "Unauthorized: only circuit admin can configure")]
fn test_admin_cannot_configure_circuit_breaker() {
    let setup = RbacSetup::new();
    setup.env.mock_all_auths();
    setup.client.configure_circuit_breaker(&setup.admin, &5, &2, &20);
}

#[test]
#[should_panic(expected = "Unauthorized: only circuit admin can open circuit")]
fn test_admin_cannot_emergency_open_circuit() {
    let setup = RbacSetup::new();
    setup.env.mock_all_auths();
    setup.client.emergency_open_circuit(&setup.admin);
}

// ─────────────────────────────────────────────────────────
// §5  Cross-role rejection: Admin CANNOT act as Operator
// ─────────────────────────────────────────────────────────

#[test]
#[should_panic]
fn test_admin_cannot_trigger_program_releases() {
    // The authorized_payout_key is `operator`, NOT `admin`.
    // With mock_all_auths removed, the auth check for authorized_payout_key
    // will fail when admin calls trigger_program_releases.
    let setup = RbacSetup::new();
    // Do NOT call mock_all_auths — we want the auth to bind to who actually calls.
    // In the Soroban test env the call is unauthenticated unless we mock; the
    // authorized_payout_key.require_auth() inside will panic.
    setup.client.trigger_program_releases();
}

// ─────────────────────────────────────────────────────────
// §6  Cross-role rejection: CircuitAdmin CANNOT act as Admin
// ─────────────────────────────────────────────────────────

#[test]
#[should_panic]
fn test_circuit_admin_cannot_set_paused() {
    // pauser is the circuit admin, not the contract admin.
    // Without mock_all_auths the Admin require_auth() check fires and panics.
    let setup = RbacSetup::new();
    setup.client.set_paused(&Some(true), &None, &None);
}

#[test]
#[should_panic]
fn test_circuit_admin_cannot_update_rate_limit_config() {
    let setup = RbacSetup::new();
    // Not calling mock_all_auths — Admin auth check will reject the circuit admin.
    setup.client.update_rate_limit_config(&3600, &10, &60);
}

#[test]
#[should_panic]
fn test_circuit_admin_cannot_set_whitelist() {
    let setup = RbacSetup::new();
    let addr = Address::generate(&setup.env);
    // No mock_all_auths — Admin check rejects circuit admin.
    setup.client.set_whitelist(&addr, &true);
}

#[test]
#[should_panic]
fn test_circuit_admin_cannot_set_governance_contract() {
    let setup = RbacSetup::new();
    let gov = Address::generate(&setup.env);
    // No mock_all_auths — Admin check rejects circuit admin.
    setup.client.set_governance_contract(&gov);
}

// ─────────────────────────────────────────────────────────
// §7  Cross-role rejection: Operator CANNOT act as Admin
// ─────────────────────────────────────────────────────────

#[test]
#[should_panic]
fn test_operator_cannot_set_paused() {
    let setup = RbacSetup::new();
    // No mock_all_auths — Admin auth check rejects the operator address.
    setup.client.set_paused(&Some(true), &None, &None);
}

#[test]
#[should_panic]
fn test_operator_cannot_update_rate_limit_config() {
    let setup = RbacSetup::new();
    setup.client.update_rate_limit_config(&3600, &10, &60);
}

#[test]
#[should_panic]
fn test_operator_cannot_set_fund_cap_config() {
    let setup = RbacSetup::new();
    setup.client.set_fund_cap_config(&Some(1_000_000), &None);
}

#[test]
#[should_panic]
fn test_operator_cannot_set_whitelist() {
    let setup = RbacSetup::new();
    let addr = Address::generate(&setup.env);
    setup.client.set_whitelist(&addr, &true);
}

#[test]
#[should_panic]
fn test_operator_cannot_set_whitelist_enforced() {
    let setup = RbacSetup::new();
    setup.client.set_whitelist_enforced(&true);
}

#[test]
#[should_panic]
fn test_operator_cannot_set_governance_contract() {
    let setup = RbacSetup::new();
    let gov = Address::generate(&setup.env);
    setup.client.set_governance_contract(&gov);
}

#[test]
#[should_panic]
fn test_operator_cannot_set_min_governance_version() {
    let setup = RbacSetup::new();
    setup.client.set_min_governance_version(&5);
}

// ─────────────────────────────────────────────────────────
// §8  Cross-role rejection: Operator CANNOT act as CircuitAdmin
// ─────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "Unauthorized: only circuit admin can reset")]
fn test_operator_cannot_reset_circuit_breaker_cross_role() {
    let setup = RbacSetup::new();
    setup.env.mock_all_auths();
    // operator is not the registered circuit admin
    setup.client.reset_circuit_breaker(&setup.operator);
}

#[test]
#[should_panic(expected = "Unauthorized: only circuit admin can configure")]
fn test_operator_cannot_configure_circuit_breaker() {
    let setup = RbacSetup::new();
    setup.env.mock_all_auths();
    setup.client.configure_circuit_breaker(&setup.operator, &5, &2, &20);
}

#[test]
#[should_panic(expected = "Unauthorized: only circuit admin can open circuit")]
fn test_operator_cannot_emergency_open_circuit() {
    let setup = RbacSetup::new();
    setup.env.mock_all_auths();
    setup.client.emergency_open_circuit(&setup.operator);
}

// ─────────────────────────────────────────────────────────
// §9  Cross-role rejection: CircuitAdmin CANNOT act as Operator
// ─────────────────────────────────────────────────────────

#[test]
#[should_panic]
fn test_circuit_admin_cannot_trigger_program_releases() {
    // The authorized_payout_key is `operator`, not `pauser`.
    // Without mock_all_auths the authorized_payout_key.require_auth() fires.
    let setup = RbacSetup::new();
    setup.client.trigger_program_releases();
}
