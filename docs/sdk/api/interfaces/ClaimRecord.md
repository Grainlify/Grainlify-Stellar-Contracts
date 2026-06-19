[**@grainlify/contracts-sdk**](../README.md)

***

[@grainlify/contracts-sdk](../README.md) / ClaimRecord

# Interface: ClaimRecord

Defined in: [src/bounty-escrow-client.ts:52](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L52)

Pending claim authorization for a bounty recipient.

## Properties

### amount

> **amount**: `bigint`

Defined in: [src/bounty-escrow-client.ts:58](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L58)

Claimable amount in the contract token's smallest unit.

***

### bounty\_id

> **bounty\_id**: `bigint`

Defined in: [src/bounty-escrow-client.ts:54](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L54)

Application-level bounty identifier.

***

### claimed

> **claimed**: `boolean`

Defined in: [src/bounty-escrow-client.ts:62](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L62)

Whether the authorized claim has already been consumed.

***

### expires\_at

> **expires\_at**: `number`

Defined in: [src/bounty-escrow-client.ts:60](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L60)

Unix timestamp when the claim authorization expires.

***

### recipient

> **recipient**: `string`

Defined in: [src/bounty-escrow-client.ts:56](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L56)

Stellar account authorized to claim the bounty.
