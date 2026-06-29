[**@grainlify/contracts-sdk**](../README.md)

***

[@grainlify/contracts-sdk](../README.md) / RefundApproval

# Interface: RefundApproval

Defined in: [src/bounty-escrow-client.ts:126](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L126)

Admin approval record required before a refund can be executed.

## Properties

### amount

> **amount**: `bigint`

Defined in: [src/bounty-escrow-client.ts:130](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L130)

Approved refund amount.

***

### approved\_at

> **approved\_at**: `number`

Defined in: [src/bounty-escrow-client.ts:138](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L138)

Unix timestamp when the approval was recorded.

***

### approved\_by

> **approved\_by**: `string`

Defined in: [src/bounty-escrow-client.ts:136](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L136)

Admin account that approved the refund.

***

### bounty\_id

> **bounty\_id**: `bigint`

Defined in: [src/bounty-escrow-client.ts:128](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L128)

Application-level bounty identifier.

***

### mode

> **mode**: [`RefundMode`](../type-aliases/RefundMode.md)

Defined in: [src/bounty-escrow-client.ts:134](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L134)

Approved refund mode.

***

### recipient

> **recipient**: `string`

Defined in: [src/bounty-escrow-client.ts:132](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L132)

Stellar account that may receive the refund.
