import { Keypair } from '@stellar/stellar-sdk';
import {
  BountyEscrowClient,
  EscrowQueryFilter,
  LockFundsItem,
  RefundMode,
  ReleaseFundsItem,
} from '../index';
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

  describe('Admin methods validation', () => {
    describe('updateFeeConfig', () => {
      it('throws on negative lockFeeRate', async () => {
        await expect(
          client.updateFeeConfig(-5n, null, null, null, sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });

      it('throws on negative releaseFeeRate', async () => {
        await expect(
          client.updateFeeConfig(null, -10n, null, null, sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });

      it('throws on invalid feeRecipient address', async () => {
        await expect(
          client.updateFeeConfig(null, null, 'invalid', null, sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });
    });

    describe('setPaused', () => {
      it('successfully routes setPaused', async () => {
        const invoke = mockInvoke();
        await client.setPaused(true, null, false, sourceKeypair);
        expect(invoke).toHaveBeenCalledWith('set_paused', [true, null, false], sourceKeypair);
      });
    });

    describe('setGovernanceContract', () => {
      it('throws on invalid governance contract address', async () => {
        await expect(
          client.setGovernanceContract('invalid', sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });
    });

    describe('setMinGovernanceVersion', () => {
      it('throws on negative minVersion', async () => {
        await expect(
          client.setMinGovernanceVersion(-1, sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });

      it('throws on non-integer minVersion', async () => {
        await expect(
          client.setMinGovernanceVersion(1.5, sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });
    });

    describe('setCircuitBreakerAdmin', () => {
      it('throws on invalid admin address', async () => {
        await expect(
          client.setCircuitBreakerAdmin('invalid', sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });
    });

    describe('setCircuitBreakerConfig', () => {
      it('throws on negative/zero failureThreshold', async () => {
        await expect(
          client.setCircuitBreakerConfig(0, 5, 10, sourceKeypair)
        ).rejects.toThrow(ValidationError);
        await expect(
          client.setCircuitBreakerConfig(-1, 5, 10, sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });

      it('throws on negative/zero successThreshold', async () => {
        await expect(
          client.setCircuitBreakerConfig(5, 0, 10, sourceKeypair)
        ).rejects.toThrow(ValidationError);
        await expect(
          client.setCircuitBreakerConfig(5, -1, 10, sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });

      it('throws on negative maxErrorLog', async () => {
        await expect(
          client.setCircuitBreakerConfig(5, 5, -1, sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });
    });

    describe('resetCircuit', () => {
      it('throws on invalid admin address', async () => {
        await expect(
          client.resetCircuit('invalid', sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });
    });

    describe('updateMultisigConfig', () => {
      it('throws on negative thresholdAmount', async () => {
        await expect(
          client.updateMultisigConfig(-1n, [validGAddress1], 1, sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });

      it('throws on empty signers array', async () => {
        await expect(
          client.updateMultisigConfig(100n, [], 1, sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });

      it('throws on invalid signer address', async () => {
        await expect(
          client.updateMultisigConfig(100n, [validGAddress1, 'invalid'], 1, sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });

      it('throws on negative/zero requiredSignatures', async () => {
        await expect(
          client.updateMultisigConfig(100n, [validGAddress1], 0, sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });

      it('throws when requiredSignatures exceeds signers length', async () => {
        await expect(
          client.updateMultisigConfig(100n, [validGAddress1], 2, sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });
    });

    describe('approveLargeRelease', () => {
      it('throws on invalid contributor address', async () => {
        await expect(
          client.approveLargeRelease(1n, 'invalid', validGAddress2, sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });

      it('throws on invalid approver address', async () => {
        await expect(
          client.approveLargeRelease(1n, validGAddress1, 'invalid', sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });
    });

    describe('setAmountPolicy', () => {
      it('throws on invalid caller address', async () => {
        await expect(
          client.setAmountPolicy('invalid', 10n, 100n, sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });

      it('throws on negative minAmount', async () => {
        await expect(
          client.setAmountPolicy(validGAddress1, -10n, 100n, sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });

      it('throws when maxAmount is less than minAmount', async () => {
        await expect(
          client.setAmountPolicy(validGAddress1, 100n, 50n, sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });
    });

    describe('setAntiAbuseAdmin', () => {
      it('throws on invalid admin address', async () => {
        await expect(
          client.setAntiAbuseAdmin('invalid', sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });
    });

    describe('setWhitelist', () => {
      it('throws on invalid whitelistedAddress', async () => {
        await expect(
          client.setWhitelist('invalid', true, sourceKeypair)
        ).rejects.toThrow(ValidationError);
      });
    });
  });

  describe('Admin method routing', () => {
    it('routes updateFeeConfig correctly', async () => {
      const invoke = mockInvoke();
      await client.updateFeeConfig(10n, 20n, validGAddress1, true, sourceKeypair);
      expect(invoke).toHaveBeenCalledWith(
        'update_fee_config',
        [10n, 20n, validGAddress1, true],
        sourceKeypair
      );
    });

    it('routes setGovernanceContract correctly', async () => {
      const invoke = mockInvoke();
      await client.setGovernanceContract(validGAddress2, sourceKeypair);
      expect(invoke).toHaveBeenCalledWith('set_governance_contract', [validGAddress2], sourceKeypair);
    });

    it('routes setMinGovernanceVersion correctly', async () => {
      const invoke = mockInvoke();
      await client.setMinGovernanceVersion(3, sourceKeypair);
      expect(invoke).toHaveBeenCalledWith('set_min_governance_version', [3], sourceKeypair);
    });

    it('routes setCircuitBreakerAdmin correctly', async () => {
      const invoke = mockInvoke();
      await client.setCircuitBreakerAdmin(validGAddress2, sourceKeypair);
      expect(invoke).toHaveBeenCalledWith('set_circuit_breaker_admin', [validGAddress2], sourceKeypair);
    });

    it('routes setCircuitBreakerConfig correctly', async () => {
      const invoke = mockInvoke();
      await client.setCircuitBreakerConfig(5, 3, 100, sourceKeypair);
      expect(invoke).toHaveBeenCalledWith('set_circuit_breaker_config', [5, 3, 100], sourceKeypair);
    });

    it('routes resetCircuit correctly', async () => {
      const invoke = mockInvoke();
      await client.resetCircuit(validGAddress2, sourceKeypair);
      expect(invoke).toHaveBeenCalledWith('reset_circuit', [validGAddress2], sourceKeypair);
    });

    it('routes updateMultisigConfig correctly', async () => {
      const invoke = mockInvoke();
      await client.updateMultisigConfig(1000n, [validGAddress1, validGAddress2], 2, sourceKeypair);
      expect(invoke).toHaveBeenCalledWith(
        'update_multisig_config',
        [1000n, [validGAddress1, validGAddress2], 2],
        sourceKeypair
      );
    });

    it('routes approveLargeRelease correctly', async () => {
      const invoke = mockInvoke();
      await client.approveLargeRelease(5n, validGAddress1, validGAddress2, sourceKeypair);
      expect(invoke).toHaveBeenCalledWith(
        'approve_large_release',
        [5n, validGAddress1, validGAddress2],
        sourceKeypair
      );
    });

    it('routes setAmountPolicy correctly', async () => {
      const invoke = mockInvoke();
      await client.setAmountPolicy(validGAddress1, 50n, 500n, sourceKeypair);
      expect(invoke).toHaveBeenCalledWith(
        'set_amount_policy',
        [validGAddress1, 50n, 500n],
        sourceKeypair
      );
    });

    it('routes setAntiAbuseAdmin correctly', async () => {
      const invoke = mockInvoke();
      await client.setAntiAbuseAdmin(validGAddress2, sourceKeypair);
      expect(invoke).toHaveBeenCalledWith('set_anti_abuse_admin', [validGAddress2], sourceKeypair);
    });

    it('routes setWhitelist correctly', async () => {
      const invoke = mockInvoke();
      await client.setWhitelist(validGAddress2, true, sourceKeypair);
      expect(invoke).toHaveBeenCalledWith('set_whitelist', [validGAddress2, true], sourceKeypair);
    });
  });

  describe('Admin views and getters routing', () => {
    it('routes getMultisigConfig correctly', async () => {
      const config = { threshold_amount: 100n, signers: [validGAddress1], required_signatures: 1 };
      const invoke = mockInvoke(config);
      await expect(client.getMultisigConfig()).resolves.toEqual(config);
      expect(invoke).toHaveBeenCalledWith('get_multisig_config', []);
    });

    it('routes getCircuitBreakerAdmin correctly', async () => {
      const invoke = mockInvoke(validGAddress2);
      await expect(client.getCircuitBreakerAdmin()).resolves.toBe(validGAddress2);
      expect(invoke).toHaveBeenCalledWith('get_circuit_breaker_admin', []);
    });

    it('routes getCircuitBreakerConfig correctly', async () => {
      const config = { failure_threshold: 5, success_threshold: 3, max_error_log: 10 };
      const invoke = mockInvoke(config);
      await expect(client.getCircuitBreakerConfig()).resolves.toEqual(config);
      expect(invoke).toHaveBeenCalledWith('get_circuit_breaker_config', []);
    });

    it('routes getCircuitBreakerStatus correctly', async () => {
      const status = {
        state: 'Closed',
        failure_count: 0,
        success_count: 0,
        last_failure_timestamp: 0n,
        opened_at: 0n,
        failure_threshold: 5,
        success_threshold: 3
      };
      const invoke = mockInvoke(status);
      await expect(client.getCircuitBreakerStatus()).resolves.toEqual(status);
      expect(invoke).toHaveBeenCalledWith('get_circuit_breaker_status', []);
    });

    it('routes getAntiAbuseAdmin correctly', async () => {
      const invoke = mockInvoke(validGAddress2);
      await expect(client.getAntiAbuseAdmin()).resolves.toBe(validGAddress2);
      expect(invoke).toHaveBeenCalledWith('get_anti_abuse_admin', []);
    });

    it('routes getGovernanceContract correctly', async () => {
      const invoke = mockInvoke(validGAddress2);
      await expect(client.getGovernanceContract()).resolves.toBe(validGAddress2);
      expect(invoke).toHaveBeenCalledWith('get_governance_contract', []);
    });

    it('routes getMinGovernanceVersion correctly', async () => {
      const invoke = mockInvoke(5n);
      await expect(client.getMinGovernanceVersion()).resolves.toBe(5);
      expect(invoke).toHaveBeenCalledWith('get_min_governance_version', []);
    });

    it('routes getAdminAuditView correctly', async () => {
      const snapshot = {
        version: 1,
        admin: validGAddress1,
        token: validGAddress2,
        fee_config: { lock_fee_rate: 0n, release_fee_rate: 0n, fee_recipient: validGAddress1, fee_enabled: false },
        pause_flags: { lock_paused: false, release_paused: false, refund_paused: false },
        governance_contract: validGAddress2,
        min_governance_version: 1,
        claim_window: 3600n,
        has_amount_policy: false,
        min_lock_amount: 0n,
        max_lock_amount: 0n
      };
      const invoke = mockInvoke(snapshot);
      await expect(client.getAdminAuditView()).resolves.toEqual(snapshot);
      expect(invoke).toHaveBeenCalledWith('get_admin_audit_view', []);
    });
  });

  describe('Standard method routing success paths', () => {
    it('routes init correctly', async () => {
      const invoke = mockInvoke();
      await client.init(validGAddress1, validGAddress2, sourceKeypair);
      expect(invoke).toHaveBeenCalledWith('init', [validGAddress1, validGAddress2], sourceKeypair);
    });

    it('routes lockFunds correctly', async () => {
      const invoke = mockInvoke();
      await client.lockFunds(validGAddress1, 1n, 100n, 1000, sourceKeypair);
      expect(invoke).toHaveBeenCalledWith('lock_funds', [validGAddress1, 1n, 100n, 1000], sourceKeypair);
    });

    it('routes releaseFunds correctly', async () => {
      const invoke = mockInvoke();
      await client.releaseFunds(1n, validGAddress1, sourceKeypair);
      expect(invoke).toHaveBeenCalledWith('release_funds', [1n, validGAddress1], sourceKeypair);
    });

    it('routes partialRelease correctly', async () => {
      const invoke = mockInvoke();
      await client.partialRelease(1n, validGAddress1, 50n, sourceKeypair);
      expect(invoke).toHaveBeenCalledWith('partial_release', [1n, validGAddress1, 50n], sourceKeypair);
    });

    it('routes refund correctly', async () => {
      const invoke = mockInvoke();
      await client.refund(1n, sourceKeypair);
      expect(invoke).toHaveBeenCalledWith('refund', [1n], sourceKeypair);
    });

    it('routes authorizeClaim correctly', async () => {
      const invoke = mockInvoke();
      await client.authorizeClaim(1n, validGAddress1, sourceKeypair);
      expect(invoke).toHaveBeenCalledWith('authorize_claim', [1n, validGAddress1], sourceKeypair);
    });

    it('routes claim correctly', async () => {
      const invoke = mockInvoke();
      await client.claim(1n, sourceKeypair);
      expect(invoke).toHaveBeenCalledWith('claim', [1n], sourceKeypair);
    });

    it('routes batchLockFunds correctly', async () => {
      const invoke = mockInvoke(5);
      const items = [{ bounty_id: 1n, depositor: validGAddress1, amount: 100n, deadline: 1000 }];
      const count = await client.batchLockFunds(items, sourceKeypair);
      expect(count).toBe(5);
      expect(invoke).toHaveBeenCalledWith('batch_lock_funds', [items], sourceKeypair);
    });

    it('routes batchReleaseFunds correctly', async () => {
      const invoke = mockInvoke(3);
      const items = [{ bounty_id: 1n, contributor: validGAddress1 }];
      const count = await client.batchReleaseFunds(items, sourceKeypair);
      expect(count).toBe(3);
      expect(invoke).toHaveBeenCalledWith('batch_release_funds', [items], sourceKeypair);
    });

    it('routes getEscrowInfo correctly', async () => {
      const escrow = { depositor: validGAddress1, amount: 100n, remaining_amount: 100n, status: 'Locked', deadline: 1000, refund_history: [] };
      const invoke = mockInvoke(escrow);
      const result = await client.getEscrowInfo(1n);
      expect(result).toEqual(escrow);
      expect(invoke).toHaveBeenCalledWith('get_escrow_info', [1n]);
    });

    it('routes getBalance correctly', async () => {
      const invoke = mockInvoke(5000n);
      const balance = await client.getBalance();
      expect(balance).toBe(5000n);
      expect(invoke).toHaveBeenCalledWith('get_balance', []);
    });
  });

  describe('Contract error decoding for admin methods', () => {
    it('properly decodes unauthorized error for admin method', async () => {
      jest.spyOn(client as any, 'invokeContract').mockRejectedValue(new Error('bounty Unauthorized'));
      await expect(client.setPaused(true, null, null, sourceKeypair)).rejects.toThrow(ContractError);
      await expect(client.setPaused(true, null, null, sourceKeypair)).rejects.toThrow('Unauthorized');
    });

    it('properly decodes invalid fee rate error', async () => {
      jest.spyOn(client as any, 'invokeContract').mockRejectedValue(new Error('InvalidFeeRate'));
      await expect(client.updateFeeConfig(10n, null, null, null, sourceKeypair)).rejects.toThrow(ContractError);
      await expect(client.updateFeeConfig(10n, null, null, null, sourceKeypair)).rejects.toThrow('Fee rate is invalid');
    });
  });
});
