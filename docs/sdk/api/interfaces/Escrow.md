[**@grainlify/contracts-sdk**](../README.md)

***

[@grainlify/contracts-sdk](../README.md) / Escrow

# Interface: Escrow

Defined in: [src/bounty-escrow-client.ts:66](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L66)

Current state for one bounty escrow.

## Properties

### amount

> **amount**: `bigint`

Defined in: [src/bounty-escrow-client.ts:70](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L70)

Original locked amount in the contract token's smallest unit.

***

### deadline

> **deadline**: `number`

Defined in: [src/bounty-escrow-client.ts:76](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L76)

Unix timestamp used by refund eligibility checks.

***

### depositor

> **depositor**: `string`

Defined in: [src/bounty-escrow-client.ts:68](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L68)

Stellar account that deposited the escrow funds.

***

### refund\_history

> **refund\_history**: [`RefundRecord`](RefundRecord.md)[]

Defined in: [src/bounty-escrow-client.ts:78](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L78)

Refund events recorded for this escrow.

***

### remaining\_amount

> **remaining\_amount**: `bigint`

Defined in: [src/bounty-escrow-client.ts:72](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L72)

Remaining escrow balance after releases or partial refunds.

***

### status

> **status**: [`EscrowStatus`](../type-aliases/EscrowStatus.md)

Defined in: [src/bounty-escrow-client.ts:74](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L74)

Current on-chain escrow lifecycle state.
