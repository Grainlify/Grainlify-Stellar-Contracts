use soroban_sdk::{contracttype, symbol_short, Address, Env, Symbol};

pub const EVENT_VERSION_V2: u32 = 2;

#[contracttype]
#[derive(Clone, Debug)]
pub struct BountyEscrowInitialized {
    pub version: u32,
    pub admin: Address,
    pub token: Address,
    pub timestamp: u64,
}

pub fn emit_bounty_initialized(env: &Env, event: BountyEscrowInitialized) {
    let topics = (symbol_short!("init"),);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct FundsLocked {
    pub version: u32,
    pub bounty_id: u64,
    pub amount: i128,
    pub depositor: Address,
    pub deadline: u64,
}

pub fn emit_funds_locked(env: &Env, event: FundsLocked) {
    let topics = (symbol_short!("f_lock"), event.bounty_id);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct FundsReleased {
    pub version: u32,
    pub bounty_id: u64,
    pub amount: i128,
    pub recipient: Address,
    pub timestamp: u64,
}

pub fn emit_funds_released(env: &Env, event: FundsReleased) {
    let topics = (symbol_short!("f_rel"), event.bounty_id);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct FundsRefunded {
    pub version: u32,
    pub bounty_id: u64,
    pub amount: i128,
    pub refund_to: Address,
    pub timestamp: u64,
}

pub fn emit_funds_refunded(env: &Env, event: FundsRefunded) {
    let topics = (symbol_short!("f_ref"), event.bounty_id);
    env.events().publish(topics, event.clone());
}

/// Emitted when a sweep observes an expired bounty immediately before
/// refunding the remaining locked funds to the depositor.
#[contracttype]
#[derive(Clone, Debug)]
pub struct BountyExpired {
    pub version: u32,
    pub bounty_id: u64,
    pub depositor: Address,
    pub amount: i128,
    pub deadline: u64,
    pub expired_at: u64,
}

pub fn emit_bounty_expired(env: &Env, event: BountyExpired) {
    let topics = (symbol_short!("b_exp"), event.bounty_id);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FeeOperationType {
    Lock,
    Release,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct FeeCollected {
    pub version: u32,
    pub operation_type: FeeOperationType,
    pub amount: i128,
    pub fee_rate: i128,
    pub recipient: Address,
    pub timestamp: u64,
}

pub fn emit_fee_collected(env: &Env, event: FeeCollected) {
    let topics = (symbol_short!("fee"),);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct BatchFundsLocked {
    pub version: u32,
    pub count: u32,
    pub total_amount: i128,
    pub timestamp: u64,
}

pub fn emit_batch_funds_locked(env: &Env, event: BatchFundsLocked) {
    let topics = (symbol_short!("b_lock"),);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct FeeConfigUpdated {
    pub version: u32,
    pub lock_fee_rate: i128,
    pub release_fee_rate: i128,
    pub fee_recipient: Address,
    pub fee_enabled: bool,
    pub timestamp: u64,
}

pub fn emit_fee_config_updated(env: &Env, event: FeeConfigUpdated) {
    let topics = (symbol_short!("fee_cfg"),);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct BatchFundsReleased {
    pub version: u32,
    pub count: u32,
    pub total_amount: i128,
    pub timestamp: u64,
}

pub fn emit_batch_funds_released(env: &Env, event: BatchFundsReleased) {
    let topics = (symbol_short!("b_rel"),);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct ApprovalAdded {
    pub version: u32,
    pub bounty_id: u64,
    pub contributor: Address,
    pub approver: Address,
    pub timestamp: u64,
}

pub fn emit_approval_added(env: &Env, event: ApprovalAdded) {
    let topics = (symbol_short!("approval"), event.bounty_id);
    env.events().publish(topics, event.clone());
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimCreated {
    pub version: u32,
    pub bounty_id: u64, // use program_id+schedule_id equivalent in program-escrow
    pub recipient: Address,
    pub amount: i128,
    pub expires_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimExecuted {
    pub version: u32,
    pub bounty_id: u64,
    pub recipient: Address,
    pub amount: i128,
    pub claimed_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimCancelled {
    pub version: u32,
    pub bounty_id: u64,
    pub recipient: Address,
    pub amount: i128,
    pub cancelled_at: u64,
    pub cancelled_by: Address,
    pub reason: Symbol,
}

pub fn emit_pause_state_changed(env: &Env, event: crate::PauseStateChanged) {
    let topics = (symbol_short!("pause"), event.operation.clone());
    env.events().publish(topics, event);
}
