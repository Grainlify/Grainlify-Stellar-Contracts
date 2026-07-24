import { xdr, scValToNative } from '@stellar/stellar-sdk';
import { ValidationError } from './errors';

// Helper to convert input to xdr.ScVal
function toScVal(val: string | xdr.ScVal): xdr.ScVal {
  if (typeof val === 'string') {
    return xdr.ScVal.fromXDR(val, 'base64');
  }
  return val;
}

// -----------------------------------------------------------------------------
// Bounty Escrow Events Interfaces
// -----------------------------------------------------------------------------

export interface BountyEscrowInitializedEvent {
  type: 'init';
  version: number;
  admin: string;
  token: string;
  timestamp: bigint;
}

export interface FundsLockedEvent {
  type: 'f_lock';
  version: number;
  bounty_id: bigint;
  amount: bigint;
  depositor: string;
  deadline: bigint;
}

export interface FundsReleasedEvent {
  type: 'f_rel';
  version: number;
  bounty_id: bigint;
  amount: bigint;
  recipient: string;
  timestamp: bigint;
}

export interface FundsRefundedEvent {
  type: 'f_ref';
  version: number;
  bounty_id: bigint;
  amount: bigint;
  refund_to: string;
  timestamp: bigint;
}

export interface BountyExpiredEvent {
  type: 'b_exp';
  version: number;
  bounty_id: bigint;
  depositor: string;
  amount: bigint;
  deadline: bigint;
  expired_at: bigint;
}

export interface FeeCollectedEvent {
  type: 'fee';
  version: number;
  operation_type: 'Lock' | 'Release';
  amount: bigint;
  fee_rate: bigint;
  recipient: string;
  timestamp: bigint;
}

export interface BatchFundsLockedEvent {
  type: 'b_lock';
  version: number;
  count: number;
  total_amount: bigint;
  timestamp: bigint;
}

export interface FeeConfigUpdatedEvent {
  type: 'fee_cfg';
  version: number;
  lock_fee_rate: bigint;
  release_fee_rate: bigint;
  fee_recipient: string;
  fee_enabled: boolean;
  timestamp: bigint;
}

export interface BatchFundsReleasedEvent {
  type: 'b_rel';
  version: number;
  count: number;
  total_amount: bigint;
  timestamp: bigint;
}

export interface ApprovalAddedEvent {
  type: 'approval';
  version: number;
  bounty_id: bigint;
  contributor: string;
  approver: string;
  timestamp: bigint;
}

export interface PauseStateChangedEvent {
  type: 'pause';
  version: number;
  operation: string;
  paused: boolean;
  timestamp: bigint;
}

export interface BountyStateTransitionedEvent {
  type: 'state_tx';
  version: number;
  bounty_id: bigint;
  previous_state: string;
  new_state: string;
  amount: bigint;
  actor: string;
  timestamp: bigint;
}

export interface ContractAnalytics {
  active_bounty_count: number;
  released_bounty_count: number;
  refunded_bounty_count: number;
  total_locked: bigint;
  total_released: bigint;
  total_refunded: bigint;
  average_bounty_amount: bigint;
  snapshot_timestamp: bigint;
}

export interface AnalyticsSnapshotEvent {
  type: 'snap';
  version: number;
  metrics: ContractAnalytics;
}

export interface BountyActivityEvent {
  type: 'activity';
  version: number;
  bounty_id: bigint;
  activity_type: string;
  amount: bigint;
  timestamp: bigint;
}

// -----------------------------------------------------------------------------
// Program Escrow Events Interfaces
// -----------------------------------------------------------------------------

export interface ProgramInitializedEvent {
  type: 'PrgInit';
  version: number;
  program_id: string;
  authorized_payout_key: string;
  token_address: string;
  total_funds: bigint;
}

export interface ProgramFundsLockedEvent {
  type: 'FndsLock';
  version: number;
  program_id: string;
  amount: bigint;
  remaining_balance: bigint;
}

