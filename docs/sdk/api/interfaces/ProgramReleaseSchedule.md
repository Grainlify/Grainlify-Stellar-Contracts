[**@grainlify/contracts-sdk**](../README.md)

***

[@grainlify/contracts-sdk](../README.md) / ProgramReleaseSchedule

# Interface: ProgramReleaseSchedule

Defined in: [src/program-escrow-client.ts:40](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/program-escrow-client.ts#L40)

Scheduled release entry for program escrow funds.

## Properties

### amount

> **amount**: `bigint`

Defined in: [src/program-escrow-client.ts:46](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/program-escrow-client.ts#L46)

Scheduled amount in the contract token's smallest unit.

***

### recipient

> **recipient**: `string`

Defined in: [src/program-escrow-client.ts:44](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/program-escrow-client.ts#L44)

Stellar account that should receive the scheduled release.

***

### release\_timestamp

> **release\_timestamp**: `number`

Defined in: [src/program-escrow-client.ts:48](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/program-escrow-client.ts#L48)

Unix timestamp when the release becomes executable.

***

### released

> **released**: `boolean`

Defined in: [src/program-escrow-client.ts:50](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/program-escrow-client.ts#L50)

Whether the scheduled release has already been executed.

***

### schedule\_id

> **schedule\_id**: `bigint`

Defined in: [src/program-escrow-client.ts:42](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/program-escrow-client.ts#L42)

Unique schedule identifier.
