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
 * Configuration for contract invocation
 */
export interface InvocationConfig {
  /** RPC server instance */
  server: SorobanRpc.Server;
  /** Contract instance */
  contract: Contract;
  /** Network passphrase */
  networkPassphrase: string;
  /** RPC URL (for error messages) */
  rpcUrl: string;
}

/**
 * Options for contract method invocation
 */
export interface InvokeOptions {
  /** Source keypair for signing (required for state-changing operations) */
  sourceKeypair?: Keypair;
  /** Whether this is a read-only call (uses simulation only) */
  readOnly?: boolean;
  /** Custom timeout in milliseconds */
  timeoutMs?: number;
  /** Maximum number of confirmation polling attempts */
  maxRetries?: number;
}

/**
 * Wait for transaction confirmation with exponential backoff
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
 * Invoke a contract method with proper build/simulate/sign/submit flow
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
