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
  RefundRecord,
  ClaimRecord,
  Escrow,
  FeeConfig,
  PauseFlags
} from './bounty-escrow-client';

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
  BOUNTY_ESCROW_ERROR_MAP,
  GOVERNANCE_ERROR_MAP,
  CIRCUIT_BREAKER_ERROR_MAP,
} from './errors';