export interface BatchPayoutEvent {
  type: 'BatchPay';
  version: number;
  program_id: string;
  recipient_count: number;
  total_amount: bigint;
  remaining_balance: bigint;
  gas_proxy_transfer_ops: number;
  gas_proxy_history_appends: number;
  gas_proxy_storage_reads: number;
  gas_proxy_storage_writes: number;
  gas_proxy_events_emitted: number;
}

export interface PayoutEvent {
  type: 'Payout';
  version: number;
  program_id: string;
  recipient: string;
  amount: bigint;
  remaining_balance: bigint;
}

export type DisputeScope =
  | { type: 'Global' }
  | { type: 'Recipient'; value: string }
  | { type: 'Schedule'; value: bigint };

export interface DisputeOpenedEvent {
  type: 'DispOpen';
  version: number;
  program_id: string;
  scope: DisputeScope;
  opened_by: string;
  reason: string;
  timestamp: bigint;
}

export interface DisputeResolvedEvent {
  type: 'DispRes';
  version: number;
  program_id: string;
  scope: DisputeScope;
  resolved_by: string;
  timestamp: bigint;
}

export interface DisputeCancelledEvent {
  type: 'DispCanc';
  version: number;
  program_id: string;
  scope: DisputeScope;
  cancelled_by: string;
  timestamp: bigint;
}

export interface ProgramPauseStateChangedEvent {
  type: 'PauseSt';
  version: number;
  operation: string;
  paused: boolean;
  admin: string;
}

export interface AggregateStatsEvent {
  type: 'AggStats';
  version: number;
  program_id: string;
  total_funds: bigint;
  remaining_balance: bigint;
  total_paid_out: bigint;
  payout_count: number;
  scheduled_count: number;
}

export interface LargePayoutEvent {
  type: 'LrgPay';
  version: number;
  program_id: string;
  recipient: string;
  amount: bigint;
  threshold: bigint;
}

export interface ScheduleTriggeredEvent {
  type: 'SchedTrg';
  version: number;
  program_id: string;
  schedule_id: bigint;
  recipient: string;
  amount: bigint;
  trigger_type: 'Manual' | 'Automatic' | 'Oracle';
}

export interface OperationMetricEvent {
  type: 'metric_op';
  version: number;
  operation: string;
  caller: string;
  timestamp: bigint;
  success: boolean;
}

export interface PerformanceMetricEvent {
  type: 'metric_perf';
  version: number;
  function: string;
  duration: bigint;
  timestamp: bigint;
}

// -----------------------------------------------------------------------------
// Union Types
// -----------------------------------------------------------------------------

export type BountyEscrowEvent =
  | BountyEscrowInitializedEvent
  | FundsLockedEvent
  | FundsReleasedEvent
  | FundsRefundedEvent
  | BountyExpiredEvent
  | FeeCollectedEvent
  | BatchFundsLockedEvent
  | FeeConfigUpdatedEvent
  | BatchFundsReleasedEvent
  | ApprovalAddedEvent
  | PauseStateChangedEvent
  | BountyStateTransitionedEvent
  | AnalyticsSnapshotEvent
  | BountyActivityEvent;

export type ProgramEscrowEvent =
  | ProgramInitializedEvent
  | ProgramFundsLockedEvent
  | BatchPayoutEvent
  | PayoutEvent
  | DisputeOpenedEvent
  | DisputeResolvedEvent
  | DisputeCancelledEvent
  | ProgramPauseStateChangedEvent
  | AggregateStatsEvent
  | LargePayoutEvent
  | ScheduleTriggeredEvent
  | OperationMetricEvent
  | PerformanceMetricEvent;

export type DecodedEvent = BountyEscrowEvent | ProgramEscrowEvent;

// -----------------------------------------------------------------------------
// Helpers & Validation
// -----------------------------------------------------------------------------

