[**@grainlify/contracts-sdk**](../README.md)

***

[@grainlify/contracts-sdk](../README.md) / RefundEligibility

# Interface: RefundEligibility

Defined in: [src/bounty-escrow-client.ts:142](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L142)

Refund eligibility result for a bounty escrow.

## Properties

### approval?

> `optional` **approval?**: [`RefundApproval`](RefundApproval.md)

Defined in: [src/bounty-escrow-client.ts:150](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L150)

Optional approval details for admin-approved refunds.

***

### can\_refund

> **can\_refund**: `boolean`

Defined in: [src/bounty-escrow-client.ts:144](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L144)

True when the escrow can be refunded immediately.

***

### deadline\_passed

> **deadline\_passed**: `boolean`

Defined in: [src/bounty-escrow-client.ts:146](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L146)

Whether the escrow deadline has elapsed.

***

### remaining\_amount

> **remaining\_amount**: `bigint`

Defined in: [src/bounty-escrow-client.ts:148](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L148)

Remaining refundable amount.
