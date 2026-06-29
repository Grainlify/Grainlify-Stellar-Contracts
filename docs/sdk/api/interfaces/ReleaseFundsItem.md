[**@grainlify/contracts-sdk**](../README.md)

***

[@grainlify/contracts-sdk](../README.md) / ReleaseFundsItem

# Interface: ReleaseFundsItem

Defined in: [src/bounty-escrow-client.ts:26](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L26)

Input item for batch-releasing a bounty escrow.

## Properties

### bounty\_id

> **bounty\_id**: `bigint`

Defined in: [src/bounty-escrow-client.ts:28](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L28)

Application-level bounty identifier.

***

### contributor

> **contributor**: `string`

Defined in: [src/bounty-escrow-client.ts:30](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L30)

Stellar account that should receive the released bounty funds.
