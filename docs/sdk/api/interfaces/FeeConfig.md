[**@grainlify/contracts-sdk**](../README.md)

***

[@grainlify/contracts-sdk](../README.md) / FeeConfig

# Interface: FeeConfig

Defined in: [src/bounty-escrow-client.ts:154](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L154)

Fee policy configured on the bounty escrow contract.

## Properties

### fee\_enabled

> **fee\_enabled**: `boolean`

Defined in: [src/bounty-escrow-client.ts:162](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L162)

Whether fee collection is currently enabled.

***

### fee\_recipient

> **fee\_recipient**: `string`

Defined in: [src/bounty-escrow-client.ts:160](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L160)

Stellar account that receives fees.

***

### lock\_fee\_rate

> **lock\_fee\_rate**: `bigint`

Defined in: [src/bounty-escrow-client.ts:156](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L156)

Fee charged when locking funds, in basis points.

***

### release\_fee\_rate

> **release\_fee\_rate**: `bigint`

Defined in: [src/bounty-escrow-client.ts:158](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L158)

Fee charged when releasing funds, in basis points.
