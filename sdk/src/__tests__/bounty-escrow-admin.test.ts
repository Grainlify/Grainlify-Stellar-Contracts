import { Keypair } from '@stellar/stellar-sdk';
import { BountyEscrowClient } from '../bounty-escrow-client';
import { ValidationError } from '../errors';

describe('BountyEscrowClient Admin Methods', () => {
  const mockConfig = {
    contractId: 'CBTG2M4XXWNDH7GCHXZT6E2I3J644MFRZQK6CUKL4WJY6WQZXY3P2M6L',
    rpcUrl: 'http://localhost:8000/rpc',
    networkPassphrase: 'Test SDF Network ; September 2015',
  };

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

  describe('parameter validation', () => {
    it('throws validation error on invalid fee rates', async () => {
      await expect(
        client.updateFeeConfig(-1n, null, null, null, sourceKeypair)
      ).rejects.toThrow(ValidationError);

      await expect(
        client.updateFeeConfig(10001n, null, null, null, sourceKeypair)
      ).rejects.toThrow(ValidationError);

      await expect(
        client.updateFeeConfig(null, -5n, null, null, sourceKeypair)
      ).rejects.toThrow(ValidationError);

      await expect(
        client.updateFeeConfig(null, 10050n, null, null, sourceKeypair)
      ).rejects.toThrow(ValidationError);
    });

    it('throws validation error on invalid addresses in fee config', async () => {
      await expect(
        client.updateFeeConfig(null, null, 'invalid', null, sourceKeypair)
      ).rejects.toThrow(ValidationError);
    });

    it('throws validation error on invalid governance minVersion', async () => {
      await expect(
        client.setMinGovernanceVersion(-1, sourceKeypair)
      ).rejects.toThrow(ValidationError);

      await expect(
        client.setMinGovernanceVersion(2.5, sourceKeypair)
      ).rejects.toThrow(ValidationError);
    });

    it('throws validation error on invalid circuit breaker configs', async () => {
      await expect(
        client.setCircuitBreakerConfig(-1, 5, 10, sourceKeypair)
      ).rejects.toThrow(ValidationError);

      await expect(
        client.setCircuitBreakerConfig(5, -1, 10, sourceKeypair)
      ).rejects.toThrow(ValidationError);

      await expect(
        client.setCircuitBreakerConfig(5, 5, -10, sourceKeypair)
      ).rejects.toThrow(ValidationError);
    });

    it('throws validation error on invalid multisig config inputs', async () => {
      await expect(
        client.updateMultisigConfig(-10n, [validGAddress1], 1, sourceKeypair)
      ).rejects.toThrow(ValidationError);

      await expect(
        client.updateMultisigConfig(100n, [validGAddress1], 2, sourceKeypair)
      ).rejects.toThrow(ValidationError);

      await expect(
        client.updateMultisigConfig(100n, ['invalid'], 1, sourceKeypair)
      ).rejects.toThrow(ValidationError);
    });

    it('throws validation error on invalid amount policy range', async () => {
      await expect(
        client.setAmountPolicy(validGAddress1, -10n, 100n, sourceKeypair)
      ).rejects.toThrow(ValidationError);

      await expect(
        client.setAmountPolicy(validGAddress1, 500n, 100n, sourceKeypair)
      ).rejects.toThrow(ValidationError);
    });
  });

  describe('method routing', () => {
    it('routes fee and pause configurations', async () => {
      const invoke = mockInvoke();

      await client.updateFeeConfig(100n, 200n, validGAddress1, true, sourceKeypair);
      await client.setPaused(true, false, null, sourceKeypair);
      await client.setEmergencyPause(true, sourceKeypair);

      expect(invoke).toHaveBeenNthCalledWith(1, 'update_fee_config', [100n, 200n, validGAddress1, true], sourceKeypair);
      expect(invoke).toHaveBeenNthCalledWith(2, 'set_paused', [true, false, null], sourceKeypair);
      expect(invoke).toHaveBeenNthCalledWith(3, 'set_emergency_pause', [true], sourceKeypair);
    });

    it('routes governance configuration and getter methods', async () => {
      const invoke = mockInvoke();
      invoke
        .mockResolvedValueOnce(undefined)
        .mockResolvedValueOnce(validGAddress1)
        .mockResolvedValueOnce(undefined)
        .mockResolvedValueOnce(42);

      await client.setGovernanceContract(validGAddress1, sourceKeypair);
      await expect(client.getGovernanceContract()).resolves.toBe(validGAddress1);
      await client.setMinGovernanceVersion(10, sourceKeypair);
      await expect(client.getMinGovernanceVersion()).resolves.toBe(42);

      expect(invoke).toHaveBeenNthCalledWith(1, 'set_governance_contract', [validGAddress1], sourceKeypair);
      expect(invoke).toHaveBeenNthCalledWith(2, 'get_governance_contract', []);
      expect(invoke).toHaveBeenNthCalledWith(3, 'set_min_governance_version', [10], sourceKeypair);
      expect(invoke).toHaveBeenNthCalledWith(4, 'get_min_governance_version', []);
    });

    it('routes circuit breaker configurations and commands', async () => {
      const invoke = mockInvoke();
      const mockStatus = {
        state: 'Closed',
        consecutive_failures: 0,
        consecutive_successes: 0,
        last_state_change: 123456,
      };
      const mockConfigObj = {
        failure_threshold: 5,
        success_threshold: 3,
        max_error_log: 10,
      };
      const mockErrorLog = [
        { operation: 'lock', error_code: 1, timestamp: 1000 }
      ];

      invoke
        .mockResolvedValueOnce(undefined)
        .mockResolvedValueOnce(validGAddress1)
        .mockResolvedValueOnce(undefined)
        .mockResolvedValueOnce(mockConfigObj)
        .mockResolvedValueOnce(mockStatus)
        .mockResolvedValueOnce(undefined)
        .mockResolvedValueOnce(mockErrorLog);

      await client.setCircuitBreakerAdmin(validGAddress1, sourceKeypair);
      await expect(client.getCircuitBreakerAdmin()).resolves.toBe(validGAddress1);
      await client.setCircuitBreakerConfig(5, 3, 10, sourceKeypair);
      await expect(client.getCircuitBreakerConfig()).resolves.toEqual(mockConfigObj);
      await expect(client.getCircuitBreakerStatus()).resolves.toEqual(mockStatus);
      await client.resetCircuit(validGAddress1, sourceKeypair);
      await expect(client.getCircuitErrorLog()).resolves.toEqual(mockErrorLog);

      expect(invoke).toHaveBeenNthCalledWith(1, 'set_circuit_breaker_admin', [validGAddress1], sourceKeypair);
      expect(invoke).toHaveBeenNthCalledWith(2, 'get_circuit_breaker_admin', []);
      expect(invoke).toHaveBeenNthCalledWith(3, 'set_circuit_breaker_config', [5, 3, 10], sourceKeypair);
      expect(invoke).toHaveBeenNthCalledWith(4, 'get_circuit_breaker_config', []);
      expect(invoke).toHaveBeenNthCalledWith(5, 'get_circuit_breaker_status', []);
      expect(invoke).toHaveBeenNthCalledWith(6, 'reset_circuit', [validGAddress1], sourceKeypair);
      expect(invoke).toHaveBeenNthCalledWith(7, 'get_circuit_error_log', []);
    });

    it('routes multisig, amount-policy, anti-abuse, and whitelist methods', async () => {
      const invoke = mockInvoke();
      const mockMultisig = {
        threshold_amount: 1000n,
        signers: [validGAddress1, validGAddress2],
        required_signatures: 2,
      };

      invoke
        .mockResolvedValueOnce(undefined)
        .mockResolvedValueOnce(mockMultisig)
        .mockResolvedValueOnce(undefined)
        .mockResolvedValueOnce(undefined)
        .mockResolvedValueOnce(undefined)
        .mockResolvedValueOnce(validGAddress1)
        .mockResolvedValueOnce(undefined);

      await client.updateMultisigConfig(1000n, [validGAddress1, validGAddress2], 2, sourceKeypair);
      await expect(client.getMultisigConfig()).resolves.toEqual(mockMultisig);
      await client.approveLargeRelease(5n, validGAddress1, validGAddress2, sourceKeypair);
      await client.setAmountPolicy(validGAddress1, 100n, 2000n, sourceKeypair);
      await client.setAntiAbuseAdmin(validGAddress1, sourceKeypair);
      await expect(client.getAntiAbuseAdmin()).resolves.toBe(validGAddress1);
      await client.setWhitelist(validGAddress2, true, sourceKeypair);

      expect(invoke).toHaveBeenNthCalledWith(1, 'update_multisig_config', [1000n, [validGAddress1, validGAddress2], 2], sourceKeypair);
      expect(invoke).toHaveBeenNthCalledWith(2, 'get_multisig_config', []);
      expect(invoke).toHaveBeenNthCalledWith(3, 'approve_large_release', [5n, validGAddress1, validGAddress2], sourceKeypair);
      expect(invoke).toHaveBeenNthCalledWith(4, 'set_amount_policy', [validGAddress1, 100n, 2000n], sourceKeypair);
      expect(invoke).toHaveBeenNthCalledWith(5, 'set_anti_abuse_admin', [validGAddress1], sourceKeypair);
      expect(invoke).toHaveBeenNthCalledWith(6, 'get_anti_abuse_admin', []);
      expect(invoke).toHaveBeenNthCalledWith(7, 'set_whitelist', [validGAddress2, true], sourceKeypair);
    });
  });
});
