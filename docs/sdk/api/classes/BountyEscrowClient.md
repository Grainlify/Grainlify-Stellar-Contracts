[**@grainlify/contracts-sdk**](../README.md)

***

[@grainlify/contracts-sdk](../README.md) / BountyEscrowClient

# Class: BountyEscrowClient

Defined in: [src/bounty-escrow-client.ts:178](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L178)

Client for interacting with the BountyEscrow Soroban contract

## Constructors

### Constructor

> **new BountyEscrowClient**(`config`): `BountyEscrowClient`

Defined in: [src/bounty-escrow-client.ts:186](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L186)

Create a client bound to one BountyEscrow contract and Soroban RPC endpoint.

#### Parameters

##### config

[`BountyEscrowConfig`](../interfaces/BountyEscrowConfig.md)

#### Returns

`BountyEscrowClient`

## Methods

### approveRefund()

> **approveRefund**(`bountyId`, `amount`, `recipient`, `mode`, `sourceKeypair`): `Promise`\<`void`\>

Defined in: [src/bounty-escrow-client.ts:284](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L284)

Approve a refund for a bounty

#### Parameters

##### bountyId

`bigint`

##### amount

`bigint`

##### recipient

`string`

##### mode

[`RefundMode`](../type-aliases/RefundMode.md)

##### sourceKeypair

`Keypair`

#### Returns

`Promise`\<`void`\>

***

### authorizeClaim()

> **authorizeClaim**(`bountyId`, `recipient`, `sourceKeypair`): `Promise`\<`void`\>

Defined in: [src/bounty-escrow-client.ts:321](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L321)

Authorize a claim for a bounty

#### Parameters

##### bountyId

`bigint`

##### recipient

`string`

##### sourceKeypair

`Keypair`

#### Returns

`Promise`\<`void`\>

***

### batchLockFunds()

> **batchLockFunds**(`items`, `sourceKeypair`): `Promise`\<`number`\>

Defined in: [src/bounty-escrow-client.ts:384](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L384)

Batch lock funds for multiple bounties

#### Parameters

##### items

[`LockFundsItem`](../interfaces/LockFundsItem.md)[]

##### sourceKeypair

`Keypair`

#### Returns

`Promise`\<`number`\>

***

### batchReleaseFunds()

> **batchReleaseFunds**(`items`, `sourceKeypair`): `Promise`\<`number`\>

Defined in: [src/bounty-escrow-client.ts:410](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L410)

Batch release funds for multiple bounties

#### Parameters

##### items

[`ReleaseFundsItem`](../interfaces/ReleaseFundsItem.md)[]

##### sourceKeypair

`Keypair`

#### Returns

`Promise`\<`number`\>

***

### cancelPendingClaim()

> **cancelPendingClaim**(`bountyId`, `sourceKeypair`): `Promise`\<`void`\>

Defined in: [src/bounty-escrow-client.ts:370](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L370)

Cancel a pending claim. Admin-only on chain.

#### Parameters

##### bountyId

`bigint`

##### sourceKeypair

`Keypair`

#### Returns

`Promise`\<`void`\>

***

### claim()

> **claim**(`bountyId`, `sourceKeypair`): `Promise`\<`void`\>

Defined in: [src/bounty-escrow-client.ts:356](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L356)

Execute a claim for a bounty

#### Parameters

##### bountyId

`bigint`

##### sourceKeypair

`Keypair`

#### Returns

`Promise`\<`void`\>

***

### getAggregateStats()

> **getAggregateStats**(): `Promise`\<[`AggregateStats`](../interfaces/AggregateStats.md)\>

Defined in: [src/bounty-escrow-client.ts:577](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L577)

Get aggregate escrow statistics.

#### Returns

`Promise`\<[`AggregateStats`](../interfaces/AggregateStats.md)\>

***

### getBalance()

> **getBalance**(): `Promise`\<`bigint`\>

Defined in: [src/bounty-escrow-client.ts:457](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L457)

Get the current contract balance

#### Returns

`Promise`\<`bigint`\>

***

### getEscrowCount()

> **getEscrowCount**(): `Promise`\<`number`\>

Defined in: [src/bounty-escrow-client.ts:589](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L589)

Get the total number of indexed escrows.

#### Returns

`Promise`\<`number`\>

***

### getEscrowIdsByStatus()

> **getEscrowIdsByStatus**(`status`, `offset?`, `limit?`): `Promise`\<`bigint`[]\>

