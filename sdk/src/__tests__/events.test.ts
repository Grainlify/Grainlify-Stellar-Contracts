import {
  DecodedContractEvent,
  decodeSorobanEvent,
  decodeSorobanEvents,
} from '../events';
import { nativeToScVal } from '@stellar/stellar-sdk';

const depositor = 'GAXN6265B5U2ZIK2QFWIYYXGZ5B47L7Z236L72G66Z3S7MHT7XZQ5WZG';
const recipient = 'GBZN6265B5U2ZIK2QFWIYYXGZ5B47L7Z236L72G66Z3S7MHT7XZQ5WZG';

describe('event decoding', () => {
  it('decodes a versioned bounty funds-locked event from map payloads', () => {
    const decoded = decodeSorobanEvent({
      topics: ['f_lock', 42n],
      value: new Map<string, unknown>([
        ['version', 2],
        ['bounty_id', 42n],
        ['amount', 1_500n],
        ['depositor', depositor],
        ['deadline', 1_800_000_000n],
      ]),
    });

    expect(decoded).toEqual({
      kind: 'bounty.funds_locked',
      contract: 'bounty_escrow',
      topic: 'f_lock',
      version: 2,
      bountyId: 42n,
      amount: 1_500n,
      depositor,
      deadline: 1_800_000_000n,
    });
  });

  it('decodes bounty claim-created and analytics snapshot events', () => {
    const claim = decodeSorobanEvent({
      topic: ['claim', 'created'],
      data: {
        version: 2,
        bounty_id: 7,
        recipient,
        amount: '2500',
        expires_at: 1_900_000_000,
      },
    });
    const snapshot = decodeSorobanEvent({
      topics: ['analytics', 'snap'],
      value: {
        version: 1,
        metrics: {
          active_bounty_count: 2,
          released_bounty_count: 3,
          refunded_bounty_count: 1,
          total_locked: 5000n,
          total_released: 3000n,
          total_refunded: 1000n,
          average_bounty_amount: 2500n,
          snapshot_timestamp: 1_900_000_010n,
        },
      },
    });

    expect(claim.kind).toBe('bounty.claim_created');
    expect((claim as DecodedContractEvent & { kind: 'bounty.claim_created' }).version).toBe(2);
    expect((claim as DecodedContractEvent & { kind: 'bounty.claim_created' }).expiresAt).toBe(1_900_000_000n);

    expect(snapshot.kind).toBe('bounty.analytics_snapshot');
    expect((snapshot as DecodedContractEvent & { kind: 'bounty.analytics_snapshot' }).metrics.totalLocked).toBe(5000n);
  });

  it('decodes versioned program escrow payout, batch, schedule, and dispute events', () => {
    const payout = decodeSorobanEvent({
      topics: ['Payout'],
      value: {
        version: 2,
        program_id: 'program-1',
        recipient,
        amount: 500n,
        remaining_balance: 4_500n,
      },
    });
    const batch = decodeSorobanEvent({
      topics: ['BatchPay'],
      value: {
        version: 2,
        program_id: 'program-1',
        recipient_count: 3,
        total_amount: 1_500n,
        remaining_balance: 3_000n,
        gas_proxy_transfer_ops: 3,
        gas_proxy_history_appends: 3,
        gas_proxy_storage_reads: 1,
        gas_proxy_storage_writes: 1,
        gas_proxy_events_emitted: 1,
      },
    });
    const schedule = decodeSorobanEvent({
      topics: ['SchedTrg'],
      value: {
        version: 2,
        program_id: 'program-1',
        schedule_id: 11n,
        recipient,
        amount: 700n,
        trigger_type: 'Automatic',
      },
    });
    const dispute = decodeSorobanEvent({
      topics: ['DispOpen'],
      value: {
        version: 2,
        program_id: 'program-1',
        opened_by: depositor,
        reason: 'missing milestone evidence',
        timestamp: 1_900_000_111n,
      },
    });

    expect(payout.kind).toBe('program.payout');
    expect((payout as DecodedContractEvent & { kind: 'program.payout' }).remainingBalance).toBe(4_500n);
    expect(batch.kind).toBe('program.batch_payout');
    expect((batch as DecodedContractEvent & { kind: 'program.batch_payout' }).recipientCount).toBe(3);
    expect(schedule.kind).toBe('program.schedule_triggered');
    expect((schedule as DecodedContractEvent & { kind: 'program.schedule_triggered' }).triggerType).toBe('Automatic');
    expect(dispute.kind).toBe('program.dispute_opened');
    expect((dispute as DecodedContractEvent & { kind: 'program.dispute_opened' }).openedBy).toBe(depositor);
  });

  it('decodes ScVal payloads from Soroban RPC responses', () => {
    const decoded = decodeSorobanEvent({
      topics: ['FndsLock'],
      value: nativeToScVal({
        version: 2,
        program_id: 'program-1',
        amount: 10_000n,
        remaining_balance: 90_000n,
      }),
    });

    expect(decoded.kind).toBe('program.funds_locked');
    expect((decoded as DecodedContractEvent & { kind: 'program.funds_locked' }).amount).toBe(10_000n);
  });

  it('surfaces unknown topics and unsupported versions explicitly', () => {
    expect(decodeSorobanEvent({ topics: ['unexpected'], value: { version: 1 } })).toEqual({
      kind: 'unknown',
      topics: ['unexpected'],
      rawPayload: { version: 1 },
    });

    expect(
      decodeSorobanEvent({
        topics: ['Payout'],
        value: {
          version: 99,
          program_id: 'program-1',
          recipient,
          amount: 500n,
          remaining_balance: 4_500n,
        },
      })
    ).toEqual({
      kind: 'unknown_version',
      eventName: 'program.payout',
      version: 99,
      supportedVersions: [2],
      topics: ['Payout'],
      rawPayload: {
        version: 99,
        program_id: 'program-1',
        recipient,
        amount: 500n,
        remaining_balance: 4_500n,
      },
    });
  });

  it('returns malformed instead of guessing when payload fields are missing', () => {
    const decoded = decodeSorobanEvent({
      topics: ['f_rel', 99n],
      value: {
        version: 2,
        bounty_id: 99n,
        recipient,
        timestamp: 1_900_000_222n,
      },
    });

    expect(decoded.kind).toBe('malformed');
    expect(decoded).toMatchObject({
      topics: ['f_rel', 99n],
      rawPayload: {
        version: 2,
        bounty_id: 99n,
        recipient,
        timestamp: 1_900_000_222n,
      },
    });
    expect((decoded as { reason: string }).reason).toContain('amount');
  });

  it('decodes arrays or transaction-like objects containing events', () => {
    const events = decodeSorobanEvents({
      events: [
        {
          topics: ['Payout'],
          value: {
            version: 2,
            program_id: 'program-1',
            recipient,
            amount: 500n,
            remaining_balance: 4_500n,
          },
        },
        {
          topics: ['analytics', 'state_tx'],
          value: {
            version: 1,
            bounty_id: 99n,
            previous_state: 'locked',
            new_state: 'released',
            amount: 500n,
            actor: depositor,
            timestamp: 1_900_000_333n,
          },
        },
      ],
    });

    expect(events.map((event) => event.kind)).toEqual([
      'program.payout',
      'bounty.state_transitioned',
    ]);
  });
});
