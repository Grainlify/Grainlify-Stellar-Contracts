import { Contract, SorobanRpc, Keypair } from '@stellar/stellar-sdk';
import { NetworkError, ValidationError, parseContractError, ContractError } from './errors';
import { invokeContract, InvocationConfig } from './invocation';

export interface BountyEscrowConfig {
  /** Deployed BountyEscrow contract address. */
  contractId: string;
  /** Soroban RPC endpoint used for reads and transaction submission. */
  rpcUrl: string;
  /** Stellar network passphrase for the target network. */
  networkPassphrase: string;
}

/** Input item for batch-locking a bounty escrow. */
export interface LockFundsItem {
  /** Application-level bounty identifier. */
  bounty_id: bigint;
  /** Stellar account that deposits the escrowed funds. */
  depositor: string;
  /** Amount to lock, expressed in the contract token's smallest unit. */
  amount: bigint;
  /** Unix timestamp after which the bounty may become refundable. */
  deadline: number;
}

/** Input item for batch-releasing a bounty escrow. */
export interface ReleaseFundsItem {
  /** Application-level bounty identifier. */
  bounty_id: bigint;
  /** Stellar account that should receive the released bounty funds. */
  contributor: string;
}

/** On-chain lifecycle states for a bounty escrow. */
export type EscrowStatus = 'Locked' | 'Released' | 'Refunded' | 'PartiallyRefunded';

/** Supported refund modes for admin-approved refunds. */
export type RefundMode = 'Full' | 'Partial';

/** Historical refund record attached to an escrow. */
export interface RefundRecord {
  /** Refunded amount in the contract token's smallest unit. */
  amount: bigint;
  /** Stellar account that received the refund. */
  recipient: string;
  /** Unix timestamp when the refund was executed. */
  timestamp: number;
  /** Whether the refund closed the escrow or returned a partial amount. */
  mode: RefundMode;
}

/** Pending claim authorization for a bounty recipient. */
export interface ClaimRecord {
  /** Application-level bounty identifier. */
  bounty_id: bigint;
  /** Stellar account authorized to claim the bounty. */
  recipient: string;
  /** Claimable amount in the contract token's smallest unit. */
  amount: bigint;
  /** Unix timestamp when the claim authorization expires. */
  expires_at: number;
  /** Whether the authorized claim has already been consumed. */
  claimed: boolean;
}

/** Current state for one bounty escrow. */
export interface Escrow {
  /** Stellar account that deposited the escrow funds. */
  depositor: string;
  /** Original locked amount in the contract token's smallest unit. */
  amount: bigint;
  /** Remaining escrow balance after releases or partial refunds. */
  remaining_amount: bigint;
  /** Current on-chain escrow lifecycle state. */
  status: EscrowStatus;
  /** Unix timestamp used by refund eligibility checks. */
  deadline: number;
  /** Refund events recorded for this escrow. */
  refund_history: RefundRecord[];
}

/** Escrow record paired with its bounty identifier. */
export interface EscrowWithId {
  /** Application-level bounty identifier. */
  bounty_id: bigint;
  /** Escrow state for the identifier. */
  escrow: Escrow;
}

/** Composite filter supported by the bounty escrow query endpoint. */
export interface EscrowQueryFilter {
  /** Enables filtering by lifecycle status when true. */
  has_status_filter: boolean;
  /** Lifecycle status to match when status filtering is enabled. */
  status: EscrowStatus;
  /** Enables filtering by depositor account when true. */
  has_depositor_filter: boolean;
  /** Depositor account to match when depositor filtering is enabled. */
  depositor: string;
  /** Inclusive minimum escrow amount. */
  min_amount: bigint;
  /** Inclusive maximum escrow amount. */
  max_amount: bigint;
  /** Inclusive minimum deadline timestamp. */
  min_deadline: number;
  /** Inclusive maximum deadline timestamp. */
  max_deadline: number;
}

