import {
  Account,
  Keypair,
  TransactionBuilder,
  SorobanRpc,
  Contract,
  xdr,
  BASE_FEE,
} from '@stellar/stellar-sdk';
import { NetworkError, ContractError } from './errors';

/**
 * Configuration for contract invocation with Soroban RPC
 * 
 * @example
 * ```typescript
 * const config: InvocationConfig = {
 *   server: new SorobanRpc.Server('https://soroban-testnet.stellar.org'),
 *   contract: new Contract('CAAAAAAA...'),
 *   networkPassphrase: 'Test SDF Network ; September 2015',
 *   rpcUrl: 'https://soroban-testnet.stellar.org'
 * };
 * ```
 */
export interface InvocationConfig {
  /** Soroban RPC server instance for simulating and submitting transactions */
  server: SorobanRpc.Server;
  /** Contract instance containing the contract ID */
  contract: Contract;
  /** Network passphrase (e.g., "Test SDF Network ; September 2015" for testnet) */
  networkPassphrase: string;
  /** RPC endpoint URL for error messages and diagnostics */
  rpcUrl: string;
}

/**
 * Options for contract method invocation behavior
 * 
 * @example
 * ```typescript
 * // State-changing operation
 * const opts: InvokeOptions = {
 *   sourceKeypair: keypair,
 *   maxRetries: 30
 * };
 * 
 * // Read-only operation
 * const readOnlyOpts: InvokeOptions = {
 *   readOnly: true
 * };
 * ```
 */
export interface InvokeOptions {
  /** Keypair for signing transactions (required for state-changing operations) */
  sourceKeypair?: Keypair;
  /** If true, only simulates without submitting the transaction */
  readOnly?: boolean;
  /** Custom timeout in milliseconds (reserved for future use) */
  timeoutMs?: number;
  /** Maximum confirmation polling attempts (default: 30) */
  maxRetries?: number;
}

/**
 * Wait for a submitted transaction to be confirmed on the Soroban network.
 * 
 * Polls transaction status with exponential backoff, starting at `baseDelayMs`
 * and doubling on each retry (capped at 30 seconds). Returns immediately on
 * SUCCESS, throws on FAILED, and retries on PENDING.
 * 
 * **Security:** Signing keys are never logged or exposed in error messages.
 * 
 * @param server - Soroban RPC server instance
 * @param txHash - Transaction hash returned from server.sendTransaction()
 * @param maxRetries - Maximum polling attempts (default: 30)
 * @param baseDelayMs - Initial delay in milliseconds (default: 1000)
 * @returns Confirmed transaction response
 * @throws {ContractError} If transaction status is FAILED
 * @throws {NetworkError} If max retries exceeded or connection fails
 * 
 * @example
 * ```typescript
 * const submitResult = await server.sendTransaction(signedTx);
 * if (submitResult.status === 'SUCCESS') {
 *   const confirmed = await waitForConfirmation(
 *     server,
 *     submitResult.hash,
 *     30,    // maxRetries
 *     1000   // baseDelayMs
 *   );
 *   console.log('Transaction confirmed:', confirmed);
 * }
 * ```
 */
export async function waitForConfirmation(
  server: SorobanRpc.Server,
  txHash: string,
  maxRetries: number = 30,
  baseDelayMs: number = 1000
): Promise<any> {
  let lastError: Error | null = null;

  for (let attempt = 0; attempt < maxRetries; attempt++) {
    try {
      const response = await server.getTransaction(txHash);

      if (response.status === 'SUCCESS') {
        return response;
      }

      if (response.status === 'FAILED') {
        throw new ContractError(
          `Transaction failed`,
          'TRANSACTION_FAILED',
          undefined
        );
      }

      // PENDING status, wait and retry
      const delayMs = Math.min(
        baseDelayMs * Math.pow(2, attempt),
        30000 // max 30 seconds
      );
      await new Promise(resolve => setTimeout(resolve, delayMs));
    } catch (error: any) {
      if (error instanceof ContractError) {
        throw error;
      }
      lastError = error;

      // Continue retrying on transient errors
      if (attempt < maxRetries - 1) {
        const delayMs = Math.min(
          baseDelayMs * Math.pow(2, attempt),
          30000
        );
        await new Promise(resolve => setTimeout(resolve, delayMs));
      }
    }
  }

  if (lastError) {
    throw new NetworkError(
      `Failed to confirm transaction after ${maxRetries} attempts`,
      undefined,
      lastError
    );
  }

  throw new NetworkError(
    `Transaction confirmation timeout after ${maxRetries} attempts`,
    undefined
  );
}

/**
 * Invoke a contract method with full transaction lifecycle management.
 * 
 * Implements the complete Soroban invocation flow:
 * 1. Build - Create contract invocation operation
 * 2. Simulate - Validate against network state (always done first)
 * 3. Sign - Sign with keypair (if provided)
 * 4. Submit - Send to network (if keypair provided)
 * 5. Confirm - Poll for completion (if submitted)
 * 
 * For read-only calls (no keypair), returns simulation result immediately.
 * For state-changing calls, builds, signs, submits, and confirms the transaction.
 * 
 * **Security:** Simulation always precedes submission. Simulation errors are
 * surfaced to caller. Keypairs are never logged. Always validate input with
 * your own ValidationError checks before calling this function.
 * 
 * @param method - Contract method name to invoke
 * @param args - Array of method arguments (will be converted to Soroban types)
 * @param config - Invocation configuration (server, contract, network)
 * @param options - Optional behavior modifiers
 * @returns The contract method's return value (parsed from XDR)
 * @throws {ValidationError} If parameters are invalid
 * @throws {NetworkError} If connection fails or RPC error occurs
 * @throws {ContractError} If simulation or contract execution fails
 * 
 * @example
 * ```typescript
 * import { invokeContract } from '@grainlify/contracts-sdk';
 * import { Keypair } from '@stellar/stellar-sdk';
 * 
 * // Read-only call
 * const balance = await invokeContract(
 *   'get_balance',
 *   [],
 *   config,
 *   { readOnly: true }
 * );
 * 
 * // State-changing call
 * const keypair = Keypair.fromSecret(process.env.SECRET_KEY);
 * await invokeContract(
 *   'lock_funds',
 *   [depositor, bountyId, amount, deadline],
 *   config,
 *   { sourceKeypair: keypair, maxRetries: 30 }
 * );
 * ```
 */
