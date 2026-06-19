[**@grainlify/contracts-sdk**](../README.md)

***

[@grainlify/contracts-sdk](../README.md) / EscrowQueryFilter

# Interface: EscrowQueryFilter

Defined in: [src/bounty-escrow-client.ts:90](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L90)

Composite filter supported by the bounty escrow query endpoint.

## Properties

### depositor

> **depositor**: `string`

Defined in: [src/bounty-escrow-client.ts:98](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L98)

Depositor account to match when depositor filtering is enabled.

***

### has\_depositor\_filter

> **has\_depositor\_filter**: `boolean`

Defined in: [src/bounty-escrow-client.ts:96](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L96)

Enables filtering by depositor account when true.

***

### has\_status\_filter

> **has\_status\_filter**: `boolean`

Defined in: [src/bounty-escrow-client.ts:92](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L92)

Enables filtering by lifecycle status when true.

***

### max\_amount

> **max\_amount**: `bigint`

Defined in: [src/bounty-escrow-client.ts:102](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L102)

Inclusive maximum escrow amount.

***

### max\_deadline

> **max\_deadline**: `number`

Defined in: [src/bounty-escrow-client.ts:106](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L106)

Inclusive maximum deadline timestamp.

***

### min\_amount

> **min\_amount**: `bigint`

Defined in: [src/bounty-escrow-client.ts:100](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L100)

Inclusive minimum escrow amount.

***

### min\_deadline

> **min\_deadline**: `number`

Defined in: [src/bounty-escrow-client.ts:104](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L104)

Inclusive minimum deadline timestamp.

***

### status

> **status**: [`EscrowStatus`](../type-aliases/EscrowStatus.md)

Defined in: [src/bounty-escrow-client.ts:94](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L94)

Lifecycle status to match when status filtering is enabled.