function assertBigInt(val: any, fieldName: string) {
  if (typeof val !== 'bigint') {
    throw new ValidationError(`Field '${fieldName}' must be a bigint, got ${typeof val}`);
  }
}

function assertNumber(val: any, fieldName: string) {
  if (typeof val !== 'number' && typeof val !== 'bigint') {
    throw new ValidationError(`Field '${fieldName}' must be a number, got ${typeof val}`);
  }
}

function assertString(val: any, fieldName: string) {
  if (typeof val !== 'string') {
    throw new ValidationError(`Field '${fieldName}' must be a string, got ${typeof val}`);
  }
}

function assertBoolean(val: any, fieldName: string) {
  if (typeof val !== 'boolean') {
    throw new ValidationError(`Field '${fieldName}' must be a boolean, got ${typeof val}`);
  }
}

function assertVersion(version: any, expected: number[], eventName: string) {
  if (typeof version !== 'number' && typeof version !== 'bigint') {
    throw new ValidationError(`Field 'version' must be a number/bigint in ${eventName}, got ${typeof version}`);
  }
  const v = Number(version);
  if (!expected.includes(v)) {
    throw new ValidationError(`Unsupported version ${v} for ${eventName}. Expected one of: ${expected.join(', ')}`);
  }
}

function parseDisputeScope(nativeScope: any): DisputeScope {
  if (typeof nativeScope === 'string') {
    if (nativeScope === 'Global') {
      return { type: 'Global' };
    }
    throw new ValidationError(`Invalid DisputeScope string: ${nativeScope}`);
  }
  if (typeof nativeScope === 'object' && nativeScope !== null) {
    if ('Recipient' in nativeScope) {
      assertString(nativeScope.Recipient, 'DisputeScope.Recipient');
      return { type: 'Recipient', value: nativeScope.Recipient };
    }
    if ('Schedule' in nativeScope) {
      assertBigInt(nativeScope.Schedule, 'DisputeScope.Schedule');
      return { type: 'Schedule', value: nativeScope.Schedule };
    }
  }
  throw new ValidationError('Invalid DisputeScope format');
}

// -----------------------------------------------------------------------------
// Main Decoder
// -----------------------------------------------------------------------------

/**
 * Parses raw contract event topics and value (either as base64 strings or xdr.ScVal objects)
 * into a typed TypeScript contract event object.
 * 
 * Supports all event versions, enforcing strict type checks on the payload fields.
 * 
 * @param topics - List of event topics
 * @param value - Event payload data
 * @returns Parsed and validated event object
 * @throws {ValidationError} If fields are invalid, malformed, or have unsupported versions
 */
