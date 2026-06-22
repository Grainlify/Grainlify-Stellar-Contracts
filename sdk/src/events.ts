import { scValToNative } from '@stellar/stellar-sdk';

type NativeRecord = Record<string, unknown>;
type NativeTopic = string | number | bigint | boolean;

export interface RawSorobanEvent {
  topic?: unknown;
  topics?: unknown;
  data?: unknown;
  value?: unknown;
  payload?: unknown;
  [key: string]: unknown;
}

export interface BountyFundsLockedEvent {
  kind: 'bounty.funds_locked';
  contract: 'bounty_escrow';
  topic: 'f_lock';
  version: 2;
  bountyId: bigint;
  amount: bigint;
  depositor: string;
  deadline: bigint;
}

export interface BountyFundsReleasedEvent {
  kind: 'bounty.funds_released';
  contract: 'bounty_escrow';
  topic: 'f_rel';
  version: 2;
  bountyId: bigint;
  amount: bigint;
  recipient: string;
  timestamp: bigint;
}

export interface BountyFundsRefundedEvent {
  kind: 'bounty.funds_refunded';
  contract: 'bounty_escrow';
  topic: 'f_ref';
  version: 2;
  bountyId: bigint;
  amount: bigint;
  refundTo: string;
  timestamp: bigint;
}

export interface BountyExpiredEvent {
  kind: 'bounty.expired';
  contract: 'bounty_escrow';
  topic: 'b_exp';
  version: 2;
  bountyId: bigint;
  depositor: string;
  amount: bigint;
  deadline: bigint;
  expiredAt: bigint;
}

export interface BountyInitializedEvent {
  kind: 'bounty.initialized';
  contract: 'bounty_escrow';
  topic: 'init';
  version: 2;
  admin: string;
  token: string;
  timestamp: bigint;
}

export interface BountyClaimCreatedEvent {
  kind: 'bounty.claim_created';
  contract: 'bounty_escrow';
  topic: 'claim/created';
  version: 2;
  bountyId: bigint;
  recipient: string;
  amount: bigint;
  expiresAt: bigint;
}

export interface BountyStateTransitionedEvent {
  kind: 'bounty.state_transitioned';
  contract: 'bounty_escrow';
  topic: 'analytics/state_tx';
  version: 1;
  bountyId: bigint;
  previousState: string;
  newState: string;
  amount: bigint;
  actor: string;
  timestamp: bigint;
}

export interface BountyActivityEvent {
  kind: 'bounty.activity';
  contract: 'bounty_escrow';
  topic: 'analytics/activity';
  version: 1;
  bountyId: bigint;
  activityType: string;
  amount: bigint;
  timestamp: bigint;
}

export interface BountyAnalyticsSnapshotEvent {
  kind: 'bounty.analytics_snapshot';
  contract: 'bounty_escrow';
  topic: 'analytics/snap';
  version: 1;
  metrics: {
    activeBountyCount: number;
    releasedBountyCount: number;
    refundedBountyCount: number;
    totalLocked: bigint;
    totalReleased: bigint;
    totalRefunded: bigint;
    averageBountyAmount: bigint;
    snapshotTimestamp: bigint;
  };
}

export interface ProgramInitializedEvent {
  kind: 'program.initialized';
  contract: 'program_escrow';
  topic: 'PrgInit';
  version: 2;
  programId: string;
  authorizedPayoutKey: string;
  tokenAddress: string;
  totalFunds: bigint;
}

export interface ProgramFundsLockedEvent {
  kind: 'program.funds_locked';
  contract: 'program_escrow';
  topic: 'FndsLock';
  version: 2;
  programId: string;
  amount: bigint;
  remainingBalance: bigint;
}

export interface ProgramPayoutEvent {
  kind: 'program.payout';
  contract: 'program_escrow';
  topic: 'Payout';
  version: 2;
  programId: string;
  recipient: string;
  amount: bigint;
  remainingBalance: bigint;
}

export interface ProgramBatchPayoutEvent {
  kind: 'program.batch_payout';
  contract: 'program_escrow';
  topic: 'BatchPay';
  version: 2;
  programId: string;
  recipientCount: number;
  totalAmount: bigint;
  remainingBalance: bigint;
  gasProxyTransferOps: number;
  gasProxyHistoryAppends: number;
  gasProxyStorageReads: number;
  gasProxyStorageWrites: number;
  gasProxyEventsEmitted: number;
}

