#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger, MockAuth, MockAuthInvoke},
    token, Address, Env, IntoVal,
};

fn create_token_contract<'a>(
    e: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract_address = e
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    (
        token::Client::new(e, &contract_address),
        token::StellarAssetClient::new(e, &contract_address),
    )
}

fn create_escrow_contract<'a>(e: &Env) -> BountyEscrowContractClient<'a> {
    let contract_id = e.register_contract(None, BountyEscrowContract);
    BountyEscrowContractClient::new(e, &contract_id)
}

fn setup_contract(env: &Env) -> (Address, Address, token::Client, BountyEscrowContractClient) {
    let admin = Address::generate(env);
    let depositor = Address::generate(env);
    let (token_client, token_admin) = create_token_contract(env, &admin);
    let escrow_client = create_escrow_contract(env);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &escrow_client.address,
            fn_name: "init",
            args: (&admin, &token_client.address).into_val(env),
            sub_invokes: &[],
        },
    }]);
    escrow_client.init(&admin, &token_client.address);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &token_client.address,
            fn_name: "mint",
            args: (depositor.clone(), 100000i128).into_val(env),
            sub_invokes: &[],
        },
    }]);
    token_admin.mint(&depositor, &100000);

    (admin, depositor, token_client, escrow_client)
}

// ==================== Deadline Validation Tests (Issue #40) ====================

