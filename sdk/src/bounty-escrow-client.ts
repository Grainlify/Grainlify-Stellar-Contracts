import { Contract, SorobanRpc, Keypair } from '@stellar/stellar-sdk';
import { NetworkError, ValidationError, parseContractError, ContractError } from './errors';

export interface BountyEscrowConfig {
  contractId: string;
  rpcUrl: string;
  networkPassphrase: string;
}

export interface LockFundsItem {
  bounty_id: bigint;
  depositor: string;
  amount: bigint;
  deadline: number;
}

export interface ReleaseFundsItem {
  bounty_id: bigint;
  contributor: string;
}

export type EscrowStatus = 'Locked' | 'Released' | 'Refunded' | 'PartiallyRefunded';
export type RefundMode = 'Full' | 'Partial';

export interface RefundRecord {
  amount: bigint;
  recipient: string;
  timestamp: number;
  mode: RefundMode;
}

export interface ClaimRecord {
  bounty_id: bigint;
  recipient: string;
  amount: bigint;
  expires_at: number;
  claimed: boolean;
}

export interface Escrow {
  depositor: string;
  amount: bigint;
  remaining_amount: bigint;
  status: EscrowStatus;
  deadline: number;
  refund_history: RefundRecord[];
}

export interface EscrowWithId {
  bounty_id: bigint;
  escrow: Escrow;
}

export interface EscrowQueryFilter {
  has_status_filter: boolean;
  status: EscrowStatus;
  has_depositor_filter: boolean;
  depositor: string;
  min_amount: bigint;
  max_amount: bigint;
  min_deadline: number;
  max_deadline: number;
}

export interface AggregateStats {
  total_locked: bigint;
  total_released: bigint;
  total_refunded: bigint;
  count_locked: number;
  count_released: number;
  count_refunded: number;
}

export interface RefundApproval {
  bounty_id: bigint;
  amount: bigint;
  recipient: string;
  mode: RefundMode;
  approved_by: string;
  approved_at: number;
}

export interface RefundEligibility {
  can_refund: boolean;
  deadline_passed: boolean;
  remaining_amount: bigint;
  approval?: RefundApproval;
}

export interface FeeConfig {
  lock_fee_rate: bigint;
  release_fee_rate: bigint;
  fee_recipient: string;
  fee_enabled: boolean;
}

export interface PauseFlags {
  lock_paused: boolean;
  release_paused: boolean;
  refund_paused: boolean;
}

/**
 * Client for interacting with the BountyEscrow Soroban contract
 */
export class BountyEscrowClient {
  private contract: Contract;
  private server: SorobanRpc.Server;
  private config: BountyEscrowConfig;

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
    try {
      // Mock implementation to mirror ProgramEscrowClient
      throw new Error('Contract invocation not implemented - this is a mock for testing');
    } catch (error: any) {
      if (error.code === 'ECONNREFUSED' || error.code === 'ETIMEDOUT') {
        throw new NetworkError(
          `Failed to connect to RPC server: ${this.config.rpcUrl}`,
          undefined,
          error
        );
      }
      
      if (error.response?.status) {
        throw new NetworkError(
          `RPC request failed with status ${error.response.status}`,
          error.response.status,
          error
        );
      }
      
      throw error;
    }
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
