#![cfg(test)]
use crate::{
    BountyEscrowContract, BountyEscrowContractClient, Error as ContractError, EscrowStatus,
};
use soroban_sdk::{
    contract, contractimpl, contracttype, testutils::Address as _, token, Address, Env, String,
};

#[derive(Clone, Copy)]
#[repr(u32)]
enum AttackMode {
    None = 0,
    Claim = 1,
    PartialRelease = 2,
}

#[contracttype]
#[derive(Clone)]
enum HostileTokenKey {
    Balance(Address),
    Target,
    AttackMode,
    BountyId,
    AttackCount,
    EscrowTransferCount,
    ReentryBlocked,
}

#[contract]
struct HostileTokenContract;

#[contractimpl]
impl HostileTokenContract {
    pub fn init(env: Env, target: Address) {
        env.storage()
            .instance()
            .set(&HostileTokenKey::Target, &target);
        env.storage()
            .instance()
            .set(&HostileTokenKey::AttackMode, &(AttackMode::None as u32));
        env.storage()
            .instance()
            .set(&HostileTokenKey::BountyId, &0u64);
        env.storage()
            .instance()
            .set(&HostileTokenKey::AttackCount, &0u32);
        env.storage()
            .instance()
            .set(&HostileTokenKey::EscrowTransferCount, &0u32);
        env.storage()
            .instance()
            .set(&HostileTokenKey::ReentryBlocked, &false);
    }

    pub fn mint(env: Env, to: Address, amount: i128) {
        let balance = Self::balance(env.clone(), to.clone());
        env.storage()
            .persistent()
            .set(&HostileTokenKey::Balance(to), &(balance + amount));
    }

    pub fn set_attack(env: Env, mode: u32, bounty_id: u64) {
        env.storage()
            .instance()
            .set(&HostileTokenKey::AttackMode, &mode);
        env.storage()
            .instance()
            .set(&HostileTokenKey::BountyId, &bounty_id);
        env.storage()
            .instance()
            .set(&HostileTokenKey::AttackCount, &0u32);
        env.storage()
            .instance()
            .set(&HostileTokenKey::EscrowTransferCount, &0u32);
        env.storage()
            .instance()
            .set(&HostileTokenKey::ReentryBlocked, &false);
    }

    pub fn attack_count(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&HostileTokenKey::AttackCount)
            .unwrap_or(0)
    }

    pub fn escrow_transfer_count(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&HostileTokenKey::EscrowTransferCount)
            .unwrap_or(0)
    }

    pub fn reentry_blocked(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&HostileTokenKey::ReentryBlocked)
            .unwrap_or(false)
    }

    pub fn allowance(_env: Env, _from: Address, _spender: Address) -> i128 {
        0
    }

    pub fn approve(
        env: Env,
        from: Address,
        _spender: Address,
        _amount: i128,
        _expiration_ledger: u32,
    ) {
        from.require_auth();
        env.events().publish(("approve",), from);
    }

    pub fn balance(env: Env, id: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&HostileTokenKey::Balance(id))
            .unwrap_or(0)
    }

    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();

        let target: Address = env
            .storage()
            .instance()
            .get(&HostileTokenKey::Target)
            .unwrap();

        if from == target {
            let count = Self::escrow_transfer_count(env.clone());
            env.storage()
                .instance()
                .set(&HostileTokenKey::EscrowTransferCount, &(count + 1));
            Self::attempt_reentry(&env, &to, amount);
        }

        let from_balance = Self::balance(env.clone(), from.clone());
        let to_balance = Self::balance(env.clone(), to.clone());
        env.storage()
            .persistent()
            .set(&HostileTokenKey::Balance(from), &(from_balance - amount));
        env.storage()
            .persistent()
            .set(&HostileTokenKey::Balance(to), &(to_balance + amount));
    }

    pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
        spender.require_auth();
        Self::transfer(env, from, to, amount);
    }

    pub fn burn(env: Env, from: Address, amount: i128) {
        from.require_auth();
        let balance = Self::balance(env.clone(), from.clone());
        env.storage()
            .persistent()
            .set(&HostileTokenKey::Balance(from), &(balance - amount));
    }

    pub fn burn_from(env: Env, spender: Address, from: Address, amount: i128) {
        spender.require_auth();
        Self::burn(env, from, amount);
    }

    pub fn decimals(_env: Env) -> u32 {
        7
    }

    pub fn name(env: Env) -> String {
        String::from_str(&env, "Hostile Token")
    }

    pub fn symbol(env: Env) -> String {
        String::from_str(&env, "HOST")
    }

    fn attempt_reentry(env: &Env, recipient: &Address, amount: i128) {
        let attack_count = Self::attack_count(env.clone());
        if attack_count > 0 {
            return;
        }

        let mode: u32 = env
            .storage()
            .instance()
            .get(&HostileTokenKey::AttackMode)
            .unwrap_or(AttackMode::None as u32);
        if mode == AttackMode::None as u32 {
            return;
        }

        env.storage()
            .instance()
            .set(&HostileTokenKey::AttackCount, &(attack_count + 1));

        let target: Address = env
            .storage()
            .instance()
            .get(&HostileTokenKey::Target)
            .unwrap();
        let bounty_id: u64 = env
            .storage()
            .instance()
            .get(&HostileTokenKey::BountyId)
            .unwrap();
        let client = BountyEscrowContractClient::new(env, &target);

        let blocked = if mode == AttackMode::Claim as u32 {
            match client.try_claim(&bounty_id) {
                Ok(inner) => inner.is_err(),
                Err(_) => true,
            }
        } else {
            match client.try_partial_release(&bounty_id, recipient, &amount) {
                Ok(inner) => inner.is_err(),
                Err(_) => true,
            }
        };

        env.storage()
            .instance()
            .set(&HostileTokenKey::ReentryBlocked, &blocked);
    }
}

