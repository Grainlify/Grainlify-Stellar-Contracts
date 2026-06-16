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

export interface RefundRecord {
  amount: bigint;
  recipient: string;
  timestamp: number;
  mode: string; // "Full" | "Partial"
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
  status: string; // "Locked" | "Released" | "Refunded" | "PartiallyRefunded"
  deadline: number;
  refund_history: RefundRecord[];
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
    sourceKeypair: Keypair
  ): Promise<void> {
    this.validateAddress(recipient, 'recipient');
    if (amount <= 0n) {
      throw new ValidationError('Amount must be greater than zero', 'amount');
    }

    try {
      await this.invokeContract('approve_refund', [bountyId, amount, recipient], sourceKeypair);
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