Defined in: [src/bounty-escrow-client.ts:601](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L601)

Get escrow IDs matching a status filter.

#### Parameters

##### status

[`EscrowStatus`](../type-aliases/EscrowStatus.md)

##### offset?

`number` = `0`

##### limit?

`number` = `50`

#### Returns

`Promise`\<`bigint`[]\>

***

### getEscrowInfo()

> **getEscrowInfo**(`bountyId`): `Promise`\<[`Escrow`](../interfaces/Escrow.md)\>

Defined in: [src/bounty-escrow-client.ts:433](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L433)

Get information about a specific escrow

#### Parameters

##### bountyId

`bigint`

#### Returns

`Promise`\<[`Escrow`](../interfaces/Escrow.md)\>

***

### getFeeConfig()

> **getFeeConfig**(): `Promise`\<[`FeeConfig`](../interfaces/FeeConfig.md)\>

Defined in: [src/bounty-escrow-client.ts:672](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L672)

Get the current fee configuration

#### Returns

`Promise`\<[`FeeConfig`](../interfaces/FeeConfig.md)\>

***

### getPauseFlags()

> **getPauseFlags**(): `Promise`\<[`PauseFlags`](../interfaces/PauseFlags.md)\>

Defined in: [src/bounty-escrow-client.ts:684](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L684)

Get the current pause flags

#### Returns

`Promise`\<[`PauseFlags`](../interfaces/PauseFlags.md)\>

***

### getPendingClaim()

> **getPendingClaim**(`bountyId`): `Promise`\<[`ClaimRecord`](../interfaces/ClaimRecord.md)\>

Defined in: [src/bounty-escrow-client.ts:445](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L445)

Get the pending claim for a bounty.

#### Parameters

##### bountyId

`bigint`

#### Returns

`Promise`\<[`ClaimRecord`](../interfaces/ClaimRecord.md)\>

***

### getRefundEligibility()

> **getRefundEligibility**(`bountyId`): `Promise`\<[`RefundEligibility`](../interfaces/RefundEligibility.md)\>

Defined in: [src/bounty-escrow-client.ts:631](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L631)

Get refund eligibility and optional approval details for a bounty.

#### Parameters

##### bountyId

`bigint`

#### Returns

`Promise`\<[`RefundEligibility`](../interfaces/RefundEligibility.md)\>

***

### getRefundHistory()

> **getRefundHistory**(`bountyId`): `Promise`\<[`RefundRecord`](../interfaces/RefundRecord.md)[]\>

Defined in: [src/bounty-escrow-client.ts:619](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L619)

Get refund history for a bounty.

#### Parameters

##### bountyId

`bigint`

#### Returns

`Promise`\<[`RefundRecord`](../interfaces/RefundRecord.md)[]\>

***

### init()

> **init**(`adminAddress`, `tokenAddress`, `sourceKeypair`): `Promise`\<`void`\>

Defined in: [src/bounty-escrow-client.ts:203](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L203)

Initialize the bounty escrow contract

#### Parameters

##### adminAddress

`string`

##### tokenAddress

`string`

##### sourceKeypair

`Keypair`

#### Returns

`Promise`\<`void`\>

***

### lockFunds()

> **lockFunds**(`depositor`, `bountyId`, `amount`, `deadline`, `sourceKeypair`): `Promise`\<`void`\>

Defined in: [src/bounty-escrow-client.ts:221](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L221)

Lock funds into a bounty escrow

#### Parameters

##### depositor

`string`

##### bountyId

`bigint`

##### amount

`bigint`

##### deadline

`number`

##### sourceKeypair

`Keypair`

#### Returns

`Promise`\<`void`\>

***

### partialRelease()

> **partialRelease**(`bountyId`, `contributor`, `amount`, `sourceKeypair`): `Promise`\<`void`\>

Defined in: [src/bounty-escrow-client.ts:263](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L263)

Release partial funds for a bounty to a contributor

#### Parameters

##### bountyId

`bigint`

##### contributor

`string`

##### amount

`bigint`

##### sourceKeypair

`Keypair`

#### Returns

`Promise`\<`void`\>

***

### queryEscrows()

> **queryEscrows**(`filter`, `offset?`, `limit?`): `Promise`\<[`EscrowWithId`](../interfaces/EscrowWithId.md)[]\>

Defined in: [src/bounty-escrow-client.ts:550](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L550)

