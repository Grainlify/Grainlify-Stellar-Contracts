import { Keypair } from '@stellar/stellar-sdk';
import { ProgramEscrowClient } from '../index';
import { ValidationError } from '../errors';

describe('ProgramEscrowClient', () => {
  const mockConfig = {
    contractId: 'CBTG2M4XXWNDH7GCHXZT6E2I3J644MFRZQK6CUKL4WJY6WQZXY3P2M6L',
    rpcUrl: 'http://localhost:8000/rpc',
    networkPassphrase: 'Test SDF Network ; September 2015',
  };

  const validAddress = 'GAXN6265B5U2ZIK2QFWIYYXGZ5B47L7Z236L72G66Z3S7MHT7XZQ5WZG';

  let client: ProgramEscrowClient;
  let sourceKeypair: Keypair;

  beforeEach(() => {
    client = new ProgramEscrowClient(mockConfig);
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

  describe('existing methods (regression)', () => {
    it('initProgram succeeds and calls init_program', async () => {
      const invoke = mockInvoke({ program_id: 'p1' });
      await client.initProgram('p1', validAddress, validAddress, sourceKeypair);
      expect(invoke).toHaveBeenCalledWith(
        'init_program',
        ['p1', validAddress, validAddress],
        sourceKeypair
      );
    });

    it('getRemainingBalance parses a bigint result', async () => {
      mockInvoke(500n);
      const result = await client.getRemainingBalance();
      expect(result).toBe(500n);
    });
  });

  describe('dispute resolution', () => {
    it('openDispute calls open_dispute with the reason', async () => {
      const invoke = mockInvoke();
      await client.openDispute('fraud suspected', sourceKeypair);
      expect(invoke).toHaveBeenCalledWith(
        'open_dispute',
        ['fraud suspected'],
        sourceKeypair
      );
    });

    it('resolveDispute calls resolve_dispute', async () => {
      const invoke = mockInvoke();
      await client.resolveDispute(sourceKeypair);
      expect(invoke).toHaveBeenCalledWith('resolve_dispute', [], sourceKeypair);
    });

    it('cancelDispute calls cancel_dispute', async () => {
      const invoke = mockInvoke();
      await client.cancelDispute(sourceKeypair);
      expect(invoke).toHaveBeenCalledWith('cancel_dispute', [], sourceKeypair);
    });

    it('getDispute returns the contract result', async () => {
      const record = {
        opened_by: validAddress,
        opened_at: 100,
        reason: 'fraud suspected',
        status: 'Open',
      };
      mockInvoke(record);
      await expect(client.getDispute()).resolves.toEqual(record);
    });

    it('getDispute returns undefined when there is no dispute', async () => {
      mockInvoke(null);
      await expect(client.getDispute()).resolves.toBeUndefined();
    });

    it('isDisputed returns the contract result', async () => {
      const invoke = mockInvoke(true);
      await expect(client.isDisputed()).resolves.toBe(true);
      expect(invoke).toHaveBeenCalledWith('is_disputed', []);
    });

    it('isRecipientDisputed rejects an invalid address', async () => {
      await expect(client.isRecipientDisputed('invalid')).rejects.toThrow(ValidationError);
    });

    it('isRecipientDisputed returns the contract result', async () => {
      mockInvoke(true);
      const result = await client.isRecipientDisputed(validAddress);
      expect(result).toBe(true);
    });

    it('isScheduleDisputed returns the contract result', async () => {
      mockInvoke(false);
      const result = await client.isScheduleDisputed(1n);
      expect(result).toBe(false);
    });
  });

  describe('whitelist management', () => {
    it('setWhitelist rejects an invalid address', async () => {
      await expect(
        client.setWhitelist('invalid', true, sourceKeypair)
      ).rejects.toThrow(ValidationError);
    });

    it('setWhitelist calls set_whitelist with the flag', async () => {
      const invoke = mockInvoke();
      await client.setWhitelist(validAddress, true, sourceKeypair);
      expect(invoke).toHaveBeenCalledWith(
        'set_whitelist',
        [validAddress, true],
        sourceKeypair
      );
    });

    it('isWhitelisted returns the contract result', async () => {
      mockInvoke(true);
      const result = await client.isWhitelisted(validAddress);
      expect(result).toBe(true);
    });

    it('setWhitelistEnforced calls set_whitelist_enforced', async () => {
      const invoke = mockInvoke();
      await client.setWhitelistEnforced(false, sourceKeypair);
      expect(invoke).toHaveBeenCalledWith(
        'set_whitelist_enforced',
        [false],
        sourceKeypair
      );
    });
  });

  describe('circuit breaker admin controls', () => {
    it('setCircuitAdmin rejects an invalid new admin address', async () => {
      await expect(
        client.setCircuitAdmin('invalid', null, sourceKeypair)
      ).rejects.toThrow(ValidationError);
    });

    it('resetCircuitBreaker rejects an invalid caller address', async () => {
      await expect(
        client.resetCircuitBreaker('invalid', sourceKeypair)
      ).rejects.toThrow(ValidationError);
    });

    it('configureCircuitBreaker calls configure_circuit_breaker with thresholds', async () => {
      const invoke = mockInvoke();
      await client.configureCircuitBreaker(3, 1, 10, validAddress, sourceKeypair);
      expect(invoke).toHaveBeenCalledWith(
        'configure_circuit_breaker',
        [validAddress, 3, 1, 10],
        sourceKeypair
      );
    });

    it('emergencyOpenCircuit calls emergency_open_circuit', async () => {
      const invoke = mockInvoke();
      await client.emergencyOpenCircuit(validAddress, sourceKeypair);
      expect(invoke).toHaveBeenCalledWith(
        'emergency_open_circuit',
        [validAddress],
        sourceKeypair
      );
    });

    it('getCircuitStatus returns the contract result', async () => {
      const status = { state: 'Closed', failure_count: 0, success_count: 0, last_failure_timestamp: 0n, opened_at: 0n };
      mockInvoke(status);
      const result = await client.getCircuitStatus();
      expect(result).toEqual(status);
    });
  });

  describe('governance integration', () => {
    it('setGovernanceContract rejects an invalid address', async () => {
      await expect(
        client.setGovernanceContract('invalid', sourceKeypair)
      ).rejects.toThrow(ValidationError);
    });

    it('setGovernanceContract calls set_governance_contract', async () => {
      const invoke = mockInvoke();
      await client.setGovernanceContract(validAddress, sourceKeypair);
      expect(invoke).toHaveBeenCalledWith(
        'set_governance_contract',
        [validAddress],
        sourceKeypair
      );
    });

    it('getGovernanceContract returns the contract result', async () => {
      mockInvoke(validAddress);
      const result = await client.getGovernanceContract();
      expect(result).toBe(validAddress);
    });

    it('setMinGovernanceVersion calls set_min_governance_version', async () => {
      const invoke = mockInvoke();
      await client.setMinGovernanceVersion(2, sourceKeypair);
      expect(invoke).toHaveBeenCalledWith(
        'set_min_governance_version',
        [2],
        sourceKeypair
      );
    });

    it('getMinGovernanceVersion parses a numeric result', async () => {
      mockInvoke(2);
      const result = await client.getMinGovernanceVersion();
      expect(result).toBe(2);
    });
  });

  describe('monitoring / analytics', () => {
    it('healthCheck returns the contract result', async () => {
      const health = { is_healthy: true, last_operation: 0n, total_operations: 0n, contract_version: '1.0.0' };
      mockInvoke(health);
      const result = await client.healthCheck();
      expect(result).toEqual(health);
    });

    it('getMonitoringAnalytics returns the contract result', async () => {
      const analytics = { total_locked: 0n, total_released: 0n, total_payouts: 0, active_programs: 0, operation_count: 0 };
      mockInvoke(analytics);
      const result = await client.getMonitoringAnalytics();
      expect(result).toEqual(analytics);
    });
  });
});
