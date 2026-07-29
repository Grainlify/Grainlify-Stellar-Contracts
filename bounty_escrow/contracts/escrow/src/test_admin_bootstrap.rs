#![cfg(test)]

//! Issue #491 — admin bootstrap authorization and the deploy-then-initialize
//! race window.
//!
//! `init` is the only writer of `DataKey::Admin` in this contract, and there is
//! no rotation entrypoint at all: whoever wins the bootstrap holds the admin
//! role permanently and irrecoverably. #491 adds `admin.require_auth()` as the
//! hardening available without deploy-time initialization.
//!
//! These tests pin down BOTH sides of that mitigation — what it buys, and what
//! it explicitly does NOT close — so a later constructor-style fix can be
//! validated against them.

use super::*;
use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    Address, Env, IntoVal,
};

fn create_escrow<'a>(e: &Env) -> BountyEscrowContractClient<'a> {
    let contract_id = e.register_contract(None, BountyEscrowContract);
    BountyEscrowContractClient::new(e, &contract_id)
}

fn create_token(e: &Env) -> Address {
    let token_admin = Address::generate(e);
    e.register_stellar_asset_contract_v2(token_admin).address()
}

/// Baseline: the legitimate flow still works when the incoming admin signs.
#[test]
fn admin_bootstrap_succeeds_when_incoming_admin_authorizes() {
    let env = Env::default();
    let client = create_escrow(&env);
    let token = create_token(&env);
    let admin = Address::generate(&env);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "init",
            args: (&admin, &token).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.init(&admin, &token);

    assert_eq!(client.get_admin_audit_view().admin, admin);
}

/// The core of the #491 mitigation: with no authorization at all, the bootstrap
/// is rejected and no admin is installed.
#[test]
fn admin_bootstrap_rejects_call_with_no_authorization() {
    let env = Env::default();
    let client = create_escrow(&env);
    let token = create_token(&env);
    let admin = Address::generate(&env);

    // Deliberately no mocked auth of any kind.
    assert!(
        client.try_init(&admin, &token).is_err(),
        "unauthorized bootstrap must be rejected"
    );

    // Nothing was written: a properly authorized bootstrap still succeeds.
    env.mock_all_auths();
    client.init(&admin, &token);
    assert_eq!(client.get_admin_audit_view().admin, admin);
}

/// What `require_auth()` actually buys: a front-runner can no longer name an
/// address they do not control. Authorizing themself is not enough — the
/// *incoming admin* is the address whose signature is checked.
#[test]
fn admin_bootstrap_front_runner_cannot_install_unconsenting_admin() {
    let env = Env::default();
    let client = create_escrow(&env);
    let token = create_token(&env);
    let attacker = Address::generate(&env);
    let unconsenting = Address::generate(&env);

    // The attacker signs as themself but names a third party as admin.
    env.mock_auths(&[MockAuth {
        address: &attacker,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "init",
            args: (&unconsenting, &token).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    assert!(
        client.try_init(&unconsenting, &token).is_err(),
        "bootstrap must require the incoming admin's own signature, not the caller's"
    );
}

/// The residual race, documented deliberately.
///
/// `require_auth()` does NOT close the deploy-then-initialize window: an
/// attacker who front-runs the legitimate deployer and names an address they
/// control signs for it themselves and wins permanently. This test is expected
/// to be rewritten when deploy-time (constructor) initialization lands — it
/// exists so that change has something concrete to invalidate.
#[test]
fn admin_bootstrap_race_self_authorized_front_runner_still_wins() {
    let env = Env::default();
    let client = create_escrow(&env);
    let token = create_token(&env);
    let attacker = Address::generate(&env);
    let legitimate_deployer = Address::generate(&env);

    // The attacker front-runs, naming an address they control and signing for it.
    env.mock_auths(&[MockAuth {
        address: &attacker,
        invoke: &MockAuthInvoke {
            contract: &client.address,
            fn_name: "init",
            args: (&attacker, &token).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.init(&attacker, &token);

    assert_eq!(
        client.get_admin_audit_view().admin,
        attacker,
        "front-runner wins the bootstrap: require_auth does not close the race"
    );

    // The legitimate deployer is now permanently locked out, even fully authorized.
    env.mock_all_auths();
    assert!(
        client.try_init(&legitimate_deployer, &token).is_err(),
        "second bootstrap must be refused"
    );
    assert_eq!(
        client.get_admin_audit_view().admin,
        attacker,
        "and there is no path back: the attacker remains admin"
    );
}

/// Canary for the premise of this issue in `bounty_escrow`: the loser of the
/// bootstrap race has no way back. Every entrypoint that can reach the admin
/// key requires the *current* admin's authorization, so the front-runner must
/// cooperate — which is what makes a lost race here unrecoverable rather than
/// merely inconvenient.
///
/// If a rotation entrypoint reachable by anyone else is ever added, this test
/// should fail and the "permanently unrecoverable" framing must be revisited.
#[test]
fn admin_bootstrap_lost_race_cannot_be_recovered_by_the_victim() {
    let env = Env::default();
    let client = create_escrow(&env);
    let token = create_token(&env);
    let attacker = Address::generate(&env);
    let victim = Address::generate(&env);

    env.mock_all_auths();
    client.init(&attacker, &token);
    env.mock_auths(&[]);

    // Re-bootstrapping is closed.
    assert!(client.try_init(&victim, &token).is_err());

    // And the one entrypoint that can reach the admin key is gated on the
    // current admin's auth, i.e. on the attacker's cooperation.
    assert!(
        client.try_set_anti_abuse_admin(&victim).is_err(),
        "admin-key mutation must require the current admin's authorization"
    );

    assert_eq!(client.get_admin_audit_view().admin, attacker);
}