export interface ProgramScheduleTriggeredEvent {
  kind: 'program.schedule_triggered';
  contract: 'program_escrow';
  topic: 'SchedTrg';
  version: 2;
  programId: string;
  scheduleId: bigint;
  recipient: string;
  amount: bigint;
  triggerType: string;
}

export interface ProgramDisputeOpenedEvent {
  kind: 'program.dispute_opened';
  contract: 'program_escrow';
  topic: 'DispOpen';
  version: 2;
  programId: string;
  openedBy: string;
  reason: string;
  timestamp: bigint;
}

export interface ProgramAggregateStatsEvent {
  kind: 'program.aggregate_stats';
  contract: 'program_escrow';
  topic: 'AggStats';
  version: 2;
  programId: string;
  totalFunds: bigint;
  remainingBalance: bigint;
  totalPaidOut: bigint;
  payoutCount: number;
  scheduledCount: number;
}

export interface ProgramLargePayoutEvent {
  kind: 'program.large_payout';
  contract: 'program_escrow';
  topic: 'LrgPay';
  version: 2;
  programId: string;
  recipient: string;
  amount: bigint;
  threshold: bigint;
}

export type DecodedContractEvent =
  | BountyFundsLockedEvent
  | BountyFundsReleasedEvent
  | BountyFundsRefundedEvent
  | BountyExpiredEvent
  | BountyInitializedEvent
  | BountyClaimCreatedEvent
  | BountyStateTransitionedEvent
  | BountyActivityEvent
  | BountyAnalyticsSnapshotEvent
  | ProgramInitializedEvent
  | ProgramFundsLockedEvent
  | ProgramPayoutEvent
  | ProgramBatchPayoutEvent
  | ProgramScheduleTriggeredEvent
  | ProgramDisputeOpenedEvent
  | ProgramAggregateStatsEvent
  | ProgramLargePayoutEvent;

export interface UnknownSorobanEvent {
  kind: 'unknown';
  topics: NativeTopic[];
  rawPayload: unknown;
}

export interface UnknownVersionSorobanEvent {
  kind: 'unknown_version';
  eventName: DecodedContractEvent['kind'];
  version: number;
  supportedVersions: number[];
  topics: NativeTopic[];
  rawPayload: unknown;
}

export interface MalformedSorobanEvent {
  kind: 'malformed';
  topics: NativeTopic[];
  reason: string;
  rawPayload: unknown;
}

export type DecodedSorobanEvent =
  | DecodedContractEvent
  | UnknownSorobanEvent
  | UnknownVersionSorobanEvent
  | MalformedSorobanEvent;

type EventDecoder = (topics: NativeTopic[], payload: NativeRecord) => DecodedSorobanEvent;

const EVENT_DECODERS: Record<string, EventDecoder> = {
  init: decodeBountyInitialized,
  f_lock: decodeBountyFundsLocked,
  f_rel: decodeBountyFundsReleased,
  f_ref: decodeBountyFundsRefunded,
  b_exp: decodeBountyExpired,
  'claim/created': decodeBountyClaimCreated,
  'analytics/state_tx': decodeBountyStateTransitioned,
  'analytics/activity': decodeBountyActivity,
  'analytics/snap': decodeBountyAnalyticsSnapshot,
  PrgInit: decodeProgramInitialized,
  FndsLock: decodeProgramFundsLocked,
  Payout: decodeProgramPayout,
  BatchPay: decodeProgramBatchPayout,
  SchedTrg: decodeProgramScheduleTriggered,
  DispOpen: decodeProgramDisputeOpened,
  AggStats: decodeProgramAggregateStats,
  LrgPay: decodeProgramLargePayout,
};

/**
 * Decode one Soroban event emitted by Grainlify contracts into a typed union.
 * Unknown topics, unsupported versions, and malformed payloads are returned as
 * explicit result variants so indexers do not need to guess.
 */
export function decodeSorobanEvent(raw: RawSorobanEvent): DecodedSorobanEvent {
  const topics = normalizeTopics(raw.topic ?? raw.topics);
  const rawPayload = normalizeNative(raw.value ?? raw.data ?? raw.payload);
  const decoderKey = decoderKeyForTopics(topics);

  if (!decoderKey || !EVENT_DECODERS[decoderKey]) {
    return { kind: 'unknown', topics, rawPayload };
  }

  if (!isRecord(rawPayload)) {
    return malformed(topics, rawPayload, 'event payload must be an object or map');
  }

  try {
    return EVENT_DECODERS[decoderKey](topics, rawPayload);
  } catch (error) {
    return malformed(topics, rawPayload, error instanceof Error ? error.message : String(error));
  }
}

