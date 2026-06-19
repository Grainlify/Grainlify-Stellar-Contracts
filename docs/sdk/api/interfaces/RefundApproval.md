[**@grainlify/contracts-sdk**](../README.md)

***

[@grainlify/contracts-sdk](../README.md) / RefundApproval

# Interface: RefundApproval

Defined in: [src/bounty-escrow-client.ts:126](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L126)

Admin approval record required before a refund can be executed.

## Properties

### amount

> **amount**: `bigint`

Defined in: [src/bounty-escrow-client.ts:130](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L130)

Approved refund amount.

***

### approved\_at

> **approved\_at**: `number`

Defined in: [src/bounty-escrow-client.ts:138](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L138)

Unix timestamp when the approval was recorded.

***

### approved\_by

> **approved\_by**: `string`

Defined in: [src/bounty-escrow-client.ts:136](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L136)

Admin account that approved the refund.

***

### bounty\_id

> **bounty\_id**: `bigint`

Defined in: [src/bounty-escrow-client.ts:128](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L128)

Application-level bounty identifier.

***

### mode

> **mode**: [`RefundMode`](../type-aliases/RefundMode.md)

Defined in: [src/bounty-escrow-client.ts:134](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L134)

Approved refund mode.

***

### recipient

> **recipient**: `string`

Defined in: [src/bounty-escrow-client.ts:132](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L132)

Stellar account that may receive the refund.
