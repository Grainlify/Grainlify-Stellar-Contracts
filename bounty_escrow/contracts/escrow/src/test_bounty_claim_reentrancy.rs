#![cfg(test)]

//! # Hostile-token reentrancy tests for `bounty_escrow` (Issue #88)
//!
//! These tests exercise the reentrancy surface of `claim()` and
//! `partial_release()` against a deliberately malicious token contract.
//! A hostile token's `transfer` callback re-enters the bounty escrow
//! from inside the transfer hook — exactly the situation the contract
//! must defend against via checks-effects-interactions (CEI).
//!
//! Threat model
//! ------------
//! A bounty sponsor or attacker deploys a custom SAC-compatible token
//! contract and tricks a depositor into locking funds against that
//! token (e.g. through a frontend that pre-fills the token address).
//! When the beneficiary later calls `claim()` or the admin calls
//! `partial_release()`, the token's `transfer` is invoked with
//! `from = bounty_escrow` and `to = recipient`. The hostile token
//! uses this hook to re-enter the escrow. If the escrow has not
//! written its effects (`claim.claimed`, `escrow.status`,
//! `escrow.remaining_amount`) before the external `transfer` call,
//! the re-entered call observes stale state and either:
//!   (a) double-pays the recipient, or
//!   (b) transfers more funds than the bounty actually had, or
//!   (c) drains `remaining_amount` past zero.

use crate::{BountyEscrowContract, BountyEscrowContractClient};
use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    token, vec, Address, Env, IntoVal, Vec,
};

// =====================================================================
// Hostile token harness
// =====================================================================

#[derive(Clone)]
#[soroban_sdk::contracttype]
enum HostileKey {
    /// The escrow contract we will re-enter on the next transfer.
    Escrow,
    /// The bounty id we will re-claim / partial-release on the next transfer.
    BountyId,
    /// The recipient of the re-entered partial_release (only used for mode 2).
    Recipient,
    /// The payout amount of the re-entered partial_release (only used for mode 2).
    Payout,
    /// Reentry mode: 0 = none, 1 = claim, 2 = partial_release.
    Mode,
    /// Has the reentrancy already fired? Prevents infinite re-entry.
    Armed,
}

/// A minimal token contract that can re-enter the bounty escrow from
/// its `transfer` hook. Only the surface `bounty_escrow` calls is
/// implemented. `mint` is intentionally unrestricted (test-only).
#[contract]
pub struct HostileToken;

#[contractimpl]
impl HostileToken {
    /// Initialize the hostile token.
    pub fn init(env: Env) {
        env.storage().instance().set(&HostileKey::Mode, &0u32);
        env.storage().instance().set(&HostileKey::Armed, &false);
    }

    /// Configure the reentry hook. After the next `transfer`, the
    /// hostile token will (exactly once) re-enter the escrow with
    /// the configured call.
    pub fn set_reentry_target(
        env: Env,
        escrow: Address,
        bounty_id: u64,
        recipient: Address,
        payout: i128,
        mode: u32,
    ) {
        env.storage().instance().set(&HostileKey::Escrow, &escrow);
        env.storage()
            .instance()
            .set(&HostileKey::BountyId, &bounty_id);
        env.storage()
            .instance()
            .set(&HostileKey::Recipient, &recipient);
        env.storage().instance().set(&HostileKey::Payout, &payout);
        env.storage().instance().set(&HostileKey::Mode, &mode);
        env.storage().instance().set(&HostileKey::Armed, &true);
    }

    /// Returns true if the reentry hook is currently armed.
    pub fn is_armed(env: Env) -> bool {
        env.storage()
            .instance()
            .get::<HostileKey, bool>(&HostileKey::Armed)
            .unwrap_or(false)
    }

    /// Mint `amount` tokens to `to`. Unrestricted in the test harness.
    pub fn mint(env: Env, to: Address, amount: i128) {
        let balance: i128 = env
            .storage()
            .persistent()
            .get::<Address, i128>(&to)
            .unwrap_or(0i128);
        env.storage().persistent().set(&to, &(balance + amount));
    }

