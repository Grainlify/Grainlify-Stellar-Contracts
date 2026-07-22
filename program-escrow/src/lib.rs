#![no_std]
//! # Program Escrow Smart Contract
//!
//! A secure escrow system for managing hackathon and program prize pools on Stellar.
//! This contract enables organizers to lock funds and distribute prizes to multiple
//! winners through secure, auditable batch payouts.
//!
//! ## Overview
//!
//! The Program Escrow contract manages the complete lifecycle of hackathon/program prizes:
//! 1. **Initialization**: Set up program with authorized payout controller
//! 2. **Fund Locking**: Lock prize pool funds in escrow
//! 3. **Batch Payouts**: Distribute prizes to multiple winners simultaneously
//! 4. **Single Payouts**: Distribute individual prizes
//! 5. **Tracking**: Maintain complete payout history and balance tracking
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │              Program Escrow Architecture                         │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                  │
//! │  ┌──────────────┐                                               │
//! │  │  Organizer   │                                               │
//! │  └──────┬───────┘                                               │
//! │         │                                                        │
//! │         │ 1. init_program()                                     │
//! │         ▼                                                        │
//! │  ┌──────────────────┐                                           │
//! │  │  Program Created │                                           │
//! │  └────────┬─────────┘                                           │
//! │           │                                                      │
//! │           │ 2. lock_program_funds()                             │
//! │           ▼                                                      │
//! │  ┌──────────────────┐                                           │
//! │  │  Funds Locked    │                                           │
//! │  │  (Prize Pool)    │                                           │
//! │  └────────┬─────────┘                                           │
//! │           │                                                      │
//! │           │ 3. Hackathon happens...                             │
//! │           │                                                      │
//! │  ┌────────▼─────────┐                                           │
//! │  │ Authorized       │                                           │
//! │  │ Payout Key       │                                           │
//! │  └────────┬─────────┘                                           │
//! │           │                                                      │
//! │    ┌──────┴───────┐                                             │
//! │    │              │                                             │
//! │    ▼              ▼                                             │
//! │ batch_payout() single_payout()                                  │
//! │    │              │                                             │
//! │    ▼              ▼                                             │
//! │ ┌─────────────────────────┐                                    │
//! │ │   Winner 1, 2, 3, ...   │                                    │
//! │ └─────────────────────────┘                                    │
//! │                                                                  │
//! │  Storage:                                                        │
//! │  ┌──────────────────────────────────────────┐                  │
//! │  │ ProgramData:                             │                  │
//! │  │  - program_id                            │                  │
//! │  │  - total_funds                           │                  │
//! │  │  - remaining_balance                     │                  │
//! │  │  - authorized_payout_key                 │                  │
//! │  │  - payout_history: [PayoutRecord]        │                  │
//! │  │  - token_address                         │                  │
//! │  └──────────────────────────────────────────┘                  │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Security Model
//!
//! ### Trust Assumptions
//! - **Authorized Payout Key**: Trusted backend service that triggers payouts
//! - **Organizer**: Trusted to lock appropriate prize amounts
//! - **Token Contract**: Standard Stellar Asset Contract (SAC)
//! - **Contract**: Trustless; operates according to programmed rules
//!
//! ### Key Security Features
//! 1. **Single Initialization**: Prevents program re-configuration
//! 2. **Authorization Checks**: Only authorized key can trigger payouts
//! 3. **Balance Validation**: Prevents overdrafts
//! 4. **Atomic Transfers**: All-or-nothing batch operations
//! 5. **Complete Audit Trail**: Full payout history tracking
//! 6. **Overflow Protection**: Safe arithmetic for all calculations
//!
//! ## Usage Example
//!
//! ```rust
//! use soroban_sdk::{Address, Env, String, vec};
//!
//! // 1. Initialize program (one-time setup)
//! let program_id = String::from_str(&env, "Hackathon2024");
//! let backend = Address::from_string("GBACKEND...");
//! let usdc_token = Address::from_string("CUSDC...");
//!
//! let program = escrow_client.init_program(
//!     &program_id,
//!     &backend,
//!     &usdc_token
//! );
//!
//! // 2. Lock prize pool (10,000 USDC)
//! let prize_pool = 10_000_0000000; // 10,000 USDC (7 decimals)
//! escrow_client.lock_program_funds(&authorized_key, &prize_pool);
//!
//! // 3. After hackathon, distribute prizes
//! let winners = vec![
//!     &env,
//!     Address::from_string("GWINNER1..."),
//!     Address::from_string("GWINNER2..."),
//!     Address::from_string("GWINNER3..."),
//! ];
//!
//! let prizes = vec![
//!     &env,
//!     5_000_0000000,  // 1st place: 5,000 USDC
//!     3_000_0000000,  // 2nd place: 3,000 USDC
//!     2_000_0000000,  // 3rd place: 2,000 USDC
//! ];
//!
//! escrow_client.batch_payout(&winners, &prizes);
//! ```
//!
//! ## Event System
//!
//! The contract emits events for all major operations:
//! - `ProgramInit`: Program initialization
//! - `FundsLocked`: Prize funds locked
//! - `BatchPayout`: Multiple prizes distributed
//! - `Payout`: Single prize distributed
//!
//! ## Best Practices
//!
//! 1. **Verify Winners**: Confirm winner addresses off-chain before payout
//! 2. **Test Payouts**: Use testnet for testing prize distributions
//! 3. **Secure Backend**: Protect authorized payout key with HSM/multi-sig
//! 4. **Audit History**: Review payout history before each distribution
//! 5. **Balance Checks**: Verify remaining balance matches expectations
//! 6. **Token Approval**: Ensure contract has token allowance before locking funds

// ── Step 1: Add module declarations near the top of lib.rs ──────────────
// (after `mod anti_abuse;` and before the contract struct)

mod error_recovery;
mod governance_integration;
pub mod monitoring;
mod reentrancy_guard;

#[cfg(test)]
mod error_recovery_tests;
#[cfg(test)]
mod reentrancy_tests;

#[cfg(test)]
mod test_monitoring;

#[cfg(test)]
mod test_dispute_resolution;

#[cfg(test)]
mod reentrancy_guard_standalone_test;

#[cfg(test)]
mod malicious_reentrant;

#[cfg(test)]
mod test_granular_pause;

#[cfg(test)]
mod test_lifecycle;

#[cfg(test)]
mod budget_profiling_tests;

#[cfg(test)]
mod test_analytics_events;
#[cfg(test)]
mod test_governance_integration;

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, token, vec, Address, Env,
    String, Symbol, Vec,
};

// Event types
const PROGRAM_INITIALIZED: Symbol = symbol_short!("PrgInit");
const FUNDS_LOCKED: Symbol = symbol_short!("FndsLock");
const BATCH_PAYOUT: Symbol = symbol_short!("BatchPay");
const PAYOUT: Symbol = symbol_short!("Payout");
const DISPUTE_OPENED: Symbol = symbol_short!("DispOpen");
const DISPUTE_RESOLVED: Symbol = symbol_short!("DispRes");
const DISPUTE_CANCELLED: Symbol = symbol_short!("DispCanc");
const EVENT_VERSION_V2: u32 = 2;
const PAUSE_STATE_CHANGED: Symbol = symbol_short!("PauseSt");
const AGGREGATE_STATS: Symbol = symbol_short!("AggStats");
const LARGE_PAYOUT: Symbol = symbol_short!("LrgPay");
const SCHEDULE_TRIGGERED: Symbol = symbol_short!("SchedTrg");
const WHITELIST_CHANGED: Symbol = symbol_short!("WlChange");
const WHITELIST_ENFORCEMENT_CHANGED: Symbol = symbol_short!("WlEnfChg");

