[**@grainlify/contracts-sdk**](../README.md)

***

[@grainlify/contracts-sdk](../README.md) / PayoutRecord

# Interface: PayoutRecord

Defined in: [src/program-escrow-client.ts:30](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/program-escrow-client.ts#L30)

Single payout event recorded by the program escrow.

## Properties

### amount

> **amount**: `bigint`

Defined in: [src/program-escrow-client.ts:34](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/program-escrow-client.ts#L34)

Payout amount in the contract token's smallest unit.

***

### recipient

> **recipient**: `string`

Defined in: [src/program-escrow-client.ts:32](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/program-escrow-client.ts#L32)

Stellar account that received the payout.

***

### timestamp

> **timestamp**: `number`

Defined in: [src/program-escrow-client.ts:36](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/program-escrow-client.ts#L36)

Unix timestamp when the payout was recorded.
