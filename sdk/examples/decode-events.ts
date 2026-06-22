import { decodeSorobanEvents, DecodedSorobanEvent } from '../src/events';

/**
 * Example: Decode Soroban events into typed SDK results.
 */
export function decodeEventsExample(rawEvents: unknown[]): DecodedSorobanEvent[] {
  const decoded = decodeSorobanEvents(rawEvents);

  for (const event of decoded) {
    if (event.kind === 'program.payout') {
      console.log('Program payout:', event.programId, event.recipient, event.amount);
    } else if (event.kind === 'bounty.funds_locked') {
      console.log('Bounty locked:', event.bountyId, event.depositor, event.amount);
    } else if (event.kind === 'unknown_version' || event.kind === 'malformed') {
      console.warn('Event needs review:', event);
    }
  }

  return decoded;
}