/** Aggregate totals and counts across indexed bounty escrows. */
export interface AggregateStats {
  /** Sum of currently locked funds. */
  total_locked: bigint;
  /** Sum of released funds. */
  total_released: bigint;
  /** Sum of refunded funds. */
  total_refunded: bigint;
  /** Number of locked escrows. */
  count_locked: number;
  /** Number of released escrows. */
  count_released: number;
  /** Number of refunded escrows. */
  count_refunded: number;
}

/** Admin approval record required before a refund can be executed. */
export interface RefundApproval {
  /** Application-level bounty identifier. */
  bounty_id: bigint;
  /** Approved refund amount. */
  amount: bigint;
  /** Stellar account that may receive the refund. */
  recipient: string;
  /** Approved refund mode. */
  mode: RefundMode;
  /** Admin account that approved the refund. */
  approved_by: string;
  /** Unix timestamp when the approval was recorded. */
  approved_at: number;
}

/** Refund eligibility result for a bounty escrow. */
export interface RefundEligibility {
  /** True when the escrow can be refunded immediately. */
  can_refund: boolean;
  /** Whether the escrow deadline has elapsed. */
  deadline_passed: boolean;
  /** Remaining refundable amount. */
  remaining_amount: bigint;
  /** Optional approval details for admin-approved refunds. */
  approval?: RefundApproval;
}

/** Fee policy configured on the bounty escrow contract. */
export interface FeeConfig {
  /** Fee charged when locking funds, in basis points. */
  lock_fee_rate: bigint;
  /** Fee charged when releasing funds, in basis points. */
  release_fee_rate: bigint;
  /** Stellar account that receives fees. */
  fee_recipient: string;
  /** Whether fee collection is currently enabled. */
  fee_enabled: boolean;
}

/** Pause switches for bounty escrow operations. */
export interface PauseFlags {
  /** Whether operations are globally paused. */
  global_paused: boolean;
  /** Whether lock operations are paused. */
  lock_paused: boolean;
  /** Whether release operations are paused. */
  release_paused: boolean;
  /** Whether refund operations are paused. */
  refund_paused: boolean;
}

/** Multisig configuration on the bounty escrow contract. */
export interface MultisigConfig {
  /** The threshold amount above which releases require multisig approvals. */
  threshold_amount: bigint;
  /** Addresses authorized to approve large releases. */
  signers: string[];
  /** Minimum number of required signatures from signers. */
  required_signatures: number;
}

/** Configuration thresholds for the circuit breaker. */
export interface CircuitBreakerConfig {
  /** Consecutive failures before opening the circuit. */
  failure_threshold: number;
  /** Consecutive successes in HalfOpen state before closing the circuit. */
  success_threshold: number;
  /** Maximum number of records to retain in the error log. */
  max_error_log: number;
}

/** Error log record stored by the circuit breaker. */
export interface ErrorEntry {
  /** Symbol identifying the operation that failed. */
  operation: string;
  /** Error code or description. */
  error_code: number;
  /** Unix timestamp when the error occurred. */
  timestamp: number;
}

/** Status values for the circuit breaker. */
export type CircuitState = 'Closed' | 'Open' | 'HalfOpen';

/** Current operational status of the circuit breaker. */
export interface CircuitBreakerStatus {
  /** Current state of the circuit breaker. */
  state: CircuitState;
  /** Number of consecutive failures in Closed state. */
  consecutive_failures: number;
  /** Number of consecutive successes in HalfOpen state. */
  consecutive_successes: number;
  /** Unix timestamp when the state last transitioned. */
  last_state_change: number;
}

/**
 * Client for interacting with the BountyEscrow Soroban contract
 */
export class BountyEscrowClient {
  private contract: Contract;
  private server: SorobanRpc.Server;
  private config: BountyEscrowConfig;
  private invocationConfig: InvocationConfig;

  /**
   * Create a client bound to one BountyEscrow contract and Soroban RPC endpoint.
   */
  constructor(config: BountyEscrowConfig) {
    this.config = config;
    try {
      this.contract = new Contract(config.contractId);
    } catch (error) {
      this.contract = null as any;
    }
    try {
      this.server = new SorobanRpc.Server(config.rpcUrl, { allowHttp: true });
    } catch (error) {
      this.server = null as any;
    }
    this.invocationConfig = {
      server: this.server,
      contract: this.contract,
      networkPassphrase: config.networkPassphrase,
      rpcUrl: config.rpcUrl,
    };
  }

