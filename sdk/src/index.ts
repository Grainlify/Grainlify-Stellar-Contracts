export { ProgramEscrowClient } from './program-escrow-client';
export type { 
  ProgramEscrowConfig, 
  ProgramData, 
  PayoutRecord,
  ProgramReleaseSchedule 
} from './program-escrow-client';

export { BountyEscrowClient } from './bounty-escrow-client';
export type {
  BountyEscrowConfig,
  LockFundsItem,
  ReleaseFundsItem,
  EscrowStatus,
  RefundMode,
  RefundRecord,
  ClaimRecord,
  Escrow,
  EscrowWithId,
  EscrowQueryFilter,
  AggregateStats,
  RefundApproval,
  RefundEligibility,
  FeeConfig,
  PauseFlags,
  MultisigConfig,
  CircuitBreakerConfig,
  CircuitState,
  CircuitBreakerStatus,
  AdminConfigSnapshot
} from './bounty-escrow-client';

export { 
  invokeContract,
  waitForConfirmation,
} from './invocation';
export type {
  InvocationConfig,
  InvokeOptions,
} from './invocation';

export { 
  SDKError,
  ContractError,
  NetworkError,
  ValidationError,
  ContractErrorCode,
  createContractError,
  parseContractError,
  parseContractErrorByCode,
  getContractErrorMessage,
  PROGRAM_ESCROW_ERROR_MAP,
  BOUNTY_ESCROW_ERROR_MAP,
  GOVERNANCE_ERROR_MAP,
  CIRCUIT_BREAKER_ERROR_MAP,
} from './errors';

export { decodeContractEvent } from './events';
export type {
  BountyEscrowInitializedEvent,
  FundsLockedEvent,
  FundsReleasedEvent,
  FundsRefundedEvent,
  BountyExpiredEvent,
  FeeCollectedEvent,
  BatchFundsLockedEvent,
  FeeConfigUpdatedEvent,
  BatchFundsReleasedEvent,
  ApprovalAddedEvent,
  PauseStateChangedEvent,
  BountyStateTransitionedEvent,
  ContractAnalytics,
  AnalyticsSnapshotEvent,
  BountyActivityEvent,
  ProgramInitializedEvent,
  ProgramFundsLockedEvent,
  BatchPayoutEvent,
  PayoutEvent,
  DisputeScope,
  DisputeOpenedEvent,
  DisputeResolvedEvent,
  DisputeCancelledEvent,
  ProgramPauseStateChangedEvent,
  AggregateStatsEvent,
  LargePayoutEvent,
  ScheduleTriggeredEvent,
  OperationMetricEvent,
  PerformanceMetricEvent,
  BountyEscrowEvent,
  ProgramEscrowEvent,
  DecodedEvent
} from './events';
