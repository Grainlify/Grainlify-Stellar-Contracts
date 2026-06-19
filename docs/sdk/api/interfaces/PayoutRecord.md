[**@grainlify/contracts-sdk**](../README.md)

***

[@grainlify/contracts-sdk](../README.md) / PayoutRecord

# Interface: PayoutRecord

Defined in: [src/program-escrow-client.ts:30](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/program-escrow-client.ts#L30)

Single payout event recorded by the program escrow.

## Properties

### amount

> **amount**: `bigint`

Defined in: [src/program-escrow-client.ts:34](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/program-escrow-client.ts#L34)

Payout amount in the contract token's smallest unit.

***

### recipient

> **recipient**: `string`

Defined in: [src/program-escrow-client.ts:32](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/program-escrow-client.ts#L32)

Stellar account that received the payout.

***

### timestamp

> **timestamp**: `number`

Defined in: [src/program-escrow-client.ts:36](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/program-escrow-client.ts#L36)

Unix timestamp when the payout was recorded.
