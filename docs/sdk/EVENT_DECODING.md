# SDK Event Decoding

The SDK exports typed decoders for Grainlify Soroban contract events. They are
intended for indexers, dashboards, and monitoring jobs that receive raw RPC
events and need stable TypeScript objects instead of hand-written payload
parsing.

## Entry Points

```typescript
import {
  decodeSorobanEvent,
  decodeSorobanEvents,
  DecodedSorobanEvent,
} from '@grainlify/contracts-sdk';
```

- `decodeSorobanEvent(rawEvent)` decodes a single event.
- `decodeSorobanEvents(events)` decodes an array of events.
- `decodeSorobanEvents({ events })` also accepts transaction-like response
  objects that expose an `events` array.

Each decoded result is a discriminated union. Successful decodes have specific
`kind` values, such as `bounty.funds_locked`, `program.payout`, or
`program.schedule_triggered`.

## Safety Behavior

The decoder never guesses when it cannot prove the shape of an event.

- Unknown topics return `kind: 'unknown'`.
- Unsupported versions return `kind: 'unknown_version'` with the supported
  version list.
- Missing or malformed fields return `kind: 'malformed'` with a reason string.

This lets indexers store or alert on unexpected events without silently
mislabeling them.

## Covered Event Families

The decoder covers the versioned bounty escrow and program escrow events used
by monitoring and indexing flows:

- Bounty escrow: initialization, funds locked, funds released, funds refunded,
  expired bounty, claim created, analytics state transition, analytics activity,
  analytics snapshot.
- Program escrow: program initialized, funds locked, payout, batch payout,
  schedule triggered, dispute opened, aggregate stats, large payout.

## Example

```typescript
const decoded = decodeSorobanEvent({
  topics: ['Payout'],
  value: {
    version: 2,
    program_id: 'program-1',
    recipient: 'G...',
    amount: 500n,
    remaining_balance: 4500n,
  },
});

if (decoded.kind === 'program.payout') {
  console.log(decoded.programId, decoded.recipient, decoded.amount);
}

if (decoded.kind === 'unknown_version' || decoded.kind === 'malformed') {
  console.warn('Event needs review:', decoded);
}
```
