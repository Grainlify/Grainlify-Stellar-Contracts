[**@grainlify/contracts-sdk**](../README.md)

***

[@grainlify/contracts-sdk](../README.md) / parseContractErrorByCode

# Function: parseContractErrorByCode()

> **parseContractErrorByCode**(`numericCode`, `contract`): [`ContractError`](../classes/ContractError.md)

Defined in: [src/errors.ts:268](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L268)

Resolve a numeric on-chain error code to a typed ContractError.

## Parameters

### numericCode

`number`

The u32 error discriminant from the contract.

### contract

`"program_escrow"` \| `"bounty_escrow"` \| `"governance"` \| `"circuit_breaker"`

Which contract produced the error — determines the look-up table.

## Returns

[`ContractError`](../classes/ContractError.md)
