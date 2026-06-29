[**@grainlify/contracts-sdk**](../README.md)

***

[@grainlify/contracts-sdk](../README.md) / MultisigConfig

# Interface: MultisigConfig

Defined in: [src/bounty-escrow-client.ts:176](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L176)

Configuration for multisig release requirements.

## Properties

### required\_signatures

> **required\_signatures**: `number`

Defined in: [src/bounty-escrow-client.ts:182](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L182)

Minimum number of signers that must approve the release.

***

### signers

> **signers**: `string`[]

Defined in: [src/bounty-escrow-client.ts:180](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L180)

List of authorized signers for multisig releases.

***

### threshold\_amount

> **threshold\_amount**: `bigint`

Defined in: [src/bounty-escrow-client.ts:178](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L178)

Amount above which a release requires multisig approvals.
