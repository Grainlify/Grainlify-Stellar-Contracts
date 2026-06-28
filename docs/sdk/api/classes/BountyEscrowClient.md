[**@grainlify/contracts-sdk**](../README.md)

***

[@grainlify/contracts-sdk](../README.md) / BountyEscrowClient

# Class: BountyEscrowClient

Defined in: [src/bounty-escrow-client.ts:245](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L245)

Client for interacting with the BountyEscrow Soroban contract

## Constructors

### Constructor

> **new BountyEscrowClient**(`config`): `BountyEscrowClient`

Defined in: [src/bounty-escrow-client.ts:253](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L253)

Create a client bound to one BountyEscrow contract and Soroban RPC endpoint.

#### Parameters

##### config

[`BountyEscrowConfig`](../interfaces/BountyEscrowConfig.md)

#### Returns

`BountyEscrowClient`

## Methods

### approveLargeRelease()

> **approveLargeRelease**(`bountyId`, `contributor`, `approver`, `sourceKeypair`): `Promise`\<`void`\>

Defined in: [src/bounty-escrow-client.ts:1005](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L1005)

Approve a large release using multisig signatures. Signer-only.

#### Parameters

##### bountyId

`bigint`

The application-level bounty identifier.

##### contributor

`string`

Stellar address of the contributor.

##### approver

`string`

Stellar address of the signer approving the release.

##### sourceKeypair

`Keypair`

Signing keypair of the approver.

#### Returns

`Promise`\<`void`\>

#### Throws

If addresses are invalid.

#### Throws

If the caller is not authorized or the contract is not initialized.

***

### approveRefund()

> **approveRefund**(`bountyId`, `amount`, `recipient`, `mode`, `sourceKeypair`): `Promise`\<`void`\>

Defined in: [src/bounty-escrow-client.ts:351](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L351)

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

Defined in: [src/bounty-escrow-client.ts:388](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L388)

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

Defined in: [src/bounty-escrow-client.ts:451](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L451)

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

Defined in: [src/bounty-escrow-client.ts:477](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L477)

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

Defined in: [src/bounty-escrow-client.ts:437](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L437)

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

Defined in: [src/bounty-escrow-client.ts:423](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L423)

Execute a claim for a bounty

#### Parameters

##### bountyId

`bigint`

##### sourceKeypair

`Keypair`

#### Returns

`Promise`\<`void`\>

***

### getAdminAuditView()

> **getAdminAuditView**(): `Promise`\<[`AdminConfigSnapshot`](../interfaces/AdminConfigSnapshot.md)\>

Defined in: [src/bounty-escrow-client.ts:1219](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L1219)

Retrieve the complete stable administrative config snapshot (audit view).

#### Returns

`Promise`\<[`AdminConfigSnapshot`](../interfaces/AdminConfigSnapshot.md)\>

The administrative config snapshot.

#### Throws

If the contract error occurs.

***

### getAggregateStats()

> **getAggregateStats**(): `Promise`\<[`AggregateStats`](../interfaces/AggregateStats.md)\>

Defined in: [src/bounty-escrow-client.ts:644](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L644)

Get aggregate escrow statistics.

#### Returns

`Promise`\<[`AggregateStats`](../interfaces/AggregateStats.md)\>

***

### getAntiAbuseAdmin()

> **getAntiAbuseAdmin**(): `Promise`\<`string` \| `null`\>

Defined in: [src/bounty-escrow-client.ts:1174](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L1174)

Get the current anti-abuse administrator address, if set.

#### Returns

`Promise`\<`string` \| `null`\>

The anti-abuse administrator address or null.

#### Throws

If the contract error occurs.

***

### getBalance()

> **getBalance**(): `Promise`\<`bigint`\>

Defined in: [src/bounty-escrow-client.ts:524](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L524)

Get the current contract balance

#### Returns

`Promise`\<`bigint`\>

***

### getCircuitBreakerAdmin()

> **getCircuitBreakerAdmin**(): `Promise`\<`string` \| `null`\>

Defined in: [src/bounty-escrow-client.ts:1129](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L1129)

Get the current circuit breaker admin address, if set.

#### Returns

`Promise`\<`string` \| `null`\>

The circuit breaker admin address or null.

#### Throws

If the contract error occurs.

***

### getCircuitBreakerConfig()

> **getCircuitBreakerConfig**(): `Promise`\<[`CircuitBreakerConfig`](../interfaces/CircuitBreakerConfig.md)\>

Defined in: [src/bounty-escrow-client.ts:1144](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L1144)