/**
 * Decode a list of raw events or a transaction-like object with an `events`
 * property. This mirrors common Soroban RPC response shapes while staying
 * usable in tests and indexers that already normalized XDR to JS values.
 */
export function decodeSorobanEvents(raw: RawSorobanEvent[] | { events?: unknown }): DecodedSorobanEvent[] {
  const events = Array.isArray(raw) ? raw : Array.isArray(raw.events) ? raw.events : [];
  return events.map((event) => decodeSorobanEvent(event as RawSorobanEvent));
}

function decodeBountyInitialized(topics: NativeTopic[], payload: NativeRecord): DecodedSorobanEvent {
  const version = readSupportedVersion('bounty.initialized', topics, payload, [2]);
  if (isUnknownVersion(version)) return version;
  return {
    kind: 'bounty.initialized',
    contract: 'bounty_escrow',
    topic: 'init',
    version,
    admin: readString(payload, 'admin'),
    token: readString(payload, 'token'),
    timestamp: readBigInt(payload, 'timestamp'),
  };
}

function decodeBountyFundsLocked(topics: NativeTopic[], payload: NativeRecord): DecodedSorobanEvent {
  const version = readSupportedVersion('bounty.funds_locked', topics, payload, [2]);
  if (isUnknownVersion(version)) return version;
  return {
    kind: 'bounty.funds_locked',
    contract: 'bounty_escrow',
    topic: 'f_lock',
    version,
    bountyId: readBigInt(payload, 'bounty_id'),
    amount: readBigInt(payload, 'amount'),
    depositor: readString(payload, 'depositor'),
    deadline: readBigInt(payload, 'deadline'),
  };
}

function decodeBountyFundsReleased(topics: NativeTopic[], payload: NativeRecord): DecodedSorobanEvent {
  const version = readSupportedVersion('bounty.funds_released', topics, payload, [2]);
  if (isUnknownVersion(version)) return version;
  return {
    kind: 'bounty.funds_released',
    contract: 'bounty_escrow',
    topic: 'f_rel',
    version,
    bountyId: readBigInt(payload, 'bounty_id'),
    amount: readBigInt(payload, 'amount'),
    recipient: readString(payload, 'recipient'),
    timestamp: readBigInt(payload, 'timestamp'),
  };
}

function decodeBountyFundsRefunded(topics: NativeTopic[], payload: NativeRecord): DecodedSorobanEvent {
  const version = readSupportedVersion('bounty.funds_refunded', topics, payload, [2]);
  if (isUnknownVersion(version)) return version;
  return {
    kind: 'bounty.funds_refunded',
    contract: 'bounty_escrow',
    topic: 'f_ref',
    version,
    bountyId: readBigInt(payload, 'bounty_id'),
    amount: readBigInt(payload, 'amount'),
    refundTo: readString(payload, 'refund_to'),
    timestamp: readBigInt(payload, 'timestamp'),
  };
}

function decodeBountyExpired(topics: NativeTopic[], payload: NativeRecord): DecodedSorobanEvent {
  const version = readSupportedVersion('bounty.expired', topics, payload, [2]);
  if (isUnknownVersion(version)) return version;
  return {
    kind: 'bounty.expired',
    contract: 'bounty_escrow',
    topic: 'b_exp',
    version,
    bountyId: readBigInt(payload, 'bounty_id'),
    depositor: readString(payload, 'depositor'),
    amount: readBigInt(payload, 'amount'),
    deadline: readBigInt(payload, 'deadline'),
    expiredAt: readBigInt(payload, 'expired_at'),
  };
}

function decodeBountyClaimCreated(topics: NativeTopic[], payload: NativeRecord): DecodedSorobanEvent {
  const version = readSupportedVersion('bounty.claim_created', topics, payload, [2]);
  if (isUnknownVersion(version)) return version;
  return {
    kind: 'bounty.claim_created',
    contract: 'bounty_escrow',
    topic: 'claim/created',
    version,
    bountyId: readBigInt(payload, 'bounty_id'),
    recipient: readString(payload, 'recipient'),
    amount: readBigInt(payload, 'amount'),
    expiresAt: readBigInt(payload, 'expires_at'),
  };
}

