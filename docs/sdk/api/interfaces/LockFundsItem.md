[**@grainlify/contracts-sdk**](../README.md)

***

[@grainlify/contracts-sdk](../README.md) / LockFundsItem

# Interface: LockFundsItem

Defined in: [src/bounty-escrow-client.ts:14](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L14)

Input item for batch-locking a bounty escrow.

## Properties

### amount

> **amount**: `bigint`

Defined in: [src/bounty-escrow-client.ts:20](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L20)

Amount to lock, expressed in the contract token's smallest unit.

***

### bounty\_id

> **bounty\_id**: `bigint`

Defined in: [src/bounty-escrow-client.ts:16](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L16)

Application-level bounty identifier.

***

### deadline

> **deadline**: `number`

Defined in: [src/bounty-escrow-client.ts:22](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L22)

Unix timestamp after which the bounty may become refundable.

***

### depositor

> **depositor**: `string`

Defined in: [src/bounty-escrow-client.ts:18](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L18)

Stellar account that deposits the escrowed funds.