Query escrows with the composite on-chain filter.

#### Parameters

##### filter

[`EscrowQueryFilter`](../interfaces/EscrowQueryFilter.md)

##### offset?

`number` = `0`

##### limit?

`number` = `50`

#### Returns

`Promise`\<[`EscrowWithId`](../interfaces/EscrowWithId.md)[]\>

***

### queryEscrowsByAmount()

> **queryEscrowsByAmount**(`minAmount`, `maxAmount`, `offset?`, `limit?`): `Promise`\<[`EscrowWithId`](../interfaces/EscrowWithId.md)[]\>

Defined in: [src/bounty-escrow-client.ts:487](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L487)

Query escrows by amount range.

#### Parameters

##### minAmount

`bigint`

##### maxAmount

`bigint`

##### offset?

`number` = `0`

##### limit?

`number` = `50`

#### Returns

`Promise`\<[`EscrowWithId`](../interfaces/EscrowWithId.md)[]\>

***

### queryEscrowsByDeadline()

> **queryEscrowsByDeadline**(`minDeadline`, `maxDeadline`, `offset?`, `limit?`): `Promise`\<[`EscrowWithId`](../interfaces/EscrowWithId.md)[]\>

Defined in: [src/bounty-escrow-client.ts:509](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L509)

Query escrows by deadline range.

#### Parameters

##### minDeadline

`number`

##### maxDeadline

`number`

##### offset?

`number` = `0`

##### limit?

`number` = `50`

#### Returns

`Promise`\<[`EscrowWithId`](../interfaces/EscrowWithId.md)[]\>

***

### queryEscrowsByDepositor()

> **queryEscrowsByDepositor**(`depositor`, `offset?`, `limit?`): `Promise`\<[`EscrowWithId`](../interfaces/EscrowWithId.md)[]\>

Defined in: [src/bounty-escrow-client.ts:531](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L531)

Query escrows by depositor.

#### Parameters

##### depositor

`string`

##### offset?

`number` = `0`

##### limit?

`number` = `50`

#### Returns

`Promise`\<[`EscrowWithId`](../interfaces/EscrowWithId.md)[]\>

***

### queryEscrowsByStatus()

> **queryEscrowsByStatus**(`status`, `offset?`, `limit?`): `Promise`\<[`EscrowWithId`](../interfaces/EscrowWithId.md)[]\>

Defined in: [src/bounty-escrow-client.ts:469](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L469)

Query escrows by status.

#### Parameters

##### status

[`EscrowStatus`](../type-aliases/EscrowStatus.md)

##### offset?

`number` = `0`

##### limit?

`number` = `50`

#### Returns

`Promise`\<[`EscrowWithId`](../interfaces/EscrowWithId.md)[]\>

***

### queryExpiringBounties()

> **queryExpiringBounties**(`maxDeadline`, `offset?`, `limit?`): `Promise`\<`bigint`[]\>

Defined in: [src/bounty-escrow-client.ts:651](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L651)

Query locked or partially refunded bounties whose deadline is at or before maxDeadline.

#### Parameters

##### maxDeadline

`number`

##### offset?

`number` = `0`

##### limit?

`number` = `50`

#### Returns

`Promise`\<`bigint`[]\>

***

### refund()

> **refund**(`bountyId`, `sourceKeypair`): `Promise`\<`void`\>

Defined in: [src/bounty-escrow-client.ts:307](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L307)

Execute a refund for a bounty

#### Parameters

##### bountyId

`bigint`

##### sourceKeypair

`Keypair`

#### Returns

`Promise`\<`void`\>

***

### releaseFunds()

> **releaseFunds**(`bountyId`, `contributor`, `sourceKeypair`): `Promise`\<`void`\>

Defined in: [src/bounty-escrow-client.ts:246](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L246)

Release full funds for a bounty to a contributor

#### Parameters

##### bountyId

`bigint`

##### contributor

`string`

##### sourceKeypair

`Keypair`

#### Returns

`Promise`\<`void`\>

***

### setClaimWindow()

> **setClaimWindow**(`claimWindow`, `sourceKeypair`): `Promise`\<`void`\>

Defined in: [src/bounty-escrow-client.ts:338](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/bounty-escrow-client.ts#L338)

Set the global claim window in seconds. Admin-only on chain.

#### Parameters

##### claimWindow

`number`

##### sourceKeypair

`Keypair`

#### Returns

`Promise`\<`void`\>