function decodeBountyStateTransitioned(topics: NativeTopic[], payload: NativeRecord): DecodedSorobanEvent {
  const version = readSupportedVersion('bounty.state_transitioned', topics, payload, [1]);
  if (isUnknownVersion(version)) return version;
  return {
    kind: 'bounty.state_transitioned',
    contract: 'bounty_escrow',
    topic: 'analytics/state_tx',
    version,
    bountyId: readBigInt(payload, 'bounty_id'),
    previousState: readString(payload, 'previous_state'),
    newState: readString(payload, 'new_state'),
    amount: readBigInt(payload, 'amount'),
    actor: readString(payload, 'actor'),
    timestamp: readBigInt(payload, 'timestamp'),
  };
}

function decodeBountyActivity(topics: NativeTopic[], payload: NativeRecord): DecodedSorobanEvent {
  const version = readSupportedVersion('bounty.activity', topics, payload, [1]);
  if (isUnknownVersion(version)) return version;
  return {
    kind: 'bounty.activity',
    contract: 'bounty_escrow',
    topic: 'analytics/activity',
    version,
    bountyId: readBigInt(payload, 'bounty_id'),
    activityType: readString(payload, 'activity_type'),
    amount: readBigInt(payload, 'amount'),
    timestamp: readBigInt(payload, 'timestamp'),
  };
}

function decodeBountyAnalyticsSnapshot(topics: NativeTopic[], payload: NativeRecord): DecodedSorobanEvent {
  const version = readSupportedVersion('bounty.analytics_snapshot', topics, payload, [1]);
  if (isUnknownVersion(version)) return version;
  const metrics = readRecord(payload, 'metrics');
  return {
    kind: 'bounty.analytics_snapshot',
    contract: 'bounty_escrow',
    topic: 'analytics/snap',
    version,
    metrics: {
      activeBountyCount: readNumber(metrics, 'active_bounty_count'),
      releasedBountyCount: readNumber(metrics, 'released_bounty_count'),
      refundedBountyCount: readNumber(metrics, 'refunded_bounty_count'),
      totalLocked: readBigInt(metrics, 'total_locked'),
      totalReleased: readBigInt(metrics, 'total_released'),
      totalRefunded: readBigInt(metrics, 'total_refunded'),
      averageBountyAmount: readBigInt(metrics, 'average_bounty_amount'),
      snapshotTimestamp: readBigInt(metrics, 'snapshot_timestamp'),
    },
  };
}

function decodeProgramInitialized(topics: NativeTopic[], payload: NativeRecord): DecodedSorobanEvent {
  const version = readSupportedVersion('program.initialized', topics, payload, [2]);
  if (isUnknownVersion(version)) return version;
  return {
    kind: 'program.initialized',
    contract: 'program_escrow',
    topic: 'PrgInit',
    version,
    programId: readString(payload, 'program_id'),
    authorizedPayoutKey: readString(payload, 'authorized_payout_key'),
    tokenAddress: readString(payload, 'token_address'),
    totalFunds: readBigInt(payload, 'total_funds'),
  };
}

function decodeProgramFundsLocked(topics: NativeTopic[], payload: NativeRecord): DecodedSorobanEvent {
  const version = readSupportedVersion('program.funds_locked', topics, payload, [2]);
  if (isUnknownVersion(version)) return version;
  return {
    kind: 'program.funds_locked',
    contract: 'program_escrow',
    topic: 'FndsLock',
    version,
    programId: readString(payload, 'program_id'),
    amount: readBigInt(payload, 'amount'),
    remainingBalance: readBigInt(payload, 'remaining_balance'),
  };
}

function decodeProgramPayout(topics: NativeTopic[], payload: NativeRecord): DecodedSorobanEvent {
  const version = readSupportedVersion('program.payout', topics, payload, [2]);
  if (isUnknownVersion(version)) return version;
  return {
    kind: 'program.payout',
    contract: 'program_escrow',
    topic: 'Payout',
    version,
    programId: readString(payload, 'program_id'),
    recipient: readString(payload, 'recipient'),
    amount: readBigInt(payload, 'amount'),
    remainingBalance: readBigInt(payload, 'remaining_balance'),
  };
}

