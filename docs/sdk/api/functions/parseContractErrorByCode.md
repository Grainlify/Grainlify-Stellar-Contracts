[**@grainlify/contracts-sdk**](../README.md)

***

[@grainlify/contracts-sdk](../README.md) / parseContractErrorByCode

# Function: parseContractErrorByCode()

> **parseContractErrorByCode**(`numericCode`, `contract`): [`ContractError`](../classes/ContractError.md)

Defined in: [src/errors.ts:268](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/errors.ts#L268)

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
