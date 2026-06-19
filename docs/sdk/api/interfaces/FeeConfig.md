[**@grainlify/contracts-sdk**](../README.md)

***

[@grainlify/contracts-sdk](../README.md) / FeeConfig

# Interface: FeeConfig

Defined in: [src/bounty-escrow-client.ts:154](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L154)

Fee policy configured on the bounty escrow contract.

## Properties

### fee\_enabled

> **fee\_enabled**: `boolean`

Defined in: [src/bounty-escrow-client.ts:162](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L162)

Whether fee collection is currently enabled.

***

### fee\_recipient

> **fee\_recipient**: `string`

Defined in: [src/bounty-escrow-client.ts:160](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L160)

Stellar account that receives fees.

***

### lock\_fee\_rate

> **lock\_fee\_rate**: `bigint`

Defined in: [src/bounty-escrow-client.ts:156](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L156)

Fee charged when locking funds, in basis points.

***

### release\_fee\_rate

> **release\_fee\_rate**: `bigint`

Defined in: [src/bounty-escrow-client.ts:158](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L158)

Fee charged when releasing funds, in basis points.
