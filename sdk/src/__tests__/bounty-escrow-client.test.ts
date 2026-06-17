import { Keypair } from '@stellar/stellar-sdk';
import {
  BountyEscrowClient,
  EscrowQueryFilter,
  LockFundsItem,
  RefundMode,
  ReleaseFundsItem,
} from '../bounty-escrow-client';
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

  function mockInvoke(result: unknown = undefined) {
    return jest.spyOn(client as any, 'invokeContract').mockResolvedValue(result);
  }

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

    describe('claim and query helpers', () => {
      it('throws on invalid claim window', async () => {
        await expect(
          client.setClaimWindow(-1, sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });

      it('throws on invalid refund mode', async () => {
        await expect(
          client.approveRefund(1n, 10n, validGAddress1, 'Invalid' as RefundMode, sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });

      it('throws on invalid pagination', async () => {
        await expect(
          client.queryEscrowsByStatus('Locked', -1, 10)
        ).rejects.toThrow(ValidationError);

        await expect(
          client.getEscrowIdsByStatus('Locked', 0, 0)
        ).rejects.toThrow(ValidationError);
      });

      it('throws on invalid composite query depositor when enabled', async () => {
        const filter: EscrowQueryFilter = {
          has_status_filter: false,
          status: 'Locked',
          has_depositor_filter: true,
          depositor: 'invalid',
          min_amount: 0n,
          max_amount: 1_000n,
          min_deadline: 0,
          max_deadline: 100,
        };

        await expect(
          client.queryEscrows(filter)
        ).rejects.toThrow(ValidationError);
      });
    });
  });

  describe('method routing', () => {
    it('routes approveRefund with refund mode and signing keypair', async () => {
      const invoke = mockInvoke();

      await client.approveRefund(7n, 500n, validGAddress1, 'Partial', sourceKeypair);

      expect(invoke).toHaveBeenCalledWith(
        'approve_refund',
        [7n, 500n, validGAddress1, 'Partial'],
        sourceKeypair
      );
    });

    it('routes claim-window and pending-claim mutators with signing keypair', async () => {
      const invoke = mockInvoke();

      await client.setClaimWindow(3600, sourceKeypair);
      await client.cancelPendingClaim(9n, sourceKeypair);

      expect(invoke).toHaveBeenNthCalledWith(1, 'set_claim_window', [3600], sourceKeypair);
      expect(invoke).toHaveBeenNthCalledWith(2, 'cancel_pending_claim', [9n], sourceKeypair);
    });

    it('routes claim view helpers without a signing keypair', async () => {
      const claim = {
        bounty_id: 9n,
        recipient: validGAddress1,
        amount: 500n,
        expires_at: 1234,
        claimed: false,
      };
      const invoke = mockInvoke(claim);

      await expect(client.getPendingClaim(9n)).resolves.toEqual(claim);

      expect(invoke).toHaveBeenCalledWith('get_pending_claim', [9n]);
    });

    it('routes aggregate and refund audit views', async () => {
      const invoke = mockInvoke();
      invoke
        .mockResolvedValueOnce([
          { amount: 10n, recipient: validGAddress1, timestamp: 100, mode: 'Partial' },
        ])
        .mockResolvedValueOnce([true, false, 90n, undefined])
        .mockResolvedValueOnce({ total_locked: 100n, total_released: 0n, total_refunded: 10n, count_locked: 1, count_released: 0, count_refunded: 0 })
        .mockResolvedValueOnce(3);

      await client.getRefundHistory(1n);
      await expect(client.getRefundEligibility(1n)).resolves.toEqual({
        can_refund: true,
        deadline_passed: false,
        remaining_amount: 90n,
        approval: undefined,
      });
      await client.getAggregateStats();
      await expect(client.getEscrowCount()).resolves.toBe(3);

      expect(invoke).toHaveBeenNthCalledWith(1, 'get_refund_history', [1n]);
      expect(invoke).toHaveBeenNthCalledWith(2, 'get_refund_eligibility', [1n]);
      expect(invoke).toHaveBeenNthCalledWith(3, 'get_aggregate_stats', []);
      expect(invoke).toHaveBeenNthCalledWith(4, 'get_escrow_count', []);
    });

    it('routes bounty query helpers to the matching contract methods', async () => {
      const invoke = mockInvoke([]);
      const filter: EscrowQueryFilter = {
        has_status_filter: true,
        status: 'Locked',
        has_depositor_filter: true,
        depositor: validGAddress1,
        min_amount: 0n,
        max_amount: 1_000n,
        min_deadline: 0,
        max_deadline: 10_000,
      };

      await client.queryEscrowsByStatus('Locked', 0, 10);
      await client.queryEscrowsByAmount(1n, 1_000n, 2, 20);
      await client.queryEscrowsByDeadline(100, 1_000, 3, 30);
      await client.queryEscrowsByDepositor(validGAddress1, 4, 40);
      await client.queryEscrows(filter, 5, 50);
      await client.getEscrowIdsByStatus('Refunded', 6, 60);
      await client.queryExpiringBounties(2_000, 7, 70);

      expect(invoke).toHaveBeenNthCalledWith(1, 'query_escrows_by_status', ['Locked', 0, 10]);
      expect(invoke).toHaveBeenNthCalledWith(2, 'query_escrows_by_amount', [1n, 1_000n, 2, 20]);
      expect(invoke).toHaveBeenNthCalledWith(3, 'query_escrows_by_deadline', [100, 1_000, 3, 30]);
      expect(invoke).toHaveBeenNthCalledWith(4, 'query_escrows_by_depositor', [validGAddress1, 4, 40]);
      expect(invoke).toHaveBeenNthCalledWith(5, 'query_escrows', [filter, 5, 50]);
      expect(invoke).toHaveBeenNthCalledWith(6, 'get_escrow_ids_by_status', ['Refunded', 6, 60]);
      expect(invoke).toHaveBeenNthCalledWith(7, 'query_expiring_bounties', [2_000, 7, 70]);
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