export function decodeContractEvent(
  topics: (string | xdr.ScVal)[],
  value: string | xdr.ScVal
): DecodedEvent {
  if (!topics || topics.length === 0) {
    throw new ValidationError('Event topics list is empty or undefined');
  }

  const parsedTopics = topics.map(t => scValToNative(toScVal(t)));
  const parsedValue = scValToNative(toScVal(value));

  const firstTopic = parsedTopics[0];
  if (typeof firstTopic !== 'string') {
    throw new ValidationError(`First topic must be a string symbol, got ${typeof firstTopic}`);
  }

  // --- Bounty Escrow Contract Events ---
  if (firstTopic === 'init') {
    assertVersion(parsedValue.version, [2], 'init');
    assertString(parsedValue.admin, 'admin');
    assertString(parsedValue.token, 'token');
    assertBigInt(parsedValue.timestamp, 'timestamp');
    return {
      type: 'init',
      version: Number(parsedValue.version),
      admin: parsedValue.admin,
      token: parsedValue.token,
      timestamp: parsedValue.timestamp,
    };
  }

  if (firstTopic === 'f_lock') {
    assertVersion(parsedValue.version, [2], 'f_lock');
    assertBigInt(parsedValue.bounty_id, 'bounty_id');
    assertBigInt(parsedValue.amount, 'amount');
    assertString(parsedValue.depositor, 'depositor');
    assertBigInt(parsedValue.deadline, 'deadline');
    return {
      type: 'f_lock',
      version: Number(parsedValue.version),
      bounty_id: parsedValue.bounty_id,
      amount: parsedValue.amount,
      depositor: parsedValue.depositor,
      deadline: parsedValue.deadline,
    };
  }

  if (firstTopic === 'f_rel') {
    assertVersion(parsedValue.version, [2], 'f_rel');
    assertBigInt(parsedValue.bounty_id, 'bounty_id');
    assertBigInt(parsedValue.amount, 'amount');
    assertString(parsedValue.recipient, 'recipient');
    assertBigInt(parsedValue.timestamp, 'timestamp');
    return {
      type: 'f_rel',
      version: Number(parsedValue.version),
      bounty_id: parsedValue.bounty_id,
      amount: parsedValue.amount,
      recipient: parsedValue.recipient,
      timestamp: parsedValue.timestamp,
    };
  }

  if (firstTopic === 'f_ref') {
    assertVersion(parsedValue.version, [2], 'f_ref');
    assertBigInt(parsedValue.bounty_id, 'bounty_id');
    assertBigInt(parsedValue.amount, 'amount');
    assertString(parsedValue.refund_to, 'refund_to');
    assertBigInt(parsedValue.timestamp, 'timestamp');
    return {
      type: 'f_ref',
      version: Number(parsedValue.version),
      bounty_id: parsedValue.bounty_id,
      amount: parsedValue.amount,
      refund_to: parsedValue.refund_to,
      timestamp: parsedValue.timestamp,
    };
  }

  if (firstTopic === 'b_exp') {
    assertVersion(parsedValue.version, [2], 'b_exp');
    assertBigInt(parsedValue.bounty_id, 'bounty_id');
    assertString(parsedValue.depositor, 'depositor');
    assertBigInt(parsedValue.amount, 'amount');
    assertBigInt(parsedValue.deadline, 'deadline');
    assertBigInt(parsedValue.expired_at, 'expired_at');
    return {
      type: 'b_exp',
      version: Number(parsedValue.version),
      bounty_id: parsedValue.bounty_id,
      depositor: parsedValue.depositor,
      amount: parsedValue.amount,
      deadline: parsedValue.deadline,
      expired_at: parsedValue.expired_at,
    };
  }

  if (firstTopic === 'fee') {
    assertVersion(parsedValue.version, [2], 'fee');
    if (parsedValue.operation_type !== 'Lock' && parsedValue.operation_type !== 'Release') {
      throw new ValidationError(`Invalid operation_type for fee collected event: ${parsedValue.operation_type}`);
    }
    assertBigInt(parsedValue.amount, 'amount');
    assertBigInt(parsedValue.fee_rate, 'fee_rate');
    assertString(parsedValue.recipient, 'recipient');
    assertBigInt(parsedValue.timestamp, 'timestamp');
    return {
      type: 'fee',
      version: Number(parsedValue.version),
      operation_type: parsedValue.operation_type,
      amount: parsedValue.amount,
      fee_rate: parsedValue.fee_rate,
      recipient: parsedValue.recipient,
      timestamp: parsedValue.timestamp,
    };
  }

  if (firstTopic === 'b_lock') {
    assertVersion(parsedValue.version, [2], 'b_lock');
    assertNumber(parsedValue.count, 'count');
    assertBigInt(parsedValue.total_amount, 'total_amount');
    assertBigInt(parsedValue.timestamp, 'timestamp');
    return {
      type: 'b_lock',
      version: Number(parsedValue.version),
      count: Number(parsedValue.count),
      total_amount: parsedValue.total_amount,
      timestamp: parsedValue.timestamp,
    };
  }

  if (firstTopic === 'fee_cfg') {
    assertVersion(parsedValue.version, [2], 'fee_cfg');
    assertBigInt(parsedValue.lock_fee_rate, 'lock_fee_rate');
    assertBigInt(parsedValue.release_fee_rate, 'release_fee_rate');
    assertString(parsedValue.fee_recipient, 'fee_recipient');
    assertBoolean(parsedValue.fee_enabled, 'fee_enabled');
    assertBigInt(parsedValue.timestamp, 'timestamp');
    return {
      type: 'fee_cfg',
      version: Number(parsedValue.version),
      lock_fee_rate: parsedValue.lock_fee_rate,
      release_fee_rate: parsedValue.release_fee_rate,
      fee_recipient: parsedValue.fee_recipient,
      fee_enabled: parsedValue.fee_enabled,
      timestamp: parsedValue.timestamp,
    };
  }

  if (firstTopic === 'b_rel') {
    assertVersion(parsedValue.version, [2], 'b_rel');
    assertNumber(parsedValue.count, 'count');
    assertBigInt(parsedValue.total_amount, 'total_amount');
    assertBigInt(parsedValue.timestamp, 'timestamp');
    return {
      type: 'b_rel',
      version: Number(parsedValue.version),
      count: Number(parsedValue.count),
      total_amount: parsedValue.total_amount,
      timestamp: parsedValue.timestamp,
    };
  }

  if (firstTopic === 'approval') {
    assertVersion(parsedValue.version, [2], 'approval');
    assertBigInt(parsedValue.bounty_id, 'bounty_id');
    assertString(parsedValue.contributor, 'contributor');
    assertString(parsedValue.approver, 'approver');
    assertBigInt(parsedValue.timestamp, 'timestamp');
    return {
      type: 'approval',
      version: Number(parsedValue.version),
      bounty_id: parsedValue.bounty_id,
      contributor: parsedValue.contributor,
      approver: parsedValue.approver,
      timestamp: parsedValue.timestamp,
    };
  }

  if (firstTopic === 'pause') {
    assertVersion(parsedValue.version, [2], 'pause');
    assertString(parsedValue.operation, 'operation');
    assertBoolean(parsedValue.paused, 'paused');
    assertBigInt(parsedValue.timestamp, 'timestamp');
    return {
      type: 'pause',
      version: Number(parsedValue.version),
      operation: parsedValue.operation,
      paused: parsedValue.paused,
      timestamp: parsedValue.timestamp,
    };
  }

  if (firstTopic === 'analytics') {
    const subTopic = parsedTopics[1];
    if (typeof subTopic !== 'string') {
      throw new ValidationError(`Second topic for analytics must be a string, got ${typeof subTopic}`);
    }

    if (subTopic === 'state_tx') {
      assertVersion(parsedValue.version, [1], 'state_tx');
      assertBigInt(parsedValue.bounty_id, 'bounty_id');
      assertString(parsedValue.previous_state, 'previous_state');
      assertString(parsedValue.new_state, 'new_state');
      assertBigInt(parsedValue.amount, 'amount');
      assertString(parsedValue.actor, 'actor');
      assertBigInt(parsedValue.timestamp, 'timestamp');
      return {
        type: 'state_tx',
        version: Number(parsedValue.version),
        bounty_id: parsedValue.bounty_id,
        previous_state: parsedValue.previous_state,
        new_state: parsedValue.new_state,
        amount: parsedValue.amount,
        actor: parsedValue.actor,
        timestamp: parsedValue.timestamp,
      };
    }

    if (subTopic === 'snap') {
      assertVersion(parsedValue.version, [1], 'snap');
      const metrics = parsedValue.metrics;
      if (!metrics || typeof metrics !== 'object') {
        throw new ValidationError('Field metrics is missing or invalid in snap event');
      }
      assertNumber(metrics.active_bounty_count, 'metrics.active_bounty_count');
      assertNumber(metrics.released_bounty_count, 'metrics.released_bounty_count');
      assertNumber(metrics.refunded_bounty_count, 'metrics.refunded_bounty_count');
      assertBigInt(metrics.total_locked, 'metrics.total_locked');
      assertBigInt(metrics.total_released, 'metrics.total_released');
      assertBigInt(metrics.total_refunded, 'metrics.total_refunded');
      assertBigInt(metrics.average_bounty_amount, 'metrics.average_bounty_amount');
      assertBigInt(metrics.snapshot_timestamp, 'metrics.snapshot_timestamp');

      return {
        type: 'snap',
        version: Number(parsedValue.version),
        metrics: {
          active_bounty_count: Number(metrics.active_bounty_count),
          released_bounty_count: Number(metrics.released_bounty_count),
          refunded_bounty_count: Number(metrics.refunded_bounty_count),
          total_locked: metrics.total_locked,
          total_released: metrics.total_released,
          total_refunded: metrics.total_refunded,
          average_bounty_amount: metrics.average_bounty_amount,
          snapshot_timestamp: metrics.snapshot_timestamp,
        },
      };
    }

    if (subTopic === 'activity') {
      assertVersion(parsedValue.version, [1], 'activity');
      assertBigInt(parsedValue.bounty_id, 'bounty_id');
      assertString(parsedValue.activity_type, 'activity_type');
      assertBigInt(parsedValue.amount, 'amount');
      assertBigInt(parsedValue.timestamp, 'timestamp');
      return {
        type: 'activity',
        version: Number(parsedValue.version),
        bounty_id: parsedValue.bounty_id,
        activity_type: parsedValue.activity_type,
        amount: parsedValue.amount,
        timestamp: parsedValue.timestamp,
      };
    }

    throw new ValidationError(`Unknown analytics sub-topic: ${subTopic}`);
  }

  // --- Program Escrow Contract Events ---
  if (firstTopic === 'PrgInit') {
    assertVersion(parsedValue.version, [2], 'PrgInit');
    assertString(parsedValue.program_id, 'program_id');
    assertString(parsedValue.authorized_payout_key, 'authorized_payout_key');
    assertString(parsedValue.token_address, 'token_address');
    assertBigInt(parsedValue.total_funds, 'total_funds');
    return {
      type: 'PrgInit',
      version: Number(parsedValue.version),
      program_id: parsedValue.program_id,
      authorized_payout_key: parsedValue.authorized_payout_key,
      token_address: parsedValue.token_address,
      total_funds: parsedValue.total_funds,
    };
  }

  if (firstTopic === 'FndsLock') {
    assertVersion(parsedValue.version, [2], 'FndsLock');
    assertString(parsedValue.program_id, 'program_id');
    assertBigInt(parsedValue.amount, 'amount');
    assertBigInt(parsedValue.remaining_balance, 'remaining_balance');
    return {
      type: 'FndsLock',
      version: Number(parsedValue.version),
      program_id: parsedValue.program_id,
      amount: parsedValue.amount,
      remaining_balance: parsedValue.remaining_balance,
    };
  }

  if (firstTopic === 'BatchPay') {
    assertVersion(parsedValue.version, [2], 'BatchPay');
    assertString(parsedValue.program_id, 'program_id');
    assertNumber(parsedValue.recipient_count, 'recipient_count');
    assertBigInt(parsedValue.total_amount, 'total_amount');
    assertBigInt(parsedValue.remaining_balance, 'remaining_balance');
    assertNumber(parsedValue.gas_proxy_transfer_ops, 'gas_proxy_transfer_ops');
    assertNumber(parsedValue.gas_proxy_history_appends, 'gas_proxy_history_appends');
    assertNumber(parsedValue.gas_proxy_storage_reads, 'gas_proxy_storage_reads');
    assertNumber(parsedValue.gas_proxy_storage_writes, 'gas_proxy_storage_writes');
    assertNumber(parsedValue.gas_proxy_events_emitted, 'gas_proxy_events_emitted');
    return {
      type: 'BatchPay',
      version: Number(parsedValue.version),
      program_id: parsedValue.program_id,
      recipient_count: Number(parsedValue.recipient_count),
      total_amount: parsedValue.total_amount,
      remaining_balance: parsedValue.remaining_balance,
      gas_proxy_transfer_ops: Number(parsedValue.gas_proxy_transfer_ops),
      gas_proxy_history_appends: Number(parsedValue.gas_proxy_history_appends),
      gas_proxy_storage_reads: Number(parsedValue.gas_proxy_storage_reads),
      gas_proxy_storage_writes: Number(parsedValue.gas_proxy_storage_writes),
      gas_proxy_events_emitted: Number(parsedValue.gas_proxy_events_emitted),
    };
  }

  if (firstTopic === 'Payout') {
    assertVersion(parsedValue.version, [2], 'Payout');
    assertString(parsedValue.program_id, 'program_id');
    assertString(parsedValue.recipient, 'recipient');
    assertBigInt(parsedValue.amount, 'amount');
    assertBigInt(parsedValue.remaining_balance, 'remaining_balance');
    return {
      type: 'Payout',
      version: Number(parsedValue.version),
      program_id: parsedValue.program_id,
      recipient: parsedValue.recipient,
      amount: parsedValue.amount,
      remaining_balance: parsedValue.remaining_balance,
    };
  }

  if (firstTopic === 'DispOpen') {
    assertVersion(parsedValue.version, [2], 'DispOpen');
    assertString(parsedValue.program_id, 'program_id');
    const scope = parseDisputeScope(parsedValue.scope);
    assertString(parsedValue.opened_by, 'opened_by');
    assertString(parsedValue.reason, 'reason');
    assertBigInt(parsedValue.timestamp, 'timestamp');
    return {
      type: 'DispOpen',
      version: Number(parsedValue.version),
      program_id: parsedValue.program_id,
      scope,
      opened_by: parsedValue.opened_by,
      reason: parsedValue.reason,
      timestamp: parsedValue.timestamp,
    };
  }

  if (firstTopic === 'DispRes') {
    assertVersion(parsedValue.version, [2], 'DispRes');
    assertString(parsedValue.program_id, 'program_id');
    const scope = parseDisputeScope(parsedValue.scope);
    assertString(parsedValue.resolved_by, 'resolved_by');
    assertBigInt(parsedValue.timestamp, 'timestamp');
    return {
      type: 'DispRes',
      version: Number(parsedValue.version),
      program_id: parsedValue.program_id,
      scope,
      resolved_by: parsedValue.resolved_by,
      timestamp: parsedValue.timestamp,
    };
  }

  if (firstTopic === 'DispCanc') {
    assertVersion(parsedValue.version, [2], 'DispCanc');
    assertString(parsedValue.program_id, 'program_id');
    const scope = parseDisputeScope(parsedValue.scope);
    assertString(parsedValue.cancelled_by, 'cancelled_by');
    assertBigInt(parsedValue.timestamp, 'timestamp');
    return {
      type: 'DispCanc',
      version: Number(parsedValue.version),
      program_id: parsedValue.program_id,
      scope,
      cancelled_by: parsedValue.cancelled_by,
      timestamp: parsedValue.timestamp,
    };
  }

  if (firstTopic === 'PauseSt') {
    if (!Array.isArray(parsedValue) || parsedValue.length < 3) {
      throw new ValidationError('PauseSt event payload must be a tuple of at least 3 elements');
    }
    assertString(parsedValue[0], 'operation');
    assertBoolean(parsedValue[1], 'paused');
    assertString(parsedValue[2], 'admin');
    return {
      type: 'PauseSt',
      version: 2,
      operation: parsedValue[0],
      paused: parsedValue[1],
      admin: parsedValue[2],
    };
  }

  if (firstTopic === 'AggStats') {
    assertVersion(parsedValue.version, [2], 'AggStats');
    assertString(parsedValue.program_id, 'program_id');
    assertBigInt(parsedValue.total_funds, 'total_funds');
    assertBigInt(parsedValue.remaining_balance, 'remaining_balance');
    assertBigInt(parsedValue.total_paid_out, 'total_paid_out');
    assertNumber(parsedValue.payout_count, 'payout_count');
    assertNumber(parsedValue.scheduled_count, 'scheduled_count');
    return {
      type: 'AggStats',
      version: Number(parsedValue.version),
      program_id: parsedValue.program_id,
      total_funds: parsedValue.total_funds,
      remaining_balance: parsedValue.remaining_balance,
      total_paid_out: parsedValue.total_paid_out,
      payout_count: Number(parsedValue.payout_count),
      scheduled_count: Number(parsedValue.scheduled_count),
    };
  }

  if (firstTopic === 'LrgPay') {
    assertVersion(parsedValue.version, [2], 'LrgPay');
    assertString(parsedValue.program_id, 'program_id');
    assertString(parsedValue.recipient, 'recipient');
    assertBigInt(parsedValue.amount, 'amount');
    assertBigInt(parsedValue.threshold, 'threshold');
    return {
      type: 'LrgPay',
      version: Number(parsedValue.version),
      program_id: parsedValue.program_id,
      recipient: parsedValue.recipient,
      amount: parsedValue.amount,
      threshold: parsedValue.threshold,
    };
  }

  if (firstTopic === 'SchedTrg') {
    assertVersion(parsedValue.version, [2], 'SchedTrg');
    assertString(parsedValue.program_id, 'program_id');
    assertBigInt(parsedValue.schedule_id, 'schedule_id');
    assertString(parsedValue.recipient, 'recipient');
    assertBigInt(parsedValue.amount, 'amount');
    if (
      parsedValue.trigger_type !== 'Manual' &&
      parsedValue.trigger_type !== 'Automatic' &&
      parsedValue.trigger_type !== 'Oracle'
    ) {
      throw new ValidationError(`Invalid trigger_type for SchedTrg event: ${parsedValue.trigger_type}`);
    }
    return {
      type: 'SchedTrg',
      version: Number(parsedValue.version),
      program_id: parsedValue.program_id,
      schedule_id: parsedValue.schedule_id,
      recipient: parsedValue.recipient,
      amount: parsedValue.amount,
      trigger_type: parsedValue.trigger_type,
    };
  }

  if (firstTopic === 'metric') {
    const subTopic = parsedTopics[1];
    if (typeof subTopic !== 'string') {
      throw new ValidationError(`Second topic for metric must be a string, got ${typeof subTopic}`);
    }

    if (subTopic === 'op') {
      assertString(parsedValue.operation, 'operation');
      assertString(parsedValue.caller, 'caller');
      assertBigInt(parsedValue.timestamp, 'timestamp');
      assertBoolean(parsedValue.success, 'success');
      return {
        type: 'metric_op',
        version: 2,
        operation: parsedValue.operation,
        caller: parsedValue.caller,
        timestamp: parsedValue.timestamp,
        success: parsedValue.success,
      };
    }

    if (subTopic === 'perf') {
      assertString(parsedValue.function, 'function');
      assertBigInt(parsedValue.duration, 'duration');
      assertBigInt(parsedValue.timestamp, 'timestamp');
      return {
        type: 'metric_perf',
        version: 2,
        function: parsedValue.function,
        duration: parsedValue.duration,
        timestamp: parsedValue.timestamp,
      };
    }

    throw new ValidationError(`Unknown metric sub-topic: ${subTopic}`);
  }

  throw new ValidationError(`Unknown event topic: ${firstTopic}`);
}