function decodeProgramBatchPayout(topics: NativeTopic[], payload: NativeRecord): DecodedSorobanEvent {
  const version = readSupportedVersion('program.batch_payout', topics, payload, [2]);
  if (isUnknownVersion(version)) return version;
  return {
    kind: 'program.batch_payout',
    contract: 'program_escrow',
    topic: 'BatchPay',
    version,
    programId: readString(payload, 'program_id'),
    recipientCount: readNumber(payload, 'recipient_count'),
    totalAmount: readBigInt(payload, 'total_amount'),
    remainingBalance: readBigInt(payload, 'remaining_balance'),
    gasProxyTransferOps: readNumber(payload, 'gas_proxy_transfer_ops'),
    gasProxyHistoryAppends: readNumber(payload, 'gas_proxy_history_appends'),
    gasProxyStorageReads: readNumber(payload, 'gas_proxy_storage_reads'),
    gasProxyStorageWrites: readNumber(payload, 'gas_proxy_storage_writes'),
    gasProxyEventsEmitted: readNumber(payload, 'gas_proxy_events_emitted'),
  };
}

function decodeProgramScheduleTriggered(topics: NativeTopic[], payload: NativeRecord): DecodedSorobanEvent {
  const version = readSupportedVersion('program.schedule_triggered', topics, payload, [2]);
  if (isUnknownVersion(version)) return version;
  return {
    kind: 'program.schedule_triggered',
    contract: 'program_escrow',
    topic: 'SchedTrg',
    version,
    programId: readString(payload, 'program_id'),
    scheduleId: readBigInt(payload, 'schedule_id'),
    recipient: readString(payload, 'recipient'),
    amount: readBigInt(payload, 'amount'),
    triggerType: readString(payload, 'trigger_type'),
  };
}

function decodeProgramDisputeOpened(topics: NativeTopic[], payload: NativeRecord): DecodedSorobanEvent {
  const version = readSupportedVersion('program.dispute_opened', topics, payload, [2]);
  if (isUnknownVersion(version)) return version;
  return {
    kind: 'program.dispute_opened',
    contract: 'program_escrow',
    topic: 'DispOpen',
    version,
    programId: readString(payload, 'program_id'),
    openedBy: readString(payload, 'opened_by'),
    reason: readString(payload, 'reason'),
    timestamp: readBigInt(payload, 'timestamp'),
  };
}

function decodeProgramAggregateStats(topics: NativeTopic[], payload: NativeRecord): DecodedSorobanEvent {
  const version = readSupportedVersion('program.aggregate_stats', topics, payload, [2]);
  if (isUnknownVersion(version)) return version;
  return {
    kind: 'program.aggregate_stats',
    contract: 'program_escrow',
    topic: 'AggStats',
    version,
    programId: readString(payload, 'program_id'),
    totalFunds: readBigInt(payload, 'total_funds'),
    remainingBalance: readBigInt(payload, 'remaining_balance'),
    totalPaidOut: readBigInt(payload, 'total_paid_out'),
    payoutCount: readNumber(payload, 'payout_count'),
    scheduledCount: readNumber(payload, 'scheduled_count'),
  };
}

function decodeProgramLargePayout(topics: NativeTopic[], payload: NativeRecord): DecodedSorobanEvent {
  const version = readSupportedVersion('program.large_payout', topics, payload, [2]);
  if (isUnknownVersion(version)) return version;
  return {
    kind: 'program.large_payout',
    contract: 'program_escrow',
    topic: 'LrgPay',
    version,
    programId: readString(payload, 'program_id'),
    recipient: readString(payload, 'recipient'),
    amount: readBigInt(payload, 'amount'),
    threshold: readBigInt(payload, 'threshold'),
  };
}

function decoderKeyForTopics(topics: NativeTopic[]): string | undefined {
  const first = topicString(topics[0]);
  const second = topicString(topics[1]);

  if (first === 'claim' && second === 'created') return 'claim/created';
  if (first === 'analytics' && second === 'state_tx') return 'analytics/state_tx';
  if (first === 'analytics' && second === 'activity') return 'analytics/activity';
  if (first === 'analytics' && second === 'snap') return 'analytics/snap';
  return first;
}

