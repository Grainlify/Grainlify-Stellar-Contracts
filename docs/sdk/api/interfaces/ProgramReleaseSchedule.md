[**@grainlify/contracts-sdk**](../README.md)

***

[@grainlify/contracts-sdk](../README.md) / ProgramReleaseSchedule

# Interface: ProgramReleaseSchedule

Defined in: [src/program-escrow-client.ts:40](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/program-escrow-client.ts#L40)

Scheduled release entry for program escrow funds.

## Properties

### amount

> **amount**: `bigint`

Defined in: [src/program-escrow-client.ts:46](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/program-escrow-client.ts#L46)

Scheduled amount in the contract token's smallest unit.

***

### recipient

> **recipient**: `string`

Defined in: [src/program-escrow-client.ts:44](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/program-escrow-client.ts#L44)

Stellar account that should receive the scheduled release.

***

### release\_timestamp

> **release\_timestamp**: `number`

Defined in: [src/program-escrow-client.ts:48](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/program-escrow-client.ts#L48)

Unix timestamp when the release becomes executable.

***

### released

> **released**: `boolean`

Defined in: [src/program-escrow-client.ts:50](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/program-escrow-client.ts#L50)

Whether the scheduled release has already been executed.

***

### schedule\_id

> **schedule\_id**: `bigint`

Defined in: [src/program-escrow-client.ts:42](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/program-escrow-client.ts#L42)

Unique schedule identifier.