Get the current circuit breaker configuration.

#### Returns

`Promise`\<[`CircuitBreakerConfig`](../interfaces/CircuitBreakerConfig.md)\>

The circuit breaker configuration.

#### Throws

If the contract error occurs.

***

### getCircuitBreakerStatus()

> **getCircuitBreakerStatus**(): `Promise`\<[`CircuitBreakerStatus`](../interfaces/CircuitBreakerStatus.md)\>

Defined in: [src/bounty-escrow-client.ts:1159](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L1159)

Get the current circuit breaker status.

#### Returns

`Promise`\<[`CircuitBreakerStatus`](../interfaces/CircuitBreakerStatus.md)\>

The circuit breaker status.

#### Throws

If the contract error occurs.

***

### getEscrowCount()

> **getEscrowCount**(): `Promise`\<`number`\>

Defined in: [src/bounty-escrow-client.ts:656](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L656)

Get the total number of indexed escrows.

#### Returns

`Promise`\<`number`\>

***

### getEscrowIdsByStatus()

> **getEscrowIdsByStatus**(`status`, `offset?`, `limit?`): `Promise`\<`bigint`[]\>

Defined in: [src/bounty-escrow-client.ts:668](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L668)

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

Defined in: [src/bounty-escrow-client.ts:500](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L500)

Get information about a specific escrow

#### Parameters

##### bountyId

`bigint`

#### Returns

`Promise`\<[`Escrow`](../interfaces/Escrow.md)\>

***

### getFeeConfig()

> **getFeeConfig**(): `Promise`\<[`FeeConfig`](../interfaces/FeeConfig.md)\>

Defined in: [src/bounty-escrow-client.ts:739](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L739)

Get the current fee configuration

#### Returns

`Promise`\<[`FeeConfig`](../interfaces/FeeConfig.md)\>

***

### getGovernanceContract()

> **getGovernanceContract**(): `Promise`\<`string` \| `null`\>

Defined in: [src/bounty-escrow-client.ts:1189](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L1189)

Get the current governance contract address, if set.

#### Returns

`Promise`\<`string` \| `null`\>

The governance contract address or null.

#### Throws

If the contract error occurs.

***

### getMinGovernanceVersion()

> **getMinGovernanceVersion**(): `Promise`\<`number`\>

Defined in: [src/bounty-escrow-client.ts:1204](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L1204)

Get the minimum required governance version.

#### Returns

`Promise`\<`number`\>

The minimum required governance version number.

#### Throws

If the contract error occurs.

***

### getMultisigConfig()

> **getMultisigConfig**(): `Promise`\<[`MultisigConfig`](../interfaces/MultisigConfig.md)\>

Defined in: [src/bounty-escrow-client.ts:1114](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L1114)

Get the current multisig configuration.

#### Returns

`Promise`\<[`MultisigConfig`](../interfaces/MultisigConfig.md)\>

The multisig configuration.

#### Throws

If the contract error occurs.

***

### getPauseFlags()

> **getPauseFlags**(): `Promise`\<[`PauseFlags`](../interfaces/PauseFlags.md)\>

Defined in: [src/bounty-escrow-client.ts:751](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L751)

Get the current pause flags

#### Returns

`Promise`\<[`PauseFlags`](../interfaces/PauseFlags.md)\>

***

### getPendingClaim()

> **getPendingClaim**(`bountyId`): `Promise`\<[`ClaimRecord`](../interfaces/ClaimRecord.md)\>

Defined in: [src/bounty-escrow-client.ts:512](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L512)

Get the pending claim for a bounty.

#### Parameters

##### bountyId

`bigint`

#### Returns

`Promise`\<[`ClaimRecord`](../interfaces/ClaimRecord.md)\>

***

### getRefundEligibility()

> **getRefundEligibility**(`bountyId`): `Promise`\<[`RefundEligibility`](../interfaces/RefundEligibility.md)\>

Defined in: [src/bounty-escrow-client.ts:698](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L698)

Get refund eligibility and optional approval details for a bounty.

#### Parameters

##### bountyId

`bigint`

#### Returns

`Promise`\<[`RefundEligibility`](../interfaces/RefundEligibility.md)\>

***

### getRefundHistory()

> **getRefundHistory**(`bountyId`): `Promise`\<[`RefundRecord`](../interfaces/RefundRecord.md)[]\>

Defined in: [src/bounty-escrow-client.ts:686](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L686)

Get refund history for a bounty.

#### Parameters

##### bountyId

`bigint`

#### Returns