fn create_test_env() -> (Env, BountyEscrowContractClient<'static>, Address) {
    let env = Env::default();
    let contract_id = env.register_contract(None, BountyEscrowContract);
    let client = BountyEscrowContractClient::new(&env, &contract_id);

    (env, client, contract_id)
}

fn create_token_contract<'a>(
    e: &'a Env,
    admin: &Address,
) -> (Address, token::Client<'a>, token::StellarAssetClient<'a>) {
    let token_id = e.register_stellar_asset_contract_v2(admin.clone());
    let token = token_id.address();
    let token_client = token::Client::new(e, &token);
    let token_admin_client = token::StellarAssetClient::new(e, &token);
    (token, token_client, token_admin_client)
}

fn create_hostile_token<'a>(
    env: &'a Env,
    target: &Address,
) -> (Address, HostileTokenContractClient<'a>) {
    let token = env.register_contract(None, HostileTokenContract);
    let client = HostileTokenContractClient::new(env, &token);
    client.init(target);
    (token, client)
}

#[test]
fn test_reentrancy_guard_leak_fix() {
    let (env, client, contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);

    client.init(&admin, &token);

    // 1. Call release_funds with a non-existent bounty.
    // This previously leaked the guard because it was set before validation.
    let res = client.try_release_funds(&1, &contributor);
    assert_eq!(res, Err(Ok(ContractError::BountyNotFound)));

    // Verify the guard is NOT leaked
    env.as_contract(&contract_id, || {
        use crate::DataKey;
        let has_guard = env.storage().instance().has(&DataKey::ReentrancyGuard);
        assert!(
            !has_guard,
            "Reentrancy guard should NOT have leaked after failed call"
        );
    });

    // 2. Lock funds for a real bounty and release them.
    // This would have failed with "Reentrancy detected" if the guard leaked.
    token_admin_client.mint(&depositor, &1000);
    client.lock_funds(&depositor, &2, &1000, &(env.ledger().timestamp() + 100));
    client.release_funds(&2, &contributor);
}

