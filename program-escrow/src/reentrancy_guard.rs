//! # Reentrancy Guard Module
//!
//! Provides protection against reentrancy attacks in Soroban smart contracts.
//!
//! ## Overview
//!
//! Reentrancy occurs when an external contract call is made during the execution
//! of a function, and that external contract calls back into the original contract
//! before the first invocation has completed. This can lead to unexpected state
//! changes and potential exploits.
//!
//! ## Implementation
//!
//! This guard uses a simple boolean flag stored in contract storage to track
//! whether a protected function is currently executing. The guard:
//! 1. Checks if the function is already executing (flag is true)
//! 2. If yes, panics to prevent reentry
//! 3. If no, sets the flag to true
//! 4. Executes the protected code
//! 5. Resets the flag to false when done
//!
//! ## Usage
//!
//! ```rust
//! use crate::reentrancy_guard::{check_not_entered, set_entered, clear_entered};
//!
//! pub fn sensitive_function(env: Env) {
//!     // Check and set guard
//!     check_not_entered(&env);
//!     set_entered(&env);
//!     
//!     // ... protected code that makes external calls ...
//!     
//!     // Clear guard before returning
//!     clear_entered(&env);
//! }
//! ```
//!
//! ## Security Considerations
//!
//! - The guard MUST be cleared before the function returns
//! - If a panic occurs, Soroban will roll back all state changes including the guard
//! - The guard protects against same-contract reentrancy
//! - Cross-contract reentrancy requires additional considerations

use soroban_sdk::{symbol_short, Env, Symbol};

/// Storage key for the reentrancy guard flag
const REENTRANCY_GUARD: Symbol = symbol_short!("ReentGrd");

/// Check if a protected function is currently executing.
/// Panics if reentrancy is detected.
///
/// # Panics
/// * If the guard flag is already set (reentrancy detected)
pub fn check_not_entered(env: &Env) {
    let entered: bool = env
        .storage()
        .instance()
        .get(&REENTRANCY_GUARD)
        .unwrap_or(false);

    if entered {
        panic!("Reentrancy detected");
    }
}

/// Set the reentrancy guard flag to indicate a protected function is executing.
pub fn set_entered(env: &Env) {
    env.storage().instance().set(&REENTRANCY_GUARD, &true);
}

/// Clear the reentrancy guard flag to indicate the protected function has completed.
pub fn clear_entered(env: &Env) {
    env.storage().instance().set(&REENTRANCY_GUARD, &false);
}

/// Check if the guard is currently set (for testing purposes).
///
/// # Returns
/// * `true` if a protected function is currently executing
/// * `false` otherwise
pub fn is_entered(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&REENTRANCY_GUARD)
        .unwrap_or(false)
}

/// Macro to wrap a function with reentrancy protection.
///
/// This ensures the guard is properly set and cleared even if the function panics.
/// Note: In Soroban, panics roll back all state changes, so the guard will be
/// automatically cleared on panic.
#[macro_export]
macro_rules! with_reentrancy_guard {
    ($env:expr, $body:block) => {{
        $crate::reentrancy_guard::check_not_entered(&$env);
        $crate::reentrancy_guard::set_entered(&$env);

        let result = $body;

        $crate::reentrancy_guard::clear_entered(&$env);
        result
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProgramEscrowContract;
    use soroban_sdk::Env;

    fn with_contract_env<F, T>(f: F) -> T
    where
        F: FnOnce(Env) -> T,
    {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        env.as_contract(&contract_id, || f(env.clone()))
    }

    fn nested_guarded_call(env: &Env, depth: u8) {
        check_not_entered(env);
        set_entered(env);

        if depth > 0 {
            nested_guarded_call(env, depth - 1);
        }

        clear_entered(env);
    }

    fn outer_guarded_call(env: &Env, should_error: bool) -> Result<(), &'static str> {
        check_not_entered(env);
        set_entered(env);

        let result = if should_error {
            Err("outer call failed")
        } else {
            Ok(())
        };

        clear_entered(env);
        result
    }

    #[test]
    #[should_panic(expected = "Reentrancy detected")]
    fn guard_blocks_nested_call_while_entered() {
        with_contract_env(|env| {
            nested_guarded_call(&env, 1);
        });
    }

    #[test]
    fn guard_resets_after_normal_outer_call() {
        with_contract_env(|env| {
            let result = outer_guarded_call(&env, false);
            assert!(result.is_ok());
            assert!(!is_entered(&env));

            check_not_entered(&env);
            set_entered(&env);
            clear_entered(&env);
            assert!(!is_entered(&env));
        });
    }

    #[test]
    fn guard_resets_after_error_returning_outer_call() {
        with_contract_env(|env| {
            let result = outer_guarded_call(&env, true);
            assert_eq!(result, Err("outer call failed"));
            assert!(!is_entered(&env));

            check_not_entered(&env);
            set_entered(&env);
            clear_entered(&env);
            assert!(!is_entered(&env));
        });
    }
}
