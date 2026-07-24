# SDK Invocation Guide

This document describes how the Grainlify SDK performs real contract invocations on Soroban networks using the Stellar SDK.

## Overview

The SDK implements a complete transaction lifecycle for Soroban contract invocation:

1. **Build** - Construct the contract invocation operation
2. **Simulate** - Validate the operation against current network state
3. **Sign** - Sign the transaction with the caller's keypair
4. **Submit** - Send the signed transaction to the network
5. **Confirm** - Poll for transaction completion with exponential backoff

## Key Components

### `invokeContract()`

Core function that handles the complete invocation lifecycle.

```typescript
import { invokeContract } from '@grainlify/contracts-sdk';

// For state-changing operations (requires keypair)
const result = await invokeContract(
  'lock_funds',
  [depositor, bountyId, amount, deadline],
  config,
  { sourceKeypair }
);

// For read-only calls (no keypair needed)
const balance = await invokeContract(
  'get_balance',
  [],
  config,
  { readOnly: true }
);
```

**Parameters:**
- `method` - Contract method name to invoke
- `args` - Array of method arguments
- `config` - Invocation configuration (see below)
- `options` - Optional invocation options

**Returns:** The contract method's return value (parsed from XDR)

**Throws:**
- `NetworkError` - Connection or RPC server issues
- `ContractError` - Contract execution failures
- `ValidationError` - Invalid input parameters

### `waitForConfirmation()`

Polls for transaction confirmation with exponential backoff and bounded retries.

```typescript
import { waitForConfirmation } from '@grainlify/contracts-sdk';

const confirmed = await waitForConfirmation(
  server,
  transactionHash,
  maxRetries = 30,
  baseDelayMs = 1000
);
```

**Behavior:**
- Starts with `baseDelayMs` delay, doubles on each retry (capped at 30s)
- Returns immediately on `SUCCESS` status
- Throws `ContractError` on `FAILED` status
- Throws `NetworkError` after max retries exhausted
- Safe to retry on transient network errors

## Configuration

### `InvocationConfig`

```typescript
interface InvocationConfig {
  server: SorobanRpc.Server;        // RPC server instance
  contract: Contract;                // Contract instance with contractId
  networkPassphrase: string;         // Network identifier (e.g., "Test SDF Network ; September 2015")
  rpcUrl: string;                   // RPC endpoint URL (for error messages)
}
```

### `InvokeOptions`

```typescript
interface InvokeOptions {
  sourceKeypair?: Keypair;          // Required for state-changing ops
  readOnly?: boolean;               // Simulation-only, no submission
  timeoutMs?: number;               // Custom timeout (not used yet)
  maxRetries?: number;              // Confirmation polling attempts (default: 30)
}
```

## Usage Patterns

### Read Operations

```typescript
import { BountyEscrowClient } from '@grainlify/contracts-sdk';
import { Keypair } from '@stellar/stellar-sdk';

const client = new BountyEscrowClient({
  contractId: 'CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAB5V5',
  rpcUrl: 'https://soroban-testnet.stellar.org',
  networkPassphrase: 'Test SDF Network ; September 2015'
});

// No keypair needed - simulates only
const escrow = await client.getEscrowInfo(bountyId);
const balance = await client.getBalance();
```

### Write Operations

```typescript
const keypair = Keypair.fromSecret(process.env.SECRET_KEY);

// Requires keypair - builds, signs, and submits
await client.lockFunds(
  depositor,
  bountyId,
  amount,
  deadline,
  keypair
);
```

## Security Considerations

### Key Management

- ✅ Keypairs are never logged or included in error messages
- ✅ Simulation always precedes submission (validation before commit)
- ✅ Simulation failures are surfaced to caller before submission
- ⚠️ Never commit secrets to source control or environment files

### Error Handling

```typescript
import { NetworkError, ContractError, ValidationError } from '@grainlify/contracts-sdk';

try {
  await client.lockFunds(depositor, bountyId, amount, deadline, keypair);
} catch (error) {
  if (error instanceof ValidationError) {
    console.error(`Invalid input: ${error.message}`);
  } else if (error instanceof ContractError) {
    console.error(`Contract error: ${error.message}`);
  } else if (error instanceof NetworkError) {
    console.error(`Network error: ${error.message}`);
  }
}
```

### Network Resilience

The invocation system handles transient failures:
- Connection timeouts
- RPC server errors
- Transaction pending status

```typescript
// Automatically retries with backoff
const result = await invokeContract(method, args, config, {
  sourceKeypair,
  maxRetries: 30  // Customize retry count
});
```

## Testing

### Mocking RPC Server

```typescript
import { invokeContract } from '@grainlify/contracts-sdk';

const mockServer = {
  simulateTransaction: jest.fn().mockResolvedValue({
    results: [{ xdr: 'mock-result-xdr' }]
  }),
  sendTransaction: jest.fn().mockResolvedValue({
    status: 'SUCCESS',
    hash: 'test-hash'
  }),
  getTransaction: jest.fn().mockResolvedValue({
    status: 'SUCCESS'
  }),
  getAccount: jest.fn().mockResolvedValue({
    sequence: '1'
  })
};

const config = {
  server: mockServer,
  contract: mockContract,
  networkPassphrase: 'Test SDF Network ; September 2015',
  rpcUrl: 'http://localhost:8000'
};

const result = await invokeContract('method', [], config, {
  sourceKeypair: Keypair.random()
});
```

## Troubleshooting

### "Failed to connect to RPC server"

**Cause:** Network unreachable or RPC endpoint down

**Solution:**
- Verify RPC URL is correct
- Check network connectivity
- Try alternate RPC endpoint

### "Transaction failed"

**Cause:** Contract validation failed or execution error

**Solution:**
- Check contract error details in exception
- Verify input parameters are valid
- Ensure account has sufficient balance

### "Transaction confirmation timeout"

**Cause:** Transaction not confirmed after 30 retries

**Solution:**
- Increase `maxRetries` option
- Check network status
- Verify transaction was submitted (check on-chain)

## Performance

Typical invocation timelines:
- Simulation: ~100-200ms
- Signing: <1ms
- Submission: ~50-100ms
- Confirmation: 5-30 seconds (depends on network load)

Total: 5-30+ seconds for state-changing operations

## Implementation Details

### Transaction Flow

```
invokeContract()
  ├─ contract.call(method, ...args)
  ├─ simulateTransaction()
  │  └─ server.simulateTransaction()
  ├─ (if no keypair)
  │  └─ return parseInvocationResult(simulation)
  ├─ getAccount()
  │  └─ server.getAccount(publicKey)
  ├─ new TransactionBuilder()
  ├─ SorobanRpc.assembleTransaction()
  ├─ transaction.sign(keypair)
  ├─ server.sendTransaction(signedTx)
  ├─ waitForConfirmation()
  │  └─ (exponential backoff polling)
  └─ parseInvocationResult(confirmed)
```

### Error Handling Hierarchy

1. **ValidationError** - Before any RPC call
2. **ContractError** - Simulation failures or contract errors
3. **NetworkError** - Connection/RPC issues
4. **Unknown errors** - Wrapped in ContractError

## References

- [Stellar SDK Documentation](https://developers.stellar.org/docs/learn/building-with-js/stellar-sdk)
- [Soroban Documentation](https://developers.stellar.org/docs/soroban)
- [SDK Error Mapping](./ERROR_MAPPING.md)
- [BountyEscrowClient API](./api/classes/BountyEscrowClient.md)
- [ProgramEscrowClient API](./api/classes/ProgramEscrowClient.md)