export async function invokeContract(
  method: string,
  args: any[],
  config: InvocationConfig,
  options: InvokeOptions = {}
): Promise<any> {
  const { sourceKeypair, readOnly = false, maxRetries = 30 } = options;

  try {
    // Build the invocation
    const invocation = config.contract.call(method, ...args);

    // If no keypair provided, do simulation only (for read operations)
    if (!sourceKeypair) {
      const simulationResult = await simulateTransaction(
        invocation,
        config,
        null
      );
      return parseInvocationResult(simulationResult);
    }

    // Get account information for the source keypair
    const account = await getAccount(config.server, sourceKeypair.publicKey());

    // Build the transaction
    const transaction = new TransactionBuilder(account, {
      fee: BASE_FEE,
      networkPassphrase: config.networkPassphrase,
    })
      .addOperation(invocation)
      .setTimeout(30)
      .build();

    // Simulate the transaction
    let simulationResult = await simulateTransaction(
      transaction,
      config,
      sourceKeypair
    );

    // Assemble the transaction
    const assembled = SorobanRpc.assembleTransaction(
      transaction,
      simulationResult
    ).build();

    // Sign the transaction
    assembled.sign(sourceKeypair);

    // For read-only operations, return simulation result
    if (readOnly) {
      return parseInvocationResult(simulationResult);
    }

    // Submit the transaction
    const submitResult = await config.server.sendTransaction(assembled);

    if (submitResult.status === 'ERROR') {
      throw new ContractError(
        `Failed to submit transaction`,
        'SUBMIT_FAILED'
      );
    }

    // Wait for confirmation
    const confirmed = await waitForConfirmation(
      config.server,
      submitResult.hash,
      maxRetries
    );

    // Parse and return the result
    return parseInvocationResult(confirmed);
  } catch (error: any) {
    // Re-throw known errors
    if (error instanceof ContractError || error instanceof NetworkError) {
      throw error;
    }

    // Handle network errors
    if (
      error.code === 'ECONNREFUSED' ||
      error.code === 'ETIMEDOUT' ||
      error.code === 'ENOTFOUND'
    ) {
      throw new NetworkError(
        `Failed to connect to RPC server: ${config.rpcUrl}`,
        undefined,
        error
      );
    }

    // Handle RPC response errors
    if (error.response?.status) {
      throw new NetworkError(
        `RPC request failed with status ${error.response.status}`,
        error.response.status,
        error
      );
    }

    // Wrap unknown errors
    throw new ContractError(
      `Contract invocation failed: ${error.message}`,
      'INVOCATION_FAILED',
      undefined
    );
  }
}

/**
 * Simulate a transaction without submitting it
 */
async function simulateTransaction(
  transaction: any,
  config: InvocationConfig,
  sourceKeypair: Keypair | null
): Promise<any> {
  try {
    const response = await config.server.simulateTransaction(transaction);

    // Check if it's an error response
    if ((response as any).error || (response as any).errorMessage) {
      throw new ContractError(
        `Simulation failed: ${(response as any).error || (response as any).errorMessage}`,
        'SIMULATION_FAILED'
      );
    }

    return response;
  } catch (error: any) {
    if (error instanceof ContractError) {
      throw error;
    }

    // Let the invokeContract handler deal with network errors
    throw error;
  }
}

/**
 * Get account information from the server
 */
async function getAccount(
  server: SorobanRpc.Server,
  publicKey: string
): Promise<Account> {
  try {
    const response = await server.getAccount(publicKey);
    // Handle both possible return types for sequence
    const sequence = typeof (response as any).sequence === 'string' 
      ? (response as any).sequence 
      : ((response as any).sequence?.toString() || '0');
    return new Account(publicKey, sequence);
  } catch (error: any) {
    if (
      error.code === 'ECONNREFUSED' ||
      error.code === 'ETIMEDOUT' ||
      error.code === 'ENOTFOUND'
    ) {
      throw new NetworkError(
        'Failed to fetch account information',
        undefined,
        error
      );
    }

    throw new NetworkError(
      `Failed to get account: ${error.message}`,
      undefined,
      error
    );
  }
}

/**
 * Parse the result from a simulation or confirmed transaction
 */
function parseInvocationResult(response: any): any {
  try {
    if (!response.result && !response.results) {
      return null;
    }

    // For GetTransactionResponse (confirmed transaction)
    if (response.resultMetaXdr) {
      // Parse from confirmed transaction meta
      return response;
    }

    // For SimulateTransactionSuccessResponse
    if (response.results && response.results.length > 0) {
      const firstResult = response.results[0];
      if (firstResult.xdr) {
        return xdr.ScVal.fromXDR(firstResult.xdr, 'base64');
      }
    }

    return response.result || null;
  } catch (error: any) {
    throw new ContractError(
      `Failed to parse invocation result: ${error.message}`,
      'PARSE_FAILED'
    );
  }
}
