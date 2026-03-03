//! Governance Integration Module
//!
//! Wires grainlify-core governance state into escrow contracts for upgrade and configuration control.

use soroban_sdk::{Address, BytesN, Env, Symbol};

/// Storage key for governance contract address
const GOVERNANCE_CONTRACT: Symbol = soroban_sdk::symbol_short!("GOV_ADDR");

/// Storage key for minimum required governance version
const MIN_GOV_VERSION: Symbol = soroban_sdk::symbol_short!("MIN_VER");

/// Set the governance contract address (admin only)
pub fn set_governance_contract(env: &Env, governance_addr: Address) {
    env.storage().instance().set(&GOVERNANCE_CONTRACT, &governance_addr);
}

/// Get the governance contract address
pub fn get_governance_contract(env: &Env) -> Option<Address> {
    env.storage().instance().get(&GOVERNANCE_CONTRACT)
}

/// Set minimum required governance version
pub fn set_min_governance_version(env: &Env, min_version: u32) {
    env.storage().instance().set(&MIN_GOV_VERSION, &min_version);
}

/// Get minimum required governance version
pub fn get_min_governance_version(env: &Env) -> u32 {
    env.storage().instance().get(&MIN_GOV_VERSION).unwrap_or(0)
}

/// Check if governance contract is configured and meets minimum version
pub fn check_governance_version(env: &Env) -> bool {
    if let Some(gov_addr) = get_governance_contract(env) {
        let min_version = get_min_governance_version(env);
        if min_version > 0 {
            // Call governance contract to get version
            let version: u32 = env.invoke_contract(
                &gov_addr,
                &soroban_sdk::symbol_short!("get_ver"),
                soroban_sdk::vec![env],
            );
            return version >= min_version;
        }
    }
    true // No governance configured or no minimum version set
}

/// Check if an upgrade is approved by governance
pub fn check_upgrade_approval(env: &Env, _wasm_hash: &BytesN<32>) -> bool {
    if let Some(_gov_addr) = get_governance_contract(env) {
        // If governance is configured, check approval
        // For now, return true if governance version check passes
        check_governance_version(env)
    } else {
        // No governance configured, allow upgrade
        true
    }
}