#[test]
#[should_panic] // Soroban host may escalate "Contract re-entry is not allowed" to HostError
fn test_genuine_reentrancy_blocked() {
    let (env, client, contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    client.init(&admin, &token);

    // Simulate a reentrant call by setting the guard manually.
    env.as_contract(&contract_id, || {
        use crate::DataKey;
        env.storage()
            .instance()
            .set(&DataKey::ReentrancyGuard, &true);

        // This should panic
        client.release_funds(&1, &Address::generate(&env));
    });
}

#[test]
#[should_panic]
fn test_claim_respects_reentrancy_guard() {
    let (env, client, contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    let bounty_id = 10_u64;
    let amount = 1_000_i128;

    client.init(&admin, &token);
    client.set_claim_window(&100);
    token_admin_client.mint(&depositor, &amount);
    client.lock_funds(
        &depositor,
        &bounty_id,
        &amount,
        &(env.ledger().timestamp() + 100),
    );
    client.authorize_claim(&bounty_id, &contributor);

    env.as_contract(&contract_id, || {
        use crate::DataKey;
        env.storage()
            .instance()
            .set(&DataKey::ReentrancyGuard, &true);
    });

    client.claim(&bounty_id);
}

#[test]
#[should_panic]
fn test_partial_release_respects_reentrancy_guard() {
    let (env, client, contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let (token, _token_client, token_admin_client) = create_token_contract(&env, &token_admin);
    let bounty_id = 11_u64;
    let amount = 1_000_i128;

    client.init(&admin, &token);
    token_admin_client.mint(&depositor, &amount);
    client.lock_funds(
        &depositor,
        &bounty_id,
        &amount,
        &(env.ledger().timestamp() + 100),
    );

    env.as_contract(&contract_id, || {
        use crate::DataKey;
        env.storage()
            .instance()
            .set(&DataKey::ReentrancyGuard, &true);
    });

    client.partial_release(&bounty_id, &contributor, &400);
}

#[test]
fn hostile_token_cannot_double_spend_claim_during_transfer() {
    let (env, client, contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let (token, hostile_token) = create_hostile_token(&env, &contract_id);
    let bounty_id = 77_u64;
    let amount = 1_000_i128;

    client.init(&admin, &token);
    client.set_claim_window(&100);
    hostile_token.mint(&depositor, &amount);

    client.lock_funds(
        &depositor,
        &bounty_id,
        &amount,
        &(env.ledger().timestamp() + 100),
    );
    client.authorize_claim(&bounty_id, &contributor);
    hostile_token.set_attack(&(AttackMode::Claim as u32), &bounty_id);

    client.claim(&bounty_id);

    let claim = client.get_pending_claim(&bounty_id);
    let escrow = client.get_escrow_info(&bounty_id);
    assert!(claim.claimed, "claim must be consumed exactly once");
    assert_eq!(escrow.status, EscrowStatus::Released);
    assert_eq!(escrow.remaining_amount, 0);
    assert_eq!(
        hostile_token.attack_count(),
        1,
        "hostile token should have attempted reentry during claim transfer"
    );
    assert!(
        hostile_token.reentry_blocked(),
        "reentrant claim attempt should be rejected by contract state or guard"
    );
    assert_eq!(
        hostile_token.escrow_transfer_count(),
        1,
        "claim must not execute a second escrow-to-recipient transfer"
    );
    assert_eq!(hostile_token.balance(&contributor), amount);
}

#[test]
fn hostile_token_cannot_double_spend_partial_release_during_transfer() {
    let (env, client, contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let depositor = Address::generate(&env);
    let contributor = Address::generate(&env);
    let (token, hostile_token) = create_hostile_token(&env, &contract_id);
    let bounty_id = 88_u64;
    let amount = 1_000_i128;
    let payout_amount = 400_i128;

    client.init(&admin, &token);
    hostile_token.mint(&depositor, &amount);
    client.lock_funds(
        &depositor,
        &bounty_id,
        &amount,
        &(env.ledger().timestamp() + 100),
    );
    hostile_token.set_attack(&(AttackMode::PartialRelease as u32), &bounty_id);

    client.partial_release(&bounty_id, &contributor, &payout_amount);

    let escrow = client.get_escrow_info(&bounty_id);
    assert_eq!(escrow.status, EscrowStatus::Locked);
    assert_eq!(escrow.remaining_amount, amount - payout_amount);
    assert_eq!(
        hostile_token.attack_count(),
        1,
        "hostile token should have attempted reentry during partial release transfer"
    );
    assert!(
        hostile_token.reentry_blocked(),
        "reentrant partial release should be rejected by contract state or guard"
    );
    assert_eq!(
        hostile_token.escrow_transfer_count(),
        1,
        "partial release must not execute a second escrow-to-recipient transfer"
    );
    assert_eq!(hostile_token.balance(&contributor), payout_amount);
}

#[test]
fn test_all_methods_guard_not_leaked_on_error() {
    let (env, client, contract_id) = create_test_env();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    client.init(&admin, &token);

    // Helper to assert guard is NOT set in storage
    let assert_no_guard = || {
        env.as_contract(&contract_id, || {
            use crate::DataKey;
            let has_guard = env.storage().instance().has(&DataKey::ReentrancyGuard);
            assert!(!has_guard, "Reentrancy guard leaked!");
        });
    };

    // 1. release_funds with non-existent bounty (returns Err)
    let _ = client.try_release_funds(&999, &Address::generate(&env));
    assert_no_guard();

    // 2. claim with non-existent bounty (returns Err)
    let _ = client.try_claim(&999);
    assert_no_guard();

    // 3. partial_release with non-existent bounty (returns Err)
    let _ = client.try_partial_release(&999, &Address::generate(&env), &100);
    assert_no_guard();

    // 4. refund with non-existent bounty (returns Err)
    let _ = client.try_refund(&999);
    assert_no_guard();

    // 5. batch_release_funds with empty list (returns Err)
    let _ = client.try_batch_release_funds(&soroban_sdk::vec![&env]);
    assert_no_guard();
}
