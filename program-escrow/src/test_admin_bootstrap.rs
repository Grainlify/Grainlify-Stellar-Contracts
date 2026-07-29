#![cfg(test)]

//! Issue #491 — admin bootstrap authorization and the deploy-then-initialize
//! race window.
//!
//! `initialize_contract` and `setadmin` are the two bootstrap paths for
//! `DataKey::Admin`. #491 adds `admin.require_auth()` to both as the hardening
//! available without deploy-time initialization.
//!
//! Unlike `bounty_escrow`, this contract does keep a recovery path
//! (`propose_admin`/`accept_admin`), but only the *current* admin can start it
//! — so a front-runner who wins the bootstrap still cannot be evicted.
//!
//! These tests pin down both what the mitigation buys and what it does NOT
//! close, so a later constructor-style fix can be validated against them.

use super::*;
use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    Address, Env, IntoVal,
};

fn create_contract<'a>(e: &Env) -> ProgramEscrowContractClient<'a> {
    let contract_id = e.register_contract(None, ProgramEscrowContract);
    ProgramEscrowContractClient::new(e, &contract_id)
}

// ─────────────────────────────────────────────────────────
// initialize_contract
// ─────────────────────────────────────────────────────────

#[test]
fn admin_bootstrap_initialize_contract_succeeds_when_admin_authorizes() {
    let env = Env::default();
    let client = create_contract(&env);
    let admin = Address::generate(&env);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "initialize_contract",
            args: (&admin,).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.initialize_contract(&admin);

    assert_eq!(client.getadmin(), Some(admin));
}

#[test]
fn admin_bootstrap_initialize_contract_rejects_unauthorized_call() {
    let env = Env::default();
    let client = create_contract(&env);
    let admin = Address::generate(&env);

    // Deliberately no mocked auth of any kind.
    assert!(
        client.try_initialize_contract(&admin).is_err(),
        "unauthorized bootstrap must be rejected"
    );
    assert_eq!(client.getadmin(), None, "no admin may have been installed");
}

#[test]
fn admin_bootstrap_initialize_contract_requires_incoming_admins_own_signature() {
    let env = Env::default();
    let client = create_contract(&env);
    let attacker = Address::generate(&env);
    let unconsenting = Address::generate(&env);

    // The attacker signs as themself but names a third party as admin.
    env.mock_auths(&[MockAuth {
        address: &attacker,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "initialize_contract",
            args: (&unconsenting,).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    assert!(
        client.try_initialize_contract(&unconsenting).is_err(),
        "bootstrap must check the incoming admin's signature, not the caller's"
    );
    assert_eq!(client.getadmin(), None);
}

// ─────────────────────────────────────────────────────────
// setadmin
// ─────────────────────────────────────────────────────────

#[test]
fn admin_bootstrap_setadmin_succeeds_when_admin_authorizes() {
    let env = Env::default();
    let client = create_contract(&env);
    let admin = Address::generate(&env);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "setadmin",
            args: (&admin,).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.setadmin(&admin);

    assert_eq!(client.getadmin(), Some(admin));
}

#[test]
fn admin_bootstrap_setadmin_rejects_unauthorized_call() {
    let env = Env::default();
    let client = create_contract(&env);
    let admin = Address::generate(&env);

    assert!(
        client.try_setadmin(&admin).is_err(),
        "unauthorized bootstrap must be rejected"
    );
    assert_eq!(client.getadmin(), None, "no admin may have been installed");
}

#[test]
fn admin_bootstrap_setadmin_requires_incoming_admins_own_signature() {
    let env = Env::default();
    let client = create_contract(&env);
    let attacker = Address::generate(&env);
    let unconsenting = Address::generate(&env);

    env.mock_auths(&[MockAuth {
        address: &attacker,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "setadmin",
            args: (&unconsenting,).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    assert!(
        client.try_setadmin(&unconsenting).is_err(),
        "bootstrap must check the incoming admin's signature, not the caller's"
    );
    assert_eq!(client.getadmin(), None);
}

// ─────────────────────────────────────────────────────────
// The residual race
// ─────────────────────────────────────────────────────────

/// `require_auth()` does NOT close the deploy-then-initialize window: an
/// attacker who front-runs the legitimate deployer with an address they control
/// signs for it themselves and wins. Expected to be rewritten when deploy-time
/// (constructor) initialization lands.
#[test]
fn admin_bootstrap_race_self_authorized_front_runner_still_wins() {
    let env = Env::default();
    let client = create_contract(&env);
    let attacker = Address::generate(&env);
    let legitimate_deployer = Address::generate(&env);

    env.mock_auths(&[MockAuth {
        address: &attacker,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "initialize_contract",
            args: (&attacker,).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.initialize_contract(&attacker);

    assert_eq!(
        client.getadmin(),
        Some(attacker.clone()),
        "front-runner wins the bootstrap: require_auth does not close the race"
    );

    // Even fully authorized, the legitimate deployer cannot take over: both
    // bootstrap paths are now closed by the already-initialized guard.
    env.mock_all_auths();
    assert!(client
        .try_initialize_contract(&legitimate_deployer)
        .is_err());
    assert!(client.try_setadmin(&legitimate_deployer).is_err());
    assert_eq!(client.getadmin(), Some(attacker));
}

/// The recovery path exists here (unlike `bounty_escrow`) but is controlled by
/// the current admin, so it does not help the victim of a lost bootstrap race:
/// only the front-runner can initiate the rotation away from themselves.
#[test]
fn admin_bootstrap_race_recovery_requires_the_front_runner_to_cooperate() {
    let env = Env::default();
    let client = create_contract(&env);
    let attacker = Address::generate(&env);
    let legitimate_deployer = Address::generate(&env);

    env.mock_all_auths();
    client.initialize_contract(&attacker);
    env.mock_auths(&[]);

    // The victim cannot propose themselves: `propose_admin` requires the
    // current admin's auth, and the current admin is the attacker.
    assert!(
        client.try_propose_admin(&legitimate_deployer).is_err(),
        "rotation must not be startable by a non-admin"
    );
    assert_eq!(client.getadmin(), Some(attacker));
}