    /// Read the balance of `who`.
    pub fn balance(env: Env, who: Address) -> i128 {
        env.storage()
            .persistent()
            .get::<Address, i128>(&who)
            .unwrap_or(0i128)
    }

    /// SAC-compatible `transfer`. This is the re-entry hook: on the
    /// first call (when `Armed == true`) the contract performs the
    /// balance transfer, then re-enters the escrow through the
    /// configured mode. Subsequent calls fall through to a plain
    /// transfer.
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        // Effects: balance bookkeeping before the external re-entry.
        let from_balance: i128 = env
            .storage()
            .persistent()
            .get::<Address, i128>(&from)
            .unwrap_or(0i128);
        if from_balance < amount {
            panic!("HostileToken: insufficient balance");
        }
        env.storage()
            .persistent()
            .set(&from, &(from_balance - amount));
        let to_balance: i128 = env
            .storage()
            .persistent()
            .get::<Address, i128>(&to)
            .unwrap_or(0i128);
        env.storage()
            .persistent()
            .set(&to, &(to_balance + amount));

        // Interaction: attempt reentry exactly once.
        let armed: bool = env
            .storage()
            .instance()
            .get::<HostileKey, bool>(&HostileKey::Armed)
            .unwrap_or(false);
        if !armed {
            return;
        }
        env.storage().instance().set(&HostileKey::Armed, &false);

        let mode: u32 = env
            .storage()
            .instance()
            .get::<HostileKey, u32>(&HostileKey::Mode)
            .unwrap_or(0);
        if mode == 0 {
            return;
        }

        let escrow: Address = env.storage().instance().get(&HostileKey::Escrow).unwrap();
        let bounty_id: u64 = env.storage().instance().get(&HostileKey::BountyId).unwrap();
        let recipient: Address = env.storage().instance().get(&HostileKey::Recipient).unwrap();
        let payout: i128 = env.storage().instance().get(&HostileKey::Payout).unwrap();

        let escrow_client = BountyEscrowContractClient::new(&env, &escrow);

        if mode == 1 {
            // Re-enter `claim`. The recipient is the auth source for
            // the require_auth inside claim.
            escrow_client
                .mock_auths(&[MockAuth {
                    address: &recipient,
                    invoke: &MockAuthInvoke {
                        contract: &escrow,
                        fn_name: &"claim",
                        args: vec![&env, bounty_id.into_val(&env)],
                        sub_invokes: &[],
                    },
                }])
                .claim(&bounty_id);
        } else if mode == 2 {
            // Re-enter `partial_release`. The recipient is the admin
            // address, which is the auth source for require_auth inside
            // partial_release.
            escrow_client
                .mock_auths(&[MockAuth {
                    address: &recipient,
                    invoke: &MockAuthInvoke {
                        contract: &escrow,
                        fn_name: &"partial_release",
                        args: vec![
                            &env,
                            bounty_id.into_val(&env),
                            recipient.clone().into_val(&env),
                            payout.into_val(&env),
                        ],
                        sub_invokes: &[],
                    },
                }])
                .partial_release(&bounty_id, &recipient, &payout);
        }
    }
}

// =====================================================================
// Helpers
// =====================================================================

fn init_env() -> (
    Env,
    BountyEscrowContractClient<'static>,
    Address, // admin
    Address, // depositor
    Address, // recipient
    Address, // token_admin
) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token_admin = Address::generate(&env);

    (env, client, admin, depositor, recipient, token_admin)
}

// =====================================================================
// Tests
// =====================================================================