  /**
   * Initialize the bounty escrow contract
   */
  async init(
    adminAddress: string,
    tokenAddress: string,
    sourceKeypair: Keypair
  ): Promise<void> {
    this.validateAddress(adminAddress, 'adminAddress');
    this.validateAddress(tokenAddress, 'tokenAddress');

    try {
      await this.invokeContract('init', [adminAddress, tokenAddress], sourceKeypair);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Lock funds into a bounty escrow
   */
  async lockFunds(
    depositor: string,
    bountyId: bigint,
    amount: bigint,
    deadline: number,
    sourceKeypair: Keypair
  ): Promise<void> {
    this.validateAddress(depositor, 'depositor');
    if (amount <= 0n) {
      throw new ValidationError('Amount must be greater than zero', 'amount');
    }
    if (deadline <= 0) {
      throw new ValidationError('Deadline must be in the future', 'deadline');
    }

    try {
      await this.invokeContract('lock_funds', [depositor, bountyId, amount, deadline], sourceKeypair);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Release full funds for a bounty to a contributor
   */
  async releaseFunds(
    bountyId: bigint,
    contributor: string,
    sourceKeypair: Keypair
  ): Promise<void> {
    this.validateAddress(contributor, 'contributor');

    try {
      await this.invokeContract('release_funds', [bountyId, contributor], sourceKeypair);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Release partial funds for a bounty to a contributor
   */
  async partialRelease(
    bountyId: bigint,
    contributor: string,
    amount: bigint,
    sourceKeypair: Keypair
  ): Promise<void> {
    this.validateAddress(contributor, 'contributor');
    if (amount <= 0n) {
      throw new ValidationError('Amount must be greater than zero', 'amount');
    }

    try {
      await this.invokeContract('partial_release', [bountyId, contributor, amount], sourceKeypair);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Approve a refund for a bounty
   */
  async approveRefund(
    bountyId: bigint,
    amount: bigint,
    recipient: string,
    mode: RefundMode,
    sourceKeypair: Keypair
  ): Promise<void> {
    this.validateAddress(recipient, 'recipient');
    if (amount <= 0n) {
      throw new ValidationError('Amount must be greater than zero', 'amount');
    }
    this.validateRefundMode(mode);

    try {
      await this.invokeContract('approve_refund', [bountyId, amount, recipient, mode], sourceKeypair);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Execute a refund for a bounty
   */
  async refund(
    bountyId: bigint,
    sourceKeypair: Keypair
  ): Promise<void> {
    try {
      await this.invokeContract('refund', [bountyId], sourceKeypair);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Authorize a claim for a bounty
   */
  async authorizeClaim(
    bountyId: bigint,
    recipient: string,
    sourceKeypair: Keypair
  ): Promise<void> {
    this.validateAddress(recipient, 'recipient');

    try {
      await this.invokeContract('authorize_claim', [bountyId, recipient], sourceKeypair);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Set the global claim window in seconds. Admin-only on chain.
   */
  async setClaimWindow(
    claimWindow: number,
    sourceKeypair: Keypair
  ): Promise<void> {
    if (!Number.isInteger(claimWindow) || claimWindow < 0) {
      throw new ValidationError('Claim window must be a non-negative integer', 'claimWindow');
    }

    try {
      await this.invokeContract('set_claim_window', [claimWindow], sourceKeypair);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Execute a claim for a bounty
   */
  async claim(
    bountyId: bigint,
    sourceKeypair: Keypair
  ): Promise<void> {
    try {
      await this.invokeContract('claim', [bountyId], sourceKeypair);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Cancel a pending claim. Admin-only on chain.
   */
  async cancelPendingClaim(
    bountyId: bigint,
    sourceKeypair: Keypair
  ): Promise<void> {
    try {
      await this.invokeContract('cancel_pending_claim', [bountyId], sourceKeypair);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Batch lock funds for multiple bounties
   */
  async batchLockFunds(
    items: LockFundsItem[],
    sourceKeypair: Keypair
  ): Promise<number> {
    if (items.length === 0) {
      throw new ValidationError('Items array cannot be empty', 'items');
    }
    
    for (let i = 0; i < items.length; i++) {
      this.validateAddress(items[i].depositor, `items[${i}].depositor`);
      if (items[i].amount <= 0n) {
        throw new ValidationError(`Amount at index ${i} must be greater than zero`, 'amount');
      }
    }

    try {
      const result = await this.invokeContract('batch_lock_funds', [items], sourceKeypair);
      return Number(result);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Batch release funds for multiple bounties
   */
  async batchReleaseFunds(
    items: ReleaseFundsItem[],
    sourceKeypair: Keypair
  ): Promise<number> {
    if (items.length === 0) {
      throw new ValidationError('Items array cannot be empty', 'items');
    }
    
    for (let i = 0; i < items.length; i++) {
      this.validateAddress(items[i].contributor, `items[${i}].contributor`);
    }

    try {
      const result = await this.invokeContract('batch_release_funds', [items], sourceKeypair);
      return Number(result);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Get information about a specific escrow
   */
  async getEscrowInfo(bountyId: bigint): Promise<Escrow> {
    try {
      const result = await this.invokeContract('get_escrow_info', [bountyId]);
      return result as Escrow;
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Get the pending claim for a bounty.
   */
  async getPendingClaim(bountyId: bigint): Promise<ClaimRecord> {
    try {
      const result = await this.invokeContract('get_pending_claim', [bountyId]);
      return result as ClaimRecord;
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Get the current contract balance
   */
  async getBalance(): Promise<bigint> {
    try {
      const result = await this.invokeContract('get_balance', []);
      return BigInt(result);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Query escrows by status.
   */
  async queryEscrowsByStatus(
    status: EscrowStatus,
    offset = 0,
    limit = 50
  ): Promise<EscrowWithId[]> {
    this.validatePagination(offset, limit);

    try {
      const result = await this.invokeContract('query_escrows_by_status', [status, offset, limit]);
      return result as EscrowWithId[];
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Query escrows by amount range.
   */
  async queryEscrowsByAmount(
    minAmount: bigint,
    maxAmount: bigint,
    offset = 0,
    limit = 50
  ): Promise<EscrowWithId[]> {
    if (minAmount < 0n || maxAmount < minAmount) {
      throw new ValidationError('Amount range is invalid', 'amount');
    }
    this.validatePagination(offset, limit);

    try {
      const result = await this.invokeContract('query_escrows_by_amount', [minAmount, maxAmount, offset, limit]);
      return result as EscrowWithId[];
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Query escrows by deadline range.
   */
  async queryEscrowsByDeadline(
    minDeadline: number,
    maxDeadline: number,
    offset = 0,
    limit = 50
  ): Promise<EscrowWithId[]> {
    if (!Number.isInteger(minDeadline) || !Number.isInteger(maxDeadline) || minDeadline < 0 || maxDeadline < minDeadline) {
      throw new ValidationError('Deadline range is invalid', 'deadline');
    }
    this.validatePagination(offset, limit);

    try {
      const result = await this.invokeContract('query_escrows_by_deadline', [minDeadline, maxDeadline, offset, limit]);
      return result as EscrowWithId[];
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Query escrows by depositor.
   */
  async queryEscrowsByDepositor(
    depositor: string,
    offset = 0,
    limit = 50
  ): Promise<EscrowWithId[]> {
    this.validateAddress(depositor, 'depositor');
    this.validatePagination(offset, limit);

    try {
      const result = await this.invokeContract('query_escrows_by_depositor', [depositor, offset, limit]);
      return result as EscrowWithId[];
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Query escrows with the composite on-chain filter.
   */
  async queryEscrows(
    filter: EscrowQueryFilter,
    offset = 0,
    limit = 50
  ): Promise<EscrowWithId[]> {
    if (filter.has_depositor_filter) {
      this.validateAddress(filter.depositor, 'filter.depositor');
    }
    if (filter.min_amount < 0n || filter.max_amount < filter.min_amount) {
      throw new ValidationError('Filter amount range is invalid', 'filter.amount');
    }
    if (filter.min_deadline < 0 || filter.max_deadline < filter.min_deadline) {
      throw new ValidationError('Filter deadline range is invalid', 'filter.deadline');
    }
    this.validatePagination(offset, limit);

    try {
      const result = await this.invokeContract('query_escrows', [filter, offset, limit]);
      return result as EscrowWithId[];
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Get aggregate escrow statistics.
   */
  async getAggregateStats(): Promise<AggregateStats> {
    try {
      const result = await this.invokeContract('get_aggregate_stats', []);
      return result as AggregateStats;
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Get the total number of indexed escrows.
   */
  async getEscrowCount(): Promise<number> {
    try {
      const result = await this.invokeContract('get_escrow_count', []);
      return Number(result);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Get escrow IDs matching a status filter.
   */
  async getEscrowIdsByStatus(
    status: EscrowStatus,
    offset = 0,
    limit = 50
  ): Promise<bigint[]> {
    this.validatePagination(offset, limit);

    try {
      const result = await this.invokeContract('get_escrow_ids_by_status', [status, offset, limit]);
      return result as bigint[];
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Get refund history for a bounty.
   */
  async getRefundHistory(bountyId: bigint): Promise<RefundRecord[]> {
    try {
      const result = await this.invokeContract('get_refund_history', [bountyId]);
      return result as RefundRecord[];
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Get refund eligibility and optional approval details for a bounty.
   */
  async getRefundEligibility(bountyId: bigint): Promise<RefundEligibility> {
    try {
      const result = await this.invokeContract('get_refund_eligibility', [bountyId]);
      if (Array.isArray(result)) {
        return {
          can_refund: Boolean(result[0]),
          deadline_passed: Boolean(result[1]),
          remaining_amount: BigInt(result[2]),
          approval: result[3] ?? undefined,
        };
      }
      return result as RefundEligibility;
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Query locked or partially refunded bounties whose deadline is at or before maxDeadline.
   */
  async queryExpiringBounties(
    maxDeadline: number,
    offset = 0,
    limit = 50
  ): Promise<bigint[]> {
    if (!Number.isInteger(maxDeadline) || maxDeadline < 0) {
      throw new ValidationError('Max deadline must be a non-negative integer', 'maxDeadline');
    }
    this.validatePagination(offset, limit);

    try {
      const result = await this.invokeContract('query_expiring_bounties', [maxDeadline, offset, limit]);
      return result as bigint[];
    } catch (error) {
      throw this.handleError(error);
    }
  }
  
  /**
   * Get the current fee configuration
   */
  async getFeeConfig(): Promise<FeeConfig> {
    try {
      const result = await this.invokeContract('get_fee_config', []);
      return result as FeeConfig;
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Get the current pause flags
   */
  async getPauseFlags(): Promise<PauseFlags> {
    try {
      const result = await this.invokeContract('get_pause_flags', []);
      return result as PauseFlags;
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Update fee configuration. Admin-only.
   * @param lockFeeRate Optional lock fee rate in basis points.
   * @param releaseFeeRate Optional release fee rate in basis points.
   * @param feeRecipient Optional Stellar address of the fee recipient.
   * @param feeEnabled Optional boolean to enable/disable fees.
   * @param sourceKeypair Signing keypair of the admin.
   */
  async updateFeeConfig(
    lockFeeRate: bigint | null,
    releaseFeeRate: bigint | null,
    feeRecipient: string | null,
    feeEnabled: boolean | null,
    sourceKeypair: Keypair
  ): Promise<void> {
    if (feeRecipient) {
      this.validateAddress(feeRecipient, 'feeRecipient');
    }
    if (lockFeeRate !== null && (lockFeeRate < 0n || lockFeeRate > 10000n)) {
      throw new ValidationError('Lock fee rate must be between 0 and 10000 basis points', 'lockFeeRate');
    }
    if (releaseFeeRate !== null && (releaseFeeRate < 0n || releaseFeeRate > 10000n)) {
      throw new ValidationError('Release fee rate must be between 0 and 10000 basis points', 'releaseFeeRate');
    }

    try {
      await this.invokeContract(
        'update_fee_config',
        [lockFeeRate, releaseFeeRate, feeRecipient, feeEnabled],
        sourceKeypair
      );
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Update pause flags. Admin-only.
   * @param lock Optional boolean to pause lock operations.
   * @param release Optional boolean to pause release operations.
   * @param refund Optional boolean to pause refund operations.
   * @param sourceKeypair Signing keypair of the admin.
   */
  async setPaused(
    lock: boolean | null,
    release: boolean | null,
    refund: boolean | null,
    sourceKeypair: Keypair
  ): Promise<void> {
    try {
      await this.invokeContract('set_paused', [lock, release, refund], sourceKeypair);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Set the emergency pause switch. Admin-only.
   * @param paused Boolean indicating whether contract is globally paused.
   * @param sourceKeypair Signing keypair of the admin.
   */
  async setEmergencyPause(
    paused: boolean,
    sourceKeypair: Keypair
  ): Promise<void> {
    try {
      await this.invokeContract('set_emergency_pause', [paused], sourceKeypair);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Set the governance contract address. Admin-only.
   * @param governanceAddr Stellar address of the governance contract.
   * @param sourceKeypair Signing keypair of the admin.
   */
  async setGovernanceContract(
    governanceAddr: string,
    sourceKeypair: Keypair
  ): Promise<void> {
    this.validateAddress(governanceAddr, 'governanceAddr');
    try {
      await this.invokeContract('set_governance_contract', [governanceAddr], sourceKeypair);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Get the governance contract address.
   * @returns Governance contract address or null if not set.
   */
  async getGovernanceContract(): Promise<string | null> {
    try {
      const result = await this.invokeContract('get_governance_contract', []);
      return result as string | null;
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Set the minimum required governance version. Admin-only.
   * @param minVersion Minimum required version number.
   * @param sourceKeypair Signing keypair of the admin.
   */
  async setMinGovernanceVersion(
    minVersion: number,
    sourceKeypair: Keypair
  ): Promise<void> {
    if (!Number.isInteger(minVersion) || minVersion < 0) {
      throw new ValidationError('Min version must be a non-negative integer', 'minVersion');
    }
    try {
      await this.invokeContract('set_min_governance_version', [minVersion], sourceKeypair);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Get the minimum required governance version.
   * @returns Minimum required version number.
   */
  async getMinGovernanceVersion(): Promise<number> {
    try {
      const result = await this.invokeContract('get_min_governance_version', []);
      return Number(result);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Set the circuit breaker admin address. Admin-only.
   * @param admin Stellar address of the circuit breaker admin.
   * @param sourceKeypair Signing keypair of the admin.
   */
  async setCircuitBreakerAdmin(
    admin: string,
    sourceKeypair: Keypair
  ): Promise<void> {
    this.validateAddress(admin, 'admin');
    try {
      await this.invokeContract('set_circuit_breaker_admin', [admin], sourceKeypair);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Get the circuit breaker admin address.
   * @returns Circuit breaker admin address or null if not set.
   */
  async getCircuitBreakerAdmin(): Promise<string | null> {
    try {
      const result = await this.invokeContract('get_circuit_breaker_admin', []);
      return result as string | null;
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Configure circuit breaker threshold parameters. Admin-only.
   * @param failureThreshold Number of failures before tripping circuit.
   * @param successThreshold Number of successes to close circuit from half-open.
   * @param maxErrorLog Max length of error log history.
   * @param sourceKeypair Signing keypair of the admin.
   */
  async setCircuitBreakerConfig(
    failureThreshold: number,
    successThreshold: number,
    maxErrorLog: number,
    sourceKeypair: Keypair
  ): Promise<void> {
    if (!Number.isInteger(failureThreshold) || failureThreshold <= 0) {
      throw new ValidationError('Failure threshold must be a positive integer', 'failureThreshold');
    }
    if (!Number.isInteger(successThreshold) || successThreshold <= 0) {
      throw new ValidationError('Success threshold must be a positive integer', 'successThreshold');
    }
    if (!Number.isInteger(maxErrorLog) || maxErrorLog < 0) {
      throw new ValidationError('Max error log must be a non-negative integer', 'maxErrorLog');
    }
    try {
      await this.invokeContract(
        'set_circuit_breaker_config',
        [failureThreshold, successThreshold, maxErrorLog],
        sourceKeypair
      );
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Get the current circuit breaker configuration.
   * @returns Config details including thresholds.
   */
  async getCircuitBreakerConfig(): Promise<CircuitBreakerConfig> {
    try {
      const result = await this.invokeContract('get_circuit_breaker_config', []);
      return result as CircuitBreakerConfig;
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Get current operational status of the circuit breaker.
   * @returns Current state and counter details.
   */
  async getCircuitBreakerStatus(): Promise<CircuitBreakerStatus> {
    try {
      const result = await this.invokeContract('get_circuit_breaker_status', []);
      return result as CircuitBreakerStatus;
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Reset the circuit breaker. Circuit breaker admin-only.
   * @param admin Stellar address of the circuit breaker admin executing the reset.
   * @param sourceKeypair Signing keypair of the circuit breaker admin.
   */
  async resetCircuit(
    admin: string,
    sourceKeypair: Keypair
  ): Promise<void> {
    this.validateAddress(admin, 'admin');
    try {
      await this.invokeContract('reset_circuit', [admin], sourceKeypair);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Get the error log recorded by the circuit breaker.
   * @returns List of failure entries.
   */
  async getCircuitErrorLog(): Promise<ErrorEntry[]> {
    try {
      const result = await this.invokeContract('get_circuit_error_log', []);
      return result as ErrorEntry[];
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Update the multisig configuration thresholds and signers. Admin-only.
   * @param thresholdAmount Minimum amount requiring multisig approval.
   * @param signers Array of authorized signer Stellar addresses.
   * @param requiredSignatures Minimum number of approvals required.
   * @param sourceKeypair Signing keypair of the admin.
   */
  async updateMultisigConfig(
    thresholdAmount: bigint,
    signers: string[],
    requiredSignatures: number,
    sourceKeypair: Keypair
  ): Promise<void> {
    if (thresholdAmount < 0n) {
      throw new ValidationError('Threshold amount cannot be negative', 'thresholdAmount');
    }
    if (!Number.isInteger(requiredSignatures) || requiredSignatures < 0) {
      throw new ValidationError('Required signatures must be a non-negative integer', 'requiredSignatures');
    }
    if (requiredSignatures > signers.length) {
      throw new ValidationError('Required signatures cannot exceed the number of signers', 'requiredSignatures');
    }
    for (let i = 0; i < signers.length; i++) {
      this.validateAddress(signers[i], `signers[${i}]`);
    }
    try {
      await this.invokeContract(
        'update_multisig_config',
        [thresholdAmount, signers, requiredSignatures],
        sourceKeypair
      );
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Get current multisig configuration.
   * @returns Current threshold amount and signers.
   */
  async getMultisigConfig(): Promise<MultisigConfig> {
    try {
      const result = await this.invokeContract('get_multisig_config', []);
      return result as MultisigConfig;
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Approve a release that exceeds the multisig threshold amount. Signer-only.
   * @param bountyId Application-level bounty identifier.
   * @param contributor Stellar address of the contributor receiving released funds.
   * @param approver Stellar address of the signer giving approval.
   * @param sourceKeypair Signing keypair of the approver.
   */
  async approveLargeRelease(
    bountyId: bigint,
    contributor: string,
    approver: string,
    sourceKeypair: Keypair
  ): Promise<void> {
    this.validateAddress(contributor, 'contributor');
    this.validateAddress(approver, 'approver');
    try {
      await this.invokeContract('approve_large_release', [bountyId, contributor, approver], sourceKeypair);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Configure minimum/maximum lock amount limits. Admin-only.
   * @param caller Stellar address of the admin caller.
   * @param minAmount Minimum lock amount allowed.
   * @param maxAmount Maximum lock amount allowed.
   * @param sourceKeypair Signing keypair of the admin.
   */
  async setAmountPolicy(
    caller: string,
    minAmount: bigint,
    maxAmount: bigint,
    sourceKeypair: Keypair
  ): Promise<void> {
    this.validateAddress(caller, 'caller');
    if (minAmount < 0n) {
      throw new ValidationError('Min amount cannot be negative', 'minAmount');
    }
    if (maxAmount < minAmount) {
      throw new ValidationError('Max amount cannot be less than min amount', 'maxAmount');
    }
    try {
      await this.invokeContract('set_amount_policy', [caller, minAmount, maxAmount], sourceKeypair);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Set the anti-abuse admin address. Admin-only.
   * @param admin Stellar address of the anti-abuse admin.
   * @param sourceKeypair Signing keypair of the admin.
   */
  async setAntiAbuseAdmin(
    admin: string,
    sourceKeypair: Keypair
  ): Promise<void> {
    this.validateAddress(admin, 'admin');
    try {
      await this.invokeContract('set_anti_abuse_admin', [admin], sourceKeypair);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Get the anti-abuse admin address.
   * @returns Anti-abuse admin address or null if not set.
   */
  async getAntiAbuseAdmin(): Promise<string | null> {
    try {
      const result = await this.invokeContract('get_anti_abuse_admin', []);
      return result as string | null;
    } catch (error) {
      throw this.handleError(error);
    }
  }

  /**
   * Set whitelist status for an address. Admin-only.
   * @param whitelistedAddress Stellar address to update.
   * @param whitelisted Boolean whitelist status.
   * @param sourceKeypair Signing keypair of the admin.
   */
  async setWhitelist(
    whitelistedAddress: string,
    whitelisted: boolean,
    sourceKeypair: Keypair
  ): Promise<void> {
    this.validateAddress(whitelistedAddress, 'whitelistedAddress');
    try {
      await this.invokeContract('set_whitelist', [whitelistedAddress, whitelisted], sourceKeypair);
    } catch (error) {
      throw this.handleError(error);
    }
  }

  private validateAddress(address: string, fieldName: string): void {
    if (!address || address.trim().length === 0) {
      throw new ValidationError(`${fieldName} cannot be empty`, fieldName);
    }
    // Basic Stellar address validation (starts with G and is 56 chars)
    if (!address.match(/^G[A-Z0-9]{55}$/)) {
      throw new ValidationError(`${fieldName} is not a valid Stellar address`, fieldName);
    }
  }

  private validateRefundMode(mode: RefundMode): void {
    if (mode !== 'Full' && mode !== 'Partial') {
      throw new ValidationError('Refund mode must be Full or Partial', 'mode');
    }
  }

  private validatePagination(offset: number, limit: number): void {
    if (!Number.isInteger(offset) || offset < 0) {
      throw new ValidationError('Offset must be a non-negative integer', 'offset');
    }
    if (!Number.isInteger(limit) || limit <= 0) {
      throw new ValidationError('Limit must be a positive integer', 'limit');
    }
  }

  private async invokeContract(
    method: string,
    args: any[],
    sourceKeypair?: Keypair
  ): Promise<any> {
    return invokeContract(
      method,
      args,
      this.invocationConfig,
      {
        sourceKeypair,
        readOnly: !sourceKeypair,
      }
    );
  }

  private handleError(error: any): Error {
    if (error instanceof ValidationError || 
        error instanceof NetworkError || 
        error instanceof ContractError) {
      return error;
    }
    
    if (error.code === 'ECONNREFUSED' || error.code === 'ETIMEDOUT' || error.code === 'ENOTFOUND') {
      return new NetworkError(
        `Failed to connect to RPC server: ${this.config.rpcUrl}`,
        undefined,
        error
      );
    }
    
    if (error.response?.status) {
      return new NetworkError(
        `RPC request failed with status ${error.response.status}`,
        error.response.status,
        error
      );
    }
    
    return parseContractError(error);
  }
}