`Promise`\<[`RefundRecord`](../interfaces/RefundRecord.md)[]\>

***

### init()

> **init**(`adminAddress`, `tokenAddress`, `sourceKeypair`): `Promise`\<`void`\>

Defined in: [src/bounty-escrow-client.ts:270](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L270)

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

Defined in: [src/bounty-escrow-client.ts:288](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L288)

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

Defined in: [src/bounty-escrow-client.ts:330](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L330)

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

Defined in: [src/bounty-escrow-client.ts:617](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L617)

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

Defined in: [src/bounty-escrow-client.ts:554](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L554)

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

Defined in: [src/bounty-escrow-client.ts:576](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L576)

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

Defined in: [src/bounty-escrow-client.ts:598](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L598)

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

Defined in: [src/bounty-escrow-client.ts:536](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L536)

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

Defined in: [src/bounty-escrow-client.ts:718](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L718)

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

Defined in: [src/bounty-escrow-client.ts:374](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L374)

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

Defined in: [src/bounty-escrow-client.ts:313](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L313)

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

### resetCircuit()

> **resetCircuit**(`admin`, `sourceKeypair`): `Promise`\<`void`\>

Defined in: [src/bounty-escrow-client.ts:939](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L939)

Reset the circuit breaker status. Circuit breaker admin only.

#### Parameters

##### admin

`string`

The Stellar address of the circuit breaker admin resetting the circuit.

##### sourceKeypair

`Keypair`

Signing keypair of the circuit breaker admin.

#### Returns

`Promise`\<`void`\>

#### Throws

If the address is invalid.

#### Throws

If the caller is not authorized or the contract is not initialized.

***

### setAmountPolicy()

> **setAmountPolicy**(`caller`, `minAmount`, `maxAmount`, `sourceKeypair`): `Promise`\<`void`\>

Defined in: [src/bounty-escrow-client.ts:1035](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L1035)

Configure the minimum and maximum allowed lock amounts. Admin-only.

#### Parameters

##### caller

`string`

The Stellar address of the administrator making the call.

##### minAmount

`bigint`

Minimum allowed lock amount.

##### maxAmount

`bigint`

Maximum allowed lock amount.

##### sourceKeypair

`Keypair`

Signing keypair of the admin.

#### Returns

`Promise`\<`void`\>

#### Throws

If amounts are invalid.

#### Throws

If the caller is not authorized or the contract is not initialized.

***

### setAntiAbuseAdmin()

> **setAntiAbuseAdmin**(`admin`, `sourceKeypair`): `Promise`\<`void`\>

Defined in: [src/bounty-escrow-client.ts:1068](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L1068)

Set the anti-abuse administrator address. Admin-only.

#### Parameters

##### admin

`string`

The Stellar address of the anti-abuse admin.

##### sourceKeypair

`Keypair`

Signing keypair of the admin.

#### Returns

`Promise`\<`void`\>

#### Throws

If the address is invalid.

#### Throws

If the caller is not authorized or the contract is not initialized.

***

### setCircuitBreakerAdmin()

> **setCircuitBreakerAdmin**(`admin`, `sourceKeypair`): `Promise`\<`void`\>

Defined in: [src/bounty-escrow-client.ts:881](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L881)

Set the circuit breaker admin address. Admin-only.

#### Parameters

##### admin

`string`

The Stellar address of the new circuit breaker admin.

##### sourceKeypair

`Keypair`

Signing keypair of the admin.

#### Returns

`Promise`\<`void`\>

#### Throws

If the address is invalid.

#### Throws

If the caller is not authorized or the contract is not initialized.

***

### setCircuitBreakerConfig()

> **setCircuitBreakerConfig**(`failureThreshold`, `successThreshold`, `maxErrorLog`, `sourceKeypair`): `Promise`\<`void`\>

Defined in: [src/bounty-escrow-client.ts:904](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L904)

Configure the circuit breaker thresholds. Admin-only.

#### Parameters

##### failureThreshold

`number`

Threshold count of errors to open the circuit.

##### successThreshold

`number`

Threshold count of successes to close the circuit in half-open state.

##### maxErrorLog

`number`

Maximum entries in the error log.

##### sourceKeypair

`Keypair`

Signing keypair of the admin.

#### Returns

`Promise`\<`void`\>

#### Throws

If thresholds or log size are invalid.

#### Throws

If the caller is not authorized or the contract is not initialized.

***

### setClaimWindow()

> **setClaimWindow**(`claimWindow`, `sourceKeypair`): `Promise`\<`void`\>