/// After the CEI fix, a re-entered `claim()` is rejected because the
/// outer call already wrote `claim.claimed = true` and
/// `escrow.status = Released` before the external `transfer`.
#[test]
fn claim_reentrancy_blocked_by_cei() {
    let (env, client, admin, depositor, recipient, _token_admin) = init_env();
    let bounty_id = 1u64;
    let amount: i128 = 1_000;

    let hostile_id = env.register_contract(None, HostileToken);
    let hostile_client = HostileTokenClient::new(&env, &hostile_id);
    hostile_client.init();
    hostile_client.mint(&depositor, &amount);

    // Wire the hostile token into the escrow and lock funds.
    client.init(&admin, &hostile_id);
    client.lock_funds(
        &depositor,
        &bounty_id,
        &amount,
        &(env.ledger().timestamp() + 10_000),
    );
    // authorize_claim(bounty_id, recipient) — no amount parameter
    client.authorize_claim(&bounty_id, &recipient);

    // Arm the hostile token to re-enter `claim` on its next transfer.
    // "recipient" is the auth source for the re-entered claim.
    hostile_client.set_reentry_target(
        &client.address, // escrow contract
        &bounty_id,
        &recipient,
        &0i128, // payout (unused for claim)
        &1u32,  // mode 1 = claim
    );
    assert!(hostile_client.is_armed());

    // Recipient invokes claim().
    let outer = client
        .mock_auths(&[MockAuth {
            address: &recipient,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: &"claim",
                args: vec![&env, bounty_id.into_val(&env)],
                sub_invokes: &[],
            },
        }])
        .try_claim(&bounty_id);

    // The reentrancy guard ("Reentrancy detected" panic) kills the
    // entire transaction — all state changes are rolled back.
    assert!(outer.is_err(), "claim() must abort when hostile token attempts reentrancy");

    // Because the transaction was rolled back, no funds moved.
    assert_eq!(hostile_client.balance(&recipient), 0);

    // Escrow state is unchanged — still Locked, funds intact.
    env.as_contract(&client.address, || {
        let claim: crate::ClaimRecord = env
            .storage()
            .persistent()
            .get(&crate::DataKey::PendingClaim(bounty_id))
            .unwrap();
        assert!(!claim.claimed, "ClaimRecord.claimed must be false (tx rolled back)");
        let escrow: crate::Escrow = env
            .storage()
            .persistent()
            .get(&crate::DataKey::Escrow(bounty_id))
            .unwrap();
        assert_eq!(escrow.status, crate::EscrowStatus::Locked);
        assert_eq!(escrow.remaining_amount, amount);
    });
}

/// After the CEI fix on `partial_release`, a re-entered
/// `partial_release()` is rejected because the outer call already
/// wrote `escrow.remaining_amount = 0` and `escrow.status = Released`
/// before the external `transfer`.
#[test]
fn partial_release_reentrancy_blocked_by_cei() {
    let (env, client, admin, depositor, contributor, _token_admin) = init_env();
    let bounty_id = 7u64;
    let amount: i128 = 5_000;

    let hostile_id = env.register_contract(None, HostileToken);
    let hostile_client = HostileTokenClient::new(&env, &hostile_id);
    hostile_client.init();
    hostile_client.mint(&depositor, &amount);

    // Wire the hostile token into the escrow and lock funds.
    client.init(&admin, &hostile_id);
    client.lock_funds(
        &depositor,
        &bounty_id,
        &amount,
        &(env.ledger().timestamp() + 10_000),
    );

    // Arm the hostile token to re-enter `partial_release` on its
    // next transfer. Use a second contributor for the inner call.
    let inner_contributor = Address::generate(&env);
    let inner_payout: i128 = 2_000;
    hostile_client.set_reentry_target(
        &client.address, // escrow contract
        &bounty_id,
        &admin, // admin is the auth source for partial_release
        &inner_payout,
        &2u32, // mode 2 = partial_release
    );

    // Admin invokes partial_release() for the full amount.
    // Without the CEI fix, the hostile token's transfer would re-enter
    // partial_release with stale state, allowing a second drain.
    // With the CEI fix, the inner call observes remaining_amount == 0
    // and returns InsufficientFunds.
    let outer = client
        .mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: &"partial_release",
                args: vec![
                    &env,
                    bounty_id.into_val(&env),
                    contributor.clone().into_val(&env),
                    amount.into_val(&env),
                ],
                sub_invokes: &[],
            },
        }])
        .try_partial_release(&bounty_id, &contributor, &amount);

    // The outer partial_release must abort when the hostile token
    // attempts reentrancy.  `try_partial_release` returns Err when
    // the reentrancy guard fires, which reverts the entire Soroban
    // transaction — including the hostile token's disarm and balance
    // transfers.  Post-revert state is identical to pre-call state.
    assert!(outer.is_err(), "partial_release() must abort when hostile token attempts reentrancy");

    // Hostile token is STILL armed because the transaction reverted
    // before the disarm could persist.
    assert!(hostile_client.is_armed());

    // No funds moved — transaction was reverted.
    assert_eq!(hostile_client.balance(&contributor), 0);
    assert_eq!(hostile_client.balance(&inner_contributor), 0);

    // Escrow state is unchanged — still Locked, funds intact.
    env.as_contract(&client.address, || {
        let escrow: crate::Escrow = env
            .storage()
            .persistent()
            .get(&crate::DataKey::Escrow(bounty_id))
            .unwrap();
        assert_eq!(escrow.status, crate::EscrowStatus::Locked);
        assert_eq!(escrow.remaining_amount, amount);
    });
}

