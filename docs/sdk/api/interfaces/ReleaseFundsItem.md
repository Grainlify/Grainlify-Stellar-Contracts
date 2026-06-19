[**@grainlify/contracts-sdk**](../README.md)

***

[@grainlify/contracts-sdk](../README.md) / ReleaseFundsItem

# Interface: ReleaseFundsItem

Defined in: [src/bounty-escrow-client.ts:26](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L26)

Input item for batch-releasing a bounty escrow.

## Properties

### bounty\_id

> **bounty\_id**: `bigint`

Defined in: [src/bounty-escrow-client.ts:28](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L28)

Application-level bounty identifier.

***

### contributor

> **contributor**: `string`

Defined in: [src/bounty-escrow-client.ts:30](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L30)

Stellar account that should receive the released bounty funds.
