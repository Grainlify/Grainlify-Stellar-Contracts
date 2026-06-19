[**@grainlify/contracts-sdk**](../README.md)

***

[@grainlify/contracts-sdk](../README.md) / ProgramData

# Interface: ProgramData

Defined in: [src/program-escrow-client.ts:14](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/program-escrow-client.ts#L14)

Program escrow state returned by contract read methods.

## Properties

### authorized\_payout\_key

> **authorized\_payout\_key**: `string`

Defined in: [src/program-escrow-client.ts:22](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/program-escrow-client.ts#L22)

Stellar account authorized to execute payouts.

***

### payout\_history

> **payout\_history**: [`PayoutRecord`](PayoutRecord.md)[]

Defined in: [src/program-escrow-client.ts:24](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/program-escrow-client.ts#L24)

Historical payout records for the program.

***

### program\_id

> **program\_id**: `string`

Defined in: [src/program-escrow-client.ts:16](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/program-escrow-client.ts#L16)

Application-level program identifier.

***

### remaining\_balance

> **remaining\_balance**: `bigint`

Defined in: [src/program-escrow-client.ts:20](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/program-escrow-client.ts#L20)

Remaining spendable balance in the program escrow.

***

### token\_address

> **token\_address**: `string`

Defined in: [src/program-escrow-client.ts:26](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/program-escrow-client.ts#L26)

Token contract address used by the program escrow.

***

### total\_funds

> **total\_funds**: `bigint`

Defined in: [src/program-escrow-client.ts:18](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/program-escrow-client.ts#L18)

Total funds deposited into the program escrow.