/// Regression: a normal (non-reentrant) claim still works after the
/// CEI fix. Uses a plain Stellar Asset Contract.
#[test]
fn claim_happy_path_no_reentry() {
    let (env, client, admin, depositor, recipient, token_admin) = init_env();
    let bounty_id = 2u64;
    let amount: i128 = 2_500;

    let token_addr = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let sac_admin = token::StellarAssetClient::new(&env, &token_addr);
    sac_admin
        .mock_auths(&[MockAuth {
            address: &token_admin,
            invoke: &MockAuthInvoke {
                contract: &token_addr,
                fn_name: &"mint",
                args: vec![&env, depositor.into_val(&env), amount.into_val(&env)],
                sub_invokes: &[],
            },
        }])
        .mint(&depositor, &amount);

    client.init(&admin, &token_addr);
    client.lock_funds(
        &depositor,
        &bounty_id,
        &amount,
        &(env.ledger().timestamp() + 10_000),
    );
    client.authorize_claim(&bounty_id, &recipient);

    let res = client
        .mock_auths(&[MockAuth {
            address: &recipient,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: &"claim",
                args: vec![&env, bounty_id.into_val(&env)],
                sub_invokes: &[],
            },
        }])
        .claim(&bounty_id);
    assert_eq!(res, ());

    let token_client = token::Client::new(&env, &token_addr);
    assert_eq!(token_client.balance(&recipient), amount);
}

/// Regression: a normal (non-reentrant) partial_release still works
/// after the CEI fix. Uses a plain Stellar Asset Contract.
#[test]
fn partial_release_happy_path_no_reentry() {
    let (env, client, admin, depositor, contributor, token_admin) = init_env();
    let bounty_id = 4u64;
    let amount: i128 = 1_000;

    let token_addr = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let sac_admin = token::StellarAssetClient::new(&env, &token_addr);
    sac_admin
        .mock_auths(&[MockAuth {
            address: &token_admin,
            invoke: &MockAuthInvoke {
                contract: &token_addr,
                fn_name: &"mint",
                args: vec![&env, depositor.into_val(&env), amount.into_val(&env)],
                sub_invokes: &[],
            },
        }])
        .mint(&depositor, &amount);

    client.init(&admin, &token_addr);
    client.lock_funds(
        &depositor,
        &bounty_id,
        &amount,
        &(env.ledger().timestamp() + 10_000),
    );

    let res = client
        .mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: &"partial_release",
                args: vec![
                    &env,
                    bounty_id.into_val(&env),
                    contributor.clone().into_val(&env),
                    amount.into_val(&env),
                ],
                sub_invokes: &[],
            },
        }])
        .partial_release(&bounty_id, &contributor, &amount);
    assert_eq!(res, ());

    let token_client = token::Client::new(&env, &token_addr);
    assert_eq!(token_client.balance(&contributor), amount);

    env.as_contract(&client.address, || {
        let escrow: crate::Escrow = env
            .storage()
            .persistent()
            .get(&crate::DataKey::Escrow(bounty_id))
            .unwrap();
        assert_eq!(escrow.status, crate::EscrowStatus::Released);
        assert_eq!(escrow.remaining_amount, 0);
    });
}