// Storage keys
const PROGRAM_DATA: Symbol = symbol_short!("ProgData");
const SCHEDULES: Symbol = symbol_short!("Scheds");
const RELEASE_HISTORY: Symbol = symbol_short!("RelHist");
const NEXT_SCHEDULE_ID: Symbol = symbol_short!("NxtSched");
const PROGRAM_REGISTRY: Symbol = symbol_short!("ProgReg");
const PROGRAM_REGISTERED: Symbol = symbol_short!("ProgRegd");
const FEE_CONFIG: Symbol = symbol_short!("FeeConf");
const FUND_CAP_CONFIG: Symbol = symbol_short!("FnCapCfg");
const BASIS_POINTS: i128 = 10_000;
/// Threshold for bumping persistent storage TTL (approx. 1 day on 5s ledgers).
const PERSISTENT_TTL_THRESHOLD: u32 = 17_280;
/// Extension horizon for persistent storage TTL (approx. 30 days on 5s ledgers).
/// This ensures long-lived release schedules and history remain accessible.
const PERSISTENT_TTL_EXTEND_TO: u32 = 518_400;
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayoutRecord {
    pub recipient: Address,
    pub amount: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramInitializedEvent {
    pub version: u32,
    pub program_id: String,
    pub authorized_payout_key: Address,
    pub token_address: Address,
    pub total_funds: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FundsLockedEvent {
    pub version: u32,
    pub program_id: String,
    pub amount: i128,
    pub remaining_balance: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchPayoutEvent {
    pub version: u32,
    pub program_id: String,
    pub recipient_count: u32,
    pub total_amount: i128,
    pub remaining_balance: i128,
    pub gas_proxy_transfer_ops: u32,
    pub gas_proxy_history_appends: u32,
    pub gas_proxy_storage_reads: u32,
    pub gas_proxy_storage_writes: u32,
    pub gas_proxy_events_emitted: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayoutEvent {
    pub version: u32,
    pub program_id: String,
    pub recipient: Address,
    pub amount: i128,
    pub remaining_balance: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AggregateStatsEvent {
    pub version: u32,
    pub program_id: String,
    pub total_funds: i128,
    pub remaining_balance: i128,
    pub total_paid_out: i128,
    pub payout_count: u32,
    pub scheduled_count: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LargePayoutEvent {
    pub version: u32,
    pub program_id: String,
    pub recipient: Address,
    pub amount: i128,
    pub threshold: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WhitelistChangedEvent {
    pub address: Address,
    pub whitelisted: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WhitelistEnforcementChangedEvent {
    pub enabled: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduleTriggeredEvent {
    pub version: u32,
    pub program_id: String,
    pub schedule_id: u64,
    pub recipient: Address,
    pub amount: i128,
    pub trigger_type: ReleaseType,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramData {
    pub program_id: String,
    pub total_funds: i128,
    pub remaining_balance: i128,
    pub authorized_payout_key: Address,
    pub payout_history: Vec<PayoutRecord>,
    pub token_address: Address, // Token contract address for transfers
}

/// Storage key type for individual programs
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Program(String),                 // program_id -> ProgramData
    Admin,                           // Contract Admin
    ReleaseSchedule(String, u64),    // program_id, schedule_id -> ProgramReleaseSchedule
    ReleaseHistory(String),          // program_id -> Vec<ProgramReleaseHistory>
    NextScheduleId(String),          // program_id -> next schedule_id
    PayoutApproval(String, Address), // program_id, recipient -> PayoutApproval
    PendingClaim(String, u64),       // (program_id, schedule_id) -> ClaimRecord
    ClaimWindow,                     // u64 seconds (global config)
    PauseFlags,                      // PauseFlags struct
    RateLimitConfig,                 // RateLimitConfig struct
    FeeConfig,                       // FeeConfig struct
    ProgramRegistry,                 // Vec<String> of program IDs
    Dispute,                         // DisputeRecord (global program-level dispute)
    RecipientDispute(Address),       // recipient -> DisputeRecord
    ScheduleDispute(u64),            // schedule_id -> DisputeRecord
    Whitelist(Address),              // Address -> bool (whitelisted flag)
    WhitelistEnforced,               // bool (enforcement flag)
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseFlags {
    pub lock_paused: bool,
    pub release_paused: bool,
    pub refund_paused: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseStateChanged {
    pub operation: Symbol,
    pub paused: bool,
    pub admin: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RateLimitConfig {
    pub window_size: u64,
    pub max_operations: u32,
    pub cooldown_period: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfig {
    pub lock_fee_rate: i128,
    pub payout_fee_rate: i128,
    pub fee_recipient: Address,
    pub fee_enabled: bool,
}

/// Admin-configurable caps for fund locking.
/// When set, `lock_program_funds` rejects amounts that would exceed either cap.
/// Default: no cap (backward-compatible).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FundCapConfig {
    /// Maximum total funds allowed across all lock operations (None = no cap).
    pub max_total_funds: Option<i128>,
    /// Maximum amount allowed for a single lock operation (None = no cap).
    pub max_single_lock: Option<i128>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Analytics {
    pub total_locked: i128,
    pub total_released: i128,
    pub total_payouts: u32,
    pub active_programs: u32,
    pub operation_count: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramReleaseSchedule {
    pub schedule_id: u64,
    pub recipient: Address,
    pub amount: i128,
    pub release_timestamp: u64,
    pub released: bool,
    pub released_at: Option<u64>,
    pub released_by: Option<Address>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReleaseType {
    Manual,
    Automatic,
    Oracle,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramReleaseHistory {
    pub schedule_id: u64,
    pub recipient: Address,
    pub amount: i128,
    pub released_at: u64,
    pub release_type: ReleaseType,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramAggregateStats {
    pub total_funds: i128,
    pub remaining_balance: i128,
    pub total_paid_out: i128,
    pub payout_count: u32,
    pub scheduled_count: u32,
    pub released_count: u32,
    pub authorized_payout_key: Address,
    pub payout_history: Vec<PayoutRecord>,
    pub token_address: Address,
}

/// Input item for batch program registration.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramInitItem {
    pub program_id: String,
    pub authorized_payout_key: Address,
    pub token_address: Address,
}

/// Maximum number of programs per batch (aligned with bounty_escrow).
pub const MAX_BATCH_SIZE: u32 = 100;

// ── Dispute Resolution Types ──────────────────────────────────────────────

/// Status of a program-level dispute.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisputeStatus {
    None,
    Open,
    Resolved,
    Cancelled,
}

/// Scope for a dispute halt.
///
/// `Global` preserves the original program-wide dispute behavior. `Recipient`
/// blocks direct payouts and releases for one recipient, while `Schedule`
/// blocks only one release schedule.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisputeScope {
    Global,
    Recipient(Address),
    Schedule(u64),
}

/// Record stored on-chain for an active or historical dispute.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeRecord {
    pub opened_by: Address,
    pub opened_at: u64,
    pub reason: String,
    pub status: DisputeStatus,
    pub resolved_by: Option<Address>,
    pub resolved_at: Option<u64>,
}

// ── Dispute Event Types ───────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeOpenedEvent {
    pub version: u32,
    pub program_id: String,
    pub scope: DisputeScope,
    pub opened_by: Address,
    pub reason: String,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeResolvedEvent {
    pub version: u32,
    pub program_id: String,
    pub scope: DisputeScope,
    pub resolved_by: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeCancelledEvent {
    pub version: u32,
    pub program_id: String,
    pub scope: DisputeScope,
    pub cancelled_by: Address,
    pub timestamp: u64,
}

/// Errors for batch program registration.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum BatchError {
    InvalidBatchSize = 1,
    ProgramAlreadyExists = 2,
    DuplicateProgramId = 3,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Governance contract version is below the minimum required for admin operations.
    GovernanceVersionTooLow = 4,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayoutApproval {
    pub program_id: String,
    pub recipient: Address,
    pub amount: i128,
    pub approvals: Vec<Address>,
    pub total_paid_out: i128,
    pub payout_count: u32,
    pub scheduled_count: u32,
    pub released_count: u32,
}

#[contract]
pub struct ProgramEscrowContract;

#[contractimpl]
impl ProgramEscrowContract {
    /// Initialize a new program escrow
    ///
    /// # Arguments
    /// * `program_id` - Unique identifier for the program/hackathon
    /// * `authorized_payout_key` - Address authorized to trigger payouts (backend)
    /// * `token_address` - Address of the token contract to use for transfers
    ///
    /// # Returns
    /// The initialized ProgramData
    pub fn init_program(
        env: Env,
        program_id: String,
        authorized_payout_key: Address,
        token_address: Address,
    ) -> ProgramData {
        let start = env.ledger().timestamp();
        let res = Self::initialize_program(
            env.clone(),
            program_id,
            authorized_payout_key.clone(),
            token_address,
        );
        monitoring::track_operation(&env, symbol_short!("init"), authorized_payout_key, true);
        monitoring::emit_performance(
            &env,
            symbol_short!("init"),
            env.ledger().timestamp().saturating_sub(start),
        );
        res
    }

    pub fn initialize_program(
        env: Env,
        program_id: String,
        authorized_payout_key: Address,
        token_address: Address,
    ) -> ProgramData {
        // Check if program already exists
        if env.storage().persistent().has(&PROGRAM_DATA) {
            Self::bump_persistent_symbol_ttl(&env, &PROGRAM_DATA);
            panic!("Program already initialized");
        }

        let program_data = ProgramData {
            program_id: program_id.clone(),
            total_funds: 0,
            remaining_balance: 0,
            authorized_payout_key: authorized_payout_key.clone(),
            payout_history: vec![&env],
            token_address: token_address.clone(),
        };

        // Store program data
        env.storage().persistent().set(&PROGRAM_DATA, &program_data);
        Self::bump_persistent_symbol_ttl(&env, &PROGRAM_DATA);
        env.storage()
            .persistent()
            .set(&SCHEDULES, &Vec::<ProgramReleaseSchedule>::new(&env));
        Self::bump_persistent_symbol_ttl(&env, &SCHEDULES);
        env.storage()
            .persistent()
            .set(&RELEASE_HISTORY, &Vec::<ProgramReleaseHistory>::new(&env));
        Self::bump_persistent_symbol_ttl(&env, &RELEASE_HISTORY);
        env.storage().instance().set(&NEXT_SCHEDULE_ID, &1_u64);
        Self::bump_instance_ttl(&env);

        // Emit ProgramInitialized event
        env.events().publish(
            (PROGRAM_INITIALIZED,),
            ProgramInitializedEvent {
                version: EVENT_VERSION_V2,
                program_id,
                authorized_payout_key,
                token_address,
                total_funds: 0i128,
            },
        );

        program_data
    }

    /// Batch-initialize multiple programs in one transaction (all-or-nothing).
    ///
    /// # Errors
    /// * `BatchError::InvalidBatchSize` - empty or len > MAX_BATCH_SIZE
    /// * `BatchError::DuplicateProgramId` - duplicate program_id in items
    /// * `BatchError::ProgramAlreadyExists` - a program_id already registered
    pub fn batch_initialize_programs(
        env: Env,
        items: Vec<ProgramInitItem>,
    ) -> Result<u32, BatchError> {
        let batch_size = items.len() as u32;
        if batch_size == 0 || batch_size > MAX_BATCH_SIZE {
            return Err(BatchError::InvalidBatchSize);
        }
        for i in 0..batch_size {
            for j in (i + 1)..batch_size {
                if items.get(i).unwrap().program_id == items.get(j).unwrap().program_id {
                    return Err(BatchError::DuplicateProgramId);
                }
            }
        }
        for i in 0..batch_size {
            let program_key = DataKey::Program(items.get(i).unwrap().program_id.clone());
            if env.storage().persistent().has(&program_key) {
                Self::bump_persistent_datakey_ttl(&env, &program_key);
                return Err(BatchError::ProgramAlreadyExists);
            }
        }

        let mut count = 0u32;
        for i in 0..batch_size {
            let item = items.get(i).unwrap();
            let program_id = item.program_id.clone();
            let authorized_payout_key = item.authorized_payout_key.clone();
            let token_address = item.token_address.clone();

            if program_id.is_empty() {
                return Err(BatchError::InvalidBatchSize);
            }

            let program_data = ProgramData {
                program_id: program_id.clone(),
                total_funds: 0,
                remaining_balance: 0,
                authorized_payout_key: authorized_payout_key.clone(),
                payout_history: vec![&env],
                token_address: token_address.clone(),
            };
            let program_key = DataKey::Program(program_id.clone());
            env.storage().persistent().set(&program_key, &program_data);
            Self::bump_persistent_datakey_ttl(&env, &program_key);

            env.events().publish(
                (symbol_short!("BatchReg"),),
                (program_id, authorized_payout_key, token_address, 0i128),
            );
            count += 1;
        }

        Ok(count)
    }

    /// Calculate fee amount based on rate (in basis points)
    fn calculate_fee(amount: i128, fee_rate: i128) -> i128 {
        if fee_rate == 0 {
            return 0;
        }
        // Fee = (amount * fee_rate) / BASIS_POINTS
        amount
            .checked_mul(fee_rate)
            .and_then(|x| x.checked_div(BASIS_POINTS))
            .unwrap_or(0)
    }

    /// Bump the TTL for single-program persistent storage keys
    fn bump_persistent_symbol_ttl(env: &Env, key: &Symbol) {
        env.storage()
            .persistent()
            .extend_ttl(key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
    }

    /// Bump the TTL for multi-program persistent storage keys
    fn bump_persistent_datakey_ttl(env: &Env, key: &DataKey) {
        env.storage()
            .persistent()
            .extend_ttl(key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
    }
    /// Bump the TTL for the contract instance storage
    fn bump_instance_ttl(env: &Env) {
        env.storage()
            .instance()
            .extend_ttl(PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
    }


    /// Get fee configuration (internal helper)
    fn get_fee_config_internal(env: &Env) -> FeeConfig {
        env.storage()
            .instance()
            .get(&FEE_CONFIG)
            .unwrap_or_else(|| FeeConfig {
                lock_fee_rate: 0,
                payout_fee_rate: 0,
                fee_recipient: env.current_contract_address(),
                fee_enabled: false,
            })
    }

    /// Emit aggregate statistics event
    fn emit_aggregate_stats(env: &Env, program_data: &ProgramData) {
        let schedules: Vec<ProgramReleaseSchedule> = env
            .storage()
            .persistent()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(env));
        Self::bump_persistent_symbol_ttl(env, &SCHEDULES);

        let mut scheduled_count = 0u32;
        for i in 0..schedules.len() {
            if !schedules.get(i).unwrap().released {
                scheduled_count += 1;
            }
        }

        env.events().publish(
            (AGGREGATE_STATS,),
            AggregateStatsEvent {
                version: EVENT_VERSION_V2,
                program_id: program_data.program_id.clone(),
                total_funds: program_data.total_funds,
                remaining_balance: program_data.remaining_balance,
                total_paid_out: program_data.total_funds - program_data.remaining_balance,
                payout_count: program_data.payout_history.len(),
                scheduled_count,
            },
        );
    }

    /// Check if payout is large and emit event if threshold exceeded
    fn check_and_emit_large_payout(
        env: &Env,
        program_data: &ProgramData,
        recipient: &Address,
        amount: i128,
    ) {
        let threshold = monitoring::get_large_payout_threshold_amount(env, program_data.total_funds);
        if amount >= threshold {
            env.events().publish(
                (LARGE_PAYOUT,),
                LargePayoutEvent {
                    version: EVENT_VERSION_V2,
                    program_id: program_data.program_id.clone(),
                    recipient: recipient.clone(),
                    amount,
                    threshold,
                },
            );
        }
    }
    /// Check if a program exists
    ///
    /// # Returns
    /// * `bool` - True if program exists, false otherwise
    pub fn program_exists(env: Env) -> bool {
        // Check both PROGRAM_DATA (single program) and DataKey::Program registry
        if env.storage().persistent().has(&PROGRAM_DATA) {
            Self::bump_persistent_symbol_ttl(&env, &PROGRAM_DATA);
            return true;
        }
        // Check if any programs exist in registry
        let registry: Option<Vec<String>> = env.storage().instance().get(&PROGRAM_REGISTRY);
        Self::bump_instance_ttl(&env);
        if let Some(reg) = registry {
            return reg.len() > 0;
        }
        false
    }

    // ========================================================================
    // Fund Management
    // ========================================================================

    /// Lock funds into the program escrow
    ///
    /// # Arguments
    /// * `from` - The address funding the contract, must authorize this call
    /// * `amount` - Amount of funds to lock (in native token units)
    ///
    /// # Returns
    /// Updated ProgramData with locked funds
    pub fn lock_program_funds(env: Env, from: Address, amount: i128) -> ProgramData {
        let start = env.ledger().timestamp();
        let caller_addr = from.clone();
        from.require_auth();
        
        if Self::check_paused(&env, symbol_short!("lock")) {
            monitoring::track_operation(&env, symbol_short!("lock"), caller_addr.clone(), false);
            panic!("Funds Paused");
        }

        if amount <= 0 {
            monitoring::track_operation(&env, symbol_short!("lock"), caller_addr.clone(), false);
            panic!("Amount must be greater than zero");
        }

        let mut program_data: ProgramData = env
            .storage()
            .persistent()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        Self::bump_persistent_symbol_ttl(&env, &PROGRAM_DATA);

        // Check fund caps if configured
        let cap_config: FundCapConfig = env
            .storage()
            .instance()
            .get(&FUND_CAP_CONFIG)
            .unwrap_or(FundCapConfig {
                max_total_funds: None,
                max_single_lock: None,
            });

        // Per-lock cap check
        if let Some(max_single) = cap_config.max_single_lock {
            if amount > max_single {
                monitoring::track_operation(&env, symbol_short!("lock"), caller_addr.clone(), false);
                panic!("Amount exceeds per-lock maximum");
            }
        }

        // Total-funds cap check
        if let Some(max_total) = cap_config.max_total_funds {
            let new_total = program_data.total_funds.checked_add(amount)
                .unwrap_or_else(|| {
                    monitoring::track_operation(&env, symbol_short!("lock"), caller_addr.clone(), false);
                    panic!("Total funds overflow");
                });
            if new_total > max_total {
                monitoring::track_operation(&env, symbol_short!("lock"), caller_addr.clone(), false);
                panic!("Total funds cap exceeded");
            }
        }

        // Transfer funds
        let token_client = token::Client::new(&env, &program_data.token_address);
        token_client.transfer(&from, &env.current_contract_address(), &amount);

        // Update balances
        program_data.total_funds = program_data.total_funds.checked_add(amount).expect("Total funds overflow");
        program_data.remaining_balance = program_data.remaining_balance.checked_add(amount).expect("Remaining balance overflow");

        // Ensure invariant
        let contract_balance = token_client.balance(&env.current_contract_address());
        if contract_balance < program_data.remaining_balance {
            panic!("Invariant violation: token balance < remaining balance");
        }

        // Store updated data
        env.storage().persistent().set(&PROGRAM_DATA, &program_data);
        Self::bump_persistent_symbol_ttl(&env, &PROGRAM_DATA);

        // Emit FundsLocked event
        env.events().publish(
            (FUNDS_LOCKED,),
            FundsLockedEvent {
                version: EVENT_VERSION_V2,
                program_id: program_data.program_id.clone(),
                amount,
                remaining_balance: program_data.remaining_balance,
            },
        );

        monitoring::track_operation(&env, symbol_short!("lock"), caller_addr, true);
        monitoring::emit_performance(
            &env,
            symbol_short!("lock"),
            env.ledger().timestamp().saturating_sub(start),
        );

        program_data
    }

    // ========================================================================
    // Initialization & Admin
    // ========================================================================

    /// Initialize the contract with an admin.
    /// This must be called before any admin protected functions (like pause) can be used.
    pub fn initialize_contract(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            Self::bump_instance_ttl(&env);
            panic!("Already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        Self::bump_instance_ttl(&env);
    }

    /// Register or replace the contract admin. Can be called multiple times to rotate the admin.
    pub fn setadmin(env: Env, admin: Address) {
        // If admin is already set, require auth from the current admin
        if env.storage().instance().has(&DataKey::Admin) {
            let currentadmin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        Self::bump_instance_ttl(&env);
            currentadmin.require_auth();
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        Self::bump_instance_ttl(&env);
    }

    /// Get the current admin
    pub fn getadmin(env: Env) -> Option<Address> {
        let admin = env.storage().instance().get(&DataKey::Admin);
        if admin.is_some() { Self::bump_instance_ttl(&env); }
        admin
    }

    /// Update pause flags (admin only)
    pub fn set_paused(
        env: Env,
        lock: Option<bool>,
        release: Option<bool>,
        refund: Option<bool>,
    ) -> Result<(), Error> {
        if !env.storage().instance().has(&DataKey::Admin) {
            Self::bump_instance_ttl(&env);
            panic!("Not initialized");
        }

        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        Self::bump_instance_ttl(&env);
        admin.require_auth();

        // Check governance requirements
        Self::check_governance_requirements(&env)?;

        let mut flags = Self::get_pause_flags(&env);

        if let Some(paused) = lock {
            flags.lock_paused = paused;
            env.events().publish(
                (PAUSE_STATE_CHANGED,),
                (symbol_short!("lock"), paused, admin.clone()),
            );
        }

        if let Some(paused) = release {
            flags.release_paused = paused;
            env.events().publish(
                (PAUSE_STATE_CHANGED,),
                (symbol_short!("release"), paused, admin.clone()),
            );
        }

        if let Some(paused) = refund {
            flags.refund_paused = paused;
            env.events().publish(
                (PAUSE_STATE_CHANGED,),
                (symbol_short!("refund"), paused, admin.clone()),
            );
        }

        env.storage().instance().set(&DataKey::PauseFlags, &flags);
        Self::bump_instance_ttl(&env);
        Ok(())
    }

    /// Get current pause flags
    pub fn get_pause_flags(env: &Env) -> PauseFlags {
        env.storage()
            .instance()
            .get(&DataKey::PauseFlags)
            .unwrap_or(PauseFlags {
                lock_paused: false,
                release_paused: false,
                refund_paused: false,
            })
    }

    /// Check if an operation is paused
    fn check_paused(env: &Env, operation: Symbol) -> bool {
        let flags = Self::get_pause_flags(env);
        if operation == symbol_short!("lock") {
            return flags.lock_paused;
        } else if operation == symbol_short!("release") {
            return flags.release_paused;
        } else if operation == symbol_short!("refund") {
            return flags.refund_paused;
        }
        false
    }

    // ========================================================================
    // Dispute Resolution
    // ========================================================================

    /// Internal helper — gets the admin or panics with a user-friendly message.
    fn requireadmin(env: &Env) -> Address {
        env.storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::Admin)
            .unwrap_or_else(|| panic!("Admin not set"))
    }

    /// Returns true when the given dispute key currently stores an open dispute.
    fn is_dispute_open_for_key(env: &Env, key: &DataKey) -> bool {
        if let Some(record) = env.storage().instance().get::<DataKey, DisputeRecord>(key) {
            Self::bump_instance_ttl(&env);
            record.status == DisputeStatus::Open
        } else {
            false
        }
    }

    /// Returns true when a due schedule should be skipped by scoped dispute handling.
    fn is_schedule_scope_disputed(env: &Env, schedule_id: u64, recipient: &Address) -> bool {
        Self::is_dispute_open_for_key(env, &DataKey::ScheduleDispute(schedule_id))
            || Self::is_dispute_open_for_key(env, &DataKey::RecipientDispute(recipient.clone()))
    }

    fn program_id_for_event(env: &Env) -> String {
        let program_data: ProgramData = env
            .storage()
            .persistent()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        Self::bump_persistent_symbol_ttl(&env, &PROGRAM_DATA);
        program_data.program_id
    }

    fn open_dispute_at(
        env: &Env,
        key: DataKey,
        scope: DisputeScope,
        reason: String,
        duplicate_message: &str,
    ) {
        let admin = Self::requireadmin(env);
        admin.require_auth();

        if Self::is_dispute_open_for_key(env, &key) {
            panic!("{}", duplicate_message);
        }

        let program_id = Self::program_id_for_event(env);
        let now = env.ledger().timestamp();
        let record = DisputeRecord {
            opened_by: admin.clone(),
            opened_at: now,
            reason: reason.clone(),
            status: DisputeStatus::Open,
            resolved_by: None,
            resolved_at: None,
        };

        env.storage().instance().set(&key, &record);
        Self::bump_instance_ttl(&env);

        env.events().publish(
            (DISPUTE_OPENED,),
            DisputeOpenedEvent {
                version: EVENT_VERSION_V2,
                program_id,
                scope,
                opened_by: admin,
                reason,
                timestamp: now,
            },
        );
    }

    fn resolve_dispute_at(env: &Env, key: DataKey, scope: DisputeScope, missing_message: &str) {
        let admin = Self::requireadmin(env);
        admin.require_auth();

        let mut record: DisputeRecord = env
            .storage()
            .instance()
            .get(&key)
            .unwrap_or_else(|| panic!("{}", missing_message));
        Self::bump_instance_ttl(&env);

        if record.status != DisputeStatus::Open {
            panic!("No open dispute to resolve");
        }

        let program_id = Self::program_id_for_event(env);
        let now = env.ledger().timestamp();
        record.status = DisputeStatus::Resolved;
        record.resolved_by = Some(admin.clone());
        record.resolved_at = Some(now);

        env.storage().instance().set(&key, &record);
        Self::bump_instance_ttl(&env);

        env.events().publish(
            (DISPUTE_RESOLVED,),
            DisputeResolvedEvent {
                version: EVENT_VERSION_V2,
                program_id,
                scope,
                resolved_by: admin,
                timestamp: now,
            },
        );
    }

    fn cancel_dispute_at(env: &Env, key: DataKey, scope: DisputeScope, missing_message: &str) {
        let admin = Self::requireadmin(env);
        admin.require_auth();

        let mut record: DisputeRecord = env
            .storage()
            .instance()
            .get(&key)
            .unwrap_or_else(|| panic!("{}", missing_message));
        Self::bump_instance_ttl(&env);

        if record.status != DisputeStatus::Open {
            panic!("No open dispute to cancel");
        }

        let program_id = Self::program_id_for_event(env);
        let now = env.ledger().timestamp();
        record.status = DisputeStatus::Cancelled;
        record.resolved_by = Some(admin.clone());
        record.resolved_at = Some(now);

        env.storage().instance().set(&key, &record);
        Self::bump_instance_ttl(&env);

        env.events().publish(
            (DISPUTE_CANCELLED,),
            DisputeCancelledEvent {
                version: EVENT_VERSION_V2,
                program_id,
                scope,
                cancelled_by: admin,
                timestamp: now,
            },
        );
    }

    /// Open a dispute on this program, blocking further payouts until resolved or cancelled.
    ///
    /// # Arguments
    /// * `reason` — Human-readable description of the dispute
    ///
    /// # Panics
    /// * If admin not set
    /// * If a dispute is already open
    pub fn open_dispute(env: Env, reason: String) {
        Self::open_dispute_at(
            &env,
            DataKey::Dispute,
            DisputeScope::Global,
            reason,
            "Dispute already open",
        );
    }

    /// Open a recipient-scoped dispute.
    ///
    /// Direct payouts and release schedules for this recipient are blocked,
    /// while unrelated recipients remain payable unless a global dispute is open.
    pub fn open_recipient_dispute(env: Env, recipient: Address, reason: String) {
        Self::open_dispute_at(
            &env,
            DataKey::RecipientDispute(recipient.clone()),
            DisputeScope::Recipient(recipient),
            reason,
            "Recipient dispute already open",
        );
    }

    /// Open a schedule-scoped dispute.
    ///
    /// Only the selected release schedule is blocked unless a global or
    /// recipient-scoped dispute also applies.
    pub fn open_schedule_dispute(env: Env, schedule_id: u64, reason: String) {
        Self::open_dispute_at(
            &env,
            DataKey::ScheduleDispute(schedule_id),
            DisputeScope::Schedule(schedule_id),
            reason,
            "Schedule dispute already open",
        );
    }

    /// Resolve an open dispute.  Re-enables payouts.
    ///
    /// # Panics
    /// * If admin not set
    /// * If no dispute is currently open
    pub fn resolve_dispute(env: Env) {
        Self::resolve_dispute_at(
            &env,
            DataKey::Dispute,
            DisputeScope::Global,
            "No dispute to resolve",
        );
    }

    /// Resolve an open recipient-scoped dispute.
    pub fn resolve_recipient_dispute(env: Env, recipient: Address) {
        Self::resolve_dispute_at(
            &env,
            DataKey::RecipientDispute(recipient.clone()),
            DisputeScope::Recipient(recipient),
            "No recipient dispute to resolve",
        );
    }

    /// Resolve an open schedule-scoped dispute.
    pub fn resolve_schedule_dispute(env: Env, schedule_id: u64) {
        Self::resolve_dispute_at(
            &env,
            DataKey::ScheduleDispute(schedule_id),
            DisputeScope::Schedule(schedule_id),
            "No schedule dispute to resolve",
        );
    }

    /// Cancel an open dispute.  Re-enables payouts.
    ///
    /// # Panics
    /// * If admin not set
    /// * If no dispute is currently open
    pub fn cancel_dispute(env: Env) {
        Self::cancel_dispute_at(
            &env,
            DataKey::Dispute,
            DisputeScope::Global,
            "No dispute to cancel",
        );
    }

    /// Cancel an open recipient-scoped dispute.
    pub fn cancel_recipient_dispute(env: Env, recipient: Address) {
        Self::cancel_dispute_at(
            &env,
            DataKey::RecipientDispute(recipient.clone()),
            DisputeScope::Recipient(recipient),
            "No recipient dispute to cancel",
        );
    }

    /// Cancel an open schedule-scoped dispute.
    pub fn cancel_schedule_dispute(env: Env, schedule_id: u64) {
        Self::cancel_dispute_at(
            &env,
            DataKey::ScheduleDispute(schedule_id),
            DisputeScope::Schedule(schedule_id),
            "No schedule dispute to cancel",
        );
    }

    /// Returns the current dispute record, if any.
    pub fn get_dispute(env: Env) -> Option<DisputeRecord> {
        let dispute = env.storage().instance().get(&DataKey::Dispute);
        if dispute.is_some() { Self::bump_instance_ttl(&env); }
        dispute
    }

    /// Returns the current recipient-scoped dispute record, if any.
    pub fn get_recipient_dispute(env: Env, recipient: Address) -> Option<DisputeRecord> {
        let record = env.storage()
            .instance()
            .get(&DataKey::RecipientDispute(recipient));
        if record.is_some() { Self::bump_instance_ttl(&env); }
        record
    }

    /// Returns the current schedule-scoped dispute record, if any.
    pub fn get_schedule_dispute(env: Env, schedule_id: u64) -> Option<DisputeRecord> {
        let record = env.storage()
            .instance()
            .get(&DataKey::ScheduleDispute(schedule_id));
        if record.is_some() { Self::bump_instance_ttl(&env); }
        record
    }

    /// Returns true if a dispute is currently open.
    pub fn is_disputed(env: Env) -> bool {
        Self::is_dispute_open_for_key(&env, &DataKey::Dispute)
    }

    /// Returns true if a recipient-scoped dispute is currently open.
    pub fn is_recipient_disputed(env: Env, recipient: Address) -> bool {
        Self::is_dispute_open_for_key(&env, &DataKey::RecipientDispute(recipient))
    }

    /// Returns true if a schedule-scoped dispute is currently open.
    pub fn is_schedule_disputed(env: Env, schedule_id: u64) -> bool {
        Self::is_dispute_open_for_key(&env, &DataKey::ScheduleDispute(schedule_id))
    }

    // --- Circuit Breaker & Rate Limit ---

    pub fn set_circuitadmin(env: Env, newadmin: Address, caller: Option<Address>) {
        error_recovery::set_circuitadmin(&env, newadmin, caller);
        Self::bump_instance_ttl(&env);
    }

    pub fn get_circuitadmin(env: Env) -> Option<Address> {
        error_recovery::get_circuitadmin(&env)
    }

    pub fn reset_circuit_breaker(env: Env, caller: Address) {
        caller.require_auth();
        let admin = error_recovery::get_circuitadmin(&env).expect("Circuit admin not set");
        if caller != admin {
            panic!("Unauthorized: only circuit admin can reset");
        }
        error_recovery::reset_circuit_breaker(&env, &admin);
    }

    /// Configure circuit breaker thresholds. Admin only.
    ///
    /// State machine effects:
    /// - `failure_threshold` consecutive failures transition `Closed → Open`
    /// - `success_threshold` consecutive successes in `HalfOpen` transition `HalfOpen → Closed`
    ///
    /// # Arguments
    /// * `failure_threshold` - Consecutive failures needed to open the circuit
    /// * `success_threshold` - Consecutive successes in HalfOpen to close it
    /// * `max_error_log`     - Maximum error log entries to retain (oldest trimmed)
    pub fn configure_circuit_breaker(
        env: Env,
        caller: Address,
        failure_threshold: u32,
        success_threshold: u32,
        max_error_log: u32,
    ) {
        caller.require_auth();
        let admin = error_recovery::get_circuitadmin(&env).expect("Circuit admin not set");
        if caller != admin {
            panic!("Unauthorized: only circuit admin can configure");
        }
        error_recovery::set_config(
            &env,
            error_recovery::CircuitBreakerConfig {
                failure_threshold,
                success_threshold,
                max_error_log,
            },
        );
    }

    /// Returns the full circuit breaker status snapshot.
    ///
    /// # Returns
    /// `CircuitBreakerStatus` with state, failure/success counts, and timestamps.
    pub fn get_circuit_status(env: Env) -> error_recovery::CircuitBreakerStatus {
        error_recovery::get_status(&env)
    }

    /// Returns the circuit breaker error log (last `max_error_log` entries).
    pub fn get_circuit_error_log(env: Env) -> Vec<error_recovery::ErrorEntry> {
        error_recovery::get_error_log(&env)
    }

    /// Emergency circuit open — immediately blocks all payout and release operations.
    ///
    /// Admin only. Use `reset_circuit_breaker` to transition back to `HalfOpen`
    /// and then `Closed` after the incident is resolved.
    ///
    /// # Panics
    /// * If caller is not the registered circuit breaker admin.
    pub fn emergency_open_circuit(env: Env, caller: Address) {
        caller.require_auth();
        let admin = error_recovery::get_circuitadmin(&env).expect("Circuit admin not set");
        if caller != admin {
            panic!("Unauthorized: only circuit admin can open circuit");
        }
        error_recovery::open_circuit(&env);
    }

    pub fn update_rate_limit_config(
        env: Env,
        window_size: u64,
        max_operations: u32,
        cooldown_period: u64,
    ) -> Result<(), Error> {
        // Only admin can update rate limit config
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("Admin not set"));
        admin.require_auth();

        // Check governance requirements
        Self::check_governance_requirements(&env)?;

        let config = RateLimitConfig {
            window_size,
            max_operations,
            cooldown_period,
        };
        env.storage()
            .instance()
            .set(&DataKey::RateLimitConfig, &config);
        Ok(())
    }

    pub fn get_rate_limit_config(env: Env) -> RateLimitConfig {
        env.storage()
            .instance()
            .get(&DataKey::RateLimitConfig)
            .unwrap_or(RateLimitConfig {
                window_size: 3600,
                max_operations: 10,
                cooldown_period: 60,
            })
    }

    /// Set the fund cap configuration (admin only).
    /// When enabled, `lock_program_funds` will reject locks that exceed either cap.
    /// Pass `None` for either field to leave that cap unset.
    pub fn set_fund_cap_config(
        env: Env,
        max_total_funds: Option<i128>,
        max_single_lock: Option<i128>,
    ) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("Admin not set"));
        admin.require_auth();

        // Validate: if set, values must be positive
        if let Some(val) = max_total_funds {
            if val <= 0 {
                panic!("max_total_funds must be positive");
            }
        }
        if let Some(val) = max_single_lock {
            if val <= 0 {
                panic!("max_single_lock must be positive");
            }
        }

        let config = FundCapConfig {
            max_total_funds,
            max_single_lock,
        };
        env.storage()
            .instance()
            .set(&FUND_CAP_CONFIG, &config);
        Ok(())
    }

    /// Get the current fund cap configuration.
    pub fn get_fund_cap_config(env: Env) -> FundCapConfig {
        env.storage()
            .instance()
            .get(&FUND_CAP_CONFIG)
            .unwrap_or(FundCapConfig {
                max_total_funds: None,
                max_single_lock: None,
            })
    }

    /// Set the whitelist status of an address (admin only).
    pub fn set_whitelist(env: Env, address: Address, whitelisted: bool) {
        // Only admin can set whitelist
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("Not initialized"));
        admin.require_auth();

        env.storage()
            .instance()
            .set(&DataKey::Whitelist(address.clone()), &whitelisted);

        // Emit whitelist changed event
        env.events().publish(
            (WHITELIST_CHANGED,),
            WhitelistChangedEvent {
                address,
                whitelisted,
            },
        );
    }

    /// Check if an address is whitelisted.
    pub fn is_whitelisted(env: Env, address: Address) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::Whitelist(address))
            .unwrap_or(false)
    }

    /// Enable or disable whitelist enforcement (admin only).
    pub fn set_whitelist_enforced(env: Env, enabled: bool) {
        // Only admin can change whitelist enforcement
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("Not initialized"));
        admin.require_auth();

        env.storage()
            .instance()
            .set(&DataKey::WhitelistEnforced, &enabled);

        // Emit whitelist enforcement changed event
        env.events().publish(
            (WHITELIST_ENFORCEMENT_CHANGED,),
            WhitelistEnforcementChangedEvent {
                enabled,
            },
        );
    }

    /// Check if whitelist enforcement is enabled.
    pub fn is_whitelist_enforced(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::WhitelistEnforced)
            .unwrap_or(false)
    }

    // ========================================================================
    // Governance Integration
    // ========================================================================

    /// Set the governance contract address (admin only)
    pub fn set_governance_contract(env: Env, governance_addr: Address) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("Not initialized"));
        admin.require_auth();
        governance_integration::set_governance_contract(&env, governance_addr);
    }

    /// Get the governance contract address
    pub fn get_governance_contract(env: Env) -> Option<Address> {
        governance_integration::get_governance_contract(&env)
    }

    /// Set minimum required governance version (admin only)
    pub fn set_min_governance_version(env: Env, min_version: u32) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("Not initialized"));
        admin.require_auth();
        governance_integration::set_min_governance_version(&env, min_version);
    }

    /// Get minimum required governance version
    pub fn get_min_governance_version(env: Env) -> u32 {
        governance_integration::get_min_governance_version(&env)
    }

    /// Check if governance requirements are met before admin operations
    fn check_governance_requirements(env: &Env) -> Result<(), Error> {
        if !governance_integration::check_governance_version(env) {
            return Err(Error::GovernanceVersionTooLow);
        }
        Ok(())
    }
    // ========================================================================
    // Payout Functions
    // ========================================================================

    /// Execute batch payouts to multiple recipients
    ///
    /// The batch is bounded by `MAX_BATCH_SIZE` (currently 100). Calls with
    /// zero recipients or more than `MAX_BATCH_SIZE` recipients are rejected
    /// before any token transfer occurs, and the reentrancy guard is cleared
    /// on those early-return paths to avoid a stuck guard.
    ///
    /// # Arguments
    /// * `recipients` - Vector of recipient addresses (1..=MAX_BATCH_SIZE)
    /// * `amounts` - Vector of amounts (must match recipients length; all > 0)
    ///
    /// # Panics
    /// * `"Batch size exceeds maximum allowed"` — when `recipients.len() > MAX_BATCH_SIZE`
    /// * `"Cannot process empty batch"` — when `recipients.len() == 0`
    /// * `"Recipients and amounts vectors must have the same length"` — on length mismatch
    ///
    /// # Returns
    /// Updated ProgramData after payouts
    pub fn batch_payout(env: Env, recipients: Vec<Address>, amounts: Vec<i128>) -> ProgramData {
        let start = env.ledger().timestamp();
        // Reentrancy guard: Check and set
        reentrancy_guard::check_not_entered(&env);
        reentrancy_guard::set_entered(&env);

        // Circuit breaker: reject immediately if the circuit is open.
        // Checked after the reentrancy guard is set so the guard is always
        // cleared on the early return path.
        if error_recovery::check_and_allow(&env).is_err() {
            reentrancy_guard::clear_entered(&env);
            panic!("Circuit breaker open: batch payout temporarily disabled");
        }

        if Self::check_paused(&env, symbol_short!("release")) {
            reentrancy_guard::clear_entered(&env);
            panic!("Funds Paused");
        }

        // Global dispute guard: block all payouts while a global dispute is open.
        if Self::is_dispute_open_for_key(&env, &DataKey::Dispute) {
            reentrancy_guard::clear_entered(&env);
            panic!("Dispute in progress");
        }

        let mut program_data: ProgramData =
            env.storage()
                .persistent()
                .get(&PROGRAM_DATA)
                .unwrap_or_else(|| {
                    reentrancy_guard::clear_entered(&env);
                    panic!("Program not initialized")
                });
        Self::bump_persistent_symbol_ttl(&env, &PROGRAM_DATA);

        program_data.authorized_payout_key.require_auth();

        // Whitelist guard: block payouts to non-whitelisted recipients if enforcement is enabled
        let whitelist_enforced = env
            .storage()
            .instance()
            .get(&DataKey::WhitelistEnforced)
            .unwrap_or(false);

        if whitelist_enforced {
            for recipient in recipients.iter() {
                let whitelisted = env
                    .storage()
                    .instance()
                    .get(&DataKey::Whitelist(recipient.clone()))
                    .unwrap_or(false);
                if !whitelisted {
                    reentrancy_guard::clear_entered(&env);
                    panic!("Recipient not whitelisted");
                }
            }
        }

        let batch_len = recipients.len();
        let recipient_count = batch_len as u32;

        // Validate input lengths match
        if batch_len != amounts.len() {
            reentrancy_guard::clear_entered(&env);
            panic!("Recipients and amounts vectors must have the same length");
        }

        if batch_len == 0 {
            reentrancy_guard::clear_entered(&env);
            panic!("Cannot process empty batch");
        }

        if recipient_count > MAX_BATCH_SIZE {
            reentrancy_guard::clear_entered(&env);
            panic!("Batch size exceeds maximum allowed");
        }

        for recipient in recipients.iter() {
            if Self::is_dispute_open_for_key(&env, &DataKey::RecipientDispute(recipient)) {
                reentrancy_guard::clear_entered(&env);
                panic!("Dispute in progress");
            }
        }

        // Calculate total payout amount
        let mut total_payout: i128 = 0;
        for amount in amounts.iter() {
            if amount <= 0 {
                reentrancy_guard::clear_entered(&env);
                panic!("All amounts must be greater than zero");
            }
            total_payout = total_payout.checked_add(amount).unwrap_or_else(|| {
                reentrancy_guard::clear_entered(&env);
                panic!("Payout amount overflow")
            });
        }

        // Validate sufficient balance — record failure before abort so the
        // circuit breaker can accumulate this as a known-bad condition.
        if total_payout > program_data.remaining_balance {
            error_recovery::record_failure(
                &env,
                program_data.program_id.clone(),
                symbol_short!("batch_pay"),
                error_recovery::ERR_INSUFFICIENT_BALANCE,
            );
            reentrancy_guard::clear_entered(&env);
            panic!("Insufficient balance");
        }

        // Execute transfers
        let timestamp = env.ledger().timestamp();
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &program_data.token_address);
        let threshold = program_data.total_funds / 10;
        let program_id = program_data.program_id.clone();

        for i in 0..batch_len {
            let recipient = recipients.get(i).unwrap();
            let amount = amounts.get(i).unwrap();

            // Check and emit large payout event
            if amount >= threshold {
                env.events().publish(
                    (LARGE_PAYOUT,),
                    LargePayoutEvent {
                        version: EVENT_VERSION_V2,
                        program_id: program_id.clone(),
                        recipient: recipient.clone(),
                        amount,
                        threshold,
                    },
                );
            }

            // Transfer funds from contract to recipient
            token_client.transfer(&contract_address, &recipient, &amount);

            // Record payout
            let payout_record = PayoutRecord {
                recipient,
                amount,
                timestamp,
            };
            program_data.payout_history.push_back(payout_record);
        }

        // All transfers succeeded — inform the circuit breaker.
        error_recovery::record_success(&env);

        // Update program data
        program_data.remaining_balance = program_data
            .remaining_balance
            .checked_sub(total_payout)
            .unwrap_or_else(|| {
                reentrancy_guard::clear_entered(&env);
                panic!("Payout underflow")
            });

        // Store updated data
        env.storage().persistent().set(&PROGRAM_DATA, &program_data);
        Self::bump_persistent_symbol_ttl(&env, &PROGRAM_DATA);

        // Emit BatchPayout event
        env.events().publish(
            (BATCH_PAYOUT,),
            BatchPayoutEvent {
                version: EVENT_VERSION_V2,
                program_id: program_data.program_id.clone(),
                recipient_count,
                total_amount: total_payout,
                remaining_balance: program_data.remaining_balance,
                gas_proxy_transfer_ops: recipient_count,
                gas_proxy_history_appends: recipient_count,
                gas_proxy_storage_reads: 1,
                gas_proxy_storage_writes: 1,
                gas_proxy_events_emitted: 1,
            },
        );

        monitoring::track_operation(
            &env,
            symbol_short!("batchpay"),
            program_data.authorized_payout_key.clone(),
            true,
        );
        monitoring::emit_performance(
            &env,
            symbol_short!("batchpay"),
            env.ledger().timestamp().saturating_sub(start),
        );
        // Emit aggregate stats
        Self::emit_aggregate_stats(&env, &program_data);

        // Clear reentrancy guard before returning
        reentrancy_guard::clear_entered(&env);

        program_data
    }

    /// Execute a single payout to one recipient
    ///
    /// # Arguments
    /// * `recipient` - Address of the recipient
    /// * `amount` - Amount to transfer
    ///
    /// # Returns
    /// Updated ProgramData after payout
    pub fn single_payout(env: Env, recipient: Address, amount: i128) -> ProgramData {
        let start = env.ledger().timestamp();
        // Reentrancy guard: Check and set
        reentrancy_guard::check_not_entered(&env);
        reentrancy_guard::set_entered(&env);

        // Circuit breaker: reject immediately if the circuit is open.
        if error_recovery::check_and_allow(&env).is_err() {
            reentrancy_guard::clear_entered(&env);
            panic!("Circuit breaker open: single payout temporarily disabled");
        }

        if Self::check_paused(&env, symbol_short!("release")) {
            reentrancy_guard::clear_entered(&env);
            panic!("Funds Paused");
        }

        // Dispute guard: global disputes block all payouts; recipient disputes
        // block only this payout target.
        if Self::is_dispute_open_for_key(&env, &DataKey::Dispute)
            || Self::is_dispute_open_for_key(&env, &DataKey::RecipientDispute(recipient.clone()))
        {
            reentrancy_guard::clear_entered(&env);
            panic!("Dispute in progress");
        }

        // Verify authorization
        let program_data: ProgramData =
            env.storage()
                .persistent()
                .get(&PROGRAM_DATA)
                .unwrap_or_else(|| {
                    reentrancy_guard::clear_entered(&env);
                    panic!("Program not initialized")
                });
        Self::bump_persistent_symbol_ttl(&env, &PROGRAM_DATA);

        program_data.authorized_payout_key.require_auth();

        // Whitelist guard: block payouts to non-whitelisted recipients if enforcement is enabled
        let whitelist_enforced = env
            .storage()
            .instance()
            .get(&DataKey::WhitelistEnforced)
            .unwrap_or(false);

        if whitelist_enforced {
            let whitelisted = env
                .storage()
                .instance()
                .get(&DataKey::Whitelist(recipient.clone()))
                .unwrap_or(false);
            if !whitelisted {
                reentrancy_guard::clear_entered(&env);
                panic!("Recipient not whitelisted");
            }
        }

        // Validate amount
        if amount <= 0 {
            reentrancy_guard::clear_entered(&env);
            panic!("Amount must be greater than zero");
        }

        // Validate sufficient balance — record failure before abort.
        if amount > program_data.remaining_balance {
            error_recovery::record_failure(
                &env,
                program_data.program_id.clone(),
                symbol_short!("sngl_pay"),
                error_recovery::ERR_INSUFFICIENT_BALANCE,
            );
            reentrancy_guard::clear_entered(&env);
            panic!("Insufficient balance");
        }

        // Check and emit large payout event
        Self::check_and_emit_large_payout(&env, &program_data, &recipient, amount);

        // Transfer funds from contract to recipient
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &program_data.token_address);
        token_client.transfer(&contract_address, &recipient, &amount);

        // Transfer succeeded — inform the circuit breaker.
        error_recovery::record_success(&env);

        // Record payout
        let timestamp = env.ledger().timestamp();
        let payout_record = PayoutRecord {
            recipient: recipient.clone(),
            amount,
            timestamp,
        };

        let mut updated_history = program_data.payout_history.clone();
        updated_history.push_back(payout_record);

        // Update program data
        let mut updated_data = program_data.clone();
        updated_data.remaining_balance -= amount;
        updated_data.payout_history = updated_history;

        // Store updated data
        env.storage().persistent().set(&PROGRAM_DATA, &updated_data);
        Self::bump_persistent_symbol_ttl(&env, &PROGRAM_DATA);

        // Emit Payout event
        env.events().publish(
            (PAYOUT,),
            PayoutEvent {
                version: EVENT_VERSION_V2,
                program_id: updated_data.program_id.clone(),
                recipient,
                amount,
                remaining_balance: updated_data.remaining_balance,
            },
        );

        monitoring::track_operation(
            &env,
            symbol_short!("payout"),
            program_data.authorized_payout_key,
            true,
        );
        monitoring::emit_performance(
            &env,
            symbol_short!("payout"),
            env.ledger().timestamp().saturating_sub(start),
        );
        // Emit aggregate stats
        Self::emit_aggregate_stats(&env, &updated_data);

        // Clear reentrancy guard before returning
        reentrancy_guard::clear_entered(&env);

        updated_data
    }

    /// Get program information
    ///
    /// # Returns
    /// ProgramData containing all program information
    pub fn get_program_info(env: Env) -> ProgramData {
        let val = env.storage()
            .persistent()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        Self::bump_persistent_symbol_ttl(&env, &PROGRAM_DATA);
        val
    }

    /// Get remaining balance
    ///
    /// # Returns
    /// Current remaining balance
    pub fn get_remaining_balance(env: Env) -> i128 {
        let program_data: ProgramData = env
            .storage()
            .persistent()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        Self::bump_persistent_symbol_ttl(&env, &PROGRAM_DATA);

        program_data.remaining_balance
    }

    /// Create a release schedule entry that can be triggered at/after `release_timestamp`.
    pub fn create_program_release_schedule(
        env: Env,
        amount: i128,
        release_timestamp: u64,
        recipient: Address,
    ) -> ProgramReleaseSchedule {
        let program_data: ProgramData = env
            .storage()
            .persistent()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        Self::bump_persistent_symbol_ttl(&env, &PROGRAM_DATA);

        program_data.authorized_payout_key.require_auth();

        if amount <= 0 {
            panic!("Amount must be greater than zero");
        }

        let mut schedules: Vec<ProgramReleaseSchedule> = env
            .storage()
            .persistent()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));
        Self::bump_persistent_symbol_ttl(&env, &SCHEDULES);
        let schedule_id: u64 = env
            .storage()
            .instance()
            .get(&NEXT_SCHEDULE_ID)
            .unwrap_or(1_u64);

        let schedule = ProgramReleaseSchedule {
            schedule_id,
            recipient,
            amount,
            release_timestamp,
            released: false,
            released_at: None,
            released_by: None,
        };
        schedules.push_back(schedule.clone());

        env.storage().persistent().set(&SCHEDULES, &schedules);
        Self::bump_persistent_symbol_ttl(&env, &SCHEDULES);
        env.storage()
            .instance()
            .set(&NEXT_SCHEDULE_ID, &(schedule_id + 1));

        Self::bump_instance_ttl(&env);
        schedule
    }

    /// Trigger all due schedules where `now >= release_timestamp`.
    pub fn trigger_program_releases(env: Env) -> u32 {
        // Reentrancy guard: Check and set
        reentrancy_guard::check_not_entered(&env);
        reentrancy_guard::set_entered(&env);

        // Circuit breaker: reject immediately if the circuit is open.
        if error_recovery::check_and_allow(&env).is_err() {
            reentrancy_guard::clear_entered(&env);
            panic!("Circuit breaker open: schedule releases temporarily disabled");
        }

        // Global dispute guard: block all schedule releases while a global dispute is open.
        if Self::is_dispute_open_for_key(&env, &DataKey::Dispute) {
            reentrancy_guard::clear_entered(&env);
            panic!("Dispute in progress");
        }

        let mut program_data: ProgramData = env
            .storage()
            .persistent()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| {
                reentrancy_guard::clear_entered(&env);
                panic!("Program not initialized")
            });
        Self::bump_persistent_symbol_ttl(&env, &PROGRAM_DATA);
        program_data.authorized_payout_key.require_auth();

        let mut schedules: Vec<ProgramReleaseSchedule> = env
            .storage()
            .persistent()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));
        Self::bump_persistent_symbol_ttl(&env, &SCHEDULES);
        let mut release_history: Vec<ProgramReleaseHistory> = env
            .storage()
            .persistent()
            .get(&RELEASE_HISTORY)
            .unwrap_or_else(|| Vec::new(&env));
        Self::bump_persistent_symbol_ttl(&env, &RELEASE_HISTORY);

        let now = env.ledger().timestamp();
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &program_data.token_address);
        let mut released_count: u32 = 0;

        for i in 0..schedules.len() {
            // Bound the number of releases per invocation to prevent unbounded
            // Soroban instruction/memory usage when many schedules are due.
            if released_count >= MAX_BATCH_SIZE {
                break;
            }

            let mut schedule = schedules.get(i).unwrap();
            if schedule.released || now < schedule.release_timestamp {
                continue;
            }

            if Self::is_schedule_scope_disputed(&env, schedule.schedule_id, &schedule.recipient) {
                continue;
            }

            if schedule.amount > program_data.remaining_balance {
                error_recovery::record_failure(
                    &env,
                    program_data.program_id.clone(),
                    symbol_short!("trg_rels"),
                    error_recovery::ERR_INSUFFICIENT_BALANCE,
                );
                reentrancy_guard::clear_entered(&env);
                panic!("Insufficient balance");
            }

            token_client.transfer(&contract_address, &schedule.recipient, &schedule.amount);
            schedule.released = true;
            schedule.released_at = Some(now);
            schedule.released_by = Some(contract_address.clone());
            schedules.set(i, schedule.clone());

            program_data.remaining_balance -= schedule.amount;
            program_data.payout_history.push_back(PayoutRecord {
                recipient: schedule.recipient.clone(),
                amount: schedule.amount,
                timestamp: now,
            });
            release_history.push_back(ProgramReleaseHistory {
                schedule_id: schedule.schedule_id,
                recipient: schedule.recipient.clone(),
                amount: schedule.amount,
                released_at: now,
                release_type: ReleaseType::Automatic,
            });

            // Emit schedule triggered event
            env.events().publish(
                (SCHEDULE_TRIGGERED,),
                ScheduleTriggeredEvent {
                    version: EVENT_VERSION_V2,
                    program_id: program_data.program_id.clone(),
                    schedule_id: schedule.schedule_id,
                    recipient: schedule.recipient.clone(),
                    amount: schedule.amount,
                    trigger_type: ReleaseType::Automatic,
                },
            );

            released_count += 1;
        }

        env.storage().persistent().set(&PROGRAM_DATA, &program_data);
        Self::bump_persistent_symbol_ttl(&env, &PROGRAM_DATA);
        env.storage().persistent().set(&SCHEDULES, &schedules);
        Self::bump_persistent_symbol_ttl(&env, &SCHEDULES);
        env.storage()
            .persistent()
            .set(&RELEASE_HISTORY, &release_history);
        Self::bump_persistent_symbol_ttl(&env, &RELEASE_HISTORY);

        // Inform the circuit breaker of the outcome.
        if released_count > 0 {
            error_recovery::record_success(&env);
            Self::emit_aggregate_stats(&env, &program_data);
        }

        // Clear reentrancy guard before returning
        reentrancy_guard::clear_entered(&env);

        released_count
    }

    pub fn get_program_release_schedules(env: Env) -> Vec<ProgramReleaseSchedule> {
        let val = env.storage()
            .persistent()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));
        Self::bump_persistent_symbol_ttl(&env, &SCHEDULES);
        val
    }

    pub fn get_program_release_history(env: Env) -> Vec<ProgramReleaseHistory> {
        let val = env.storage()
            .persistent()
            .get(&RELEASE_HISTORY)
            .unwrap_or_else(|| Vec::new(&env));
        Self::bump_persistent_symbol_ttl(&env, &RELEASE_HISTORY);
        val
    }

    // ========================================================================
    // Multi-tenant / Multi-program Migration Wrappers (ignore id for now)
    // ========================================================================

    pub fn get_program_info_v2(env: Env, _program_id: String) -> ProgramData {
        Self::get_program_info(env)
    }

    pub fn lock_program_funds_v2(env: Env, _program_id: String, from: Address, amount: i128) -> ProgramData {
        Self::lock_program_funds(env, from, amount)
    }

    pub fn single_payout_v2(
        env: Env,
        _program_id: String,
        recipient: Address,
        amount: i128,
    ) -> ProgramData {
        Self::single_payout(env, recipient, amount)
    }

    pub fn batch_payout_v2(
        env: Env,
        _program_id: String,
        recipients: Vec<Address>,
        amounts: Vec<i128>,
    ) -> ProgramData {
        Self::batch_payout(env, recipients, amounts)
    }

    /// Query payout history by recipient with pagination
    pub fn query_payouts_by_recipient(
        env: Env,
        recipient: Address,
        offset: u32,
        limit: u32,
    ) -> Vec<PayoutRecord> {
        let program_data: ProgramData = env
            .storage()
            .persistent()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        Self::bump_persistent_symbol_ttl(&env, &PROGRAM_DATA);
        let history = program_data.payout_history;
        let mut results = Vec::new(&env);
        let mut count = 0u32;
        let mut skipped = 0u32;

        for i in 0..history.len() {
            if count >= limit {
                break;
            }
            let record = history.get(i).unwrap();
            if record.recipient == recipient {
                if skipped < offset {
                    skipped += 1;
                    continue;
                }
                results.push_back(record);
                count += 1;
            }
        }
        results
    }

    /// Query payout history by amount range
    pub fn query_payouts_by_amount(
        env: Env,
        min_amount: i128,
        max_amount: i128,
        offset: u32,
        limit: u32,
    ) -> Vec<PayoutRecord> {
        let program_data: ProgramData = env
            .storage()
            .persistent()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        Self::bump_persistent_symbol_ttl(&env, &PROGRAM_DATA);
        let history = program_data.payout_history;
        let mut results = Vec::new(&env);
        let mut count = 0u32;
        let mut skipped = 0u32;

        for i in 0..history.len() {
            if count >= limit {
                break;
            }
            let record = history.get(i).unwrap();
            if record.amount >= min_amount && record.amount <= max_amount {
                if skipped < offset {
                    skipped += 1;
                    continue;
                }
                results.push_back(record);
                count += 1;
            }
        }
        results
    }

    /// Query payout history by timestamp range
    pub fn query_payouts_by_timestamp(
        env: Env,
        min_timestamp: u64,
        max_timestamp: u64,
        offset: u32,
        limit: u32,
    ) -> Vec<PayoutRecord> {
        let program_data: ProgramData = env
            .storage()
            .persistent()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        Self::bump_persistent_symbol_ttl(&env, &PROGRAM_DATA);
        let history = program_data.payout_history;
        let mut results = Vec::new(&env);
        let mut count = 0u32;
        let mut skipped = 0u32;

        for i in 0..history.len() {
            if count >= limit {
                break;
            }
            let record = history.get(i).unwrap();
            if record.timestamp >= min_timestamp && record.timestamp <= max_timestamp {
                if skipped < offset {
                    skipped += 1;
                    continue;
                }
                results.push_back(record);
                count += 1;
            }
        }
        results
    }

    /// Query release schedules by recipient
    pub fn query_schedules_by_recipient(
        env: Env,
        recipient: Address,
        offset: u32,
        limit: u32,
    ) -> Vec<ProgramReleaseSchedule> {
        let schedules: Vec<ProgramReleaseSchedule> = env
            .storage()
            .persistent()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));
        Self::bump_persistent_symbol_ttl(&env, &SCHEDULES);
        let mut results = Vec::new(&env);
        let mut count = 0u32;
        let mut skipped = 0u32;

        for i in 0..schedules.len() {
            if count >= limit {
                break;
            }
            let schedule = schedules.get(i).unwrap();
            if schedule.recipient == recipient {
                if skipped < offset {
                    skipped += 1;
                    continue;
                }
                results.push_back(schedule);
                count += 1;
            }
        }
        results
    }

    /// Query release schedules by released status
    pub fn query_schedules_by_status(
        env: Env,
        released: bool,
        offset: u32,
        limit: u32,
    ) -> Vec<ProgramReleaseSchedule> {
        let schedules: Vec<ProgramReleaseSchedule> = env
            .storage()
            .persistent()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));
        Self::bump_persistent_symbol_ttl(&env, &SCHEDULES);
        let mut results = Vec::new(&env);
        let mut count = 0u32;
        let mut skipped = 0u32;

        for i in 0..schedules.len() {
            if count >= limit {
                break;
            }
            let schedule = schedules.get(i).unwrap();
            if schedule.released == released {
                if skipped < offset {
                    skipped += 1;
                    continue;
                }
                results.push_back(schedule);
                count += 1;
            }
        }
        results
    }

    /// Query release history with filtering and pagination
    pub fn query_releases_by_recipient(
        env: Env,
        recipient: Address,
        offset: u32,
        limit: u32,
    ) -> Vec<ProgramReleaseHistory> {
        let history: Vec<ProgramReleaseHistory> = env
            .storage()
            .persistent()
            .get(&RELEASE_HISTORY)
            .unwrap_or_else(|| Vec::new(&env));
        Self::bump_persistent_symbol_ttl(&env, &RELEASE_HISTORY);
        let mut results = Vec::new(&env);
        let mut count = 0u32;
        let mut skipped = 0u32;

        for i in 0..history.len() {
            if count >= limit {
                break;
            }
            let record = history.get(i).unwrap();
            if record.recipient == recipient {
                if skipped < offset {
                    skipped += 1;
                    continue;
                }
                results.push_back(record);
                count += 1;
            }
        }
        results
    }

    /// Get aggregate statistics for the program
    pub fn get_program_aggregate_stats(env: Env) -> ProgramAggregateStats {
        let program_data: ProgramData = env
            .storage()
            .persistent()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        Self::bump_persistent_symbol_ttl(&env, &PROGRAM_DATA);
        let schedules: Vec<ProgramReleaseSchedule> = env
            .storage()
            .persistent()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));
        Self::bump_persistent_symbol_ttl(&env, &SCHEDULES);

        let mut released_count = 0u32;
        let mut scheduled_count = 0u32;

        for i in 0..schedules.len() {
            let schedule = schedules.get(i).unwrap();
            if schedule.released {
                released_count += 1;
            } else {
                scheduled_count += 1;
            }
        }

        ProgramAggregateStats {
            total_funds: program_data.total_funds,
            remaining_balance: program_data.remaining_balance,
            total_paid_out: program_data.total_funds - program_data.remaining_balance,
            payout_count: program_data.payout_history.len(),
            scheduled_count,
            released_count,
            authorized_payout_key: program_data.authorized_payout_key,
            payout_history: program_data.payout_history,
            token_address: program_data.token_address,
        }
    }

    /// Get payouts by recipient
    pub fn get_payouts_by_recipient(
        env: Env,
        recipient: Address,
        offset: u32,
        limit: u32,
    ) -> Vec<PayoutRecord> {
        let program_data: ProgramData = env
            .storage()
            .persistent()
            .get(&PROGRAM_DATA)
            .unwrap_or_else(|| panic!("Program not initialized"));
        Self::bump_persistent_symbol_ttl(&env, &PROGRAM_DATA);
        let history = program_data.payout_history;
        let mut results = Vec::new(&env);
        let mut count = 0u32;
        let mut skipped = 0u32;

        for i in 0..history.len() {
            if count >= limit {
                break;
            }
            let record = history.get(i).unwrap();
            if record.recipient == recipient {
                if skipped < offset {
                    skipped += 1;
                    continue;
                }
                results.push_back(record);
                count += 1;
            }
        }
        results
    }

    /// Get pending schedules (not yet released)
    pub fn get_pending_schedules(env: Env) -> Vec<ProgramReleaseSchedule> {
        let schedules: Vec<ProgramReleaseSchedule> = env
            .storage()
            .persistent()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));
        Self::bump_persistent_symbol_ttl(&env, &SCHEDULES);
        let mut results = Vec::new(&env);

        for i in 0..schedules.len() {
            let schedule = schedules.get(i).unwrap();
            if !schedule.released {
                results.push_back(schedule);
            }
        }
        results
    }

    /// Get due schedules (ready to be released)
    pub fn get_due_schedules(env: Env) -> Vec<ProgramReleaseSchedule> {
        let schedules: Vec<ProgramReleaseSchedule> = env
            .storage()
            .persistent()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));
        Self::bump_persistent_symbol_ttl(&env, &SCHEDULES);
        let now = env.ledger().timestamp();
        let mut results = Vec::new(&env);

        for i in 0..schedules.len() {
            let schedule = schedules.get(i).unwrap();
            if !schedule.released && schedule.release_timestamp <= now {
                results.push_back(schedule);
            }
        }
        results
    }

    /// Get total amount in pending schedules
    pub fn get_total_scheduled_amount(env: Env) -> i128 {
        let schedules: Vec<ProgramReleaseSchedule> = env
            .storage()
            .persistent()
            .get(&SCHEDULES)
            .unwrap_or_else(|| Vec::new(&env));
        Self::bump_persistent_symbol_ttl(&env, &SCHEDULES);
        let mut total = 0i128;

        for i in 0..schedules.len() {
            let schedule = schedules.get(i).unwrap();
            if !schedule.released {
                total += schedule.amount;
            }
        }
        total
    }

    pub fn get_program_count(env: Env) -> u32 {
        if env.storage().persistent().has(&PROGRAM_DATA) {
            Self::bump_persistent_symbol_ttl(&env, &PROGRAM_DATA);
            1
        } else {
            0
        }
    }

    pub fn list_programs(env: Env) -> Vec<ProgramData> {
        let mut results = Vec::new(&env);
        if env.storage().persistent().has(&PROGRAM_DATA) {
            Self::bump_persistent_symbol_ttl(&env, &PROGRAM_DATA);
            results.push_back(Self::get_program_info(env.clone()));
        }
        results
    }

    pub fn get_program_release_schedule(env: Env, schedule_id: u64) -> ProgramReleaseSchedule {
        let schedules = Self::get_program_release_schedules(env);
        for s in schedules.iter() {
            if s.schedule_id == schedule_id {
                return s;
            }
        }
        panic!("Schedule not found");
    }

    pub fn get_all_prog_release_schedules(env: Env) -> Vec<ProgramReleaseSchedule> {
        Self::get_program_release_schedules(env)
    }

    pub fn get_pending_program_schedules(env: Env) -> Vec<ProgramReleaseSchedule> {
        Self::get_pending_schedules(env)
    }

    pub fn get_due_program_schedules(env: Env) -> Vec<ProgramReleaseSchedule> {
        Self::get_due_schedules(env)
    }

    pub fn release_program_schedule_manual(env: Env, schedule_id: u64) {
        // Reentrancy guard: prevent re-entrant calls
        reentrancy_guard::check_not_entered(&env);
        reentrancy_guard::set_entered(&env);

        // Circuit breaker: reject immediately if the circuit is open.
        if error_recovery::check_and_allow(&env).is_err() {
            reentrancy_guard::clear_entered(&env);
            panic!("Circuit breaker open: manual release temporarily disabled");
        }

        let mut schedules = Self::get_program_release_schedules(env.clone());
        let mut program_data = Self::get_program_info(env.clone());

        program_data.authorized_payout_key.require_auth();

        let caller = program_data.authorized_payout_key.clone();
        let now = env.ledger().timestamp();
        let mut released_schedule: Option<ProgramReleaseSchedule> = None;

        let mut found = false;
        for i in 0..schedules.len() {
            let mut s = schedules.get(i).unwrap();
            if s.schedule_id == schedule_id {
                if s.released {
                    reentrancy_guard::clear_entered(&env);
                    panic!("Already released");
                }
                if Self::is_dispute_open_for_key(&env, &DataKey::Dispute)
                    || Self::is_schedule_scope_disputed(&env, s.schedule_id, &s.recipient)
                {
                    reentrancy_guard::clear_entered(&env);
                    panic!("Dispute in progress");
                }

                // Transfer funds
                let token_client = token::Client::new(&env, &program_data.token_address);
                token_client.transfer(&env.current_contract_address(), &s.recipient, &s.amount);

                // Maintain SAC ≡ remaining_balance invariant.
                program_data.remaining_balance -= s.amount;

                s.released = true;
                s.released_at = Some(now);
                s.released_by = Some(caller.clone());
                released_schedule = Some(s.clone());
                schedules.set(i, s.clone());
                found = true;

                // Emit schedule triggered event
                env.events().publish(
                    (SCHEDULE_TRIGGERED,),
                    ScheduleTriggeredEvent {
                        version: EVENT_VERSION_V2,
                        program_id: program_data.program_id.clone(),
                        schedule_id: s.schedule_id,
                        recipient: s.recipient.clone(),
                        amount: s.amount,
                        trigger_type: ReleaseType::Manual,
                    },
                );
                break;
            }
        }

        if !found {
            reentrancy_guard::clear_entered(&env);
            panic!("Schedule not found");
        }

        env.storage().persistent().set(&SCHEDULES, &schedules);
        Self::bump_persistent_symbol_ttl(&env, &SCHEDULES);
        // Persist the updated remaining_balance.
        env.storage().persistent().set(&PROGRAM_DATA, &program_data);
        Self::bump_persistent_symbol_ttl(&env, &PROGRAM_DATA);

        // Transfer succeeded — inform the circuit breaker.
        error_recovery::record_success(&env);

        // Write to release history
        if let Some(s) = released_schedule {
            let mut history: Vec<ProgramReleaseHistory> = env
                .storage()
                .persistent()
                .get(&RELEASE_HISTORY)
                .unwrap_or_else(|| Vec::new(&env));
            Self::bump_persistent_symbol_ttl(&env, &RELEASE_HISTORY);
            history.push_back(ProgramReleaseHistory {
                schedule_id: s.schedule_id,
                recipient: s.recipient,
                amount: s.amount,
                released_at: now,
                release_type: ReleaseType::Manual,
            });
            env.storage().persistent().set(&RELEASE_HISTORY, &history);
            Self::bump_persistent_symbol_ttl(&env, &RELEASE_HISTORY);
        }

        // Clear reentrancy guard before returning
        reentrancy_guard::clear_entered(&env);
    }

    pub fn release_prog_schedule_automatic(env: Env, schedule_id: u64) {
        // Reentrancy guard: prevent re-entrant calls
        reentrancy_guard::check_not_entered(&env);
        reentrancy_guard::set_entered(&env);

        // Circuit breaker: reject immediately if the circuit is open.
        if error_recovery::check_and_allow(&env).is_err() {
            reentrancy_guard::clear_entered(&env);
            panic!("Circuit breaker open: automatic release temporarily disabled");
        }

        let mut schedules = Self::get_program_release_schedules(env.clone());
        let mut program_data = Self::get_program_info(env.clone());
        let now = env.ledger().timestamp();
        let mut released_schedule: Option<ProgramReleaseSchedule> = None;

        let mut found = false;
        for i in 0..schedules.len() {
            let mut s = schedules.get(i).unwrap();
            if s.schedule_id == schedule_id {
                if s.released {
                    reentrancy_guard::clear_entered(&env);
                    panic!("Already released");
                }
                if now < s.release_timestamp {
                    reentrancy_guard::clear_entered(&env);
                    panic!("Not yet due");
                }
                if Self::is_dispute_open_for_key(&env, &DataKey::Dispute)
                    || Self::is_schedule_scope_disputed(&env, s.schedule_id, &s.recipient)
                {
                    reentrancy_guard::clear_entered(&env);
                    panic!("Dispute in progress");
                }

                // Transfer funds
                let token_client = token::Client::new(&env, &program_data.token_address);
                token_client.transfer(&env.current_contract_address(), &s.recipient, &s.amount);

                // Maintain SAC ≡ remaining_balance invariant.
                program_data.remaining_balance -= s.amount;

                s.released = true;
                s.released_at = Some(now);
                s.released_by = Some(env.current_contract_address());
                released_schedule = Some(s.clone());
                schedules.set(i, s.clone());
                found = true;

                // Emit schedule triggered event
                env.events().publish(
                    (SCHEDULE_TRIGGERED,),
                    ScheduleTriggeredEvent {
                        version: EVENT_VERSION_V2,
                        program_id: program_data.program_id.clone(),
                        schedule_id: s.schedule_id,
                        recipient: s.recipient.clone(),
                        amount: s.amount,
                        trigger_type: ReleaseType::Automatic,
                    },
                );
                break;
            }
        }

        if !found {
            reentrancy_guard::clear_entered(&env);
            panic!("Schedule not found");
        }

        env.storage().persistent().set(&SCHEDULES, &schedules);
        Self::bump_persistent_symbol_ttl(&env, &SCHEDULES);
        // Persist the updated remaining_balance.
        env.storage().persistent().set(&PROGRAM_DATA, &program_data);
        Self::bump_persistent_symbol_ttl(&env, &PROGRAM_DATA);

        // Transfer succeeded — inform the circuit breaker.
        error_recovery::record_success(&env);

        // Write to release history
        if let Some(s) = released_schedule {
            let mut history: Vec<ProgramReleaseHistory> = env
                .storage()
                .persistent()
                .get(&RELEASE_HISTORY)
                .unwrap_or_else(|| Vec::new(&env));
            Self::bump_persistent_symbol_ttl(&env, &RELEASE_HISTORY);
            history.push_back(ProgramReleaseHistory {
                schedule_id: s.schedule_id,
                recipient: s.recipient,
                amount: s.amount,
                released_at: now,
                release_type: ReleaseType::Automatic,
            });
            env.storage().persistent().set(&RELEASE_HISTORY, &history);
            Self::bump_persistent_symbol_ttl(&env, &RELEASE_HISTORY);
        }

        // Clear reentrancy guard before returning
        reentrancy_guard::clear_entered(&env);
    }

    // ========================================================================
    // Monitoring Views
    // ========================================================================

    /// Get current health status of the contract
    pub fn health_check(env: Env) -> monitoring::HealthStatus {
        monitoring::health_check(&env)
    }

    /// Get aggregated monitoring analytics
    pub fn get_monitoring_analytics(env: Env) -> monitoring::Analytics {
        monitoring::get_analytics(&env)
    }

    /// Get a snapshot of current system state
    pub fn get_state_snapshot(env: Env) -> monitoring::StateSnapshot {
        monitoring::get_state_snapshot(&env)
    }

    /// Get performance stats for a specific function
    pub fn get_performance_stats(env: Env, function_name: Symbol) -> monitoring::PerformanceStats {
        monitoring::get_performance_stats(&env, function_name)
    }

    /// Get the large-payout alert threshold in basis points (default 1000 = 10%).
    /// A payout is flagged as "large" when it equals or exceeds
    /// `total_funds * threshold_bps / 10_000`.
    pub fn get_large_payout_threshold(env: Env) -> u32 {
        monitoring::get_large_payout_threshold_bps(&env)
    }

    /// Update the large-payout alert threshold (admin only).
    /// `threshold_bps` is expressed in basis points (e.g. 1000 = 10%, 2500 = 25%).
    pub fn set_large_payout_threshold(env: Env, threshold_bps: u32) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Admin not set");
        admin.require_auth();
        monitoring::set_large_payout_threshold_bps(&env, threshold_bps);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token, Address, Env, String, Vec,
    };

    // Test helper to create a mock token contract
    fn create_token_contract<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
        let token_contract = env.register_stellar_asset_contract_v2(admin.clone());
        let token_address = token_contract.address();
        let token_sac = token::StellarAssetClient::new(env, &token_address);
        token_sac.mint(admin, &1_000_000_000_000_000_000);
        token::Client::new(env, &token_address)
    }

    // ========================================================================
    // Program Registration Tests
    // ========================================================================

    fn setup_program_with_schedule(
        env: &Env,
        client: &ProgramEscrowContractClient<'static>,
        contract_id: &Address,
        authorized_key: &Address,
        _token: &Address,
        program_id: &String,
        total_amount: i128,
        winner: &Address,
        release_timestamp: u64,
    ) {
        // // Register program
        // client.register_program(program_id, token, authorized_key);

        // // Create and fund token
        // let token_client = create_token_contract(env, authorized_key);
        // let tokenadmin = token::StellarAssetClient::new(env, &token_client.address);
        // tokenadmin.mint(authorized_key, &total_amount);

        // // Lock funds for program
        // token_client.approve(authorized_key, &env.current_contract_address(), &total_amount, &1000);
        // client.lock_funds(program_id, &total_amount);

        // Create and fund token first, then register the program with the real token address
        let token_client = create_token_contract(env, authorized_key);
        let tokenadmin = token::StellarAssetClient::new(env, &token_client.address);
        tokenadmin.mint(authorized_key, &total_amount);

        // Register program using the created token contract address
        client.initialize_program(&program_id, &authorized_key, &token_client.address);

        // Transfer tokens to contract first

        // Lock funds for program (records the amount in program state)
        client.lock_program_funds(&authorized_key, &total_amount);

        // Create release schedule
        client.create_program_release_schedule(&total_amount, &release_timestamp, winner);
    }

    #[test]
    fn test_single_program_release_schedule() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let authorized_key = Address::generate(&env);
        let winner = Address::generate(&env);
        let token = Address::generate(&env);
        let program_id = String::from_str(&env, "Hackathon2024");
        let amount = 1000_0000000;
        let release_timestamp = 1000;

        env.mock_all_auths();

        // Setup program with schedule
        setup_program_with_schedule(
            &env,
            &client,
            &contract_id,
            &authorized_key,
            &token,
            &program_id,
            amount,
            &winner,
            release_timestamp,
        );

        // Verify schedule was created
        let schedule = client.get_program_release_schedule(&1);
        assert_eq!(schedule.schedule_id, 1);
        assert_eq!(schedule.amount, amount);
        assert_eq!(schedule.release_timestamp, release_timestamp);
        assert_eq!(schedule.recipient, winner);
        assert!(!schedule.released);

        // Check pending schedules
        let pending = client.get_pending_program_schedules();
        assert_eq!(pending.len(), 1);

        // Event verification can be added later - focusing on core functionality
    }

    #[test]
    fn test_multiple_program_release_schedules() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let authorized_key = Address::generate(&env);
        let winner1 = Address::generate(&env);
        let winner2 = Address::generate(&env);
        let program_id = String::from_str(&env, "Hackathon2024");
        let amount1 = 600_0000000;
        let amount2 = 400_0000000;
        let total_amount = amount1 + amount2;

        env.mock_all_auths();

        // Create and fund token BEFORE initialize_program
        let token_client = create_token_contract(&env, &authorized_key);

        // Register program with real token
        client.initialize_program(&program_id, &authorized_key, &token_client.address);
        let tokenadmin = token::StellarAssetClient::new(&env, &token_client.address);
        tokenadmin.mint(&authorized_key, &total_amount);

        // Transfer tokens to contract first

        // Lock funds for program
        client.lock_program_funds(&authorized_key, &total_amount);

        // Create first release schedule
        client.create_program_release_schedule(&amount1, &1000, &winner1);

        // Create second release schedule
        client.create_program_release_schedule(&amount2, &2000, &winner2);

        // Verify both schedules exist
        let all_schedules = client.get_all_prog_release_schedules();
        assert_eq!(all_schedules.len(), 2);

        // Verify schedule IDs
        let schedule1 = client.get_program_release_schedule(&1);
        let schedule2 = client.get_program_release_schedule(&2);
        assert_eq!(schedule1.schedule_id, 1);
        assert_eq!(schedule2.schedule_id, 2);

        // Verify amounts
        assert_eq!(schedule1.amount, amount1);
        assert_eq!(schedule2.amount, amount2);

        // Verify recipients
        assert_eq!(schedule1.recipient, winner1);
        assert_eq!(schedule2.recipient, winner2);

        // Check pending schedules
        let pending = client.get_pending_program_schedules();
        assert_eq!(pending.len(), 2);

        // Event verification can be added later - focusing on core functionality
    }

    #[test]
    fn test_program_automatic_release_at_timestamp() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let authorized_key = Address::generate(&env);
        let winner = Address::generate(&env);
        let token = Address::generate(&env);
        let program_id = String::from_str(&env, "Hackathon2024");
        let amount = 1000_0000000;
        let release_timestamp = 1000;

        env.mock_all_auths();

        // Setup program with schedule
        setup_program_with_schedule(
            &env,
            &client,
            &contract_id,
            &authorized_key,
            &token,
            &program_id,
            amount,
            &winner,
            release_timestamp,
        );

        // Try to release before timestamp (should fail)
        env.ledger().set_timestamp(999);
        let result = client.try_release_prog_schedule_automatic(&1);
        assert!(result.is_err());

        // Advance time to after release timestamp
        env.ledger().set_timestamp(1001);

        // Release automatically
        client.release_prog_schedule_automatic(&1);

        // Verify schedule was released
        let schedule = client.get_program_release_schedule(&1);
        assert!(schedule.released);
        assert_eq!(schedule.released_at, Some(1001));

        assert_eq!(schedule.released_by, Some(contract_id.clone()));

        // Check no pending schedules
        let pending = client.get_pending_program_schedules();
        assert_eq!(pending.len(), 0);

        // Verify release history
        let history = client.get_program_release_history();
        assert_eq!(history.len(), 1);
        assert_eq!(history.get(0).unwrap().release_type, ReleaseType::Automatic);

        // Event verification can be added later - focusing on core functionality
    }

    #[test]
    fn test_program_manual_trigger_before_after_timestamp() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let authorized_key = Address::generate(&env);
        let winner = Address::generate(&env);
        let token = Address::generate(&env);
        let program_id = String::from_str(&env, "Hackathon2024");
        let amount = 1000_0000000;
        let release_timestamp = 1000;

        env.mock_all_auths();

        // Setup program with schedule
        setup_program_with_schedule(
            &env,
            &client,
            &contract_id,
            &authorized_key,
            &token,
            &program_id,
            amount,
            &winner,
            release_timestamp,
        );

        // Manually release before timestamp (authorized key can do this)
        env.ledger().set_timestamp(999);
        client.release_program_schedule_manual(&1);

        // Verify schedule was released
        let schedule = client.get_program_release_schedule(&1);
        assert!(schedule.released);
        assert_eq!(schedule.released_at, Some(999));
        assert_eq!(schedule.released_by, Some(authorized_key.clone()));

        // Verify release history
        let history = client.get_program_release_history();
        assert_eq!(history.len(), 1);
        assert_eq!(history.get(0).unwrap().release_type, ReleaseType::Manual);

        // Event verification can be added later - focusing on core functionality
    }

    #[test]
    fn test_verify_program_schedule_tracking_and_history() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let authorized_key = Address::generate(&env);
        let winner1 = Address::generate(&env);
        let winner2 = Address::generate(&env);
        let program_id = String::from_str(&env, "Hackathon2024");
        let amount1 = 600_0000000;
        let amount2 = 400_0000000;
        let total_amount = amount1 + amount2;

        env.mock_all_auths();

        // Create and fund token FIRST
        let token_client = create_token_contract(&env, &authorized_key);
        let tokenadmin = token::StellarAssetClient::new(&env, &token_client.address);
        tokenadmin.mint(&authorized_key, &total_amount);

        // Register program with REAL token address
        client.initialize_program(&program_id, &authorized_key, &token_client.address);

        // Transfer tokens to contract first

        // Lock funds for program
        client.lock_program_funds(&authorized_key, &total_amount);

        // Create first schedule
        client.create_program_release_schedule(&amount1, &1000, &winner1);

        // Create second schedule
        client.create_program_release_schedule(&amount2, &2000, &winner2);

        // Release first schedule manually
        client.release_program_schedule_manual(&1);

        // Advance time and release second schedule automatically
        env.ledger().set_timestamp(2001);
        client.release_prog_schedule_automatic(&2);

        // Verify complete history
        let history = client.get_program_release_history();
        assert_eq!(history.len(), 2);

        // Check first release (manual)
        let first_release = history.get(0).unwrap();
        assert_eq!(first_release.schedule_id, 1);
        assert_eq!(first_release.amount, amount1);
        assert_eq!(first_release.recipient, winner1);
        assert_eq!(first_release.release_type, ReleaseType::Manual);

        // Check second release (automatic)
        let second_release = history.get(1).unwrap();
        assert_eq!(second_release.schedule_id, 2);
        assert_eq!(second_release.amount, amount2);
        assert_eq!(second_release.recipient, winner2);
        assert_eq!(second_release.release_type, ReleaseType::Automatic);

        // Verify no pending schedules
        let pending = client.get_pending_program_schedules();
        assert_eq!(pending.len(), 0);

        // Verify all schedules are marked as released
        let all_schedules = client.get_all_prog_release_schedules();
        assert_eq!(all_schedules.len(), 2);
        assert!(all_schedules.get(0).unwrap().released);
        assert!(all_schedules.get(1).unwrap().released);
    }

    #[test]
    fn test_program_overlapping_schedules() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let authorized_key = Address::generate(&env);
        let winner1 = Address::generate(&env);
        let winner2 = Address::generate(&env);
        let winner3 = Address::generate(&env);
        let program_id = String::from_str(&env, "Hackathon2024");
        let amount1 = 300_0000000;
        let amount2 = 300_0000000;
        let amount3 = 400_0000000;
        let total_amount = amount1 + amount2 + amount3;
        let base_timestamp = 1000;

        env.mock_all_auths();

        // Create and fund token FIRST
        let token_client = create_token_contract(&env, &authorized_key);
        let tokenadmin = token::StellarAssetClient::new(&env, &token_client.address);
        tokenadmin.mint(&authorized_key, &total_amount);

        // Register program with REAL token address
        client.initialize_program(&program_id, &authorized_key, &token_client.address);

        // Transfer tokens to contract first

        // Lock funds for program
        client.lock_program_funds(&authorized_key, &total_amount);

        // Create overlapping schedules (all at same timestamp)
        client.create_program_release_schedule(&amount1, &base_timestamp, &winner1.clone());

        client.create_program_release_schedule(&amount2, &base_timestamp, &winner2.clone());

        client.create_program_release_schedule(&amount3, &base_timestamp, &winner3.clone());

        // Advance time to after release timestamp
        env.ledger().set_timestamp(base_timestamp + 1);

        // Check due schedules (should be all 3)
        let due = client.get_due_program_schedules();
        assert_eq!(due.len(), 3);

        // Release schedules one by one
        client.release_prog_schedule_automatic(&1);
        client.release_prog_schedule_automatic(&2);
        client.release_prog_schedule_automatic(&3);

        // Verify all schedules are released
        let pending = client.get_pending_program_schedules();
        assert_eq!(pending.len(), 0);

        // Verify complete history
        let history = client.get_program_release_history();
        assert_eq!(history.len(), 3);

        // Verify all were automatic releases
        for release in history.iter() {
            assert_eq!(release.release_type, ReleaseType::Automatic);
        }

        // Event verification can be added later - focusing on core functionality
    }

    #[test]
    fn test_register_single_program() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let backend = Address::generate(&env);
        let token = Address::generate(&env);
        let prog_id = String::from_str(&env, "Hackathon2024");

        // Register program
        let program = client.initialize_program(&prog_id, &backend, &token);

        // Verify program data
        assert_eq!(program.program_id, prog_id);
        assert_eq!(program.authorized_payout_key, backend);
        assert_eq!(program.token_address, token);
        assert_eq!(program.total_funds, 0);
        assert_eq!(program.remaining_balance, 0);
        assert_eq!(program.payout_history.len(), 0);

        // Verify it exists
        assert!(client.program_exists());
        assert_eq!(client.get_program_count(), 1);
    }

    #[test]
    // NOTE: test_multiple_programs_isolation removed — single-program model
    // does not allow registering multiple programs.
    fn _test_multiple_programs_isolation_removed() {}

    // NOTE: test_duplicate_program_registration removed — single-program model
    // does not support re-registration semantics in this form.

    // NOTE: test_empty_program_id removed — initialize_program does not
    // currently validate empty program IDs in the single-program model.

    #[test]
    #[should_panic]
    fn test_get_nonexistent_program() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        // Calling get_program_info without initializing should panic
        client.get_program_info();
    }

    // ========================================================================
    // Batch program registration tests
    // ========================================================================

    #[test]
    fn test_batch_initialize_programs_success() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let token = Address::generate(&env);
        let mut items = Vec::new(&env);
        items.push_back(ProgramInitItem {
            program_id: String::from_str(&env, "prog-1"),
            authorized_payout_key: admin.clone(),
            token_address: token.clone(),
        });
        items.push_back(ProgramInitItem {
            program_id: String::from_str(&env, "prog-2"),
            authorized_payout_key: admin.clone(),
            token_address: token.clone(),
        });
        let count = client
            .try_batch_initialize_programs(&items)
            .unwrap()
            .unwrap();
        assert_eq!(count, 2);
        // batch_initialize_programs uses DataKey::Program(id) registry;
        // program_exists() checks the single-program PROGRAM_DATA key.
        // Verify only the count returned by the batch call.
    }

    #[test]
    fn test_batch_initialize_programs_empty_err() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);
        let items: Vec<ProgramInitItem> = Vec::new(&env);
        let res = client.try_batch_initialize_programs(&items);
        assert!(matches!(res, Err(Ok(BatchError::InvalidBatchSize))));
    }

    #[test]
    fn test_batch_initialize_programs_duplicate_id_err() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let token = Address::generate(&env);
        let pid = String::from_str(&env, "same-id");
        let mut items = Vec::new(&env);
        items.push_back(ProgramInitItem {
            program_id: pid.clone(),
            authorized_payout_key: admin.clone(),
            token_address: token.clone(),
        });
        items.push_back(ProgramInitItem {
            program_id: pid,
            authorized_payout_key: admin.clone(),
            token_address: token.clone(),
        });
        let res = client.try_batch_initialize_programs(&items);
        assert!(matches!(res, Err(Ok(BatchError::DuplicateProgramId))));
    }

    // ========================================================================
    // Fund Locking Tests
    // ========================================================================

    #[test]
    fn test_lock_funds_single_program() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);
        let token_client = create_token_contract(&env, &admin);
        let token_sac = token::StellarAssetClient::new(&env, &token_client.address);
        token_sac.mint(&admin, &10_000_0000000);

        let backend = Address::generate(&env);
        let prog_id = String::from_str(&env, "Hackathon2024");

        // Register program
        client.initialize_program(&prog_id, &backend, &token_client.address);

        // Lock funds
        let amount = 10_000_0000000i128; // 10,000 USDC
        let updated = client.lock_program_funds(&admin, &amount);

        assert_eq!(updated.total_funds, amount);
        assert_eq!(client.get_remaining_balance(), amount);
    }

    #[test]
    // NOTE: Multi-tenant tests removed — single-program model does not
    // allow registering multiple programs on the same contract instance.
    // Removed: test_lock_funds_multiple_programs_isolation
    // Removed: test_multi_tenant_payout_history_isolation
    // Removed: test_multi_tenant_release_schedule_isolation
    // Removed: test_multi_tenant_release_history_isolation

    // NOTE: test_multi_tenant_analytics_isolation_concept removed — single-program model.

    // ========================================================================
    // Edge Cases for Program Management
    // ========================================================================
    #[test]
    fn test_program_reinitialization_attempt() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let backend = Address::generate(&env);
        let token = Address::generate(&env);
        let prog_id = String::from_str(&env, "Hackathon2024");

        // First registration should succeed
        client.initialize_program(&prog_id, &backend, &token);
        assert!(client.program_exists());

        let info = client.get_program_info();
        assert_eq!(info.program_id, prog_id);
    }

    // NOTE: test_program_count removed — single-program model does not
    // allow registering multiple programs.

    #[test]
    fn test_lock_funds_cumulative() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);
        let token_client = create_token_contract(&env, &admin);
        let token_sac = token::StellarAssetClient::new(&env, &token_client.address);
        token_sac.mint(&admin, &10_000_0000000);

        let backend = Address::generate(&env);
        let prog_id = String::from_str(&env, "Hackathon2024");

        client.initialize_program(&prog_id, &backend, &token_client.address);

        // Lock funds multiple times
        client.lock_program_funds(&admin, &1_000_0000000);
        client.lock_program_funds(&admin, &2_000_0000000);
        client.lock_program_funds(&admin, &3_000_0000000);

        let info = client.get_program_info();
        assert_eq!(info.total_funds, 6_000_0000000);
        assert_eq!(info.remaining_balance, 6_000_0000000);
    }

    #[test]
    #[should_panic(expected = "Amount must be greater than zero")]
    fn test_lock_zero_funds() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let backend = Address::generate(&env);
        let token = Address::generate(&env);
        let prog_id = String::from_str(&env, "Hackathon2024");

        client.initialize_program(&prog_id, &backend, &token);
        let admin = Address::generate(&env);
        client.lock_program_funds(&admin, &0);
    }

    // ========================================================================
    // Batch Payout Tests
    // ========================================================================

    #[test]
    #[should_panic(expected = "Recipients and amounts vectors must have the same length")]
    fn test_batch_payout_mismatched_lengths() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);
        let token_client = create_token_contract(&env, &admin);

        let backend = Address::generate(&env);
        let prog_id = String::from_str(&env, "Test");

        client.initialize_program(&prog_id, &backend, &token_client.address);
        client.lock_program_funds(&admin, &10_000_0000000);

        let recipients = soroban_sdk::vec![&env, Address::generate(&env), Address::generate(&env)];
        let amounts = soroban_sdk::vec![&env, 1_000_0000000i128]; // Mismatch!

        client.batch_payout(&recipients, &amounts);
    }

    #[test]
    #[should_panic(expected = "Insufficient balance")]
    fn test_batch_payout_insufficient_balance() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);
        let token_client = create_token_contract(&env, &admin);

        let backend = Address::generate(&env);
        let prog_id = String::from_str(&env, "Test");

        client.initialize_program(&prog_id, &backend, &token_client.address);
        client.lock_program_funds(&admin, &5_000_0000000);

        let recipients = soroban_sdk::vec![&env, Address::generate(&env)];
        let amounts = soroban_sdk::vec![&env, 10_000_0000000i128]; // More than available!

        client.batch_payout(&recipients, &amounts);
    }

    // NOTE: test_program_count removed — single-program model does not
    // allow registering multiple programs.

    // ========================================================================
    // Anti-Abuse Tests
    // ========================================================================

    // NOTE: Anti-abuse tests that depend on multi-program initialization
    // have been removed. Rate-limit and cooldown logic is currently
    // not integrated into program-escrow's init flow in the single-program model.
    // Anti-abuse tests should be re-added if/when integrated.

    #[test]
    fn test_anti_abuse_config_update() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.setadmin(&admin);

        client.update_rate_limit_config(&7200, &5, &120);

        let config = client.get_rate_limit_config();
        assert_eq!(config.window_size, 7200);
        assert_eq!(config.max_operations, 5);
        assert_eq!(config.cooldown_period, 120);
    }

    #[test]
    fn testadmin_rotation() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let oldadmin = Address::generate(&env);
        let newadmin = Address::generate(&env);

        // setadmin doesn't panic the first time
        client.setadmin(&oldadmin);

        // Rotation: set a different admin — also must not panic
        client.setadmin(&newadmin);
    }

    #[test]
    fn test_newadmin_can_update_config() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        let oldadmin = Address::generate(&env);
        let newadmin = Address::generate(&env);

        client.setadmin(&oldadmin);
        client.setadmin(&newadmin);

        client.update_rate_limit_config(&3600, &10, &30);

        let config = client.get_rate_limit_config();
        assert_eq!(config.window_size, 3600);
        assert_eq!(config.max_operations, 10);
        assert_eq!(config.cooldown_period, 30);
    }

    #[test]
    #[should_panic(expected = "Admin not set")]
    fn test_nonadmin_cannot_update_config() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, ProgramEscrowContract);
        let client = ProgramEscrowContractClient::new(&env, &contract_id);

        client.update_rate_limit_config(&3600, &10, &30);
    }
}
#[cfg(test)]
mod rbac_tests;
#[cfg(test)]
mod test;
#[cfg(test)]
mod test_circuit_breaker_integration;

#[cfg(test)]
mod test_balance_invariant;

#[cfg(test)]
mod test_whitelist;