#[test]
fn test_lock_funds_rejects_past_deadline() {
    let env = Env::default();
    let (_admin, depositor, _token_client, escrow_client) = setup_contract(&env);

    // Use 0 as a past deadline to avoid underflow
    let past_deadline = 0u64;

    env.mock_auths(&[MockAuth {
        address: &depositor,
        invoke: &MockAuthInvoke {
            contract: &escrow_client.address,
            fn_name: "lock_funds",
            args: (depositor.clone(), 1u64, 1000i128, past_deadline).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let result = escrow_client.try_lock_funds(&depositor, &1u64, &1000i128, &past_deadline);
    assert_eq!(result, Err(Ok(Error::InvalidDeadline)));
}

#[test]
fn test_lock_funds_rejects_zero_deadline() {
    let env = Env::default();
    let (_admin, depositor, _token_client, escrow_client) = setup_contract(&env);

    env.mock_auths(&[MockAuth {
        address: &depositor,
        invoke: &MockAuthInvoke {
            contract: &escrow_client.address,
            fn_name: "lock_funds",
            args: (depositor.clone(), 1u64, 1000i128, 0u64).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let result = escrow_client.try_lock_funds(&depositor, &1u64, &1000i128, &0u64);
    assert_eq!(result, Err(Ok(Error::InvalidDeadline)));
}

#[test]
fn test_lock_funds_rejects_deadline_beyond_max_horizon() {
    let env = Env::default();
    let (admin, depositor, _token_client, escrow_client) = setup_contract(&env);

    // Set max horizon to 1 day (86400 seconds)
    let max_horizon = 86400u64;
    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &escrow_client.address,
            fn_name: "set_deadline_horizon",
            args: (admin.clone(), max_horizon).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    escrow_client.set_deadline_horizon(&admin, &max_horizon);

    // Try to lock with deadline 2 days in the future
    let too_far_deadline = env.ledger().timestamp() + 172800;

    env.mock_auths(&[MockAuth {
        address: &depositor,
        invoke: &MockAuthInvoke {
            contract: &escrow_client.address,
            fn_name: "lock_funds",
            args: (depositor.clone(), 1u64, 1000i128, too_far_deadline).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let result = escrow_client.try_lock_funds(&depositor, &1u64, &1000i128, &too_far_deadline);
    assert_eq!(result, Err(Ok(Error::InvalidDeadline)));
}

#[test]
fn test_lock_funds_accepts_valid_deadline_within_horizon() {
    let env = Env::default();
    let (admin, depositor, token_client, escrow_client) = setup_contract(&env);

    // Set max horizon to 7 days
    let max_horizon = 604800u64;
    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &escrow_client.address,
            fn_name: "set_deadline_horizon",
            args: (admin.clone(), max_horizon).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    escrow_client.set_deadline_horizon(&admin, &max_horizon);

    // Lock with deadline 1 day in the future (within horizon)
    let valid_deadline = env.ledger().timestamp() + 86400;

    env.mock_auths(&[MockAuth {
        address: &depositor,
        invoke: &MockAuthInvoke {
            contract: &escrow_client.address,
            fn_name: "lock_funds",
            args: (depositor.clone(), 1u64, 1000i128, valid_deadline).into_val(&env),
            sub_invokes: &[MockAuthInvoke {
                contract: &token_client.address,
                fn_name: "transfer",
                args: (
                    depositor.clone(),
                    escrow_client.address.clone(),
                    1000i128,
                )
                    .into_val(&env),
                sub_invokes: &[],
            }],
        },
    }]);

    let result = escrow_client.try_lock_funds(&depositor, &1u64, &1000i128, &valid_deadline);
    assert_eq!(result, Ok(Ok(())));
}

// ==================== Amount Validation Tests (Issue #40) ====================

#[test]
fn test_lock_funds_rejects_zero_amount() {
    let env = Env::default();
    let (admin, depositor, _token_client, escrow_client) = setup_contract(&env);

    let deadline = env.ledger().timestamp() + 86400;

    env.mock_auths(&[MockAuth {
        address: &depositor,
        invoke: &MockAuthInvoke {
            contract: &escrow_client.address,
            fn_name: "lock_funds",
            args: (depositor.clone(), 1u64, 0i128, deadline).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let result = escrow_client.try_lock_funds(&depositor, &1u64, &0i128, &deadline);
    assert_eq!(result, Err(Ok(Error::InvalidAmount)));
}

#[test]
fn test_lock_funds_rejects_negative_amount() {
    let env = Env::default();
    let (admin, depositor, _token_client, escrow_client) = setup_contract(&env);

    let deadline = env.ledger().timestamp() + 86400;

    env.mock_auths(&[MockAuth {
        address: &depositor,
        invoke: &MockAuthInvoke {
            contract: &escrow_client.address,
            fn_name: "lock_funds",
            args: (depositor.clone(), 1u64, -100i128, deadline).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let result = escrow_client.try_lock_funds(&depositor, &1u64, &(-100i128), &deadline);
    assert_eq!(result, Err(Ok(Error::InvalidAmount)));
}

// ==================== Deadline Horizon Admin Tests ====================

#[test]
fn test_set_deadline_horizon_only_admin() {
    let env = Env::default();
    let (admin, depositor, _token_client, escrow_client) = setup_contract(&env);

    // Non-admin tries to set horizon
    env.mock_auths(&[MockAuth {
        address: &depositor,
        invoke: &MockAuthInvoke {
            contract: &escrow_client.address,
            fn_name: "set_deadline_horizon",
            args: (depositor.clone(), 86400u64).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let result = escrow_client.try_set_deadline_horizon(&depositor, &86400u64);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_set_deadline_horizon_clear() {
    let env = Env::default();
    let (admin, depositor, token_client, escrow_client) = setup_contract(&env);

    // Set horizon
    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &escrow_client.address,
            fn_name: "set_deadline_horizon",
            args: (admin.clone(), 86400u64).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    escrow_client.set_deadline_horizon(&admin, &86400u64);

    // Clear horizon
    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &escrow_client.address,
            fn_name: "set_deadline_horizon",
            args: (admin.clone(), 0u64).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    escrow_client.set_deadline_horizon(&admin, &0u64);

    // Now any future deadline should work
    let far_future = env.ledger().timestamp() + 99999999;

    env.mock_auths(&[MockAuth {
        address: &depositor,
        invoke: &MockAuthInvoke {
            contract: &escrow_client.address,
            fn_name: "lock_funds",
            args: (depositor.clone(), 1u64, 1000i128, far_future).into_val(&env),
            sub_invokes: &[MockAuthInvoke {
                contract: &token_client.address,
                fn_name: "transfer",
                args: (
                    depositor.clone(),
                    escrow_client.address.clone(),
                    1000i128,
                )
                    .into_val(&env),
                sub_invokes: &[],
            }],
        },
    }]);

    let result = escrow_client.try_lock_funds(&depositor, &1u64, &1000i128, &far_future);
    assert_eq!(result, Ok(Ok(())));
}

// ==================== Batch Lock Deadline Validation Tests ====================

#[test]
fn test_batch_lock_funds_rejects_past_deadline() {
    let env = Env::default();
    let (_admin, depositor, _token_client, escrow_client) = setup_contract(&env);

    // Use 0 as a past deadline to avoid underflow
    let past_deadline = 0u64;

    let item = LockFundsItem {
        bounty_id: 1,
        depositor: depositor.clone(),
        amount: 1000,
        deadline: past_deadline,
    };

    env.mock_auths(&[MockAuth {
        address: &depositor,
        invoke: &MockAuthInvoke {
            contract: &escrow_client.address,
            fn_name: "batch_lock_funds",
            args: (vec![&env, item.clone()]).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let result = escrow_client.try_batch_lock_funds(&vec![&env, item]);
    assert_eq!(result, Err(Ok(Error::InvalidDeadline)));
}

#[test]
fn test_batch_lock_funds_rejects_zero_amount() {
    let env = Env::default();
    let (_admin, depositor, _token_client, escrow_client) = setup_contract(&env);

    let deadline = env.ledger().timestamp() + 86400;

    let item = LockFundsItem {
        bounty_id: 1,
        depositor: depositor.clone(),
        amount: 0,
        deadline,
    };

    env.mock_auths(&[MockAuth {
        address: &depositor,
        invoke: &MockAuthInvoke {
            contract: &escrow_client.address,
            fn_name: "batch_lock_funds",
            args: (vec![&env, item.clone()]).into_val(&env),
            sub_invokes: &[],
        },
    }]);

    let result = escrow_client.try_batch_lock_funds(&vec![&env, item]);
    assert_eq!(result, Err(Ok(Error::InvalidAmount)));
}
