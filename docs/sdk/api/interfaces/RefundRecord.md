[**@grainlify/contracts-sdk**](../README.md)

***

[@grainlify/contracts-sdk](../README.md) / RefundRecord

# Interface: RefundRecord

Defined in: [src/bounty-escrow-client.ts:40](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L40)

Historical refund record attached to an escrow.

## Properties

### amount

> **amount**: `bigint`

Defined in: [src/bounty-escrow-client.ts:42](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L42)

Refunded amount in the contract token's smallest unit.

***

### mode

> **mode**: [`RefundMode`](../type-aliases/RefundMode.md)

Defined in: [src/bounty-escrow-client.ts:48](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L48)

Whether the refund closed the escrow or returned a partial amount.

***

### recipient

> **recipient**: `string`

Defined in: [src/bounty-escrow-client.ts:44](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L44)

Stellar account that received the refund.

***

### timestamp

> **timestamp**: `number`

Defined in: [src/bounty-escrow-client.ts:46](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L46)

Unix timestamp when the refund was executed.
