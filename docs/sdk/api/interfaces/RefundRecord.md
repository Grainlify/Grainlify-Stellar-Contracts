[**@grainlify/contracts-sdk**](../README.md)

***

[@grainlify/contracts-sdk](../README.md) / RefundRecord

# Interface: RefundRecord

Defined in: [src/bounty-escrow-client.ts:40](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L40)

Historical refund record attached to an escrow.

## Properties

### amount

> **amount**: `bigint`

Defined in: [src/bounty-escrow-client.ts:42](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L42)

Refunded amount in the contract token's smallest unit.

***

### mode

> **mode**: [`RefundMode`](../type-aliases/RefundMode.md)

Defined in: [src/bounty-escrow-client.ts:48](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L48)

Whether the refund closed the escrow or returned a partial amount.

***

### recipient

> **recipient**: `string`

Defined in: [src/bounty-escrow-client.ts:44](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L44)

Stellar account that received the refund.

***

### timestamp

> **timestamp**: `number`

Defined in: [src/bounty-escrow-client.ts:46](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L46)

Unix timestamp when the refund was executed.
