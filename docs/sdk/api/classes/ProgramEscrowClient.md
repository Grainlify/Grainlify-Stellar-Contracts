[**@grainlify/contracts-sdk**](../README.md)

***

[@grainlify/contracts-sdk](../README.md) / ProgramEscrowClient

# Class: ProgramEscrowClient

Defined in: [src/program-escrow-client.ts:56](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/program-escrow-client.ts#L56)

Client for interacting with the ProgramEscrow Soroban contract

## Constructors

### Constructor

> **new ProgramEscrowClient**(`config`): `ProgramEscrowClient`

Defined in: [src/program-escrow-client.ts:64](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/program-escrow-client.ts#L64)

Create a client bound to one ProgramEscrow contract and Soroban RPC endpoint.

#### Parameters

##### config

[`ProgramEscrowConfig`](../interfaces/ProgramEscrowConfig.md)

#### Returns

`ProgramEscrowClient`

## Methods

### batchPayout()

> **batchPayout**(`recipients`, `amounts`, `sourceKeypair`): `Promise`\<[`ProgramData`](../interfaces/ProgramData.md)\>

Defined in: [src/program-escrow-client.ts:135](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/program-escrow-client.ts#L135)

Execute batch payouts to multiple recipients

#### Parameters

##### recipients

`string`[]

##### amounts

`bigint`[]

##### sourceKeypair

`Keypair`

#### Returns

`Promise`\<[`ProgramData`](../interfaces/ProgramData.md)\>

***

### createProgramReleaseSchedule()

> **createProgramReleaseSchedule**(`recipient`, `amount`, `releaseTimestamp`, `sourceKeypair`): `Promise`\<[`ProgramReleaseSchedule`](../interfaces/ProgramReleaseSchedule.md)\>

Defined in: [src/program-escrow-client.ts:229](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/program-escrow-client.ts#L229)

Create a release schedule

#### Parameters

##### recipient

`string`

##### amount

`bigint`

##### releaseTimestamp

`number`

##### sourceKeypair

`Keypair`

#### Returns

`Promise`\<[`ProgramReleaseSchedule`](../interfaces/ProgramReleaseSchedule.md)\>

***

### getProgramInfo()

> **getProgramInfo**(): `Promise`\<[`ProgramData`](../interfaces/ProgramData.md)\>

Defined in: [src/program-escrow-client.ts:205](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/program-escrow-client.ts#L205)

Get program information

#### Returns

`Promise`\<[`ProgramData`](../interfaces/ProgramData.md)\>

***

### getRemainingBalance()

> **getRemainingBalance**(): `Promise`\<`bigint`\>

Defined in: [src/program-escrow-client.ts:217](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/program-escrow-client.ts#L217)

Get remaining balance

#### Returns

`Promise`\<`bigint`\>

***

### initProgram()

> **initProgram**(`programId`, `authorizedPayoutKey`, `tokenAddress`, `sourceKeypair`): `Promise`\<[`ProgramData`](../interfaces/ProgramData.md)\>

Defined in: [src/program-escrow-client.ts:83](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/program-escrow-client.ts#L83)

Initialize a new program escrow

#### Parameters

##### programId

`string`

##### authorizedPayoutKey

`string`

##### tokenAddress

`string`

##### sourceKeypair

`Keypair`

#### Returns

`Promise`\<[`ProgramData`](../interfaces/ProgramData.md)\>

***

### lockProgramFunds()

> **lockProgramFunds**(`from`, `amount`, `sourceKeypair`): `Promise`\<[`ProgramData`](../interfaces/ProgramData.md)\>

Defined in: [src/program-escrow-client.ts:111](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/program-escrow-client.ts#L111)

Lock funds into the program escrow

#### Parameters

##### from

`string`

##### amount

`bigint`

##### sourceKeypair

`Keypair`

#### Returns

`Promise`\<[`ProgramData`](../interfaces/ProgramData.md)\>

***

### singlePayout()

> **singlePayout**(`recipient`, `amount`, `sourceKeypair`): `Promise`\<[`ProgramData`](../interfaces/ProgramData.md)\>

Defined in: [src/program-escrow-client.ts:179](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/program-escrow-client.ts#L179)

Execute a single payout

#### Parameters

##### recipient

`string`

##### amount

`bigint`

##### sourceKeypair

`Keypair`

#### Returns

`Promise`\<[`ProgramData`](../interfaces/ProgramData.md)\>

***

### triggerProgramReleases()

> **triggerProgramReleases**(`sourceKeypair`): `Promise`\<`number`\>

Defined in: [src/program-escrow-client.ts:256](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/program-escrow-client.ts#L256)

Trigger program releases

#### Parameters

##### sourceKeypair

`Keypair`

#### Returns

`Promise`\<`number`\>