function readSupportedVersion<T extends number>(
  eventName: DecodedContractEvent['kind'],
  topics: NativeTopic[],
  payload: NativeRecord,
  supportedVersions: readonly T[]
): T | UnknownVersionSorobanEvent {
  const version = readNumber(payload, 'version');
  if (!supportedVersions.some((supportedVersion) => supportedVersion === version)) {
    return {
      kind: 'unknown_version',
      eventName,
      version,
      supportedVersions: [...supportedVersions],
      topics,
      rawPayload: payload,
    };
  }
  return version as T;
}

function isUnknownVersion<T extends number>(
  version: T | UnknownVersionSorobanEvent
): version is UnknownVersionSorobanEvent {
  return typeof version === 'object';
}

function readRecord(payload: NativeRecord, field: string): NativeRecord {
  const value = readRequired(payload, field);
  if (!isRecord(value)) {
    throw new Error(`${field} must be an object`);
  }
  return value;
}

function readBigInt(payload: NativeRecord, field: string): bigint {
  const value = readRequired(payload, field);
  if (typeof value === 'bigint') return value;
  if (typeof value === 'number' && Number.isInteger(value)) return BigInt(value);
  if (typeof value === 'string' && /^-?\d+$/.test(value)) return BigInt(value);
  throw new Error(`${field} must be an integer`);
}

function readNumber(payload: NativeRecord, field: string): number {
  const value = readRequired(payload, field);
  const numericValue =
    typeof value === 'number'
      ? value
      : typeof value === 'bigint'
        ? Number(value)
        : typeof value === 'string' && /^-?\d+$/.test(value)
          ? Number(value)
          : Number.NaN;

  if (!Number.isSafeInteger(numericValue)) {
    throw new Error(`${field} must be a safe integer`);
  }
  return numericValue;
}

function readString(payload: NativeRecord, field: string): string {
  const value = readRequired(payload, field);
  if (typeof value === 'string') return value;
  if (typeof value === 'number' || typeof value === 'bigint' || typeof value === 'boolean') {
    return String(value);
  }
  if (value && typeof value === 'object' && typeof (value as { toString?: unknown }).toString === 'function') {
    return String(value);
  }
  throw new Error(`${field} must be string-like`);
}

function readRequired(payload: NativeRecord, field: string): unknown {
  if (!Object.prototype.hasOwnProperty.call(payload, field)) {
    throw new Error(`${field} is required`);
  }
  return payload[field];
}

function normalizeTopics(rawTopics: unknown): NativeTopic[] {
  const topics = Array.isArray(rawTopics) ? rawTopics : rawTopics === undefined ? [] : [rawTopics];
  return topics.map((topic) => normalizeTopicValue(normalizeNative(topic)));
}

function normalizeTopicValue(value: unknown): NativeTopic {
  if (
    typeof value === 'string' ||
    typeof value === 'number' ||
    typeof value === 'bigint' ||
    typeof value === 'boolean'
  ) {
    return value;
  }
  if (value && typeof value === 'object' && typeof (value as { toString?: unknown }).toString === 'function') {
    return String(value);
  }
  return String(value);
}

function normalizeNative(value: unknown): unknown {
  if (value === null || value === undefined) return value;

  try {
    return normalizeNative(scValToNative(value as never));
  } catch (_error) {
    // The value may already be a native JS object; keep normalizing below.
  }

  if (value instanceof Map) {
    const record: NativeRecord = {};
    for (const [key, item] of value.entries()) {
      record[String(normalizeNative(key))] = normalizeNative(item);
    }
    return record;
  }

  if (Array.isArray(value)) {
    return value.map((item) => normalizeNative(item));
  }

  if (isRecord(value)) {
    const record: NativeRecord = {};
    for (const [key, item] of Object.entries(value)) {
      record[key] = normalizeNative(item);
    }
    return record;
  }

  return value;
}

function isRecord(value: unknown): value is NativeRecord {
  if (!value || typeof value !== 'object') return false;
  if (value instanceof Map || Array.isArray(value)) return false;
  return Object.getPrototypeOf(value) === Object.prototype || Object.getPrototypeOf(value) === null;
}

function topicString(topic: NativeTopic | undefined): string | undefined {
  return topic === undefined ? undefined : String(topic);
}

function malformed(topics: NativeTopic[], rawPayload: unknown, reason: string): MalformedSorobanEvent {
  return {
    kind: 'malformed',
    topics,
    reason,
    rawPayload,
  };
}

