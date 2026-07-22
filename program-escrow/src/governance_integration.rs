//! Governance Integration Module
//!
//! Wires grainlify-core governance state into escrow contracts for upgrade and configuration control.

use soroban_sdk::{contractclient, Address, BytesN, Env, Symbol};

/// Storage key for governance contract address
const GOVERNANCE_CONTRACT: Symbol = soroban_sdk::symbol_short!("GOV_ADDR");

/// Storage key for minimum required governance version
const MIN_GOV_VERSION: Symbol = soroban_sdk::symbol_short!("MIN_VER");

#[contractclient(name = "GovernanceClient")]
#[allow(dead_code)]
pub trait GovernanceInterface {
    fn get_ver(env: Env) -> u32;
    fn is_upg_ok(env: Env, wasm_hash: BytesN<32>) -> bool;
    /// Returns true when a proposal identified by `proposal_id` has been
    /// vetoed or cancelled before execution.  A vetoed proposal must never
    /// be executed even if it previously reached `Approved` status.
    fn is_vetoed(env: Env, proposal_id: u32) -> bool;
}

/// Set the governance contract address (admin only)
pub fn set_governance_contract(env: &Env, governance_addr: Address) {
    env.storage()
        .instance()
        .set(&GOVERNANCE_CONTRACT, &governance_addr);
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
            let version = GovernanceClient::new(env, &gov_addr).get_ver();
            return version >= min_version;
        }
    }
    true // No governance configured or no minimum version set
}

/// Check if an upgrade hash is approved by an executed governance proposal.
pub fn check_upgrade_approval(env: &Env, wasm_hash: &BytesN<32>) -> bool {
    let Some(gov_addr) = get_governance_contract(env) else {
        return false;
    };

    if !check_governance_version(env) {
        return false;
    }

    GovernanceClient::new(env, &gov_addr).is_upg_ok(wasm_hash)
}

/// Check whether a governance proposal has been vetoed or cancelled.
///
/// Returns `true` when the governance contract reports the proposal as
/// vetoed/cancelled, meaning it **must not** be executed even if it
/// previously reached `Approved` status.  Returns `false` when no
/// governance contract is configured (open/permissionless mode).
pub fn check_proposal_vetoed(env: &Env, proposal_id: u32) -> bool {
    let Some(gov_addr) = get_governance_contract(env) else {
        return false;
    };

    GovernanceClient::new(env, &gov_addr).is_vetoed(&proposal_id)
}
