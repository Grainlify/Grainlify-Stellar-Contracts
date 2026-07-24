import { nativeToScVal, xdr } from '@stellar/stellar-sdk';
import { decodeContractEvent } from '../events';
import { ValidationError } from '../errors';

describe('Soroban Event Decoding', () => {
  const gAddress1 = 'GAXN6265B5U2ZIK2QFWIYYXGZ5B47L7Z236L72G66Z3S7MHT7XZQ5WZG';
  const gAddress2 = 'GBZN6265B5U2ZIK2QFWIYYXGZ5B47L7Z236L72G66Z3S7MHT7XZQ5WZG';

  describe('Bounty Escrow Events', () => {
    it('decodes init event successfully', () => {
      const topics = [nativeToScVal('init')];
      const value = nativeToScVal({
        version: 2,
        admin: gAddress1,
        token: gAddress2,
        timestamp: 1718900000n,
      });

      const decoded = decodeContractEvent(topics, value);
      expect(decoded).toEqual({
        type: 'init',
        version: 2,
        admin: gAddress1,
        token: gAddress2,
        timestamp: 1718900000n,
      });
    });

    it('decodes f_lock event successfully', () => {
      const topics = [nativeToScVal('f_lock'), nativeToScVal(1n)];
      const value = nativeToScVal({
        version: 2,
        bounty_id: 1n,
        amount: 1000n,
        depositor: gAddress1,
        deadline: 1718986400n,
      });

      const decoded = decodeContractEvent(topics, value);
      expect(decoded).toEqual({
        type: 'f_lock',
        version: 2,
        bounty_id: 1n,
        amount: 1000n,
        depositor: gAddress1,
        deadline: 1718986400n,
      });
    });

    it('decodes f_rel event successfully', () => {
      const topics = [nativeToScVal('f_rel'), nativeToScVal(5n)];
      const value = nativeToScVal({
        version: 2,
        bounty_id: 5n,
        amount: 500n,
        recipient: gAddress2,
        timestamp: 1718905000n,
      });

      const decoded = decodeContractEvent(topics, value);
      expect(decoded).toEqual({
        type: 'f_rel',
        version: 2,
        bounty_id: 5n,
        amount: 500n,
        recipient: gAddress2,
        timestamp: 1718905000n,
      });
    });

    it('decodes f_ref event successfully', () => {
      const topics = [nativeToScVal('f_ref'), nativeToScVal(3n)];
      const value = nativeToScVal({
        version: 2,
        bounty_id: 3n,
        amount: 250n,
        refund_to: gAddress1,
        timestamp: 1718906000n,
      });

      const decoded = decodeContractEvent(topics, value);
      expect(decoded).toEqual({
        type: 'f_ref',
        version: 2,
        bounty_id: 3n,
        amount: 250n,
        refund_to: gAddress1,
        timestamp: 1718906000n,
      });
    });

    it('decodes b_exp event successfully', () => {
      const topics = [nativeToScVal('b_exp'), nativeToScVal(4n)];
      const value = nativeToScVal({
        version: 2,
        bounty_id: 4n,
        depositor: gAddress1,
        amount: 1200n,
        deadline: 1718900000n,
        expired_at: 1718901000n,
      });

      const decoded = decodeContractEvent(topics, value);
      expect(decoded).toEqual({
        type: 'b_exp',
        version: 2,
        bounty_id: 4n,
        depositor: gAddress1,
        amount: 1200n,
        deadline: 1718900000n,
        expired_at: 1718901000n,
      });
    });

    it('decodes fee event successfully', () => {
      const topics = [nativeToScVal('fee')];
      const value = nativeToScVal({
        version: 2,
        operation_type: 'Lock',
        amount: 15n,
        fee_rate: 100n,
        recipient: gAddress2,
        timestamp: 1718907000n,
      });

      const decoded = decodeContractEvent(topics, value);
      expect(decoded).toEqual({
        type: 'fee',
        version: 2,
        operation_type: 'Lock',
        amount: 15n,
        fee_rate: 100n,
        recipient: gAddress2,
        timestamp: 1718907000n,
      });
    });

    it('decodes b_lock event successfully', () => {
      const topics = [nativeToScVal('b_lock')];
      const value = nativeToScVal({
        version: 2,
        count: 5,
        total_amount: 50000n,
        timestamp: 1718908000n,
      });

      const decoded = decodeContractEvent(topics, value);
      expect(decoded).toEqual({
        type: 'b_lock',
        version: 2,
        count: 5,
        total_amount: 50000n,
        timestamp: 1718908000n,
      });
    });

    it('decodes fee_cfg event successfully', () => {
      const topics = [nativeToScVal('fee_cfg')];
      const value = nativeToScVal({
        version: 2,
        lock_fee_rate: 50n,
        release_fee_rate: 100n,
        fee_recipient: gAddress2,
        fee_enabled: true,
        timestamp: 1718909000n,
      });

      const decoded = decodeContractEvent(topics, value);
      expect(decoded).toEqual({
        type: 'fee_cfg',
        version: 2,
        lock_fee_rate: 50n,
        release_fee_rate: 100n,
        fee_recipient: gAddress2,
        fee_enabled: true,
        timestamp: 1718909000n,
      });
    });

    it('decodes b_rel event successfully', () => {
      const topics = [nativeToScVal('b_rel')];
      const value = nativeToScVal({
        version: 2,
        count: 10,
        total_amount: 25000n,
        timestamp: 1718910000n,
      });

      const decoded = decodeContractEvent(topics, value);
      expect(decoded).toEqual({
        type: 'b_rel',
        version: 2,
        count: 10,
        total_amount: 25000n,
        timestamp: 1718910000n,
      });
    });

    it('decodes approval event successfully', () => {
      const topics = [nativeToScVal('approval'), nativeToScVal(42n)];
      const value = nativeToScVal({
        version: 2,
        bounty_id: 42n,
        contributor: gAddress1,
        approver: gAddress2,
        timestamp: 1718911000n,
      });

      const decoded = decodeContractEvent(topics, value);
      expect(decoded).toEqual({
        type: 'approval',
        version: 2,
        bounty_id: 42n,
        contributor: gAddress1,
        approver: gAddress2,
        timestamp: 1718911000n,
      });
    });

    it('decodes pause event successfully', () => {
      const topics = [nativeToScVal('pause'), nativeToScVal('lock')];
      const value = nativeToScVal({
        version: 2,
        operation: 'lock',
        paused: true,
        timestamp: 1718912000n,
      });

      const decoded = decodeContractEvent(topics, value);
      expect(decoded).toEqual({
        type: 'pause',
        version: 2,
        operation: 'lock',
        paused: true,
        timestamp: 1718912000n,
      });
    });

    it('decodes analytics state_tx event successfully', () => {
      const topics = [nativeToScVal('analytics'), nativeToScVal('state_tx')];
      const value = nativeToScVal({
        version: 1,
        bounty_id: 99n,
        previous_state: 'Locked',
        new_state: 'Released',
        amount: 5000n,
        actor: gAddress1,
        timestamp: 1718913000n,
      });

      const decoded = decodeContractEvent(topics, value);
      expect(decoded).toEqual({
        type: 'state_tx',
        version: 1,
        bounty_id: 99n,
        previous_state: 'Locked',
        new_state: 'Released',
        amount: 5000n,
        actor: gAddress1,
        timestamp: 1718913000n,
      });
    });

    it('decodes analytics snap event successfully', () => {
      const topics = [nativeToScVal('analytics'), nativeToScVal('snap')];
      const value = nativeToScVal({
        version: 1,
        metrics: {
          active_bounty_count: 5,
          released_bounty_count: 10,
          refunded_bounty_count: 2,
          total_locked: 50000n,
          total_released: 120000n,
          total_refunded: 15000n,
          average_bounty_amount: 10000n,
          snapshot_timestamp: 1718914000n,
        },
      });

      const decoded = decodeContractEvent(topics, value);
      expect(decoded).toEqual({
        type: 'snap',
        version: 1,
        metrics: {
          active_bounty_count: 5,
          released_bounty_count: 10,
          refunded_bounty_count: 2,
          total_locked: 50000n,
          total_released: 120000n,
          total_refunded: 15000n,
          average_bounty_amount: 10000n,
          snapshot_timestamp: 1718914000n,
        },
      });
    });

    it('decodes analytics activity event successfully', () => {
      const topics = [nativeToScVal('analytics'), nativeToScVal('activity')];
      const value = nativeToScVal({
        version: 1,
        bounty_id: 123n,
        activity_type: 'created',
        amount: 15000n,
        timestamp: 1718915000n,
      });

      const decoded = decodeContractEvent(topics, value);
      expect(decoded).toEqual({
        type: 'activity',
        version: 1,
        bounty_id: 123n,
        activity_type: 'created',
        amount: 15000n,
        timestamp: 1718915000n,
      });
    });
  });

  describe('Program Escrow Events', () => {
    it('decodes PrgInit event successfully', () => {
      const topics = [nativeToScVal('PrgInit')];
      const value = nativeToScVal({
        version: 2,
        program_id: 'hack-2026',
        authorized_payout_key: gAddress1,
        token_address: gAddress2,
        total_funds: 500000n,
      });

      const decoded = decodeContractEvent(topics, value);
      expect(decoded).toEqual({
        type: 'PrgInit',
        version: 2,
        program_id: 'hack-2026',
        authorized_payout_key: gAddress1,
        token_address: gAddress2,
        total_funds: 500000n,
      });
    });

    it('decodes FndsLock event successfully', () => {
      const topics = [nativeToScVal('FndsLock')];
      const value = nativeToScVal({
        version: 2,
        program_id: 'hack-2026',
        amount: 25000n,
        remaining_balance: 125000n,
      });

      const decoded = decodeContractEvent(topics, value);
      expect(decoded).toEqual({
        type: 'FndsLock',
        version: 2,
        program_id: 'hack-2026',
        amount: 25000n,
        remaining_balance: 125000n,
      });
    });

    it('decodes BatchPay event successfully', () => {
      const topics = [nativeToScVal('BatchPay')];
      const value = nativeToScVal({
        version: 2,
        program_id: 'hack-2026',
        recipient_count: 4,
        total_amount: 80000n,
        remaining_balance: 45000n,
        gas_proxy_transfer_ops: 2,
        gas_proxy_history_appends: 3,
        gas_proxy_storage_reads: 5,
        gas_proxy_storage_writes: 6,
        gas_proxy_events_emitted: 1,
      });

      const decoded = decodeContractEvent(topics, value);
      expect(decoded).toEqual({
        type: 'BatchPay',
        version: 2,
        program_id: 'hack-2026',
        recipient_count: 4,
        total_amount: 80000n,
        remaining_balance: 45000n,
        gas_proxy_transfer_ops: 2,
        gas_proxy_history_appends: 3,
        gas_proxy_storage_reads: 5,
        gas_proxy_storage_writes: 6,
        gas_proxy_events_emitted: 1,
      });
    });

    it('decodes Payout event successfully', () => {
      const topics = [nativeToScVal('Payout')];
      const value = nativeToScVal({
        version: 2,
        program_id: 'hack-2026',
        recipient: gAddress2,
        amount: 15000n,
        remaining_balance: 30000n,
      });

      const decoded = decodeContractEvent(topics, value);
      expect(decoded).toEqual({
        type: 'Payout',
        version: 2,
        program_id: 'hack-2026',
        recipient: gAddress2,
        amount: 15000n,
        remaining_balance: 30000n,
      });
    });

    it('decodes DispOpen event successfully with Global scope', () => {
      const topics = [nativeToScVal('DispOpen')];
      const value = nativeToScVal({
        version: 2,
        program_id: 'hack-2026',
        scope: 'Global',
        opened_by: gAddress1,
        reason: 'Broken milestones',
        timestamp: 1718920000n,
      });

      const decoded = decodeContractEvent(topics, value);
      expect(decoded).toEqual({
        type: 'DispOpen',
        version: 2,
        program_id: 'hack-2026',
        scope: { type: 'Global' },
        opened_by: gAddress1,
        reason: 'Broken milestones',
        timestamp: 1718920000n,
      });
    });

    it('decodes DispOpen event successfully with Recipient scope', () => {
      const topics = [nativeToScVal('DispOpen')];
      const value = nativeToScVal({
        version: 2,
        program_id: 'hack-2026',
        scope: { Recipient: gAddress2 },
        opened_by: gAddress1,
        reason: 'Failed audit',
        timestamp: 1718921000n,
      });

      const decoded = decodeContractEvent(topics, value);
      expect(decoded).toEqual({
        type: 'DispOpen',
        version: 2,
        program_id: 'hack-2026',
        scope: { type: 'Recipient', value: gAddress2 },
        opened_by: gAddress1,
        reason: 'Failed audit',
        timestamp: 1718921000n,
      });
    });

    it('decodes DispOpen event successfully with Schedule scope', () => {
      const topics = [nativeToScVal('DispOpen')];
      const value = nativeToScVal({
        version: 2,
        program_id: 'hack-2026',
        scope: { Schedule: 4n },
        opened_by: gAddress1,
        reason: 'Delayed release',
        timestamp: 1718922000n,
      });

      const decoded = decodeContractEvent(topics, value);
      expect(decoded).toEqual({
        type: 'DispOpen',
        version: 2,
        program_id: 'hack-2026',
        scope: { type: 'Schedule', value: 4n },
        opened_by: gAddress1,
        reason: 'Delayed release',
        timestamp: 1718922000n,
      });
    });

    it('decodes DispRes event successfully', () => {
      const topics = [nativeToScVal('DispRes')];
      const value = nativeToScVal({
        version: 2,
        program_id: 'hack-2026',
        scope: 'Global',
        resolved_by: gAddress1,
        timestamp: 1718923000n,
      });

      const decoded = decodeContractEvent(topics, value);
      expect(decoded).toEqual({
        type: 'DispRes',
        version: 2,
        program_id: 'hack-2026',
        scope: { type: 'Global' },
        resolved_by: gAddress1,
        timestamp: 1718923000n,
      });
    });

    it('decodes DispCanc event successfully', () => {
      const topics = [nativeToScVal('DispCanc')];
      const value = nativeToScVal({
        version: 2,
        program_id: 'hack-2026',
        scope: { Schedule: 1n },
        cancelled_by: gAddress1,
        timestamp: 1718924000n,
      });

      const decoded = decodeContractEvent(topics, value);
      expect(decoded).toEqual({
        type: 'DispCanc',
        version: 2,
        program_id: 'hack-2026',
        scope: { type: 'Schedule', value: 1n },
        cancelled_by: gAddress1,
        timestamp: 1718924000n,
      });
    });

    it('decodes PauseSt tuple event successfully', () => {
      const topics = [nativeToScVal('PauseSt')];
      const value = xdr.ScVal.scvVec([
        nativeToScVal('lock'),
        nativeToScVal(true),
        nativeToScVal(gAddress1),
      ]);

      const decoded = decodeContractEvent(topics, value);
      expect(decoded).toEqual({
        type: 'PauseSt',
        version: 2,
        operation: 'lock',
        paused: true,
        admin: gAddress1,
      });
    });

    it('decodes AggStats event successfully', () => {
      const topics = [nativeToScVal('AggStats')];
      const value = nativeToScVal({
        version: 2,
        program_id: 'hack-2026',
        total_funds: 250000n,
        remaining_balance: 150000n,
        total_paid_out: 100000n,
        payout_count: 8,
        scheduled_count: 3,
      });

      const decoded = decodeContractEvent(topics, value);
      expect(decoded).toEqual({
        type: 'AggStats',
        version: 2,
        program_id: 'hack-2026',
        total_funds: 250000n,
        remaining_balance: 150000n,
        total_paid_out: 100000n,
        payout_count: 8,
        scheduled_count: 3,
      });
    });

    it('decodes LrgPay event successfully', () => {
      const topics = [nativeToScVal('LrgPay')];
      const value = nativeToScVal({
        version: 2,
        program_id: 'hack-2026',
        recipient: gAddress2,
        amount: 80000n,
        threshold: 50000n,
      });

      const decoded = decodeContractEvent(topics, value);
      expect(decoded).toEqual({
        type: 'LrgPay',
        version: 2,
        program_id: 'hack-2026',
        recipient: gAddress2,
        amount: 80000n,
        threshold: 50000n,
      });
    });

    it('decodes SchedTrg event successfully', () => {
      const topics = [nativeToScVal('SchedTrg')];
      const value = nativeToScVal({
        version: 2,
        program_id: 'hack-2026',
        schedule_id: 12n,
        recipient: gAddress2,
        amount: 30000n,
        trigger_type: 'Automatic',
      });

      const decoded = decodeContractEvent(topics, value);
      expect(decoded).toEqual({
        type: 'SchedTrg',
        version: 2,
        program_id: 'hack-2026',
        schedule_id: 12n,
        recipient: gAddress2,
        amount: 30000n,
        trigger_type: 'Automatic',
      });
    });

    it('decodes metric op event successfully', () => {
      const topics = [nativeToScVal('metric'), nativeToScVal('op')];
      const value = nativeToScVal({
        operation: 'lock_funds',
        caller: gAddress1,
        timestamp: 1718925000n,
        success: true,
      });

      const decoded = decodeContractEvent(topics, value);
      expect(decoded).toEqual({
        type: 'metric_op',
        version: 2,
        operation: 'lock_funds',
        caller: gAddress1,
        timestamp: 1718925000n,
        success: true,
      });
    });

    it('decodes metric perf event successfully', () => {
      const topics = [nativeToScVal('metric'), nativeToScVal('perf')];
      const value = nativeToScVal({
        function: 'batch_payout',
        duration: 154320n,
        timestamp: 1718926000n,
      });

      const decoded = decodeContractEvent(topics, value);
      expect(decoded).toEqual({
        type: 'metric_perf',
        version: 2,
        function: 'batch_payout',
        duration: 154320n,
        timestamp: 1718926000n,
      });
    });
  });

  describe('Edge Cases and Validation Errors', () => {
    it('throws ValidationError for empty topics list', () => {
      expect(() => {
        decodeContractEvent([], nativeToScVal({}));
      }).toThrow(ValidationError);
    });

    it('throws ValidationError if first topic is not a string symbol', () => {
      expect(() => {
        decodeContractEvent([nativeToScVal(42)], nativeToScVal({}));
      }).toThrow(ValidationError);
    });

    it('throws ValidationError for unknown topic', () => {
      expect(() => {
        decodeContractEvent([nativeToScVal('unknown_topic')], nativeToScVal({}));
      }).toThrow(ValidationError);
    });

    it('throws ValidationError for unknown analytics subtopic', () => {
      expect(() => {
        decodeContractEvent(
          [nativeToScVal('analytics'), nativeToScVal('unknown_subtopic')],
          nativeToScVal({})
        );
      }).toThrow(ValidationError);
    });

    it('throws ValidationError for unknown metric subtopic', () => {
      expect(() => {
        decodeContractEvent(
          [nativeToScVal('metric'), nativeToScVal('unknown_subtopic')],
          nativeToScVal({})
        );
      }).toThrow(ValidationError);
    });

    it('throws ValidationError for invalid fields/types in init event', () => {
      expect(() => {
        decodeContractEvent(
          [nativeToScVal('init')],
          nativeToScVal({
            version: 2,
            admin: 123, // should be string
            token: gAddress2,
            timestamp: 1718900000n,
          })
        );
      }).toThrow(ValidationError);
    });

    it('throws ValidationError for invalid version in init event', () => {
      expect(() => {
        decodeContractEvent(
          [nativeToScVal('init')],
          nativeToScVal({
            version: 3, // invalid version
            admin: gAddress1,
            token: gAddress2,
            timestamp: 1718900000n,
          })
        );
      }).toThrow(ValidationError);
    });

    it('throws ValidationError for invalid dispute scope format', () => {
      expect(() => {
        decodeContractEvent(
          [nativeToScVal('DispOpen')],
          nativeToScVal({
            version: 2,
            program_id: 'hack-2026',
            scope: { UnknownScopeKey: 123 }, // invalid scope key
            opened_by: gAddress1,
            reason: 'Failed validation',
            timestamp: 1718920000n,
          })
        );
      }).toThrow(ValidationError);
    });
  });
});
