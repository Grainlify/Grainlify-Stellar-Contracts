[**@grainlify/contracts-sdk**](../README.md)

***

[@grainlify/contracts-sdk](../README.md) / AdminConfigSnapshot

# Interface: AdminConfigSnapshot

Defined in: [src/bounty-escrow-client.ts:217](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L217)

A stable configuration snapshot for audit views.

## Properties

### admin

> **admin**: `string`

Defined in: [src/bounty-escrow-client.ts:221](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L221)

Contract admin address.

***

### claim\_window

> **claim\_window**: `bigint`

Defined in: [src/bounty-escrow-client.ts:233](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L233)

Time window in seconds during which claims are allowed.

***

### fee\_config

> **fee\_config**: [`FeeConfig`](FeeConfig.md)

Defined in: [src/bounty-escrow-client.ts:225](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L225)

Fee configuration.

***

### governance\_contract?

> `optional` **governance\_contract?**: `string`

Defined in: [src/bounty-escrow-client.ts:229](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L229)

Optional governance contract address.

***

### has\_amount\_policy

> **has\_amount\_policy**: `boolean`

Defined in: [src/bounty-escrow-client.ts:235](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L235)

Whether an amount policy (min/max limits) is configured.

***

### max\_lock\_amount

> **max\_lock\_amount**: `bigint`

Defined in: [src/bounty-escrow-client.ts:239](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L239)

Maximum allowed lock amount.

***

### min\_governance\_version

> **min\_governance\_version**: `number`

Defined in: [src/bounty-escrow-client.ts:231](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L231)

Minimum required governance version for admin actions.

***

### min\_lock\_amount

> **min\_lock\_amount**: `bigint`

Defined in: [src/bounty-escrow-client.ts:237](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L237)

Minimum allowed lock amount.

***

### pause\_flags

> **pause\_flags**: [`PauseFlags`](PauseFlags.md)

Defined in: [src/bounty-escrow-client.ts:227](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L227)

Pause flags.

***

### token

> **token**: `string`

Defined in: [src/bounty-escrow-client.ts:223](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L223)

Escrow token contract address.

***

### version

> **version**: `number`

Defined in: [src/bounty-escrow-client.ts:219](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L219)

Schema version for this snapshot.
