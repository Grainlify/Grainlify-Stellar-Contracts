import { Keypair } from '@stellar/stellar-sdk';
import { BountyEscrowClient, LockFundsItem, ReleaseFundsItem } from '../bounty-escrow-client';
import { NetworkError, ValidationError, ContractError, ContractErrorCode } from '../errors';

describe('BountyEscrowClient', () => {
  const mockConfig = {
    contractId: 'CBTG2M4XXWNDH7GCHXZT6E2I3J644MFRZQK6CUKL4WJY6WQZXY3P2M6L', // Must be 56 chars
    rpcUrl: 'http://localhost:8000/rpc',
    networkPassphrase: 'Test SDF Network ; September 2015',
  };

  const validAddress1 = 'GAXN...'; // Just need an address that passes basic validation. Wait, the client uses regex /^G[A-Z0-9]{55}$/
  const validGAddress1 = 'GAXN6265B5U2ZIK2QFWIYYXGZ5B47L7Z236L72G66Z3S7MHT7XZQ5WZG';
  const validGAddress2 = 'GBZN6265B5U2ZIK2QFWIYYXGZ5B47L7Z236L72G66Z3S7MHT7XZQ5WZG';
  
  let client: BountyEscrowClient;
  let sourceKeypair: Keypair;

  beforeEach(() => {
    client = new BountyEscrowClient(mockConfig);
    sourceKeypair = Keypair.random();
  });

  describe('initialization', () => {
    it('creates client with valid config', () => {
      expect(client).toBeDefined();
    });
  });

  describe('validation', () => {
    describe('addresses', () => {
      it('throws on empty address in init', async () => {
        await expect(
          client.init('', validGAddress2, sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });

      it('throws on invalid address in init', async () => {
        await expect(
          client.init('invalid', validGAddress2, sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });
      
      it('throws on invalid depositor in lockFunds', async () => {
        await expect(
          client.lockFunds('invalid', 1n, 100n, 1000, sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });
    });

    describe('amounts', () => {
      it('throws on zero amount in lockFunds', async () => {
        await expect(
          client.lockFunds(validGAddress1, 1n, 0n, 1000, sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });

      it('throws on negative amount in lockFunds', async () => {
        await expect(
          client.lockFunds(validGAddress1, 1n, -100n, 1000, sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });
    });
    
    describe('batch operations', () => {
      it('throws on empty items array in batchLockFunds', async () => {
        await expect(
          client.batchLockFunds([], sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });
      
      it('throws on invalid amount in batchLockFunds', async () => {
        const items: LockFundsItem[] = [
          { bounty_id: 1n, depositor: validGAddress1, amount: 10n, deadline: 100 },
          { bounty_id: 2n, depositor: validGAddress1, amount: -10n, deadline: 100 },
        ];
        await expect(
          client.batchLockFunds(items, sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });
    });
  });

  describe('error handling (mocked invokes)', () => {
    // Note: Since our client implementation mocks `invokeContract` and throws 
    // "Contract invocation not implemented - this is a mock for testing",
    // it will be caught and parsed by `handleError`. 
    // This allows us to ensure the mock is hit.

    it('wraps unknown errors as generic ContractError', async () => {
      // Because `parseContractError` falls back to generic ContractError
      await expect(client.getBalance()).rejects.toThrow(ContractError);
    });

    // To properly test error parsing of bounty specific errors, we would need 
    // to spy on invokeContract and make it throw specific error strings or objects.
    // We can simulate this by directly testing the errors.ts parser, but since
    // it's already tested elsewhere, we just verify the client tries to use it.
  });
});