Defined in: [src/bounty-escrow-client.ts:405](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L405)

Set the global claim window in seconds. Admin-only on chain.

#### Parameters

##### claimWindow

`number`

##### sourceKeypair

`Keypair`

#### Returns

`Promise`\<`void`\>

***

### setGovernanceContract()

> **setGovernanceContract**(`governanceAddr`, `sourceKeypair`): `Promise`\<`void`\>

Defined in: [src/bounty-escrow-client.ts:837](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L837)

Set the governance contract address. Admin-only.

#### Parameters

##### governanceAddr

`string`

The Stellar address of the governance contract.

##### sourceKeypair

`Keypair`

Signing keypair of the admin.

#### Returns

`Promise`\<`void`\>

#### Throws

If the address is invalid.

#### Throws

If the caller is not authorized or the contract is not initialized.

***

### setMinGovernanceVersion()

> **setMinGovernanceVersion**(`minVersion`, `sourceKeypair`): `Promise`\<`void`\>

Defined in: [src/bounty-escrow-client.ts:858](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L858)

Set the minimum required governance version. Admin-only.

#### Parameters

##### minVersion

`number`

The minimum version of governance protocol required.

##### sourceKeypair

`Keypair`

Signing keypair of the admin.

#### Returns

`Promise`\<`void`\>

#### Throws

If the version is invalid.

#### Throws

If the caller is not authorized or the contract is not initialized.

***

### setPaused()

> **setPaused**(`lock`, `release`, `refund`, `sourceKeypair`): `Promise`\<`void`\>

Defined in: [src/bounty-escrow-client.ts:812](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L812)

Update operations pause state. Admin-only.

#### Parameters

##### lock

`boolean` \| `null`

Optional pause flag for lock operations.

##### release

`boolean` \| `null`

Optional pause flag for release operations.

##### refund

`boolean` \| `null`

Optional pause flag for refund operations.

##### sourceKeypair

`Keypair`

Signing keypair of the admin.

#### Returns

`Promise`\<`void`\>

#### Throws

If the caller is not authorized or the contract is not initialized.

***

### setWhitelist()

> **setWhitelist**(`whitelistedAddress`, `whitelisted`, `sourceKeypair`): `Promise`\<`void`\>

Defined in: [src/bounty-escrow-client.ts:1090](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L1090)

Add or remove an address to/from the anti-abuse whitelist. Admin-only.

#### Parameters

##### whitelistedAddress

`string`

Stellar address to add or remove.

##### whitelisted

`boolean`

Whether the address should be whitelisted.

##### sourceKeypair

`Keypair`

Signing keypair of the admin.

#### Returns

`Promise`\<`void`\>

#### Throws

If the address is invalid.

#### Throws

If the caller is not authorized or the contract is not initialized.

***

### updateFeeConfig()

> **updateFeeConfig**(`lockFeeRate`, `releaseFeeRate`, `feeRecipient`, `feeEnabled`, `sourceKeypair`): `Promise`\<`void`\>

Defined in: [src/bounty-escrow-client.ts:771](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L771)

Update the contract's fee configuration. Admin-only.

#### Parameters

##### lockFeeRate

`bigint` \| `null`

Optional new lock fee rate in basis points.

##### releaseFeeRate

`bigint` \| `null`

Optional new release fee rate in basis points.

##### feeRecipient

`string` \| `null`

Optional new stellar address of the fee recipient.

##### feeEnabled

`boolean` \| `null`

Optional flag to enable or disable fee collection.

##### sourceKeypair

`Keypair`

Signing keypair of the admin.

#### Returns

`Promise`\<`void`\>

#### Throws

If inputs are invalid.

#### Throws

If the caller is not authorized (unauthorized error) or the contract is not initialized.

***

### updateMultisigConfig()

> **updateMultisigConfig**(`thresholdAmount`, `signers`, `requiredSignatures`, `sourceKeypair`): `Promise`\<`void`\>

Defined in: [src/bounty-escrow-client.ts:962](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L962)

Update the multisig configuration. Admin-only.

#### Parameters

##### thresholdAmount

`bigint`

Threshold release amount above which multisig approval is required.

##### signers

`string`[]

Array of authorized signer Stellar addresses.

##### requiredSignatures

`number`

Count of signatures required for approval.

##### sourceKeypair

`Keypair`

Signing keypair of the admin.

#### Returns

`Promise`\<`void`\>

#### Throws

If thresholds, signers, or signatures are invalid.

#### Throws

If the caller is not authorized or the contract is not initialized.
